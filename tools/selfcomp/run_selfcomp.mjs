// L2 自举编译器产物的运行 harness（self_comp 专用，L2.2/L2.3）
// 契约（self_comp.lom Part X 头注释）：
//   import env.print_i64(i64) / env.print_f64(f64)——untagged 原生值，
//   每次调用输出一行（println 语义：换行隐式）。
//   L2.3 String 批按需追加：env.print_str(i32 ptr, i32 len)（原始字节）、
//   env.ftoa(f64, i32 buf) -> i32（to_display 格式串写 memory，返回字节
//   数——与宿主 run_wasm.mjs 的 lom_ftoa 同源，JS String(v) + fmtFloat）。
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
const fmtFloat = (v) => {
  if (Number.isNaN(v)) return 'NaN';
  if (!Number.isFinite(v)) return v > 0 ? 'inf' : '-inf';
  const s = String(v);
  return s.includes('.') ? s : s + '.0';
};

const { instance } = await WebAssembly.instantiate(bytes, {
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
  },
});
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
