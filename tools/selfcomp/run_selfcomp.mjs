// L2 自举编译器产物的运行 harness（self_comp 专用，L2.2/L2.3）
// 契约（self_comp.lom Part X 头注释）：
//   import env.print_i64(i64) / env.print_f64(f64)——untagged 原生值，
//   每次调用输出一行（println 语义：换行隐式）。
//   L2.3 String 批按需追加：env.print_str(i32 ptr, i32 len)（原始字节）、
//   env.ftoa(f64, i32 buf) -> i32（to_display 格式串写 memory，返回字节
//   数——与宿主 run_wasm.mjs 的 lom_ftoa 同源，JS String(v) + fmtFloat）。
//   L2.3 json 批按需追加（designs/0007 §6.4，与宿主 run_wasm.mjs 的
//   lom_json_parse/stringify 同语义——对拍基准即 JS 语义，数字切分按
//   JS 值判定："30.0" → Int 30，已登记双后端差异不放大）：
//   env.lom_json_parse(i32 ptr, i32 len) -> i64（JSON 专用节点树根指针）
//   env.lom_json_stringify(i64 node) -> i64（st 指针）
//   节点布局（self_comp.lom js helper 头注释同步契约）：
//     [kind:i32][payload 8B]——kind 0 null/1 bool(payload i64 0|1)/
//     2 int(i64)/3 float(f64 槽)/4 str([len u32][bytes] st 指针)/
//     5 array(ls{js} cons 链指针，cell [head 8B js 节点][tail 8B])/
//     6 object(kv 定长数组指针 [n:i32][key_st:i64][val_js:i64]×n 保插入序)
//   物化分配经产物导出的 lom_alloc（宿主 lom_alloc 先例）；parse 失败
//   JS 异常抛出 → wasm trap → rc 1（宿主 WASM 同款）。
// 格式化口径与宿主 eval/runner/run_wasm.mjs 的 fmtFloat 一致：
//   浮点整数值补 .0；非有限值映射 inf/-inf/NaN。
// 用法: node tools/selfcomp/run_selfcomp.mjs <file.wasm>
import fs from 'node:fs';

const path = process.argv[2];
if (!path) {
  console.error('usage: node run_selfcomp.mjs <file.wasm>');
  process.exit(2);
}
const bytes = fs.readFileSync(path);

let out = '';
let memory = null;
let instance = null;
const fmtFloat = (v) => {
  if (Number.isNaN(v)) return 'NaN';
  if (!Number.isFinite(v)) return v > 0 ? 'inf' : '-inf';
  const s = String(v);
  return s.includes('.') ? s : s + '.0';
};

// ===== L2.3 json 批：内存读写辅助（对齐 run_wasm.mjs 的 rd/readStr/writeStr） =====
const rd = () => new DataView(memory.buffer);
const readStr = (off) => {
  const len = rd().getUint32(off, true);
  return Buffer.from(new Uint8Array(memory.buffer, off + 4, len)).toString('utf8');
};
const writeStr = (s) => {
  const b = Buffer.from(s, 'utf8');
  const p = instance.exports.lom_alloc(4 + b.length);
  rd().setUint32(p, b.length, true);
  new Uint8Array(memory.buffer, p + 4, b.length).set(b);
  return p;
};

// JS 值 → JSON 节点树（物化）。数字按 JS 值判定（裁决 3 甲）：数值整且
// 安全 → int 节点，否则 float 节点；array 物化为 ls{js} cons 链（从尾
// 往头 cons——cell [head 8B js 节点裸指针][tail 8B]，Nil=0）；object
// 物化为保插入序 kv 定长数组（Object.entries 保非整数键插入序——与
// 宿主 WASM materialize 的 Record 布局同语义）。
function materializeNode(v) {
  const node = instance.exports.lom_alloc(12);
  if (v === null) {
    rd().setInt32(node, 0, true);
    rd().setBigInt64(node + 4, 0n, true);
    return BigInt(node);
  }
  if (typeof v === 'boolean') {
    rd().setInt32(node, 1, true);
    rd().setBigInt64(node + 4, v ? 1n : 0n, true);
    return BigInt(node);
  }
  if (typeof v === 'number') {
    if (Number.isInteger(v) && Math.abs(v) <= Number.MAX_SAFE_INTEGER) {
      rd().setInt32(node, 2, true);
      rd().setBigInt64(node + 4, BigInt(v), true);
      return BigInt(node);
    }
    rd().setInt32(node, 3, true);
    rd().setFloat64(node + 4, v, true);
    return BigInt(node);
  }
  if (typeof v === 'string') {
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
  // object → 保插入序 kv 数组（不用 Map——键序重排会与宿主 Record
  // 插入序 stringify 分叉，designs/0007 §6.1）
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

// JSON 节点树 → 序列化串（run_wasm.mjs stringifyVal 的节点版移植——
// escStr/键序/float 经 String(v)（NaN/inf → null）逐条对齐）
const escStr = (s) => {
  let o = '"';
  for (const c of s) {
    const cp = c.codePointAt(0);
    if (c === '"') o += '\\"';
    else if (c === '\\') o += '\\\\';
    else if (c === '\n') o += '\\n';
    else if (c === '\r') o += '\\r';
    else if (c === '\t') o += '\\t';
    else if (cp === 8) o += '\\b';
    else if (cp === 12) o += '\\f';
    else if (cp < 0x20) o += '\\u' + cp.toString(16).padStart(4, '0');
    else o += c;
  }
  return o + '"';
};
function stringifyNode(np) {
  const p = Number(np);
  const kind = rd().getInt32(p, true);
  switch (kind) {
    case 0: return 'null';
    case 1: return rd().getBigInt64(p + 4, true) !== 0n ? 'true' : 'false';
    case 2: return rd().getBigInt64(p + 4, true).toString();
    case 3: {
      const v = rd().getFloat64(p + 4, true);
      if (Number.isNaN(v) || !Number.isFinite(v)) return 'null';
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
      return '[' + elems.join(',') + ']';
    }
    case 6: {
      const kv = Number(rd().getBigInt64(p + 4, true));
      const n = rd().getUint32(kv, true);
      const parts = [];
      for (let i = 0; i < n; i++) {
        parts.push(escStr(readStr(Number(rd().getBigInt64(kv + 4 + 16 * i, true)))) +
          ':' + stringifyNode(rd().getBigInt64(kv + 4 + 16 * i + 8, true)));
      }
      return '{' + parts.join(',') + '}';
    }
    default: throw new Error('未知 json 节点 kind: ' + kind);
  }
}

const { instance: inst } = await WebAssembly.instantiate(bytes, {
  env: {
    print_i64: (v) => { out += v.toString() + '\n'; },
    print_f64: (v) => { out += fmtFloat(v) + '\n'; },
    print_str: (ptr, len) => {
      out += Buffer.from(new Uint8Array(memory.buffer, ptr, len)).toString('utf8');
    },
    ftoa: (v, buf) => {
      const s = fmtFloat(v);
      const b = Buffer.from(s, 'utf8');
      new Uint8Array(memory.buffer, buf, b.length).set(b);
      return b.length;
    },
    lom_json_parse: (ptr, len) => {
      const text = Buffer.from(new Uint8Array(memory.buffer, ptr, len)).toString('utf8');
      const v = JSON.parse(text); // 失败抛异常 → wasm trap → rc 1（宿主 WASM 同款）
      return materializeNode(v);
    },
    lom_json_stringify: (np) => BigInt(writeStr(stringifyNode(np))),
  },
});
instance = inst;
memory = instance.exports.memory || null;
try {
  instance.exports.main();
  process.stdout.write(out);
} catch (e) {
  // 对齐宿主 run_wasm.mjs：trap 前已执行的 println 不得丢失。
  process.stdout.write(out);
  console.error('wasm trap: ' + (e && e.message ? e.message : String(e)));
  process.exit(1);
}
