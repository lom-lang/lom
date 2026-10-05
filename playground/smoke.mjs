// playground node 冒烟（designs/0019 §4.1）：同一 harness 在 node 跑
// hello / fib / 修复闭环四步 / 深度守卫，与桌面语义逐字比对。
// 用法：node playground/smoke.mjs [lom.wasm 路径]
import { createLomRuntime } from "./harness.js";
import { readFileSync } from "node:fs";
import { execFileSync } from "node:child_process";

const wasmPath = process.argv[2] || "target/wasm32-wasip1/wasm-release/lom.wasm";
const wasmBytes = readFileSync(wasmPath);
const desktopBin = process.platform === "win32" ? "target/release/lom.exe" : "target/release/lom";
const rt = await createLomRuntime(wasmBytes);

let failures = 0;
function check(name, actual, expected) {
  const ok = actual === expected;
  if (!ok) failures++;
  console.log(`${ok ? "PASS" : "FAIL"} ${name}${ok ? "" : `\n  expected: ${JSON.stringify(expected)}\n  actual:   ${JSON.stringify(actual)}`}`);
}

// ---- ① arithmetic.lom：wasm 输出 vs 桌面输出 ----
{
  const src = readFileSync("examples/arithmetic.lom", "utf8");
  const wasm = await rt.run({ source: src });
  const desktop = execFileSync(desktopBin, ["examples/arithmetic.lom"], { encoding: "utf8" });
  check("arithmetic stdout wasm==desktop", wasm.stdout, desktop);
  check("arithmetic exit 0", wasm.exitCode, 0);
}

// ---- ② fib.lom ----
{
  const src = readFileSync("examples/fib.lom", "utf8");
  const wasm = await rt.run({ source: src });
  const desktop = execFileSync(desktopBin, ["examples/fib.lom"], { encoding: "utf8" });
  check("fib stdout wasm==desktop", wasm.stdout, desktop);
}

// ---- ③ 修复闭环四步（README 门面同款）----
{
  const broken = 'fn main() -> Unit\n    println(len("hello"))\nend\n';
  // 3a 运行态：结构化 RUNTIME002 + NAM005（stderr 面）
  const run1 = await rt.run({ source: broken });
  check("fix-loop run stderr has RUNTIME002", run1.stderr.includes("[RUNTIME002] 符号 'len' 未导入"), true);
  // 3b fix --dry-run 计划（stdout 面）
  const plan = await rt.run({ source: broken, args: ["lom", "fix", "/playground.lom", "--dry-run"] });
  check("fix-loop dry-run plans NAM005 high insert", plan.stdout.includes("[NAM005]") && plan.stdout.includes("high"), true);
  // 3c fix --apply：虚拟文件写回（sourceOut 含插入的 import）
  const apply = await rt.run({ source: broken, args: ["lom", "fix", "/playground.lom", "--apply"] });
  check("fix-loop apply rewrites source", apply.sourceOut.startsWith("from string import {len}\n"), true);
  check("fix-loop apply final clean", apply.stdout.includes("0 错误 / 0 警告"), true);
  // 3d 修后运行打印 5
  const run2 = await rt.run({ source: apply.sourceOut });
  check("fix-loop rerun prints 5", run2.stdout, "5\n");
}

// ---- ④ 深度守卫：结构化诊断先于栈 trap（wasm 阈值 300，node 实测物理上限 ~500）----
{
  const deep = "fn r(n: Int) -> Int\n    r(n + 1)\nend\nfn main() -> Unit\n    println(r(0))\nend\n";
  const wasm = await rt.run({ source: deep });
  check("depth guard structured RUNTIME diag", wasm.stderr.includes("RUNTIME") && wasm.stdout === "", true);
  // 合法深度（200 < 300 守卫 < ~500 物理栈）应正常运行
  const fine = "fn r(n: Int) -> Int\n    if n <= 0\n        0\n    else\n        r(n - 1)\n    end\nend\nfn main() -> Unit\n    println(r(200))\nend\n";
  const ok200 = await rt.run({ source: fine });
  check("depth 200 runs fine", ok200.stdout, "0\n");
}

// ---- ⑤ --check 诊断面（诊断走 stdout——"诊断是产品"既有登记口径）----
{
  const bad = "fn main() -> Unit\n    let x: Int = 1.5\n    println(x)\nend\n";
  const wasm = await rt.run({ source: bad, args: ["lom", "/playground.lom", "--check"] });
  check("check TYPE001 warning on stdout", wasm.stdout.includes("TYPE001"), true);
}

console.log(failures === 0 ? "\nSMOKE: ALL PASS" : `\nSMOKE: ${failures} FAIL`);
process.exit(failures === 0 ? 0 : 1);
