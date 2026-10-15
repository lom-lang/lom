// Lom playground 自举演示 harness（designs/0024 B-only）——L2 自举链的
// 浏览器化/浏览器与 node 双跑实现。
//
// 与 node 蓝本的关系：env 导入契约逐项镜像 tools/selfcomp/run_selfcomp.mjs
// （print_i64 / print_f64 / print_str / ftoa / lom_json_parse /
//  lom_json_stringify / lom_file_read / lom_file_write / lom_file_append /
//  lom_file_exists / lom_env_args，内存布局注释见该文件头）；UTF-8 转换改用
// 全局 TextEncoder/TextDecoder（node 与浏览器同构）替代 node Buffer——对
// 合法 UTF-8 文本两者产出的字节完全一致。文件四件套不再触真实文件系统：
// 虚拟文件模型 = Map(路径 -> Uint8Array)（designs/0024 §2——自举链需要多
// 文件，不复用一期 harness 的 /playground.lom 单文件 backing）。宿主 wasm
// 内建 file_read 不可用的登记面与本链无关：L2 编译器的 env.lom_file_* 是
// 独立通道（设计既定边界）。
//
// hex2wasm 解码桩：tools/hex2wasm.py（第一实现）的 JS 第二实现——同口径
// （剥空白后两 hex 字符一字节；python 端对非 wasm magic 仅警告不失败，
// 此处同样以 magicOk 字段报告而不拦截），node 冒烟对拍锁定。
//
// 双跑形态：本模块是纯 ES 模块，零 node/浏览器专有 API；env 导入以"宿主
// 回调"注入（虚拟 FS Map + argv 余参 + stdout 收集器），浏览器 worker 与
// node 冒烟注入同一套胶水。
//
// 对外 API：
//   hex2wasmDecode(hexText) -> { bytes, hexChars, magicOk }
//   runSelfbootChain(assets, { onStep }) -> result
//     assets = { wasmBytes: Uint8Array, sourceBytes: Uint8Array,
//                baselineHex: string }
//     onStep(n, info) 每步完成即回调（n = 1..4，
//       info = { durationMs, detail }）
//     result = { steps, step1, step2, step3, quine, stdout1, stdout3, totalMs }
//       quine = { identical, bytes, baselineBytes, firstDiff }
//
// 四步链（designs/0024 §1）：
//   STEP1 分发 selfcomp.wasm 编译 self_comp.lom -> hex A（经 env.lom_file_write
//        落虚拟文件后回读）
//   STEP2 JS 版 hex2wasm 解码 hex A -> wasm 字节 B
//   STEP3 以字节 B 新建实例再次编译 self_comp.lom -> hex Q（self-apply）
//   STEP4 hex Q 与预置基准 hex 逐字符比对 -> quine 判定
//   （字节数 = hex 字符数 / 2，与 verify_selfcomp --bootstrap L3 同口径）

const SELFBOOT_LABELS = {
  wasm: "/selfcomp.wasm", // argv[0] 虚拟路径（self_comp 的 main 只消费 argv[1]/argv[2]）
  src: "/self_comp.lom", // 虚拟源文件（预载）
  out: "/selfcomp.out.hex", // 虚拟输出 hex 文件（写后回读）
};

const te = new TextEncoder();
const td = new TextDecoder();

// 浮点格式化：镜像 run_selfcomp.mjs 的 fmtFloat（与宿主 eval/runner/run_wasm.mjs
// 的 fmtFloat 同语义）——浮点整数值补 .0；非有限值映射 inf/-inf/NaN。
const fmtFloat = (v) => {
  if (Number.isNaN(v)) return "NaN";
  if (!Number.isFinite(v)) return v > 0 ? "inf" : "-inf";
  const s = String(v);
  return s.includes(".") ? s : s + ".0";
};

// ---------- hex2wasm 解码桩（与 tools/hex2wasm.py 同口径） ----------

// ASCII hex 字符 -> 值速查表（0-9 / a-f / A-F；其余 -1）
const HEXVAL = (() => {
  const t = new Int8Array(128).fill(-1);
  for (let i = 0; i < 10; i++) t[48 + i] = i;
  for (let i = 0; i < 6; i++) {
    t[65 + i] = 10 + i;
    t[97 + i] = 10 + i;
  }
  return t;
})();

export function hex2wasmDecode(hexText) {
  const compact = String(hexText).replace(/\s+/g, "");
  if (compact.length === 0 || compact.length % 2 !== 0) {
    throw new Error("hex 长度必须为偶数（得到 " + compact.length + " 字符）");
  }
  if (!/^[0-9a-fA-F]+$/.test(compact)) {
    throw new Error("含非 hex 字符");
  }
  const out = new Uint8Array(compact.length / 2);
  for (let i = 0; i < out.length; i++) {
    out[i] = (HEXVAL[compact.charCodeAt(i * 2)] << 4) | HEXVAL[compact.charCodeAt(i * 2 + 1)];
  }
  // python 桩对非 wasm magic 只警告不失败——同口径，以字段报告（STEP3 实例化
  // 会给出真正的失败信息）
  const magicOk = out[0] === 0x00 && out[1] === 0x61 && out[2] === 0x73 && out[3] === 0x6d;
  return { bytes: out, hexChars: compact.length, magicOk };
}

// ---------- env 导入契约（逐项镜像 tools/selfcomp/run_selfcomp.mjs） ----------

// "宿主回调"注入面：host = { files: Map<路径, Uint8Array>, argvRest: string[],
// wasmPath: string }——浏览器 worker 与 node 冒烟构造同语义的 host 共享本实现。
function createEnvContracts(host) {
  let memory = null;
  let instance = null;
  let out = "";

  const rd = () => new DataView(memory.buffer);
  const readStr = (off) => {
    const len = rd().getUint32(off, true);
    return td.decode(new Uint8Array(memory.buffer, off + 4, len));
  };
  const writeStr = (s) => {
    const b = te.encode(s);
    const p = instance.exports.lom_alloc(4 + b.length);
    rd().setUint32(p, b.length, true);
    new Uint8Array(memory.buffer, p + 4, b.length).set(b);
    return p;
  };

  // JS 值 -> JSON 节点树（物化）。run_selfcomp.mjs materializeNode 逐字移植：
  // 数字按 JS 值判定；array 物化为 cons 链（从尾往头）；object 物化为保插入
  // 序 kv 定长数组。节点布局契约见 run_selfcomp.mjs 头注释（本文件不重复）。
  function materializeNode(v) {
    const node = instance.exports.lom_alloc(12);
    if (v === null) {
      rd().setInt32(node, 0, true);
      rd().setBigInt64(node + 4, 0n, true);
      return BigInt(node);
    }
    if (typeof v === "boolean") {
      rd().setInt32(node, 1, true);
      rd().setBigInt64(node + 4, v ? 1n : 0n, true);
      return BigInt(node);
    }
    if (typeof v === "number") {
      if (Number.isInteger(v) && Math.abs(v) <= Number.MAX_SAFE_INTEGER) {
        rd().setInt32(node, 2, true);
        rd().setBigInt64(node + 4, BigInt(v), true);
        return BigInt(node);
      }
      rd().setInt32(node, 3, true);
      rd().setFloat64(node + 4, v, true);
      return BigInt(node);
    }
    if (typeof v === "string") {
      rd().setInt32(node, 4, true);
      rd().setBigInt64(node + 4, BigInt(writeStr(v)), true);
      return BigInt(node);
    }
    if (Array.isArray(v)) {
      let list = 0n; // Nil
      for (let i = v.length - 1; i >= 0; i--) {
        const cell = instance.exports.lom_alloc(16);
        rd().setBigInt64(cell, materializeNode(v[i]), true);
        rd().setBigInt64(cell + 8, list, true);
        list = BigInt(cell);
      }
      rd().setInt32(node, 5, true);
      rd().setBigInt64(node + 4, list, true);
      return BigInt(node);
    }
    // object -> 保插入序 kv 数组（Object.entries 保非整数键插入序——与宿主
    // WASM materialize 的 Record 布局同语义）
    const entries = Object.entries(v);
    const kv = instance.exports.lom_alloc(4 + 16 * entries.length);
    rd().setUint32(kv, entries.length, true);
    entries.forEach(([k, val], i) => {
      rd().setBigInt64(kv + 4 + 16 * i, BigInt(writeStr(k)), true);
      rd().setBigInt64(kv + 4 + 16 * i + 8, materializeNode(val), true);
    });
    rd().setInt32(node, 6, true);
    rd().setBigInt64(node + 4, BigInt(kv), true);
    return BigInt(node);
  }

  // JSON 节点树 -> 序列化串（run_selfcomp.mjs stringifyNode 的移植——
  // escStr/键序/float 经 String(v)（NaN/inf -> null）逐条对齐）
  const escStr = (s) => {
    let o = '"';
    for (const c of s) {
      const cp = c.codePointAt(0);
      if (c === '"') o += '\\"';
      else if (c === "\\") o += "\\\\";
      else if (c === "\n") o += "\\n";
      else if (c === "\r") o += "\\r";
      else if (c === "\t") o += "\\t";
      else if (cp === 8) o += "\\b";
      else if (cp === 12) o += "\\f";
      else if (cp < 0x20) o += "\\u" + cp.toString(16).padStart(4, "0");
      else o += c;
    }
    return o + '"';
  };
  function stringifyNode(np) {
    const p = Number(np);
    const kind = rd().getInt32(p, true);
    switch (kind) {
      case 0: return "null";
      case 1: return rd().getBigInt64(p + 4, true) !== 0n ? "true" : "false";
      case 2: return rd().getBigInt64(p + 4, true).toString();
      case 3: {
        const v = rd().getFloat64(p + 4, true);
        if (Number.isNaN(v) || !Number.isFinite(v)) return "null";
        return String(v);
      }
      case 4: return escStr(readStr(Number(rd().getBigInt64(p + 4, true))));
      case 5: {
        const elems = [];
        let cur = rd().getBigInt64(p + 4, true);
        while (cur !== 0n) {
          const cp = Number(cur);
          elems.push(stringifyNode(rd().getBigInt64(cp, true)));
          cur = rd().getBigInt64(cp + 8, true);
        }
        return "[" + elems.join(",") + "]";
      }
      case 6: {
        const kv = Number(rd().getBigInt64(p + 4, true));
        const n = rd().getUint32(kv, true);
        const parts = [];
        for (let i = 0; i < n; i++) {
          parts.push(escStr(readStr(Number(rd().getBigInt64(kv + 4 + 16 * i, true)))) +
            ":" + stringifyNode(rd().getBigInt64(kv + 4 + 16 * i + 8, true)));
        }
        return "{" + parts.join(",") + "}";
      }
      default: throw new Error("未知 json 节点 kind: " + kind);
    }
  }

  const env = {
    print_i64: (v) => { out += v.toString() + "\n"; },
    print_f64: (v) => { out += fmtFloat(v) + "\n"; },
    print_str: (ptr, len) => {
      out += td.decode(new Uint8Array(memory.buffer, ptr, len));
    },
    ftoa: (v, buf) => {
      const s = fmtFloat(v);
      const b = te.encode(s);
      new Uint8Array(memory.buffer, buf, b.length).set(b);
      return b.length;
    },
    lom_json_parse: (ptr, len) => {
      const text = td.decode(new Uint8Array(memory.buffer, ptr, len));
      const v = JSON.parse(text); // 失败抛异常 -> wasm trap（宿主 WASM 同款）
      return materializeNode(v);
    },
    lom_json_stringify: (np) => BigInt(writeStr(stringifyNode(np))),
    // 文件四件套：真实文件系统 -> 虚拟文件 Map（字节写/读，读侧经 utf8 文本
    // 往返——与 node 蓝本 readFileSync(p,'utf8') 同语义）
    lom_file_read: (pp, pl) => {
      const p = td.decode(new Uint8Array(memory.buffer, pp, pl));
      const f = host.files.get(p);
      if (f === undefined) {
        throw new Error("虚拟文件不存在: " + p); // 蓝本里 readFileSync 的 ENOENT -> 异常 -> trap
      }
      return BigInt(writeStr(td.decode(f)));
    },
    lom_file_write: (pp, pl, cp, cl) => {
      const p = td.decode(new Uint8Array(memory.buffer, pp, pl));
      // 拷贝出 memory 视图（后续 memory 增长不失效）
      host.files.set(p, Uint8Array.from(new Uint8Array(memory.buffer, cp, cl)));
      return 0n; // L2 import 声明无返回（void 语义）——返回值被忽略
    },
    lom_file_append: (pp, pl, cp, cl) => {
      const p = td.decode(new Uint8Array(memory.buffer, pp, pl));
      const c = Uint8Array.from(new Uint8Array(memory.buffer, cp, cl));
      const prev = host.files.get(p);
      const merged = new Uint8Array((prev ? prev.length : 0) + c.length);
      if (prev) merged.set(prev, 0);
      merged.set(c, prev ? prev.length : 0);
      host.files.set(p, merged);
      return 0n;
    },
    lom_file_exists: (pp, pl) => {
      const p = td.decode(new Uint8Array(memory.buffer, pp, pl));
      return host.files.has(p) ? 1 : 0; // i32 导入返回 JS number
    },
    lom_env_args: () => {
      // 镜像 run_selfcomp.mjs：argv = [wasm 路径, ...剩余参数]；第一个裸 "--"
      // 是分隔符，剥掉（对齐宿主 run_wasm.mjs 的 CLI 惯例）
      let rest = host.argvRest.slice();
      const dd = rest.indexOf("--");
      if (dd >= 0) rest = [rest[0], ...rest.slice(dd + 1)];
      const all = [host.wasmPath, ...rest];
      // materializeNode array 分支直抄：writeStr + 16B cell 裸指针 cons、Nil=0
      let list = 0n; // Nil
      for (let i = all.length - 1; i >= 0; i--) {
        const cell = instance.exports.lom_alloc(16);
        rd().setBigInt64(cell, BigInt(writeStr(all[i])), true);
        rd().setBigInt64(cell + 8, list, true);
        list = BigInt(cell);
      }
      return list;
    },
  };

  return {
    env,
    bind(inst) {
      instance = inst;
      memory = inst.exports.memory;
    },
    stdout: () => out,
  };
}

// ---------- 单趟编译：实例化编译器 wasm，编译预载源，产 hex 文本 ----------

async function compileOnce(wasmBytes, sourceBytes) {
  // 每趟独立虚拟文件系统：源预载、输出写后回读（状态零残留，两趟互不污染）
  const files = new Map([[SELFBOOT_LABELS.src, sourceBytes]]);
  const host = {
    files,
    wasmPath: SELFBOOT_LABELS.wasm,
    argvRest: [SELFBOOT_LABELS.src, SELFBOOT_LABELS.out], // -> argv = [wasm, 源, 输出]
  };
  const c = createEnvContracts(host);
  const { instance } = await WebAssembly.instantiate(wasmBytes, { env: c.env });
  c.bind(instance);
  if (!instance.exports.memory || !instance.exports.lom_alloc) {
    throw new Error("编译器 wasm 缺 memory/lom_alloc 导出（非 selfcomp 产物）");
  }
  try {
    instance.exports.main();
  } catch (e) {
    // 镜像 run_selfcomp.mjs：trap 前已执行的 println 不得丢失（挂到异常上
    // 交上层进度/输出；异常原样重抛——保留 instanceof RuntimeError 可判性）
    e.lomStdout = c.stdout();
    throw e;
  }
  const hexBytes = files.get(SELFBOOT_LABELS.out);
  if (hexBytes === undefined) {
    throw new Error("编译器未产出输出 hex: " + SELFBOOT_LABELS.out + "\n" + c.stdout());
  }
  const hexText = td.decode(hexBytes);
  const m = /COMPILED (\d+) bytes/.exec(c.stdout());
  return {
    hexText,
    stdout: c.stdout(),
    compiledBytes: m ? Number(m[1]) : 0,
  };
}

// ---------- 四步链驱动 ----------

export async function runSelfbootChain(assets, { onStep } = {}) {
  const tAll = Date.now();
  const steps = [];
  const emit = (n, entry) => {
    steps.push(entry);
    if (onStep) onStep(n, { durationMs: entry.durationMs, detail: entry.detail });
  };

  // STEP1 自举编译：分发 selfcomp.wasm 编译 self_comp.lom -> hex A
  let t0 = Date.now();
  const s1 = await compileOnce(assets.wasmBytes, assets.sourceBytes);
  emit(1, {
    n: 1, durationMs: Date.now() - t0, compiledBytes: s1.compiledBytes,
    detail: "COMPILED " + s1.compiledBytes + " bytes",
  });

  // STEP2 解码：JS 版 hex2wasm 将 hex A -> wasm 字节 B
  t0 = Date.now();
  const dec = hex2wasmDecode(s1.hexText);
  emit(2, {
    n: 2, durationMs: Date.now() - t0, bytes: dec.bytes.length, magicOk: dec.magicOk,
    detail: dec.bytes.length + " bytes" + (dec.magicOk ? "" : "（非 wasm magic！）"),
  });

  // STEP3 self-apply：以字节 B 新建实例，再次编译 self_comp.lom -> hex Q
  t0 = Date.now();
  const s3 = await compileOnce(dec.bytes, assets.sourceBytes);
  emit(3, {
    n: 3, durationMs: Date.now() - t0, compiledBytes: s3.compiledBytes,
    detail: "COMPILED " + s3.compiledBytes + " bytes",
  });

  // STEP4 quine 验证：hex Q 与预置基准逐字符比对（镜像 verify_selfcomp 的
  // 原始字节比对——不做空白归一；字节数 = 剥空白后字符数 / 2）
  t0 = Date.now();
  const compactLen = (s) => s.replace(/\s+/g, "").length;
  const q = s3.hexText;
  const b = assets.baselineHex;
  let firstDiff = -1;
  if (q.length !== b.length) {
    const n = Math.min(q.length, b.length);
    for (let i = 0; i < n; i++) {
      if (q.charCodeAt(i) !== b.charCodeAt(i)) { firstDiff = i; break; }
    }
    if (firstDiff < 0) firstDiff = n; // 啬前缀 + 长度差
  } else {
    for (let i = 0; i < q.length; i++) {
      if (q.charCodeAt(i) !== b.charCodeAt(i)) { firstDiff = i; break; }
    }
  }
  const identical = firstDiff < 0;
  const bytes = compactLen(q) / 2;
  const quine = {
    identical,
    bytes,
    baselineBytes: compactLen(b) / 2,
    firstDiff: identical ? -1 : firstDiff,
  };
  emit(4, {
    n: 4, durationMs: Date.now() - t0, identical,
    detail: identical
      ? "byte-identical · " + bytes + " bytes"
      : "MISMATCH（首个差异字符位 " + firstDiff + "；Q " + q.length + " vs 基准 " + b.length + " 字符）",
  });

  return {
    steps,
    step1: steps[0],
    step2: steps[1],
    step3: steps[2],
    quine,
    stdout1: s1.stdout,
    stdout3: s3.stdout,
    totalMs: Date.now() - tAll,
  };
}
