# 台账清空批：R251-R258 + R260（designs/0033）

- **文档**：docs/designs/0033-backlog-clearing.md
- **性质**：动工前置设计（MAINTENANCE §2.1 第 2 条——触 `src/`、含语义选边）。
- **基线**：main @ `200611e` / v1.18.0（tag `70548cf`）；台账 open 8 笔（R251-R258）。
- **状态**：**已裁，实施中（2026-11-25）**。用户 2026-11-25 裁决（原话）："整理一下当前状态，清理掉待办，让项目保持清爽，release和playground和tag以后都不必由我裁决。注意尽量用子智能体来做，不要干扰上下文。"——各笔的选边由规划者按既有"应修"口径代定（§2 逐条留痕，用户可否决）。发版线预授权的规则文本见 MAINTENANCE §1.4 条款 6 与 §7。
- **供料来源**：只读供料报告 `target/probes/wave4/supply/report.md`（R251-R257 逐笔七项 + 八份补丁草稿 `patches/R25x.diff`，均对 `200611e` 通过 `git apply --check`；R258 的详节因执行者被用量上限中断未写，只有草稿——由规划者亲读）。**规划者亲核**：每份草稿的触及文件清单、R254 的 L1 改动面与三档代价、R256 三项的改动点全集；R260 由规划者亲手复现（`target/probes/wave4/misc_probe/`）。

## 1. 范围与版本

九笔一次清空（八笔 open + 本轮新开一笔）。行为修复与新增 warning 实例并存——按 MAINTENANCE §2.2 决策表属 patch 级；发版线已预授权（§1.4 条款 6、§2.2.1 第 4 类），实施批通过 CI 后随即发 **v1.18.1**，使 main 不领先最新 tag。

## 2. 逐笔选边（规划者代定，留痕）

| 账 | 取定 | 理由与留痕 |
|---|---|---|
| **R251** LSP 对包项目入口文件报假 NAM003 | 按草稿：`uri_to_path`（只认 `file:` 方案）→ `PkgCtx::resolve` → 新入口 `compute_diagnostics_with_externals`；didOpen / didChange 改调 `lsp_diagnostics` | 清单 / 依赖解析失败取"同 `--check`"（一条 PKG 诊断 + 跳过类型检查）——R246 刚在 CLI 面推翻了"坏清单被吞成干净"，LSP 不保留更差的一侧。不调 `diagnose_with_pkg`（它顺带算 `fix::fn_infos`，全量同步模式下每次击键白算）。PKG007 与依赖包诊断提示行不在 LSP 发（沿 fix 面的选边 A2）。 |
| **R252** 裸 `return` 的 TYPE010 位置 0:0 | **方案甲**：`Stmt::Return` 加 `span`（`Return { value, span }`）；带值形态维持值表达式的 `e.span`，不带值取 `return` 关键字的位置 | 账面的"应修"；位置是真实坐标，满足"绝不能 = 伪造 1:1"。零 AST 改动的方案乙（取所在声明的签名 span）会把诊断指到函数头而不是出错的 `return`，弃。这是横切改动（七个文件的构造 / 匹配点机械改写），只放在类型检查器组的工作树里、排在最后落地。 |
| **R253** 用户 enum 变体构造器实参不查类型 | 按草稿：`check_variant_ctor_args`（从左到右一趟）；范围取**全部用户 enum**（含非泛型）；位置取**该实参表达式起点** | 只做带类型参数的会造成"带参数的查、不带参数的反而不查"的倒挂。新增的都是 warning 实例，靠新旧对拍一次性归因。 |
| **R254** 带类型实参被测值的穷尽性 / 变体名 / 模式个数 | 宿主两处（`check_match` 的用户枚举判定、`check_pattern` 带实参分支改查表）+ **L1 档 A**（`chk_match` 增 `TyGeneric` 臂，穷尽性 MAT001 同步） | MAT001 两侧都产，宿主补了 L1 不补就是真正的分歧面——必须做。档 B（L1 的模式个数 TYPE003）是比 R254 更老的"L1 从不产这一条"的结构缺口，要动 L1 自己的 AST 与 parser，不并入本批，登记为既有边界；档 C（NAM004）不在比对码内，不做。 |
| **R255** 内建 Result / Option 的载荷精度 | 按草稿：`variant_info` 内建分支由构造实参定型，实参未知时仍是占位符 | `types_compatible` 不动；只会让既有的比对点多报真失配，未知位保持通配。 |
| **R256(a)** 自举解释器透出被解释程序的退出码 | 按草稿：`self_interp.lom` 的 `main` 改 `-> Int`，七个非 run 叶子补 `0`，`--run` 臂改调 `run_mode`；**自带 rc 断言** | 现有门禁比不到这项改动（`run_lom` 丢弃 rc、语料里没有 `-> Int` 的 main），所以测试必须自带。既有测试与期望值不动。 |
| **R256(b)** playground 区分显示 | `classifyExit(exitCode, stderr)`：stderr 含 `[runtime] error` → 运行时错误（红）；含 `[lex]` / `[parse] error` → 语法错误（红）；否则 → 自定义退出码（中性色） | 宿主的运行时错误恒以该头走 stderr，程序自身只能写 stdout，伪造不出这个头。`harness.js` / `worker.js` 契约不动。 |
| **R256(c)** 差分对拍加 `-> Int` 模板 | 第一层（`t_exit_code` 模板族，`POOL_BASE` 101 → 102）+ 第二层的退出码断言与第 9 个定向探针 | 供料里"给其余程序补 `rc == 0` 绝对断言"一步**不取**：生成器可能合法地构造出两侧同样失败的程序，加了会出假红，且超出账面。 |
| **R257** `lom build` 包列出顺序随机 | 输出循环前按**包名**排序（人类可读与 JSON 两处）；JSON 的 `symbols` 一并排序 | 排序只作用于 `run_build` 内的输出循环，`resolve_dependencies` / `resolve_dfs` / `package_closure_externals` 不动——"绝不能 = 改动依赖解析语义"由此成立。 |
| **R258** debug 构建下 `cargo test` 在 Windows 上跑不完 | debug 档给解释器线程更大的栈（`INTERP_STACK_BYTES`，`cfg(debug_assertions)` 门；**取值由执行者实测后定**，草稿里的 8 GiB 是占位）+ 让唯一在那个测试线程上跑解释器的单测改在大栈线程里跑 | 两个集成测试钉的是"到得了 60000 层并出结构化诊断"，所以 debug 档不能靠下调阈值解决。release 的 256MB、守卫阈值 60000、两个测试的断言一字不动。 |
| **R260**（本轮新开）`lom build --json` 有 error 级诊断时 rc 1 但信封 `ok:true` 且不带诊断 | 任一被检查文件有 error 级诊断时：`ok` 为 false，并追加 `diagnostics` 数组（元素与 `check --json` 的 diagnostics 同形）；无 error 时输出逐字节不变；rc 规则不变 | JSON 消费者此前只能看到一个 `ok:true` 配一个非零退出码。append-only：新字段只在出错时出现；与既有的失败信封（`ok:false` + `error`）同一方向。 |

**不开账的顺带登记**：类型渲染用 Debug 形态、占位符会露出（`得到 Result<String, E>`）——既有问题，不在本批。

## 3. 并行分组（三个互不重叠的工作树，合并序 A → B → C）

| 组 | 账 | 文件归属 |
|---|---|---|
| **A 工具链组** | R251、R257、R260、R258、R256(b) | `src/lsp.rs`、`src/main.rs`、`src/fix_run.rs`、`src/interpreter.rs`（仅 R258 的测试辅助）、`playground/app.js`、`playground/index.html`、`playground/smoke.mjs`、相关 `tests/` |
| **B 类型检查器组**（树内按 R255 → R253 → R254 → R252 → R256(a) 的顺序落地） | R255、R253、R254、R252、R256(a) | `src/typechecker/mod.rs`、`src/typechecker/tests.rs`、`src/ast.rs`、`src/parser.rs`、`src/dump.rs`、`src/fix.rs`、`src/interpreter.rs`（R252 的一行）、`src/wasm_codegen.rs`、`examples/selfhost/self_interp.lom`、`tools/selfhost_cases/`、`tools/verify_selfhost.py` |
| **C 差分组** | R256(c) | `tools/diff_gen.py`、`tools/diff_test.py` |

跨组的同文件改动只有 `src/interpreter.rs`（A 的 R258 测试辅助与 B 的 R252 一行相距约 1500 行）——合并时按 hunk 自动合并，规划者亲读合并结果。

## 4. 测试与验收

- 每笔按供料报告第 5 项的测试清单落地（正例 + "绝不能这样修"的负向锁定）；R258 的验收 = 不加 `--release` 的 `cargo test` 在 Windows 上跑得完且全过。
- 规划者验收：三个工作树的 diff 亲读；合并后全量回归（HANDOVER §2.2，32 项）；新旧二进制全仓对拍——本批预期有差异（新增 warning 实例、build 输出顺序、`build --json` 信封、`-> Int` 相关），逐条归因；`verify_selfcomp` 368/368 与 quine 272925 逐字节不变（`self_comp.lom` 不触及）；自举六模式（`--static` 坏文件数 +1）；debug `cargo test`；性能对照（R252 触解释器一行、R258 只在 debug 档）。

## 5. 文档成对面（随批）

LANGUAGE_SPEC §7.3 / §13 一条；SPEC_FOR_AI（LSP 的包符号口径、`build` 输出顺序与 `--json` 信封、类型诊断的位置与新 warning 实例、自举解释器 `--run` 的退出码、§11f 条数不变）；README / HANDOVER / HANDOFF / TODO 的计数位（单元、集成、`--static` 坏文件数、`self_interp.lom` 行数、模板族数、playground 冒烟条数）；CHANGELOG 的 v1.18.1 条目；本文状态行。

## 6. 风险与如实边界

- 新增 warning 实例（R253 / R254 / R255）会改变部分文件的 `--check` 输出——存量程序的行为不变（warning 不拦截），但对拍会有差异，逐条归因后才放行。
- R252 是横切改动：合并后要跑 `--dump-ast` 对拍（dump 的 `Return` 渲染不应变——span 不进 dump）。
- R256(a) 的"三道闸"里只有运行期收尾与 AOT 的 tag 检查是硬保证；静态检查抓不到"叶子漏写值"的全部形态（供料实测）。
- R258 只改善开发期体验（debug 构建）；release 行为零变化，证明方式 = `cfg(debug_assertions)` 门 + release 下常量与代码路径逐字不变。
