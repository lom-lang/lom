// Lom playground WASI harness（designs/0019）——手写 wasi_snapshot_preview1 绑定，
// 零第三方。node（冒烟）与浏览器 Worker 同构。
//
// 对外 API：
//   createLomRuntime(wasmBytes) -> { run({ source, args, env }) }
//     run 每次调用都新建 wasm 实例（状态零残留）：
//     - source: 编辑器内容，预置为虚拟文件 "/playground.lom" 的字节
//     - args:   完整 argv（默认 ["lom", "/playground.lom"] 即运行模式；
//               fix 模式传 ["lom", "fix", "/playground.lom", "--apply"] 等；
//               须至少含虚拟脚本路径——未传或空数组均回落上述默认，R118 防御）
//     - 返回 { stdout, stderr, exitCode, sourceOut, durationMs }
//       其中 sourceOut 是调用结束后的虚拟文件内容——`lom fix --apply`
//       的文件写回语义由此暴露（调用方据此刷新编辑器/展示 diff）。
//
// WASI 面与理由（designs/0019 §1.2 复查修订版）：
//   args/environ/clock_time_get（fix 历史时间戳）/random_get（HashMap 种子）
//   /fd_write（stdout+stderr 捕获）/fd_read+fd_close+fd_seek+fd_fdstat+fd_filestat_get
//   /fd_prestat_get+fd_prestat_dir_name（预开目录 "/" → 仅映射 playground.lom）
//   /path_open（backing 读写字节缓冲）/proc_exit；其余一律 ENOSYS。
//   ".lom/fix-history.jsonl" 等相对路径在映射外 → ENOENT →
//   历史写失败不阻塞 apply（宿主 Phase 4.1.3 既有登记行为）。

const ESUCCESS = 0;
const EBADF = 8;
const ENOENT = 44;
const ENOSYS = 52;

const WASI_FILETYPE_UNKNOWN = 0;
const WASI_FILETYPE_REGULAR_FILE = 4;
const WASI_FILETYPE_DIRECTORY = 3;
const OFLAGS_CREAT = 1 << 0;
const OFLAGS_TRUNC = 1 << 3;
const ALL_RIGHTS = 0xffffffffffffffffn;

// ------- WASI 实例（一次 run = 一个实例 + 一套虚拟文件） -------

function createWasi(args, env) {
  // 虚拟文件系统：预开目录 "/"（fd 3）下唯一文件 playground.lom
  let backing = new Uint8Array(0);
  const openFds = new Map(); // fd -> { offset: Number(bigint-safe) }
  let nextFd = 4;

  const mem = () => apiMemory.buffer;
  const dv = () => new DataView(mem());

  const decoder = new TextDecoder();
  const encoder = new TextEncoder();

  let outBufs = { 1: [], 2: [] }; // fd 1 stdout / fd 2 stderr 的字节块

  const wasi = {
    // ---- args / environ ----
    args_sizes_get(argcPtr, bufSizePtr) {
      const dvv = dv();
      dvv.setUint32(argcPtr, args.length, true);
      let total = 0;
      for (const a of args) total += encoder.encode(a).length + 1;
      dvv.setUint32(bufSizePtr, total, true);
      return ESUCCESS;
    },
    args_get(argvPtr, argvBufPtr) {
      const dvv = dv();
      let p = argvBufPtr;
      for (let i = 0; i < args.length; i++) {
        dvv.setUint32(argvPtr + i * 4, p, true);
        const bytes = encoder.encode(args[i]);
        new Uint8Array(mem(), p, bytes.length).set(bytes);
        new Uint8Array(mem(), p + bytes.length, 1)[0] = 0;
        p += bytes.length + 1;
      }
      return ESUCCESS;
    },
    environ_sizes_get(argcPtr, bufSizePtr) {
      const dvv = dv();
      dvv.setUint32(argcPtr, env.length, true);
      let total = 0;
      for (const e of env) total += encoder.encode(e).length + 1;
      dvv.setUint32(bufSizePtr, total, true);
      return ESUCCESS;
    },
    environ_get(environPtr, bufPtr) {
      const dvv = dv();
      let p = bufPtr;
      for (let i = 0; i < env.length; i++) {
        dvv.setUint32(environPtr + i * 4, p, true);
        const bytes = encoder.encode(env[i]);
        new Uint8Array(mem(), p, bytes.length).set(bytes);
        new Uint8Array(mem(), p + bytes.length, 1)[0] = 0;
        p += bytes.length + 1;
      }
      return ESUCCESS;
    },

    // ---- clock / random ----
    clock_time_get(id, precisionPtr, timePtr) {
      const dvv = dv();
      dvv.setBigUint64(timePtr, BigInt(Date.now()) * 1000000n, true);
      return ESUCCESS;
    },
    random_get(bufPtr, len) {
      const view = new Uint8Array(mem(), bufPtr, len);
      crypto.getRandomValues(view);
      return ESUCCESS;
    },

    // ---- fd 基础 ----
    fd_write(fd, iovsPtr, iovsLen, nwrittenPtr) {
      const dvv = dv();
      let total = 0;
      const st = fd > 3 ? openFds.get(fd) : null;
      if (fd !== 1 && fd !== 2 && !st) return EBADF;
      for (let i = 0; i < iovsLen; i++) {
        const bufPtr = dvv.getUint32(iovsPtr + i * 8, true);
        const bufLen = dvv.getUint32(iovsPtr + i * 8 + 4, true);
        const chunk = Uint8Array.from(new Uint8Array(mem(), bufPtr, bufLen));
        if (fd === 1 || fd === 2) {
          outBufs[fd].push(chunk);
        } else {
          // 常规文件：按偏移写入，必要时扩容 backing（fix --apply 的写回）
          const end = st.offset + chunk.length;
          if (end > backing.length) {
            const grown = new Uint8Array(end);
            grown.set(backing);
            backing = grown;
          }
          backing.set(chunk, st.offset);
          st.offset = end;
        }
        total += bufLen;
      }
      dvv.setUint32(nwrittenPtr, total, true);
      return ESUCCESS;
    },
    fd_read(fd, iovsPtr, iovsLen, nreadPtr) {
      if (fd === 0) {
        dv().setUint32(nreadPtr, 0, true); // stdin: 恒 EOF
        return ESUCCESS;
      }
      const st = openFds.get(fd);
      if (!st) return EBADF;
      const dvv = dv();
      let total = 0;
      for (let i = 0; i < iovsLen; i++) {
        const bufPtr = dvv.getUint32(iovsPtr + i * 8, true);
        const bufLen = dvv.getUint32(iovsPtr + i * 8 + 4, true);
        const avail = backing.length - st.offset;
        const n = Math.max(0, Math.min(bufLen, avail));
        new Uint8Array(mem(), bufPtr, n).set(backing.subarray(st.offset, st.offset + n));
        st.offset += n;
        total += n;
        if (n < bufLen) break; // EOF
      }
      dvv.setUint32(nreadPtr, total, true);
      return ESUCCESS;
    },
    fd_close(fd) {
      if (fd <= 2) return ESUCCESS;
      if (!openFds.delete(fd)) return EBADF;
      return ESUCCESS;
    },
    fd_fdstat_get(fd, ptr) {
      if (fd > 3 && !openFds.has(fd)) return EBADF;
      const dvv = dv();
      // filetype u8 + pad3 + flags u16 + pad6 + rights u64 ×2 = 24B
      new Uint8Array(mem(), ptr, 24).fill(0);
      dvv.setUint8(ptr, fd <= 2 ? WASI_FILETYPE_UNKNOWN
          : fd === 3 ? WASI_FILETYPE_DIRECTORY : WASI_FILETYPE_REGULAR_FILE);
      dvv.setBigUint64(ptr + 8, ALL_RIGHTS, true);
      dvv.setBigUint64(ptr + 16, ALL_RIGHTS, true);
      return ESUCCESS;
    },
    fd_seek(fd, offsetBig, whence, newoffsetPtr) {
      const st = openFds.get(fd);
      if (!st) return EBADF;
      const off = Number(BigInt.asIntN(64, offsetBig));
      let base = st.offset;
      if (whence === 1) base = st.offset + off;
      else if (whence === 2) base = backing.length + off;
      else base = off; // 0 = SEEK_SET
      if (base < 0) return 28; // EINVAL
      st.offset = base;
      dv().setBigUint64(newoffsetPtr, BigInt(base), true);
      return ESUCCESS;
    },
    fd_filestat_get(fd, ptr) {
      const st = openFds.get(fd);
      if (!st) return EBADF;
      const dvv = dv();
      // filetype u8+pad7, size u64, mtim u64, ctim u64, ...（wasi_filestat）
      new Uint8Array(mem(), ptr, 64).fill(0);
      dvv.setUint8(ptr, WASI_FILETYPE_REGULAR_FILE);
      dvv.setBigUint64(ptr + 8, BigInt(backing.length), true);
      return ESUCCESS;
    },

    // ---- 预开目录（fd 3 = "/"）----
    fd_prestat_get(fd, ptr) {
      if (fd !== 3) return EBADF;
      const dvv = dv();
      dvv.setUint32(ptr, 0, true); // tag = preopentype_dir
      dvv.setUint32(ptr + 4, 1, true); // pr_name_len = "/" 长度 1
      return ESUCCESS;
    },
    fd_prestat_dir_name(fd, pathPtr, pathLen) {
      if (fd !== 3) return EBADF;
      new Uint8Array(mem(), pathPtr, pathLen)[0] = 0x2f; // "/"
      return ESUCCESS;
    },

    // ---- path_open：唯一真实文件 = playground.lom（读写共享 backing）----
    path_open(dirfd, dirflags, pathPtr, pathLen, oflags, fsRightsBase, fsRightsInheriting, fdflags, openedFdPtr) {
      if (dirfd !== 3) return ENOENT;
      const path = decoder.decode(new Uint8Array(mem(), pathPtr, pathLen));
      if (path !== "playground.lom") return ENOENT;
      if (oflags & OFLAGS_TRUNC) backing = new Uint8Array(0);
      const fd = nextFd++;
      openFds.set(fd, { offset: 0 });
      dv().setUint32(openedFdPtr, fd, true);
      return ESUCCESS;
    },

    proc_exit(rval) {
      throw { __lom_wasi_exit: rval };
    },
  };

  // 其余 syscall 一律 ENOSYS
  const handler = new Proxy(wasi, {
    get(target, prop) {
      if (prop in target) return target[prop];
      return function () {
        return ENOSYS;
      };
    },
  });

  return {
    imports: { wasi_snapshot_preview1: handler },
    setBacking(bytes) {
      backing = bytes;
    },
    getBacking() {
      return backing;
    },
    takeOutput() {
      const out = {
        stdout: decoder.decode(concat(outBufs[1])),
        stderr: decoder.decode(concat(outBufs[2])),
      };
      outBufs = { 1: [], 2: [] };
      return out;
    },
    bindMemory(memory) {
      apiMemory = memory;
    },
  };
}

let apiMemory = null;

function concat(list) {
  let n = 0;
  for (const b of list) n += b.length;
  const out = new Uint8Array(n);
  let p = 0;
  for (const b of list) {
    out.set(b, p);
    p += b.length;
  }
  return out;
}

// ------- 对外运行时 -------

export async function createLomRuntime(wasmBytes) {
  const compiled = await WebAssembly.compile(wasmBytes);
  return {
    async run({ source, args, env = [] }) {
      const t0 = Date.now();
      const wasi = createWasi((args && args.length) ? args : ["lom", "/playground.lom"], env);
      const encoder = new TextEncoder();
      wasi.setBacking(encoder.encode(source));
      // 注意：instantiate 以 Module 为首参时直接解析为 Instance（非 {instance,module}）
      const instance = await WebAssembly.instantiate(compiled, wasi.imports);
      wasi.bindMemory(instance.exports.memory);
      let exitCode = 0;
      try {
        instance.exports._start();
      } catch (e) {
        if (e && e.__lom_wasi_exit !== undefined) {
          exitCode = e.__lom_wasi_exit;
        } else {
          throw e; // 真 trap（如病态嵌套源码触发的 RuntimeError）——交调用方/看门狗
        }
      }
      const out = wasi.takeOutput();
      return {
        ...out,
        exitCode,
        sourceOut: new TextDecoder().decode(wasi.getBacking()),
        durationMs: Date.now() - t0,
      };
    },
  };
}
