// L2 自举编译器产物的运行 harness（self_comp 专用，L2.2）
// 契约（self_comp.lom Part X 头注释）：
//   import env.print_i64(i64) / env.print_f64(f64)——untagged 原生值，
//   每次调用输出一行（println 语义：换行隐式）。
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
  },
});
instance.exports.main();
process.stdout.write(out);
