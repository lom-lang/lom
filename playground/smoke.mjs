// playground node 冒烟（designs/0019 §4.1）：同一 harness 在 node 跑
// hello / fib / 修复闭环四步 / 深度守卫，与桌面语义逐字比对。
// 二期追加（designs/0024 B-only）：⑥ 自举链段——分发物在位检查 + node 跑
// 四步链 + 基准比对 + quine 字节数断言 272925 + 各步耗时报表。
// 三期追加（designs/0025）：⑦ file 内建段——exists 真/假二值 + read/write/
// append 的映射形态与新建边界断言（append 追加至尾 = 缺陷 B 机器锁）。
// 四期追加（v1.13.0）：⑧ 分享链接段——sharelink.js 编解码纯函数的往返一致性
// （中文/CRLF/空程序/超长跳过四形态 + base64url 字母表纯净性 + 非法输入报错）。
// 用法：node playground/smoke.mjs [lom.wasm 路径]
import { createLomRuntime } from "./harness.js";
import { runSelfbootChain } from "./selfboot.js";
import {
  SHARE_MAX_CHARS,
  encodeShareText,
  decodeShareText,
  shareTextAllowed,
} from "./sharelink.js";
import { readFileSync, existsSync } from "node:fs";
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

// ---- ⑥ 二期自举链段（designs/0024 B-only）：node 跑四步链 + 基准比对 ----
{
  const dist = "playground/dist-selfboot";
  const paths = {
    wasm: dist + "/selfcomp.wasm",
    src: dist + "/self_comp.lom",
    baseline: dist + "/selfboot-baseline.hex",
  };
  const missing = Object.values(paths).filter((p) => !existsSync(p));
  if (missing.length > 0) {
    console.error("缺自举分发物: " + missing.join(", "));
    console.error("先构建: python tools/build_selfboot.py");
    check("selfboot assets present", false, true);
  } else {
    const result = await runSelfbootChain({
      wasmBytes: new Uint8Array(readFileSync(paths.wasm)),
      sourceBytes: new Uint8Array(readFileSync(paths.src)),
      baselineHex: readFileSync(paths.baseline, "utf8"),
    });
    console.log("-- selfboot demo（各步耗时）--");
    for (const s of result.steps) {
      console.log("  STEP" + s.n + "  " + String(s.durationMs).padStart(6) + " ms   " + s.detail);
    }
    console.log("  total  " + String(result.totalMs).padStart(6) + " ms");
    check("selfboot STEP1 compiled bytes", result.step1.compiledBytes, 272925);
    check("selfboot STEP2 decoded bytes", result.step2.bytes, 272925);
    check("selfboot STEP3 compiled bytes", result.step3.compiledBytes, 272925);
    check("selfboot STEP4 quine byte-identical", result.quine.identical, true);
    check("selfboot quine bytes 272925", result.quine.bytes, 272925);
    check("selfboot baseline bytes 272925", result.quine.baselineBytes, 272925);
  }
}

// ---- ⑦ file 内建段（designs/0025）：file 族四内建在 wasm 侧的行为锁 ----
// exists 真/假二值正确（缺陷 A）+ append 追加至尾（缺陷 B）；write/append
// 新建路径 = 虚拟单文件 FS 登记边界 → 结构化 [RUNTIME000] rc 1。
{
  const srcExists = `from file import { file_exists }
from io import { println }
fn main() -> Unit
    println(file_exists("playground.lom"))
    println(file_exists("__no_such_file__.txt"))
end
`;
  const exists = await rt.run({ source: srcExists });
  check("file exists mapped true + missing false", exists.stdout, "true\nfalse\n");
  check("file exists exit 0", exists.exitCode, 0);

  const srcReadOk = `from file import { file_read }
from io import { println }
fn main() -> Unit
    println(file_read("playground.lom"))
end
`;
  const readOk = await rt.run({ source: srcReadOk });
  check("file read mapped stdout starts with source head", readOk.stdout.startsWith("from file import"), true);
  check("file read mapped exit 0", readOk.exitCode, 0);

  const srcReadMissing = `from file import { file_read }
from io import { println }
fn main() -> Unit
    println(file_read("__no_such_file__.txt"))
end
`;
  const readMissing = await rt.run({ source: srcReadMissing });
  check("file read missing rc 1", readMissing.exitCode, 1);
  check("file read missing stderr RUNTIME000", readMissing.stderr.includes("[RUNTIME000]"), true);

  const srcWriteMapped = `from file import { file_write, file_read }
from io import { println }
fn main() -> Unit
    file_write("playground.lom", "OVERWRITTEN")
    println(file_read("playground.lom"))
end
`;
  const writeMapped = await rt.run({ source: srcWriteMapped });
  check("file write mapped + read back", writeMapped.stdout, "OVERWRITTEN\n");
  check("file write mapped exit 0", writeMapped.exitCode, 0);

  const srcWriteNew = `from file import { file_write }
from io import { println }
fn main() -> Unit
    file_write("out_new.txt", "DATA")
end
`;
  const writeNew = await rt.run({ source: srcWriteNew });
  check("file write new rc 1", writeNew.exitCode, 1);
  check("file write new stderr RUNTIME000", writeNew.stderr.includes("[RUNTIME000]"), true);

  const srcAppendMapped = `from file import { file_append, file_read }
from io import { println }
fn main() -> Unit
    file_append("playground.lom", "APPENDED")
    println(file_read("playground.lom"))
end
`;
  const appendMapped = await rt.run({ source: srcAppendMapped });
  // 读回以 stdout 为准（rt.run 的 sourceOut 是写后源）：
  // 追加至尾 = 源文本尾接 "APPENDED" + println 换行（缺陷 B 的机器锁）
  check("file append mapped appends at tail", appendMapped.stdout, srcAppendMapped + "APPENDED\n");
  check("file append mapped exit 0", appendMapped.exitCode, 0);

  const srcAppendNew = `from file import { file_append }
from io import { println }
fn main() -> Unit
    file_append("out_append.txt", "FRESH")
end
`;
  const appendNew = await rt.run({ source: srcAppendNew });
  check("file append new rc 1", appendNew.exitCode, 1);
  check("file append new stderr RUNTIME000", appendNew.stderr.includes("[RUNTIME000]"), true);
}

// ---- ⑧ 分享链接段（v1.13.0）：URL hash 编解码纯函数的往返一致性 ----
// 四形态：中文（多字节 + astral emoji）/ CRLF / 空程序 / 超长跳过（app 闸门函数）。
// 实现在 playground/sharelink.js——浏览器 app.js 与本冒烟同源，CI 即机器锁。
{
  // ① 中文 + emoji（多字节与 astral 面——btoa 会崩的形态正是分享链接的目标场景）
  const zh = 'fn main() -> Unit\n    println("你好，Lom 世界 🌊")\nend\n';
  check("share roundtrip CJK+emoji", decodeShareText(encodeShareText(zh)), zh);
  check("share base64url alphabet clean", /^[A-Za-z0-9_-]*$/.test(encodeShareText(zh)), true);

  // ② CRLF（Windows 形态逐字保留——分享链接不得静默改行尾）
  const crlf = "fn main() -> Unit\r\n    println(1)\r\nend\r\n";
  check("share roundtrip CRLF", decodeShareText(encodeShareText(crlf)), crlf);

  // ③ 空程序（空文本 ↔ 空 hash——app 加载侧以空 hash 判"无分享载荷"）
  check("share empty encodes to empty", encodeShareText(""), "");
  check("share roundtrip empty", decodeShareText(encodeShareText("")), "");

  // ④ 超长跳过（app 侧闸门：>6000 拒写 hash；恰好 6000 放行——边界双侧锁定）
  check("share overlong rejected", shareTextAllowed("x".repeat(SHARE_MAX_CHARS + 1)), false);
  check("share at-limit allowed", shareTextAllowed("x".repeat(SHARE_MAX_CHARS)), true);

  // ⑤ 非法输入解码报错（app 加载侧回落默认示例的依据——不吞错不产出坏文本）
  let threw = false;
  try {
    decodeShareText("含中文的非法载荷");
  } catch {
    threw = true;
  }
  check("share invalid input throws", threw, true);
}

console.log(failures === 0 ? "\nSMOKE: ALL PASS" : `\nSMOKE: ${failures} FAIL`);
process.exit(failures === 0 ? 0 : 1);
