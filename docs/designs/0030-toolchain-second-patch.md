# 第二批 patch：工具链条目收口 R244-R247（designs/0030）

- **文档**：docs/designs/0030-toolchain-second-patch.md
- **性质**：动工前置设计（MAINTENANCE §2.1 第 2 条——触 `src/`、含退出码与信封的语义选边，各笔全部属 L0）。
- **基线**：HEAD `c7ad1b2` / v1.17.0。
- **状态**：**部分实施（2026-11-24）**——R247 已随类型检查器实施批落地（提交 `f7d0d5c`）；位点 1 取供料报告的修法 1B（位置落在右侧值表达式的起点）——给 `Stmt::LetDestruct` 加 span 会波及一批只读文件，1A 归入 R252 另议。R244–R246 由工具链实施批进行中。原状态：已裁，待实施。用户 2026-11-24 裁决（原话）："同意改。待裁事项全部做，可并行，也可顺序，按合理的方式完成。"——各笔的选边由规划者按此前已呈报的"应修"口径代定（§2 逐条留痕，用户可否决），排布由规划者定。
- **供料来源**：只读供料报告 `target/probes/wave3/s1_toolchain/report.md`（四节各含复现 / 代码路径 / 根因 / 候选修法 / 影响面 / 测试建议）。**规划者亲核**：十处行号级事实逐一打开源码核对（`target/probes/wave3/peek_supply.txt`），R244、R245 两个现象在基线二进制上亲手复现。
- **数字落盘前门禁**：写本文前后运行 `python tools/doc_audit.py`。本文不复写基线计数——期望值以 [HANDOVER §2.2](../HANDOVER.md) 为单一源。

## 1. 事实（规划者亲核）

| 笔 | 现象（v1.17.0 二进制实测） | 代码侧根因（行号为基线实况） | 亲核 |
|---|---|---|---|
| **R244** | `lom fix <包项目入口文件> --plan` 报假 NAM003（examples/pkg_demo/main.lom：`--check` 零诊断，`fix --plan` 报 5 条）；13 个被跟踪文件出假 error，其中 6 个文件共 11 条错误的 replace 建议；`fix --run` 被假 error 拒跑 | fix / apply / LSP 五处调用的 `typechecker::check_program` 不带包 externals（`main.rs:1022`、`:1216`，`apply.rs:76`，`cli.rs:641`，`lsp.rs:326`），与 run / `--check` 所用的 `collect_package_symbols` 不同源 | 复现 + 读码 |
| **R245** | 没有 main 的程序在解释器下 rc 0 静默通过；`fix --run` 报 pass。AOT rc 1、L2 编译期拒 | `Interpreter::run` 的 main 查找没有 else 分支（`interpreter.rs:1182-1186`） | 复现 + 读码 |
| **R246** | 坏清单（PKG 族错误）在 `--check` / `--json` 下被吞成"诊断通过" / `ok: true`；入口用到包符号时改出假 NAM003，`fix` 顺着它给错误替换 | `collect_package_symbols` 两处 `Err(_) => Vec::new()`（`main.rs:479`、`:481`）把"清单或依赖解析失败"与"没有依赖包"压成同一个返回值 | 读码（复现见供料报告 3.(a)） |
| **R247** | 元组解构数量不匹配的 TYPE001、变体子模式数量不匹配的 TYPE003 两条 warning 无位置（0:0） | 两处发射点写死 `0, 0`（`typechecker/mod.rs:513-514`、`:1331-1332`）；前者的 AST 节点 `Stmt::LetDestruct` 无 span，后者同函数的 NAM004 分支已在用现成的 `name_span` | 读码 |

**前提更正（必须让裁决者看到）**：designs/0029 §3.5 与台账此前写"R247 的 TYPE003 一处须自举侧同位"——**不成立为门禁约束**。L1 自举检查器不产这条 TYPE003（其 8.2 静态检查范围是 NAM003 / TYPE003 调用个数 / EFF001 / MAT001）；L2 把它当编译期错误拒、位置是"名字在源码中的首次出现处"；`verify_selfhost.py --static` 的比对集合里没有任何文件触发它。需要动 AST 的只有 TYPE001 位点，且它不在任何比对集里。

## 2. 规划者代定的选边（留痕，用户可否决）

| # | 选边 | 取定 | 理由 |
|---|---|---|---|
| 1 | R245：无 main 程序的运行路径 rc 0 → 1 | **改** | 与 AOT / L2 对齐（台账"应修"口径二选一里的前者）；落既有码 RUNTIME000，不新增码、不改 severity；ACRI §2.2 矩阵方向一致 |
| 2 | R246：坏清单项目的 `--check` / `--json` rc 0 → 1、`ok` true → false | **改** | 台账"应修 = 走诊断带并出结构化结果"；与 R234 同向；收口 R248 后 LANGUAGE_SPEC 对 `ok` 的定义至此自洽 |
| 3 | R244：PKG007 的打印位置 | **A2** | 把取 externals 拆成纯函数，PKG007 打印留在现有四个调用点——fix 面 stderr 维持为空，输出差异只有"假 NAM003 消失" |
| 4 | R244：LSP 是否同批同源修 | **L2（登记边界，另开账）** | 需 URI 到磁盘路径的转换与 `compute_diagnostics` 签名变化，超出"fix 面补包 externals"的最小面；编辑器里的假 error 如实登记 |
| 5 | R247：TYPE003 位点 | **2A** | 用现成的 `name_span`，零 AST 改动 |
| 6 | R247：TYPE001 位点 | **1A** | 给 `Stmt::LetDestruct` 加 span（`let` 关键字位置，与 `Stmt::Let.span` 同口径） |
| 7 | R246：包错误的 stage 取值 | **新增 `Stage::Package`（序列化为 `"package"`）** | ACRI 把 stage 定义为 OPTIONAL、实现自定义词表；仓内没有按 stage 取值分支的消费者；不借用语义牵强的现有 stage |

冻结面：四笔均不新增诊断码、不改既有码 severity、不动语法 / 关键字 / 内建——**不触语言面 v1.0 冻结**。

## 3. 逐笔设计

### 3.1 R246：坏清单在静态面的呈现（排在本批最前）

- **函数拆分**：`collect_package_symbols` 拆成可判错的纯函数 `package_externals(path) -> Result<Vec<String>, package::PkgError>`；PKG007 的打印（`package::warn_public_symbol_clashes`）留在现有四个调用点。
- **`--json` / `--check`**：拿到 `Err` 时追加一条结构化 PKG 诊断并**跳过入口文件的类型检查**（词法 / 语法诊断照报——带着空 externals 继续检查只会产出使用点的假 NAM003）。
  - 码 = 既有 PKG001 / 002 / 003（按 `PkgError` 的变体映射），severity = error，stage = package。
  - `Diagnostic.file` = 清单路径（信封顶层 `file` 仍是入口文件）；位置 (0:0)、anchor "none"（不伪造坐标）；`source_line` 为 null。
  - message 带清单路径（人类可读渲染的头行不打印 file）。
  - `ok` = false，rc 1。
- **构造器**：`PkgError → Diagnostic` 放在 `package.rs`（结构体字面量造诊断有生产先例：`main.rs:221-236` 的输入不可读信封），不动 `diagnostics.rs` 的现有构造器。
- **连带**：R244 取同一来源后，`lom fix` 对坏清单项目走 PKG 的 hint-only 提示臂，不再是错误替换；`fix --run` 的闸门因静态 error 落 `static_error`。
- **测试**（进程级）：坏清单四形态（解析失败 / 依赖路径不存在 / 循环依赖 / 键名与包名不符）×（`--check`、`--json`）→ rc 1、`ok: false`、恰一条 PKG 诊断（码 / file / anchor none 逐项）、入口文件里对包符号的使用不再出 NAM003；干净工程与无清单单文件的输出与改前逐字节一致。

### 3.2 R244：fix / apply 面补包 externals

- `run_fix` 开头算一次 externals，下传给 `apply_iterative`、`FinalDiag::check`、`static_diagnose`（三个签名各加一个 `&[String]`），五处里的 fix / apply / cli 三处改调 `check_program_with_externals`；`cli.rs` 的单测调用点跟着补 `&[]`。
- **对无清单单文件逐字节不变的保证**：无 `lom.toml` 时纯函数直接返回空集，`check_program_with_externals(…, &[])` 与 `check_program` 是同一个函数——检查器输入相同。实证：供料已落盘基线快照（555 个被跟踪 .lom × 4 条只读 fix 命令 = 2220 条记录），修后对拍脚本 `target/probes/wave3/s1_toolchain/r244/snapshot_fix.py after --lom <新二进制> --compare baseline`，期望 `records_different_without_manifest = 0`，差异只许落在 13 个受影响文件。
- **LSP**：本批不修，登记为边界（§7）。

### 3.3 R245：无 main 的程序

- `Interpreter::run` 在 main 查找处补 else 分支：`Err(RuntimeError::Msg("程序缺少 main 函数"))`。
  - 查**合并后的函数表**（与 AOT 一致：入口无 main、依赖包有 main 时两边都跑包里的 main）；放在 import 处理之后（既有的 import 错误照旧先报）。
  - 落既有码 RUNTIME000（`classify_runtime_error` 无"main"分支）、位置 (0:0)、anchor none；输出 `[runtime] error (0:0): [RUNTIME000] 程序缺少 main 函数`，rc 1（走 `main.rs:410-419` 既有分支）。
- **静态面不动**：`--check` / `--json` 对无 main 文件继续"诊断通过" / `ok: true`——库文件本就没有 main。
- **连带**：`fix --run` 的观察结果由 pass 变 error（计划数组追加一条 stage "runtime" 的通用提示）；playground 引擎共用 `interp.run`，无 main 的片段由"静默、退出码 0"变为"退出码 1 + stderr 说明"。
- **随批必做**：`tools/spec_examples_check.py` 的 `wrap_if_fragment` 只把"无顶层项"的块包进 main；两份规范里有 13 个"有顶层项、无 main、零 error"的正例块会被实跑并要求 rc 0。改工具：正例块不含 `fn main` 时追加一个空的 `fn main() -> Unit … end` 再检查与实跑（不改 13 个文档示例；对账计数 51 / 6 / 9 不变）。
- **影响面**：被跟踪 .lom 中无 main 的 25 个文件逐个核对过——没有测试 / CI / 工具执行它们并依赖 rc 0（供料报告 2.(e) 的逐项表）；集成测试里 36 个无 main 的程序字面量全是包源、片段或 LSP 样例。全量回归确认。

### 3.4 R247：两处 warning 的位置（归类型检查器实施批）

- 位点 2（TYPE003 子模式数量）：`mod.rs:1331-1332` 的 `0, 0` 换成 `name_span.line, name_span.col`，与同函数 NAM004 分支同款；位置 = 模式里变体名 token。
- 位点 1（TYPE001 元组解构数量）：`ast.rs` 给 `Stmt::LetDestruct` 加 `span: Span`（`let` 关键字位置），`parser.rs` 把 `let_tok` 的行列传入 `parse_let_destruct`，`mod.rs:499` 的模式加 `span` 并用于 `:513-514`。`--dump-ast` 不含 span，输出不变。
- **测试**：typechecker 单测（两处各一条：位置值；数量相符的对照零诊断）；进程级（`--json` 的 line / col 非 0、anchor exact、source_line 对应；`fix --plan --json` 的 diagnostic 坐标非 0 且仍是 hint-only；555 个被跟踪文件的 `--dump-ast` 与改前逐字节一致；两枚被跟踪负例的位置 `4:9` 与 `5:9`；`verify_selfhost.py --static` 仍 PASS；两枚负例的 L2 输出不变）。

## 4. 排布与文件归属

- **工具链实施批**（与 designs/0029 的 R234 / R243 / R236 / R237 / R242 同批、同一工作树）：R246 → R244（共用函数拆分，先后如序）→ R245。归属 `src/main.rs`、`src/interpreter.rs`、`src/apply.rs`、`src/fix.rs`、`src/fix_run.rs`、`src/package.rs`、`src/diagnostics.rs`（含 `Stage::Package`）、`src/cli.rs`、`tools/spec_examples_check.py`、`tests/`。
- **类型检查器实施批**（与 R235 / R238 / R249 同批、另一工作树）：R247。归属 `src/typechecker/**`、`src/ast.rs`、`src/parser.rs`。
- 共享文档两批执行者都不碰，合并后由规划者单写。

## 5. 验收标准（实施批；加粗项为规划者亲验）

- **十组门禁**（HANDOVER §2.2）全绿；集成测试逐 target 单跑并配对上报。
- **R246**：§3.1 的四形态 × 两模式逐条成立；干净工程与无清单单文件逐字节不变。
- **R244**：基线快照对拍 `records_different_without_manifest = 0`；13 个受影响文件的假 NAM003 消失、无清单文件不变；examples/pkg_demo 的 `fix --plan --json` 零计划项。
- **R245**：无 main 单文件 rc 1 且 stderr 文案如 §3.3；"包里有 main"形态与 AOT 同值；`--check` / `--json` 输出不变；`spec_examples_check` PASS 且计数不变。
- **R247**：§3.4 的期望逐条；dump 门逐字节一致。
- **回归**：verify_selfcomp 全过 + 强 quine 逐字节不变、自举六模式、eval 双后端、`eval_prompt_check`、playground 冒烟、fmt gate、clippy 零输出。

## 6. 文档成对（合并后由规划者单写）

- SPEC_FOR_AI：§11e Limitations 的"无位置"清单去掉元组解构一项；"Pattern carries no span"一句改正；§1 规则 11 与 §11e 补一句"无 main 的文件运行时 rc 1（RUNTIME000）"；§8 多文件规则补坏清单在 `--check` / `--json` 下的呈现。
- LANGUAGE_SPEC §7（退出码：无 main → 1；坏清单的 `--check` → 1）、信封 stage 取值列举加 `package`。
- CHANGELOG Unreleased、HANDOVER、符合性映射（退出码两处）、designs/0029 §3.5 的前提原位加注、本文状态行。

## 7. 不在本批 / 新开账（均不构成预授权）

- **LSP 同源修**（R244 的 L2 选项）：编辑器里包项目入口文件持续显示假 error——另开账。
- **另两处静态诊断位置写死 0:0 且未在 SPEC_FOR_AI §11e 登记**（R247 全量清单的顺带发现）：NAM002 枚举重复定义（`mod.rs:328-329`，`EnumDecl.span` 现成）；TYPE010 裸 `return`（`mod.rs:642`，`Stmt::Return` 无 span）。前者随类型检查器实施批顺手补位（零风险），后者另开账。`lom explain TYPE001` 的长文案只讲二元运算与比较、未提解构与 let / 赋值形态——随类型检查器实施批补全。
- 给运行期的两条元组解构错误接位置通道（有了语句 span 之后才可能）——另议。
- 给 `LetDestruct` 之外的 AST 节点补 span 的全面清理——另议。

## 8. 风险与如实边界

- 本设计建立在对 v1.17.0 二进制与基线源码的实测和读码上；**凡"修后"行为均为读码推断，未构建验证**，实施时以实测为准并如实登记。
- R245、R246 是调用方可见的退出码变化：依赖"无 main 程序 rc 0"或"坏清单项目 `--check` rc 0"的脚本会受影响。仓内没有这样的依赖（逐项表见供料报告），仓外使用者未知。
- 供料未核项：PKG004 路径未搭探针；`specs/acri/consumer.py`、`tools/diff_gen.py`、`eval/llm_eval.py` 未读；`tests/d0028_fix_run.rs` 的 12 个测试只读了 2 个。实施批覆盖。
