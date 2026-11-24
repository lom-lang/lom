# 进程退出码实施批：RFC-0005 取乙（designs/0032）

- **文档**：docs/designs/0032-process-exit-code.md
- **性质**：动工前置设计（MAINTENANCE §2.1 第 2 条——涉发射层与宿主工具面、含语义选边，属 L0）。
- **基线**：HEAD `c7ad1b2` / v1.17.0；在本波次的 patch 批合并之后实施。（实际动工基线 `17d37a1`——两个 patch 批已合并；§3 的行号是 `c7ad1b2` 时点的，动工前按现树逐点重核，见 §8。）
- **状态**：**已实施（2026-11-25；minor v1.18.0）**。实施与验收记录见 §8；提交号、门禁与 CI 回填见 docs/TODO.md 的本批交付段。原状态：已裁，待实施。用户 2026-11-24 裁决（原话）："同意改。待裁事项全部做，可并行，也可顺序，按合理的方式完成。"——按已呈报的建议取 [RFC-0005](../rfc/0005-process-exit-code.md) 的乙：`fn main() -> Int` 的返回值成为进程退出码；其余选边由规划者代定（§2 留痕，用户可否决）。
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

## 8. 实施记录（2026-11-25；规划者亲自实施——执行者通道被模型侧 5 小时用量上限（1308）中断）

**动工前重核（基线 `17d37a1`）**：§3 的改动点全部成立，行号按下表刷新；供料报告里有两处事实已被其后的 patch 批改掉，另有两处是供料时还不存在的东西——

| # | 现树位置 | 备注 |
|---|---|---|
| 1 | `src/interpreter.rs` 的 `run()`（main 查找与收尾段） | 供料时写 1182-1186；其后 R242 加了来源文件切换、R245 加了无 main 的 else 分支 |
| 2 | `src/main.rs` 默认运行路径的末尾（`trace_finish()` 与运行时错误分支之后） | 供料时写 406-421；入口路径在工具链批被重构过 |
| 3 | `src/wasm_codegen.rs` 的 `compile_fn`（`$ret` 块闭合之后） | — |
| 4 | `eval/runner/run_wasm.mjs`、`tools/wasm_debug/dbg_run.mjs` | 同构改动 |
| 5 | `tools/selfcomp/run_selfcomp.mjs`、`playground/selfboot.js` | — |
| 6 | `src/fix_run.rs`（`WorkerDone` / `run_program` / `classify` / `RunReport` / `run_fields_json` / `run_to_human`），外加 `src/fix.rs` 里一处直接写死的 `RunReport` 字面量 | 供料没有列 `fix.rs` 那一处 |
| 7 | `tools/spec_examples_check.py` | 不改——文档里的退出码正例一律以 rc 0 收尾 |

- 事实变化 ①：没有 main 的程序现在是 rc 1（R245，designs/0030）——§1 的那一句已由前一批兑现。
- 事实变化 ②：`-> Int` 的函数以 `return n` 收尾不再被误报 TYPE010（R249，designs/0029 §3.7）——规范可以直接教这种写法。
- 新发现 ③：`tests/d0029y_tail_return.rs` 的夹具 F6 是 `fn main() -> Int` + `return 3`（供料早于那一批）——按新语义它的运行 rc 由 0 变成 3。
- 新发现 ④：R250 的测试钉着 13 个 AOT 产物的字节数——main 收尾规范化之后每个恰好多 3 字节。

**与设计的偏差和细化（规划者代定，用户可否决）**：

1. **两处退出码运行时错误带位置**：取 main 的签名 span（`(1:1)`；main 来自依赖包时取包文件）。§3 第 1 项只写了"`Err(RuntimeError::Msg(..))`"。位置是真实坐标，与 v1.17.0"运行时错误带位置"同向；文案避开分类器的关键字分支，落 RUNTIME000。
2. **自定义退出码经 `process::exit` 退出之前先显式刷出 stdout**：`print` 不带换行时内容还留在行缓冲里，自然返回靠运行时的收尾清理，提前退出不等它。桌面与 wasm 引擎各有一条测试锁住。
3. **`RunReport` 的 Rust 字段名取 `program_exit_code`**，与同名的既有方法 `RunReport::exit_code()`（fix 自己的进程退出码，0 / 2）区分；JSON 键仍是 `exit_code`。
4. **`playground/selfboot.js` 的映射是防御性镜像**：那里跑的是编译器自身（selfcomp.wasm，void 的 main），非 `undefined` 的分支在现有分发物下走不到；浏览器里没有进程退出码可以透传，非零即以异常上抛。
5. **不往 SPEC_FOR_AI §11f 增编号条目**（仍是九条）：两条残余分歧写进 §11f 的非编号说明与规则 11。理由是 §11f 的条数被 `tools/diff_gen.py`、`tools/diff_test.py` 的头注释和 doc_audit 锚住，而 §3 明示这两个文件不动；它们也不是差分探针能守护的形态。
6. **AOT 存量产物的字节有变化**（供料的"主观推测：块尾本就是 Unit 值的现存程序应不变"不成立）：声明非 Int 的 main 在 `$ret` 块后多出 `drop` + `i64.const 2`（3 字节），声明 Int 的 main 多一个 i64 局部与一段 tag 检查。行为不变（下面对拍零差异）。
7. **两个既有测试的期望随新语义调整**：R250 的 13 个钉值改写成"修前值 + `MAIN_EPILOGUE_BYTES`"；R249 的 F6 运行 rc 由 0 改 3（静态面断言不动）。

**验收（规划者亲跑；探针与原始数据在 `target/probes/exitcode_impl/`，gitignored）**：

- 测试：单元 686 → 696（+10：解释器 ×6、`fix_run` ×2、`wasm_codegen` ×2），集成 117 → 131（+14：新文件 `tests/d0032_exit_code.rs` ×11、`tests/d0028_fix_run.rs` +3），二十四个 target 逐个单跑配对全部通过；clippy 与 `cargo fmt --check` 零输出；playground 冒烟 39 → 47 条断言。
- 新旧二进制全仓对拍（旧 = 动工基线 `17d37a1` 的构建，新 = 本批）：555 个被跟踪 `.lom` × 5 种命令（运行、`--check`、`--json`、`fix --plan --json`、`fix --plan --run --json`）共 2775 条，加 24 个含清单目录 × 2 种 `build` 共 48 条，**全部一致**（`build` 两模式按既有做法做顺序归一化——R257）。对拍脚本先做了自检：人为改动 3 条记录恰报 3 处；已知行为不同的程序旧 rc 0、新 rc 5。
- 普查复核：被跟踪 555 = `-> Unit` 524 + 无注解 6 + 无 main 25，声明 `-> Int` 的 0 个；138 道参考解全是 `-> Unit`；`eval/candidates` 100 份与 `eval/iter_runs` 219 份存档候选里声明 `-> Int` 的 0 份。
- 性能：同目录三段式交错（quick 九点）8 点带内、1 点反方向超带（`list_build n=20000` 为 −12.9%，5 块里只有 1 块稳定）；同一时段的同二进制对照自身也有一点超带（+4.3%）。对那一点做决定性测量（真实对比与对照在时间上交替各 30 块）：真实中位 +0.3%、对照 +0.8%，超 +3% 的块 5/30 对 7/30。**终判：无回退**——改动不在任何热路径上（`run()` 收尾处一次 `matches!`）。
- 全量回归与 CI：见 [docs/TODO.md](../TODO.md) 的本批交付段。

**过程如实登记**：

- 执行者通道：动工前派出的"文档写位只读供料"任务被模型侧 1308（5 小时用量上限）中断，没有交报告；实施、文档成对与验收全部由规划者亲自完成。
- 铁律 4 偏差一次：冷启动期间用 heredoc 向 python 直写了一段含反斜杠转义的正则脚本（把两份审查报告 HTML 转成纯文本；产物只在 gitignored 探针区，零跟踪文件改动）。此后的脚本一律经 Write 落盘。
- 顺带发现并修正的两处文档滞后：README 复现命令里 `codes --json` 的注释写 46（NAM007 加入后是 47，类型检查器批漏刷）；SECURITY.md 写 `tests/d0028_fix_run.rs` 是 12 项检查（本批之后是 15）。

**全量回归**：升版到 1.18.0 之后在最终工作树上重跑 HANDOVER §2.2 全量清单，32 项全部 rc=0（单元 696 + 集成 131、doc_audit 83/83、自举六模式、verify_selfcomp 368/368、`--bootstrap` 14/14、eval 双后端 138/138、playground 冒烟 47 等；逐项数字见 [docs/TODO.md](../TODO.md) 的本批交付段 ④）。

**采样核对**（§7 与 RFC 未决问题 3；2026-11-25，模型调用常设授权，小规模）：方法 = 不碰被跟踪文件，在探针区复制 `eval/llm_eval.py`（只改三处路径）与两个分类的 prompt，在"核心规则"第 10 条之后插入与新版 SPEC_FOR_AI 规则 11 对应的第 11 条，沿用已发表 pass@k 的同一套参数，各 3 次采样、共 9 次调用（glm-5.3：01_arithmetic 与 07_records_tuples；deepseek-v4-pro：01_arithmetic）。**结果**：90 份候选全部声明 `fn main() -> Unit`，声明 `-> Int` 的 0 份；新旧二进制下都是 90/90 通过，非零退出 0 份、翻转 0 份。**读法**：带退出码规则的 prompt 没有把模型引向改 main 的返回类型——评测可比性的风险在本次规模下没有出现；样本量小（三个分类、两个模型、各 3 采样），不构成"不会出现"的证明。留档 `target/probes/exitcode_impl/sampling/`。
