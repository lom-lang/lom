# 运行时诊断接入修复闭环设计：L1 运行时错误带位置 + L2 `fix --run`

- **文档**：docs/designs/0028-runtime-repair-loop.md
- **性质**：动工前置设计（MAINTENANCE §2.1 第 2 条——触宿主解释器 `src/`、扩展 CLI 工具面，并含一处语义选边（ACRI AXIOM 3 的适用边界）；先例 0001-0027）。
- **基线**：HEAD `5d4fd61` / v1.16.0（工作树干净；台账零 open，顺延号 R234 起）。语言面 v1.0 冻结维持——**零语法 / 零关键字 / 零诊断码 / 零内建变化**（纯 CLI flag + 解释器诊断位置通道；v1.14.0 `--trace` 批同款口径）。
- **状态**：**已交付（2026-11-23；用户裁决四点全取建议项；实施批升 minor v1.17.0——tag `v1.17.0` = `51f0f4e`，CI run `436` 七 job 绿——首跑即绿）**。执行者实施代码、测试、README 与 SPEC_FOR_AI；规划者验收 = 亲读六处关键路径 diff（L1 外壳 / 诊断层 / 主入口 / fix_run 模块 / 参数校验 / REPL）+ 十组门禁亲跑全绿（Rust 635+58、集成十二 target 逐个单跑；verify_selfcomp 368/368 与 --bootstrap 14/14 强 quine 272925 字节逐字节一致；自举六模式；eval 双后端各 138/138；wasm 目标构建 + build_selfboot 硬门 + playground 冒烟；fmt gate examples 37 / selfcomp 用例 380）+ 自写探针 25/25 + **性能终判无回退**。执行者被模型侧限流与网络故障打断四次，文档成对的后 9 项由规划者接手完成（如实登记）。**与设计的偏差（六项）**：① 闸门新增第三种跳过情形 `package_import`（同目录有 lom.toml 且 import 外部包——fix 面不解析依赖包，包源码里的写内建会绕过名字闸门）；② `run` 对象在设计的三个字段之外加可选 `reason`（稳定 token）与 `detail`（人读说明）；③ `--apply --run --json` 在 lom-apply/v1 追加 `runtime_plans` 与 `run` 两个顶层字段；④ 观察环节自身失败（trace 出口打不开、解释器线程异常终止）退出码为 2（§4.7 写"分析完成的一切情形 0"——这两种不属于分析完成）；⑤ 安全锁以集成测试 + 单元测试实现（`--apply --run` 后源码逐字节不变；六 RUNTIME 臂皆 hint-only），未用 fix_corpus 配对（零语料扰动）；⑥ 七类运行时错误如实维持 (0:0)——错误不在表达式求值路径上抛出（import 未知模块 / import 不导出符号 / 元组解构右侧非元组 / 元组解构数量不匹配 / for 不可迭代 / 语句位置 if、while 条件非 Bool；根因是这些 Stmt 节点不带 span），单测钉住。wasm 目标按设计拒绝 `--run`（rc 2）。**A/B 方法教训（随批入 HANDOVER §3）**：新旧二进制必须同目录——当晚 target/release 目录下的进程在 list_build 上慢 13-21%（同一机器码拷到别的目录即恢复）【2026-11-24 更正：方向说反——复测表明 `target/probes/` 整棵子树是异常快的一侧（其下三处位置与它互比 ≈1.00），而 `target/release/`、`target` 下其它目录、系统临时目录三处速度一致，即"正常速度"；文件系统层无差异（非接合点、无压缩、属性相同），根因未明。新旧二进制在正常位置同目录互比为 -1.3%，终判不变】，跨目录对比造出过"回退 21%"的假象，两段式交错还出现过"旧 vs 旧也 +21%"的位置伪影；终判采用同目录三段式交错 + 逐点同二进制对照（`target/probes/runtime_fix_impl/planner_ab3.py`、`ab3_points.py`），十个名义超带点的实测中位 −1.7%~+2.9%、与对照组同量级。未测量边界：本机当晚噪声底约 ±2%，更小的开销分辨不出；`map_lookup` n=5000 实测 +2.9% 贴带边。来源：2026-11-23 新鲜眼审计建议④"把运行时错误接进修复管线"，用户裁决执行。
- **供料来源**：执行者读码供料两次因模型侧限流中断，未产出综合报告；其可复现取证脚本与原始输出已落盘（`target/probes/runtime_fix_supply/`——`evidence.sh` / `evidence_eval.py` / `out/`，gitignored）。**本文件 §1 的结论由规划者亲读代码、亲跑探针、亲算计数后写下**，行号为基线实况；个别未亲算的数字就地标注来源。
- **数字落盘前门禁**：写本文前后运行 `python tools/doc_audit.py`。本文不复写基线计数——一切期望值以 [HANDOVER §2.2](../HANDOVER.md) 为单一源。

## 1. 供料结论

### 1.1 缺口是"到不了"，不是"没有"

- fix 入口 `run_fix`（[src/main.rs](../../src/main.rs) L998）在 L1012 调 `fix::generate_plan(&diags, src, &fns)`，`diags` 只来自静态检查；`--apply` 的逐轮计划同理（[src/cli.rs](../../src/cli.rs) L578）。
- 分派表（[src/fix.rs](../../src/fix.rs) L770-923）37 个具名臂 + 1 个兜底臂，覆盖全部 38 个宿主码（执行者以码表对照，集合一致；臂清单规划者亲读）。其中 RUNTIME000-005 **六臂齐备**（L853-885），且**全部是 hint-only**——RUNTIME002 的 `fix_runtime_undefined` 也只是拼一段提示文字（Low）。
- 实测（探针 `target/probes/runtime_fix_supply/progs/` p01-p17，二进制与基线源码对应——取证 S0 已核）：
  - 静态干净 + 除零（p02 / p03 / p17）→ `fix --plan --json` 返回 `"plans": []`；
  - 递归深度（p05）→ 同样 `[]`——v1.10.3 登记的 Medium 专属 hint 在分派表里，但 CLI 不可达；
  - 带静态 warning 的程序（p04 TYPE003 / p08 NAM005）→ 有计划，驱动者是静态 warning。
- fix 不接收任何外部诊断输入（执行者实测：stdin 形态 `fix - --plan` rc 2；未知 flag 被忽略后照常输出"无诊断"）。

### 1.2 运行时错误没有位置通道——一笔历史欠账

- 产生运行时错误的 16 个探针里，**15 个报 `(0:0)`**，仅递归深度 1 个报 `(1:1)`。p03 是 5 行程序，`println(a / b)` 在第 4 行，除零报的却是 `(0:0)`。
- `RuntimeError`（[src/interpreter.rs](../../src/interpreter.rs) L356）= `Msg(String)` / `EarlyReturn(Value)` / `DepthLimit { msg, line, col }`。`Msg` 是裸字符串（非测试区 90 个构造点，规划者复算一致），码由 [src/diagnostics.rs](../../src/diagnostics.rs) 的分类器事后从消息文本识别；只有 `DepthLimit` 靠专用变体自带位置——其注释原话："RuntimeError::Msg 没有位置通道，而深度错误的定位价值高"。
- `Diagnostic::from_runtime`（diagnostics.rs L160 起）头注释原话："Phase 2.3 阶段，AST 节点尚未携带位置信息（Phase 3 改造），因此运行时错误的位置需要调用方尽量提供；未知时传 (0, 0)"。而表达式级 span 自 v0.21.0 起已在 AST 上（[src/ast.rs](../../src/ast.rs) 多处 `pub span: Span`），求值入口 `eval_expr(&mut self, expr: &Expr, …)`（interpreter.rs L1388）手里就有 span。
- **结论：位置通道不是做不了，是 Phase 2.3 时做不了、AST 后来补上了、这一段没有回接。**

### 1.3 现有定位能力 = trace 的函数级

- `--trace` 对用户函数内的运行时错误逐层上报 `call-error`（三层嵌套 a → b → c 实测：三个事件，可缩到最深层函数 `c`），但 `site` 是函数**定义处**，不是出错行。
- 错误直接发生在 main 体时 **trace 零事件**（`{"seq":1,"kind":"summary","events":0}`；main 入口不出 call 事件是 [designs/0026](0026-agent-execution-trace.md) 的既定边界）。

### 1.4 评测侧事实（既有测量数据的再读，不是新测量）

- 2026-10-28 迭代修复首发采样（[eval/REPORT-2026-10-28-iter-repair-first.md](../../eval/REPORT-2026-10-28-iter-repair-first.md)；轨迹档 gitignored）：任务 131 的 14 条 round-1 失败轨迹规划者**逐条亲读**——每一条都同时带静态 warning（TYPE001 或 TYPE003）与运行时码（RUNTIME000"运算 Add 不支持 枚举变体 和 Int" / RUNTIME001"map_get 第一个参数期望 Map，得到 Unit"），运行时行全部 `(0:0)`。对这一族，静态通道本来就有带位置的诊断；round-2 回喂后 14/14 收敛是报告原结论。
- 任务 137-139 一族是"静态零诊断、运行时才失败"（v1.16.0 批的设计形态，见 HANDOVER §1）。对它们，工具链今天只给一行无位置的报错，`fix` 给空计划。**这是本设计真正的价值所在。**

### 1.5 影响面：谁依赖 `(0:0)` 形态（规划者 grep，2026-11-23）

| 面 | 含 `0:0` 字样的文件数 | 判读 |
|---|---|---|
| `tests/` | 0 | 没有测试断言运行时 `(0:0)` 形态 |
| `eval/tasks/` | 0 | 任务 130 的 prompt 不含 `(0:0)` 文本（嵌入的是静态零诊断信封加散文说明），评测资产预期不受 L1 影响 |
| `playground/` | 0 | 无依赖 |
| `SPEC_FOR_AI.md` / `LANGUAGE_SPEC.md` | 1 / 3 | 文档成对位，实施批逐处定性 |
| `src/` | 5 | 含 LEX000 的 `0:0` 哨兵——那是 ACRI anchor:none 语义，**不在改动面**；其余实施批逐处定性 |
| `tools/` / `examples/` / `specs/` | 1 / 2 / 1 | 非测试与评测面，实施批逐处定性 |

### 1.6 约束核实

- **效应系统不能当执行闸门**：main 对 EFF001 豁免（[src/typechecker/mod.rs](../../src/typechecker/mod.rs) L375"main 函数隐式拥有所有效应"）——main 内直接调用 IO 内建不会有任何诊断。
- **CLI 没有任何超时或预算机制**（main.rs / interpreter.rs / cli.rs 中 `timeout` / `Duration` / `Instant` / `budget` 零命中）。
- 语句级：`Stmt::Assign` 与 `Stmt::Let` 带 span，`Stmt::Hole` 带 line/col。
- `lom <file> --json` 是纯静态检查语义：对运行时会崩的程序返回 `ok: true`、零诊断、rc 0（规划者亲跑）。

## 2. 问题陈述与设计原则

**问题**：对"静态干净、运行时失败"的程序，修复闭环只给一行无位置的报错，`lom fix` 给空计划——语言存在的理由（修复闭环）在运行时这半边没有闭合。

**原则**：

1. **先定位，后接入**。无位置的运行时修复计划价值很低（§1.2 / §1.3），L1 是 L2 的前置。
2. **冻结面零触碰**。不新增诊断码、不新增内建、不动语法。
3. **默认路径逐字节不变**。不带 `--run` 时 `fix` 与今天一致（`--trace` 默认关的先例）。
4. **副作用必须显式**。执行程序是有副作用的行为，必须 opt-in 且有闸门。
5. **不宣称修复率**。本设计不包含、也不暗示任何"运行时修复率提升"的数字；价值验证另批（§8）。

## 3. L1 设计：运行时错误带位置

### 3.1 机制（裁决点 2）

**求值外壳 + 侧通道**，沿 `--trace` 批 `call_function` / `call_function_inner` 的外壳先例：

- 解释器新增字段 `err_site: Option<(usize, usize)>`。
- `eval_expr` 现函数体改名 `eval_expr_inner`；新的 `eval_expr` 是外壳，在 `Err(RuntimeError::Msg(_))` 时若 `err_site` 为空，记入 `expr.span` 起点。
- "最内层优先"由错误自内向外传播天然成立：内层外壳先写入，外层外壳见到非空不再覆盖。例：`foo(1 / 0)` 落在 `1 / 0` 这个表达式的起点；错误发生在被调函数体内，则落在被调函数内失败表达式所在行。
- 语句级补点：`exec_stmt` 的 `Stmt::Assign` 臂同理（赋值给未定义变量这类不是在表达式里抛出的错误），取 `Assign.span`。
- 上报点：`from_runtime` 的生产调用（diagnostics.rs L341 所在封装）以 `err_site` 取代调用方写死的 `(0, 0)`；`DepthLimit` 维持自带位置，行为不变。
- Ok 路径零新增工作（只对已经返回的 Result 多一次分支）。

**预登记回退（乙）**：实施批做交错 A/B 基准（`tools/bench.py`，[designs/0022](0022-interpreter-performance.md) 纪律）。若任一负载的回退超出 ±3% 带，改走乙——在少数最热的错误源（`eval_binary` / `eval_call` / 内建分派）显式传入 span。这不另开裁决点，属实施批验收环节的机制切换，如实登记。

### 3.2 语义与精度边界（如实登记）

- 位置是"求值时返回错误的那个最内层表达式"的**起点**，anchor 为 `exact`（与静态表达式诊断同口径）。
- 不是运算符或操作数级精度：`a + b * c` 出错，落在整个二元表达式的起点。
- 不是所有运行时错误都能定位：不在表达式求值路径上抛出的错误维持 `(0:0)`，逐类登记。
- trace 的函数级定位不变；L1 不改 trace 事件。

### 3.3 重置点与吞错点审计（实施批读码项）

- 重置：每次程序运行前、REPL 每条输入前、LSP 每次诊断前、`fix --run` 每次执行前。
- 审计：全量检查解释器内"`Err(RuntimeError::Msg)` 被接住后继续执行"的位置。Lom 没有 try/catch，预期为零；`EarlyReturn` 是控制流不是错误，不在此列。发现一处，就在该处清空 `err_site` 并加一条负向测试。

### 3.4 跨后端（三实现一致律）

- L1 只改解释器通道。`lom build --target wasm` 的 AOT 产物不经解释器，其运行时失败**继续无位置**——登记为跨后端分歧（MUT002 族的登记形态），加集成测试锁定"该通道维持无位置"。
- L2 自举编译器是静态面，不受影响；quine 与 verify_selfcomp 构造性不变（解释器改动不触 codegen）。

### 3.5 输出形态

- 人类可读：`[runtime] error (4:13): [RUNTIME000] 整数除以零`，带源码行与脱字符——以 §1.2 的 p03 为例，复用静态诊断渲染器；`DepthLimit` 今天已经走这条路径。
- 本设计不新增"运行时诊断 JSON 信封"（`--json` 保持纯静态检查语义，见 §1.6）；结构化出口由 L2 的 `lom-fix/v1` 计划承载。

## 4. L2 设计：`fix --run`

### 4.1 命令形态

`lom fix <file> --plan --run` 与 `lom fix <file> --apply --run`；可选 `--run-budget <ms>`、`--allow-io`。

### 4.2 流程

1. 静态分析与今天一致。
2. 若静态存在 error 级诊断，**不执行**（程序本就跑不起来），输出同今天，人类模式加一行说明。
3. 否则执行一次：参数为空；程序 stdout 丢弃（写入 sink，不走管道——防止死循环打印撑满管道而阻塞）；捕获首个运行时诊断（经 L1 带位置）。
4. 把分类后的运行时诊断追加喂给 `generate_plan`，由现有六个 RUNTIME 臂产出建议。
5. 运行时计划排在静态计划之后，诊断对象的 `stage` 字段为 `"runtime"`（既有字段）。

### 4.3 与 AXIOM 3 的相容性（语义选边，如实陈述）

- AXIOM 3："producing suggestions MUST be side-effect-free"。
- 本设计的立场：不带 `--run` 的 `fix --plan` / `--apply`，语义与输出**逐字节不变**，无副作用的默认路径得以保持。`--run` 是显式 opt-in 的独立调用形态，它的文档语义就是"执行程序以观察其失败"——副作用是所执行程序自身的，由用户显式选择。AXIOM 3 守的是"计划不会偷偷产生副作用"，不是"计划必须对运行时一无所知"。
- 这个立场须在 [specs/acri/lom-conformance.md](../../specs/acri/lom-conformance.md) 的 AXIOM 3 行如实登记（参考实现的 `--run` 形态说明），不改规范文本。

### 4.4 副作用闸门（裁决点 3）

**甲（推荐）——内建名扫描**：源码词法层面出现 `file_write` 或 `file_append` 即拒绝执行（`run.outcome = "skipped"` 并说明），除非同时给出 `--allow-io`。

- 依据：持久副作用只可能来自这两个内建（`println` 只写被丢弃的 stdout；`file_read` / `file_exists` / `env::args` 只读）；内建集冻结（SPEC §14），名单穷尽。
- 不用效应系统的原因：main 对 EFF001 豁免（§1.6），"无 `[IO]` 注解 + 零 EFF001"不构成保证。
- 保守面：名字出现在死分支、或被局部变量遮蔽时也会拒绝——误拒可用 `--allow-io` 放行，方向是安全的。
- 单元测试钉住闸门名单；SPEC_FOR_AI 登记"新增带持久副作用内建的 RFC 必须同步扩展此名单"。

**乙**：不设闸门，仅文档警示。不推荐——`fix` 会静默执行一个会写文件的坏程序。

### 4.5 运行预算（裁决点 4）

**甲（推荐）**：默认 5000 ms（与 playground 引擎 5 秒看门狗同量级），`--run-budget <ms>` 可调。

- 机制（零第三方 crate）：在既有大栈工作线程形态上，用 `std::sync::mpsc` 通道回传结果，主线程 `recv_timeout` 等待。超时则输出结构化说明、`run.outcome = "timeout"`，正常走完 fix 输出后结束进程（放弃失控线程）。

**乙**：不设预算。不推荐——死循环程序会挂死 `fix`。

### 4.6 输出信封（append-only）

- `lom-fix/v1` 新增可选顶层字段 `run: { "attempted": bool, "outcome": "pass" | "error" | "timeout" | "skipped", "budget_ms": int }`。消费者按前向兼容公理忽略未知字段。【v1.18.0 追加（[designs/0032](0032-process-exit-code.md)，RFC-0005）：可选的 `"exit_code": int` 置于 `budget_ms` 之后，只在 main 声明 `-> Int` 且正常跑完时出现——程序以自定义退出码结束不算失败（`outcome` 仍为 `"pass"`、无运行时计划）；超范围或非整数返回按运行时错误（`"error"`）。条件字段，其余程序的输出逐字节不变。】
- 计划数组追加 `stage: "runtime"` 的计划。它们全部是 hint（§1.1：六臂皆 hint-only），**永不被 `--apply` 自动应用**——AXIOM 1 / 2 不受影响。

### 4.7 退出码

- `fix` 在"分析完成"的一切情形维持 rc 0（观察到运行时错误、运行通过、超时、闸门拒绝都属于分析完成），ACRI §2.2 矩阵不变。
- flag 误用（`--run` 用于非 fix 子命令、`--run-budget` 非法值）走既有的工具失败带 rc ≥ 2。

### 4.8 与其它 flag 的组合

- `--apply --run`：静态迭代轮照旧收敛后做一次运行时观察，**只报告不应用**（运行时计划本就全是 hint）。
- `--trace` + `--run`：观察运行期间 trace 照常产出（默认关，不变）。

## 5. 裁决点

| # | 裁决点 | 选项 | 建议 |
|---|---|---|---|
| 1 | 批范围 | 甲 = L1 + L2 同批，升 minor（新 CLI flag，`--trace` 批 v1.14.0 先例）；乙 = 仅 L1，升 patch，L2 另批；丙 = 仅 L2 | **甲**。丙不推荐——无位置的运行时修复计划价值很低 |
| 2 | 位置通道机制 | 甲 = 求值外壳 + 侧通道（超带转乙已预登记）；乙 = 显式传 span | **甲** |
| 3 | 副作用闸门 | 甲 = 内建名扫描 + `--allow-io`；乙 = 无闸门，仅文档警示 | **甲** |
| 4 | 运行预算 | 甲 = 默认 5000 ms + `--run-budget`；乙 = 不设 | **甲** |

## 6. 验收标准（实施批；加粗项为规划者亲验）

- **L1 行为**：main 体内的除零报真实行列（覆盖 trace 零事件的缺口）；嵌套调用中被调函数内出错，落在被调函数内的失败行；实参表达式出错，落实参表达式；赋值给未定义变量，落赋值语句；`DepthLimit` 位置不变；连续两次运行无残留位置；REPL 逐条无残留。
- **L2 行为**：静态干净 + 运行时失败 → 计划含带位置的 `stage: "runtime"` 条目；静态有 error → 不执行（证法：静态有错且含死循环的程序必须立即返回）；运行通过 → `outcome: "pass"`、零计划；死循环 → `outcome: "timeout"`，在预算量级内返回；含 `file_write` 且无 `--allow-io` → `skipped`；加 `--allow-io` → 执行；程序 stdout 不泄漏进 fix 的 stdout；退出码 0 与 ≥ 2 两带。
- **安全锁**：fix_corpus 新增"运行时 hint 永不自动应用"的配对（`--apply --run` 之后源码逐字节不变）。
- **性能**：交错 A/B 基准全部负载在 ±3% 带内（规划者亲跑终判，[designs/0026](0026-agent-execution-trace.md) 先例）。
- **回归**：**verify_selfcomp 全过 + 强 quine 逐字节不变**、**doc_audit 全过**（两项规划者亲跑）、自举六模式、eval 双后端、fmt gate、clippy 与 cargo fmt 零输出、集成测试逐 target 单跑并配对上报。
- **默认路径**：不带 `--run` 的 `fix` 输出与改前逐字节一致（A/B 对照）。

## 7. 文档成对面

- [SPEC_FOR_AI.md](../../SPEC_FOR_AI.md)：运行时诊断带位置；`fix --run` 使用指南（LLM 是第一消费者）。
- [LANGUAGE_SPEC.md §13](../../LANGUAGE_SPEC.md)；README 当前版横幅；HANDOVER §1 与 §2.2。
- [eval/README.md](../../eval/README.md) 运行时题纪律段："fix 管线静态驱动"的登记对 `--run` 形态不再成立，如实改写。
- [specs/acri/lom-conformance.md](../../specs/acri/lom-conformance.md)：AXIOM 3 行的 `--run` 形态登记；运行时位置口径。
- [SECURITY.md](../../SECURITY.md)：`fix --run` 会执行程序——opt-in、闸门、预算三件。
- doc_audit 锚按 FAIL 指引同步。

## 8. 不在本批 / 后续候选（均不构成预授权）

- **L3 自动可应用的运行时修复**（如由运行时诊断驱动补 import、补通配臂）：六个 RUNTIME 臂今天全是 hint，自动改写需要新的修复逻辑与独立的安全论证，另立设计。
- **价值测量**：以 `fix --plan --run` 的 JSON 作为 round-2 回喂，对照"回喂 stderr"做迭代修复采样。需要 LLM 采样预算，另批呈裁。
- **playground 暴露 `fix --run`**：gh-pages 部署是独立呈批件。
- **WASM AOT 通道的运行时位置**：需要调试信息，不在本批。

## 9. 风险与如实边界

- **价值未测量**。本设计不含任何修复率数字。在已测量的 131 族里，静态通道本就有带位置的诊断（§1.4），`--run` 的增量是"机器可读，并确认哪条静态预言兑现了"；对 137-139 族它才是唯一通道，但增量价值同样未测量。
- **位置精度有限**（§3.2）；**解释器热路径改动**（以 A/B 基准为门）；**效应系统不可作闸门**（已改用内建名扫描）。
- **评测资产**：grep 显示 `eval/tasks/` 不含 `(0:0)` 文本，预期零影响。若实施中发现逐字采集块因位置变化而失配，eval_prompt_check 会红，届时按生成纪律重采并登记。任务 130 的失败形态是深度超限，本就带位置，预期不变。
- **升版**：新增 CLI flag 属用户可见新功能，按 MAINTENANCE §2.2 升 minor（`--trace` 批 v1.14.0 先例）。若裁决点 1 取乙，L1 单独按"行为修复、诊断文案修正"升 patch。
