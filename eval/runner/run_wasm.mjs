// Lom WASM 冒烟运行器（Phase 7.2）——node eval/runner/run_wasm.mjs <file.wasm>
// 提供 lom 编译产物的宿主导入（env.lom_print_* 一族），运行导出的 main，输出到 stdout。
// i64 参数以 BigInt 到达（WASM-JS 边界规则）；退出码：0 正常，1 trap（对齐解释器运行时错误）。
import fs from 'node:fs';

const path = process.argv[2];
if (!path) {
  console.error('usage: node run_wasm.mjs <file.wasm>');
  process.exit(2);
}
const bytes = fs.readFileSync(path);

let memory = null;
let out = '';
const nl = (flag) => { if (flag !== 0n) out += '\n'; };
// 对齐 interpreter.rs to_display：浮点整数值显示为 x.0（Rust 的 4.0 → "4" → "4.0"）
const fmtFloat = (v) => {
  const s = String(v);
  return s.includes('.') ? s : s + '.0';
};

const imports = {
  env: {
    lom_print_int: (v, nlf) => { out += v.toString(); nl(nlf); },
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
  },
};

try {
  const { instance } = await WebAssembly.instantiate(bytes, imports);
  memory = instance.exports.memory;
  instance.exports.main();
  process.stdout.write(out);
} catch (e) {
  // trap（除零、不可达等）——先吐出已产生的 stdout，再以 1 退出（对齐解释器运行时错误的退出码）
  process.stdout.write(out);
  console.error('wasm trap: ' + (e && e.message ? e.message : String(e)));
  process.exit(1);
}
