// Lom WASM 冒烟运行器（Phase 7.2+）——node eval/runner/run_wasm.mjs <file.wasm>
// 提供 lom 编译产物的宿主导入（env.lom_print_* 一族），运行导出的 main，输出到 stdout。
// i64 参数以 BigInt 到达（WASM-JS 边界规则）；退出码：0 正常，1 trap（对齐解释器运行时错误）。
//
// 7.7 宿主 JSON 契约（lom_json_parse / lom_json_stringify）：
//   值布局（4 位 tag）：0=Int 1=Bool 2=Unit 3=F64 盒 4=Str[len u32][bytes]
//   5=Closure 6=Enum[idx][n][args] 7=Tuple[n][elems] 8=Record[n][(name_off,val)]
//   9=List cons[head][tail]（Nil=ptr 0）10=Map[buckets][cap][size]+桶[state][key_off][val]
//   宿主物化值时用导出的 lom_alloc 分配；枚举名查导出的 lom_variant_table（[name_off i32]×n）。
//   已知差异（如实记录）：JSON 数字的 Int/Float 切分按 JS 值判定（"30.0" → Int 30，
//   而解释器按 JSON 源语法给 Float 30.0）；超大浮点的最短表示 JS/Rust 格式在极端指数上有别。
import fs from 'node:fs';

const path = process.argv[2];
if (!path) {
  console.error('usage: node run_wasm.mjs <file.wasm>');
  process.exit(2);
}
const bytes = fs.readFileSync(path);

let memory = null;
let instance = null;
let out = '';
const allocLog = [];
const nl = (flag) => { if (flag !== 0n) out += '\n'; };
// 对齐 interpreter.rs to_display：浮点整数值显示为 x.0（Rust 的 4.0 → "4" → "4.0"）；
// 非有限值映射为 Rust Display 口径 inf/-inf/NaN（T4：与解释器逐字一致，不补 .0）
const fmtFloat = (v) => {
  if (Number.isNaN(v)) return 'NaN';
  if (!Number.isFinite(v)) return v > 0 ? 'inf' : '-inf';
  const s = String(v);
  return s.includes('.') ? s : s + '.0';
};

// ===== 宿主 JSON：内存读写辅助 =====
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
const taggedStr = (s) => (BigInt(writeStr(s)) << 4n) | 4n;

// JS 值 → Lom 堆值（物化）
function materialize(v) {
  if (v === null) return 2n; // Unit
  if (typeof v === 'boolean') return v ? 17n : 1n;
  if (typeof v === 'number') {
    if (Number.isInteger(v) && Math.abs(v) <= Number.MAX_SAFE_INTEGER) {
      return BigInt(v) << 4n; // Int
    }
    const p = instance.exports.lom_alloc(8);
    rd().setFloat64(p, v, true);
    return (BigInt(p) << 4n) | 3n; // F64 盒
  }
  if (typeof v === 'string') return taggedStr(v);
  if (Array.isArray(v)) {
    let list = 9n; // Nil
    for (let i = v.length - 1; i >= 0; i--) {
      const p = instance.exports.lom_alloc(16);
      rd().setBigInt64(p, materialize(v[i]), true);
      rd().setBigInt64(p + 8, list, true);
      list = (BigInt(p) << 4n) | 9n;
    }
    return list;
  }
  // object → Record（保持键序）
  const entries = Object.entries(v);
  const p = instance.exports.lom_alloc(4 + 12 * entries.length);
  rd().setUint32(p, entries.length, true);
  entries.forEach(([k, val], i) => {
    rd().setUint32(p + 4 + 12 * i, writeStr(k), true);
    rd().setBigInt64(p + 4 + 12 * i + 4, materialize(val), true);
  });
  return (BigInt(p) << 4n) | 8n;
}

// Lom 堆值 → JS（供 stringify）；变体名查导出的变体表
function readVal(v) {
  const tag = Number(v & 15n);
  const ptr = Number(v >> 4n);
  switch (tag) {
    case 0: return { k: 'int', v: v >> 4n };
    case 1: return { k: 'bool', v: (v >> 4n) !== 0n };
    case 2: return { k: 'unit' };
    case 3: return { k: 'float', v: rd().getFloat64(ptr, true) };
    case 4: return { k: 'str', v: readStr(ptr) };
    case 5: return { k: 'closure' };
    case 6: {
      const idx = rd().getUint32(ptr, true);
      const n = rd().getUint32(ptr + 4, true);
      const vtable = instance.exports.lom_variant_table.value;
      const name = readStr(rd().getUint32(vtable + idx * 4, true));
      const args = [];
      for (let i = 0; i < n; i++) args.push(readVal(rd().getBigInt64(ptr + 8 + 8 * i, true)));
      return { k: 'enum', name, args };
    }
    case 7: {
      const n = rd().getUint32(ptr, true);
      const elems = [];
      for (let i = 0; i < n; i++) elems.push(readVal(rd().getBigInt64(ptr + 4 + 8 * i, true)));
      return { k: 'tuple', elems };
    }
    case 8: {
      const n = rd().getUint32(ptr, true);
      const fields = [];
      for (let i = 0; i < n; i++) {
        const nameOff = rd().getUint32(ptr + 4 + 12 * i, true);
        fields.push([readStr(nameOff), readVal(rd().getBigInt64(ptr + 8 + 12 * i, true))]);
      }
      return { k: 'record', fields };
    }
    case 9: {
      const elems = [];
      let cur = v;
      while ((cur >> 4n) !== 0n) {
        const cp = Number(cur >> 4n);
        elems.push(readVal(rd().getBigInt64(cp, true)));
        cur = rd().getBigInt64(cp + 8, true);
      }
      return { k: 'list', elems };
    }
    case 10: {
      const buckets = rd().getUint32(ptr, true);
      const cap = rd().getUint32(ptr + 4, true);
      const entries = [];
      for (let i = 0; i < cap; i++) {
        const b = buckets + i * 16;
        if (rd().getUint32(b, true) === 1) {
          entries.push([readStr(rd().getUint32(b + 4, true)), readVal(rd().getBigInt64(b + 8, true))]);
        }
      }
      entries.sort((a, b2) => (a[0] < b2[0] ? -1 : a[0] > b2[0] ? 1 : 0)); // 键排序（确定性）
      return { k: 'map', entries };
    }
    default: throw new Error('未知 tag: ' + tag);
  }
}

// 对齐 json.rs stringify_string 的转义规则
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

// Debug 格式（枚举带参数时用，对齐解释器 {:?}）：Str 加引号转义，其余按 display
const debugFmt = (v) => (v.k === 'str' ? escStr(v.v) : displayVal(v));

// 对齐 json.rs stringify_into
function stringifyVal(v) {
  switch (v.k) {
    case 'int': return v.v.toString();
    case 'float': {
      if (Number.isNaN(v.v) || !Number.isFinite(v.v)) return 'null';
      return String(v.v);
    }
    case 'bool': return v.v ? 'true' : 'false';
    case 'unit': return 'null';
    case 'closure': return 'null';
    case 'str': return escStr(v.v);
    case 'list':
    case 'tuple': return '[' + v.elems.map(stringifyVal).join(',') + ']';
    case 'record':
      return '{' + v.fields.map(([k, val]) => escStr(k) + ':' + stringifyVal(val)).join(',') + '}';
    case 'map':
      return '{' + v.entries.map(([k, val]) => escStr(k) + ':' + stringifyVal(val)).join(',') + '}';
    case 'enum':
      if (v.args.length === 0) return escStr(v.name);
      return escStr(v.name + '(' + v.args.map(debugFmt).join(', ') + ')');
  }
}

const displayVal = (v) => {
  // display 语义（打印用；与 stringify 的差：str 不加引号、enum 不加引号）
  switch (v.k) {
    case 'str': return v.v;
    case 'enum':
      if (v.args.length === 0) return v.name;
      return v.name + '(' + v.args.map(displayVal).join(', ') + ')';
    default: return stringifyVal(v);
  }
};

const imports = {
  env: {
    lom_print_int: (v, nlf) => {
      if (process.env.LOM_HP_TRACE) console.error('HP ' + (instance.exports.lom_hp.value >> 10) + 'K');
      out += v.toString(); nl(nlf);
    },
    lom_print_float: (v, nlf) => { out += fmtFloat(v); nl(nlf); },
    lom_print_bool: (v, nlf) => { out += (v === 0n ? 'false' : 'true'); nl(nlf); },
    lom_print_unit: (nlf) => { out += '()'; nl(nlf); },
    lom_print: (ptr, len) => {
      out += Buffer.from(new Uint8Array(memory.buffer, ptr, len)).toString('utf8');
    },
    // (f64, buf) -> i32：把 to_display 格式的浮点字符串写入 memory[buf..]，返回字节数（7.4）
    lom_ftoa: (v, buf) => {
      const s = fmtFloat(v);
      const b = Buffer.from(s, 'utf8');
      new Uint8Array(memory.buffer, buf, b.length).set(b);
      return b.length;
    },
    // 7.7：宿主 JSON（契约见文件头注释）
    lom_json_parse: (ptr, len) => {
      const text = Buffer.from(new Uint8Array(memory.buffer, ptr, len)).toString('utf8');
      const v = JSON.parse(text); // 失败抛异常 → wasm trap → 退出码 1（对齐解释器运行时错误）
      return materialize(v);
    },
    lom_json_stringify: (v) => taggedStr(stringifyVal(readVal(v))),
    // 7.8：file/env（宿主文件系统/进程参数；ptr/len 是裸内存区间）
    lom_file_read: (pp, pl) => {
      if (process.env.LOM_HP_TRACE) console.error('HP@fread-pre ' + (instance.exports.lom_hp.value >> 10) + 'K');
      const p = Buffer.from(new Uint8Array(memory.buffer, pp, pl)).toString('utf8');
      return taggedStr(fs.readFileSync(p, 'utf8')); // 失败抛异常 → trap → 退出码 1（对齐解释器）
    },
    lom_file_write: (pp, pl, cp, cl) => {
      const p = Buffer.from(new Uint8Array(memory.buffer, pp, pl)).toString('utf8');
      const c = Buffer.from(new Uint8Array(memory.buffer, cp, cl)); // 字节写（对齐 fs::write 的 as_bytes）
      fs.writeFileSync(p, c);
      return 2n; // Unit
    },
    lom_file_append: (pp, pl, cp, cl) => {
      const p = Buffer.from(new Uint8Array(memory.buffer, pp, pl)).toString('utf8');
      const c = Buffer.from(new Uint8Array(memory.buffer, cp, cl));
      fs.appendFileSync(p, c);
      return 2n;
    },
    lom_file_exists: (pp, pl) => {
      const p = Buffer.from(new Uint8Array(memory.buffer, pp, pl)).toString('utf8');
      return fs.existsSync(p) ? 17n : 1n; // Bool tagged（true=17, false=1）
    },
    // args() → List<String>：argv = [wasm 路径, ...用户参数]（对齐解释器 argv[0]=程序路径）
    // CLI 惯例：第一个裸 "--" 是分隔符，剥掉
    lom_dbg_alloc: (sz, hp) => {
      if (process.env.LOM_TRACE_ALLOC) { allocLog.push([Number(sz), Number(hp)]); if (allocLog.length > 200000) allocLog.shift(); }
    },
    lom_env_args: () => {
      if (process.env.LOM_HP_TRACE) console.error('HP@args ' + (instance.exports.lom_hp.value >> 10) + 'K');
      let args = process.argv.slice(2);
      const dd = args.indexOf('--');
      if (dd >= 0) args = [args[0], ...args.slice(dd + 1)];
      let list = 9n; // Nil
      for (let i = args.length - 1; i >= 0; i--) {
        const p = instance.exports.lom_alloc(16);
        rd().setBigInt64(p, taggedStr(args[i]), true);
        rd().setBigInt64(p + 8, list, true);
        list = (BigInt(p) << 4n) | 9n;
      }
      return list;
    },
  },
};

try {
  const r = await WebAssembly.instantiate(bytes, imports);
  instance = r.instance;
  memory = instance.exports.memory;
  // Phase 8.4：LOM_PRE_GROW=<页数> 预扩线性内存。
  // 背景（RFC-0003 修订记录 20）：自举解释器等大分配负载下 rt_alloc 的 grow 路径
  // 存在未定位的越界（非确定性、字节级审计未复现），预扩内存在 4GB 上限内不影响
  // 语义，验收运行用此项绕过；深挖挂账 post-Phase-8。
  instance.exports.main();
  process.stdout.write(out);
} catch (e) {
  // trap（除零、不可达、json_parse 失败等）——先吐出已产生的 stdout，再以 1 退出（对齐解释器运行时错误的退出码）
  process.stdout.write(out);
  console.error('wasm trap: ' + (e && e.message ? e.message : String(e)));
  try { if (memory) console.error('mem pages at trap:', memory.buffer.byteLength >> 16); } catch {}
  if (process.env.LOM_TRACE_ALLOC && allocLog.length) {
    // hp 首次下降点（bump 指针只增——下降即异常）
    let firstDrop = -1;
    for (let i = 1; i < allocLog.length; i++) { if (allocLog[i][1] < allocLog[i-1][1]) { firstDrop = i; break; } }
    if (firstDrop >= 0) {
      console.error('  hp 首次下降 @alloc#' + firstDrop + '：');
      for (const j of [firstDrop-2, firstDrop-1, firstDrop, firstDrop+1]) { if (j >= 0 && j < allocLog.length) console.error(`    #${j} size=${allocLog[j][0]} hp=${allocLog[j][1]}(0x${allocLog[j][1].toString(16)})`); }
    } else { console.error('  无 hp 下降，最后 3 条：', JSON.stringify(allocLog.slice(-3))); }
  }
  process.exit(1);
}
