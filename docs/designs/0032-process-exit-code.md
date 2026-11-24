# 进程退出码实施批：RFC-0005 取乙（designs/0032）

- **文档**：docs/designs/0032-process-exit-code.md
- **性质**：动工前置设计（MAINTENANCE §2.1 第 2 条——涉发射层与宿主工具面、含语义选边，属 L0）。
- **基线**：HEAD `c7ad1b2` / v1.17.0；在本波次的 patch 批合并之后实施。
- **状态**：**已裁，待实施**。用户 2026-11-24 裁决（原话）："同意改。待裁事项全部做，可并行，也可顺序，按合理的方式完成。"——按已呈报的建议取 [RFC-0005](../rfc/0005-process-exit-code.md) 的乙：`fn main() -> Int` 的返回值成为进程退出码；其余选边由规划者代定（§2 留痕，用户可否决）。
- **供料来源**：只读供料报告 `target/probes/wave3/s3_exitcode/report.md`（A–H 八节）。**规划者亲核**：`interpreter.rs:1182-1186` 的返回值丢弃、`main.rs:406-421` 的出口、`run_wasm.mjs:280` 的丢弃调用——逐一打开核对；AOT 产物的带标签返回值由规划者此前亲手实测（探针 `target/probes/spec_doc_batch/planner/planner_aot_ret.mjs`）。

## 1. 语义（RFC-0005 取乙）

- `fn main() -> Int`：尾表达式或 `return n` 的值即进程退出码；只接受 0..255，超出范围或返回值不是 Int 按运行时错误处理（rc 1，落既有码 RUNTIME000——不新增码）。
- `fn main() -> Unit` 或无返回注解：退出码恒为 0（与今天相同），即使函数体最后一句是有值的表达式。
- 运行时错误仍以 1 退出。没有 main 的程序以 1 退出（R245，patch 批先行）。
- 语法、关键字、内建数、诊断码表均不变。

## 2. 规划者代定的选边（留痕，用户可否决）

| # | 选边 | 取定 | 理由 |
|---|---|---|---|
| 1 | AOT 产物如何携带"main 的声明类型" | **代码生成侧解决**：声明非 Int 的 main 收尾一律返回带标签的 Unit 值；声明 Int 的 main 在收尾发射标签检查，不是 Int 即 `unreachable`（陷阱 → 宿主 rc 1） | 产物今天不带声明类型（`-> Unit` 的 main 尾表达式为 5 时同样返回 80），宿主分不出来；不加 custom section 或导出 global（会打破"产物不带 custom section"的既有断言与规范声明） |
| 2 | 0..255 的范围校验放哪 | **宿主脚本**（AOT 与 L2 通道）；解释器侧在 `run()` 内校验 | AOT 没有诊断信封，超范围时 stderr 文本与解释器不同——性质同 §11f 既有的"诊断文本差异" |
| 3 | L2 通道怎么办 | **只改 L2 宿主脚本的映射**（`undefined` → 0、整数 → 退出码），**不动 `self_comp.lom`**；规范登记"L2 通道对 `-> String` 等非 Int 的 main 不保证" | 动自举编译器会改变 quine 字节数（272925 是已公布数字，一整串锚随动）；L2 产物是不带标签的，`-> String` 的 main 返回裸指针，范围校验拦不住——如实登记 |
| 4 | 自举解释器（`self_interp.lom`）是否透出被解释程序的退出码 | **本批不动** | 改它同样动自举源码；登记为后续候选 |
| 5 | playground 的显示 | **本批不改 UI** | 显示已支持非零 exitCode（只是与运行时错误同为 err 态）；退出码透传即生效；区分显示登记为后续候选 |
| 6 | `lom fix --run` 如何归类自定义退出码 | **不算失败**：`outcome: "pass"`，并新增一个只在 `-> Int` 时出现的可选字段（置于 `budget_ms` 之后）；超范围 → `outcome: "error"` | lom-fix/v1 append-only；现有字节断言零改动 |
| 7 | 差分对拍（`tools/diff_test.py`） | **本批不改** | 要给退出码加差分覆盖须新增 `-> Int` 模板并同步绝对断言，登记为后续候选 |
| 8 | 评测 prompt 是否写入新规则 | **不改** | 改了就改变评测面；prompt 里 main 模板恒为 `-> Unit`，已发表数字可比 |
| 9 | 实施前的采样核对（RFC 未决问题 3） | **未做，待用户授权** | 要调用模型——费用与凭据属 L0（不可预授权），规划者不自行发起。零成本证据已足：普查 8137 份存档候选、555 个被跟踪文件、138 道参考解，main 声明与实际返回不符的为 0 |
| 10 | 版本 | **minor（v1.18.0）** | 语义新增；按 MAINTENANCE §2.2.1 事件 1，本波次累计的 patch 级修复随该 minor 一并发布 |

## 3. 改动点（文件 / 函数级；行号为基线实况，细节见供料报告 G1）

| # | 层 | 位置 | 改什么 |
|---|---|---|---|
| 1 | 解释器收尾 | `src/interpreter.rs:1182-1186`（`run()`） | 接住 `exec_block` 的值；仅当 main 声明为 `-> Int` 时取 Int 值，0..=255 存入新字段（`Option<i64>` + 访问器；`run()` 入口清空，先例是 `err_site`）；超范围或值不是 Int → `Err(RuntimeError::Msg(..))`。`run()` 签名不变 |
| 2 | CLI 出口 | `src/main.rs:406-421` | `run_result` 为 Ok 且退出码非 0 时 `process::exit(code)`，放在 `trace_finish()` **之后**；0 保持自然返回 |
| 3 | AOT 编译器 | `src/wasm_codegen.rs`（main 的收尾发射） | 见 §2 第 1 项。不加 section |
| 4 | AOT 宿主 | `eval/runner/run_wasm.mjs:280`；同构的 `tools/wasm_debug/dbg_run.mjs:315` | 标签 0 → 退出码；标签 2 → 正常结束；其它标签 → stderr 说明 + exit 1；范围校验在此 |
| 5 | L2 宿主 | `tools/selfcomp/run_selfcomp.mjs:244`、`playground/selfboot.js:317` | `undefined` → 0；整数 → 退出码（含范围校验） |
| 6 | `fix --run` | `src/fix_run.rs`（`WorkerPayload` / `run_program` / `classify` / `RunReport` / `run_fields_json` / `run_to_human`） | 见 §2 第 6 项 |
| 7 | 文档示例 gate | `tools/spec_examples_check.py:244-250` | 正例实跑要求 rc 0——文档里的退出码正例一律以 0 结尾；确需演示非零时用既有的 `spec-check: skip` 标记并注明原因 |

不动：`examples/selfhost/self_comp.lom`、`examples/selfhost/self_interp.lom`、`tools/diff_gen.py`、`tools/diff_test.py`、`eval/prompts/*`、`playground/app.js`。

## 4. 测试

- 解释器单测：`-> Int` 尾 5 → 退出码 5；256 与 -1 → `Err`；`-> Int` 返回 Unit / String → `Err`；`-> Unit` 尾 5、无注解尾 5、`-> Unit` + `return 5` → 无退出码；同一实例连续 `run` 不残留。
- 进程级（新集成测试文件）：尾 5 → rc 5；`return 7` → rc 7；256 → rc 1 且 stderr 含 RUNTIME000；`-> Unit` 尾 5 与无注解尾 5 → rc 0；`--trace` + 超范围时 trace 文件仍有 summary 行。
- AOT：`src/wasm_codegen.rs` 单测（`-> Unit` 尾 5 的产物 `main()` 返回带标签的 Unit）；`run_wasm.mjs` 宿主链路的 rc（尾 3 → 3、256 → 1、`-> Unit` 尾 5 → 0）。
- L2 宿主映射：编译一个 `-> Int` 的 main 经 `run_selfcomp.mjs` 运行，rc 与解释器相同——放在新集成测试里，**不往 `verify_selfcomp` 的用例集加题**（避免改动 368 这个已公布计数）。
- `tests/d0028_fix_run.rs`：`-> Int` 尾 3 → `outcome: "pass"` + 新字段；256 → `outcome: "error"`。
- playground：`playground/smoke.mjs` 加一条"`-> Int` 尾 3 → exitCode 3"（断言计数随批同步）。

## 5. 验收标准（加粗项为规划者亲验）

- **十组门禁**（HANDOVER §2.2）全绿；自举与验证器里现有的 rc 断言全部不受影响（供料已逐项核对：`verify_selfhost.py` 只有两处 rc=0，`verify_selfcomp.py` 是 rc=0 与"两侧 rc 相等"，被执行的 main 全为 `-> Unit`）。
- **存量不变**：quine 272925 逐字节不变；verify_selfcomp 368/368；eval 双后端 138/138；`eval_prompt_check` 41/41。
- **普查复核**：修后再跑一次 main 返回类型普查，确认被跟踪文件与参考解里没有任何"声明 `-> Int` 且返回非 0"的程序。
- **性能**：收尾处只多一次 `matches!`；A/B 对照按 HANDOVER §3（同目录交错 + 同二进制对照）。

## 6. 文档成对（发版写位；规划者单写）

- LANGUAGE_SPEC §7.5 退出码句（现为两带旧口径，一并补工具失败带 rc 2）、§14 冻结声明与 RFC 的关系说明、§13 一条。
- SPEC_FOR_AI：§1 规则 11 成对改写并写入显眼一句"成功的程序应以 0 结尾，或声明 `-> Unit`"；§11e 的 `--run` 对象字段；§11f 视残余分歧决定是否增一条；§11i 的 `exit(code)` 行替代方案改为 `fn main() -> Int`；头部字符数同步。
- CHANGELOG 该版条目、README 当前版横幅、HANDOVER 版本行、HANDOFF 的"退出码三带"句、符合性映射里的退出码说明、RFC-0005 与本文状态行。

## 7. 风险与如实边界

- **评测可比性**：已发表数字在旧语义下测得。评测 prompt 不改、main 模板恒为 `-> Unit`、存量候选无一例外——数字可比；采样核对待用户授权后可再确认。
- **残余分歧**（如实登记）：AOT 通道超范围时的 stderr 文本与解释器不同；L2 通道对非 Int 的 main 不保证。
- 供料未核：基线 `lom.exe` 与 v1.17.0 提交是否逐字对应；`cargo test` 未跑；凭据现状。
