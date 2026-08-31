# RFC 0003: Phase 8 —— 全量自举（Lom 写的完整 Lom 前端 + 检查器 + 解释器）

- **RFC**: 0003
- **Title**: Phase 8 — full self-hosting: a complete Lom front end, checker, and tree-walking interpreter written in Lom
- **Status**: draft
- **Created**: 2026-08-25

## Motivation

Phase 5 已用 1450 行的 `stmt_interp.lom` 验证"编译器流水线能用 Lom 写"（词法→语法→静态检查→求值四段，14 程序 39 条输出逐字 golden），但其迷你语言经逐项对账（2026-08-25 调研，证据见下）只是完整 Lom 约 20% 的子集——无 Float/Record/Tuple/Map/enum/match/闭包/管道/`?`/效应标注/模块/for/range/复合赋值/字符串转义/注释。Phase 5 收尾评估的原话是："完整 Lom 编译器的全量自举是独立 phase 级工作量"，正式移交编译器阶段（guide §2.7）。

RFC-0002 把全量自举正式列为 Phase 8 候选，并指出其硬阻塞项是递归深度天花板（5.18 实测树遍历 ~10⁴ 层）。Phase 7 已把该天花板变为宿主栈可调（V8 默认 ~1-3 万层，`node --stack-size=60000` 下 10⁵ 层实测通过，HANDOVER §10），且 stmt_interp 编译到 WASM 后 39 条输出与 golden 逐字一致（7.6b/7.9 已验证，CI 有 gate）。**前置解锁完成。**

全量自举对 Lom 有不可替代的自证价值：

1. **最大号的 dogfooding**：Lom 定位"LLM-repair-native"，声称"工具链本身也要 LLM 可读可修"。用 Lom 写完整 Lom 解释器（预计数千行）是对语言表达力、可维护性、诊断体系最狠的一次实测——任何 eval 任务集都替代不了。
2. **可移植的参考前端**：自举前端（lexer+parser+checker）是纯计算的 Lom 代码，天然同时跑在树遍历和 WASM 两个载体上，等于一份"可执行的 LANGUAGE_SPEC"。
3. **为 v1.0 冻结做最终压力测试**：冻结前让语言养活自己一次，漏掉的表达力缺口会全部现形（Phase 5 就靠这个抓出 6 个语言缺口）。

## Proposal

### 定义：什么算"全量自举"（防范围滑移）

- **L1 自举解释器 —— 本 RFC 的目标**：用完整 Lom 写完整 Lom 的前端（lexer + parser）+ 静态检查 + 树遍历求值器（`self_interp.lom`）。它能解释执行任意合法 Lom 程序，**包括它自己的源码**。
- **L2 自举编译器 —— 明确不做，挂账**：Lom 写的 WASM 代码生成后端。被 `char_from_code` 缺口硬阻塞（WASM 字节无法用 Lom 字符串构造，file_write 只接受 String；2026-08-25 调研核实）。可行走"宿主中介"路线（Lom 产字节整数列表、宿主写字节，与 7.7 json 宿主中介同构），但工作量与语言面取舍超出本期，移交 post-Phase-8 单独立项。

### 语言面承诺（冻结声明）

- **零新语法、零新关键字、零新诊断码**。自举所需表达能力已全部在场：AST = 递归 enum（recursive_enum.lom 实证）、符号表 = Map（stmt_interp 5.21 实证，压测 3.7-8.3×）、位置信息 = Record（5.17 实证）。
- **零新内建**。调研逐项结论：reflection 三件套（type_of/record_items/char_from_code）对 L1 不需要（match 即分派，不需要 type_of）；`join` 不需要（增量输出绕开拼接 O(n²)）；`char_from_code` 只阻塞 L2。
- 若实施中撞出非加不可的语言面需求，**回本 RFC 修订，不偷偷加**。
- 宿主工具面允许的小改（非语言面）：parser AST dump 调试输出（8.1 验收用）、三层 golden 的 CI 基建。

### 语义基准与已知差异

自举解释器以**树遍历解释器为语义基准**（其"参考实现"地位是 RFC-0002 定的）。双后端已如实记录的 4 条语义差异（闭包捕获值拷贝 vs 共享作用域、除零 trap 消息、trim ASCII vs Unicode、JSON 数字 Int/Float 判定）在 WASM 载体验收时逐条对齐或显式记录，不掩盖。

### 分阶段里程碑（Phase 8.x）

| 子阶段 | 内容 | 验收 |
|---|---|---|
| 8.1 | 完整前端 in Lom：lexer（宿主全 token 集：20 保留字、全运算符、字符串转义、注释、换行语句分隔语义）+ parser（完整优先级表；match/闭包/管道/`?`/效应标注/enum/record/tuple/range/复合赋值/for/import 全形态；容错解析对齐宿主 holey AST 哲学）。输出 AST（递归 enum）+ 诊断 List | 对 examples/ 全部 35 个 `.lom` + eval 108 参考解：自举 parser 的 AST dump 与宿主 dump 逐字一致；诊断（码 + 位置）对齐。**本期最大的一块** |
| 8.2 | 静态检查 in Lom：未定义变量/函数（NAM003 形态）、arity（TYPE003 形态）、效应标注（EFF001）、match 穷尽（MAT001）。类型检查全量移植挂账（见 Unresolved） | 对 eval 含诊断任务 + effects_bad.lom 等：诊断码与消息模板逐字对齐，位置到行 |
| 8.3 | 求值器 in Lom：值系统全覆盖（Int/Float/Str/Bool/List/Map/Record/Tuple/Enum/Closure/Unit）；42 个内建逐案决策（原则：纯计算自实现，IO 透传宿主——宿主内建即自举机的"硬件指令"）；闭包词法作用域对齐基准语义 | 自举解释器跑 examples 子集 + stmt_interp 14 程序 39 条 + eval 子集，stdout 与宿主逐字一致 |
| 8.4 | WASM 载体 + **自证闭环**：self_interp.lom 经 `lom build --target wasm` 编译，Node harness 下跑同一 golden；然后 wasm 载体上的自举解释器**解释执行 self_interp.lom 自身源码**，跑同一批测试程序 | 三层输出逐字一致：宿主跑自举 / wasm 跑自举 / wasm 跑自举套自举（quine 式验证；递归深度靠 `--stack-size`，规模上限实测后写入） |

### 退出标准（Phase 8 done 的定义）

1. 自举前端覆盖完整 Lom 语法零排除（8.1 验收集全绿，含 self_interp.lom 自身源码可解析）；
2. 自举解释器语义对齐：8.3 验收集 stdout 与宿主逐字一致；
3. **自证闭环达成**：8.4 三层 golden 逐字一致；
4. CI 加自举 gate（三层 golden 逐字比对），全部既有 gate 不倒；
5. 零依赖 / 零 unsafe 不破；语言面冻结声明未被违反（违反处必须回本 RFC 修订记录）。

### 性能预算（诚实账）

解释器套解释器的常数因子预估 ~100-1000×（5.18-5.21 的实测外推，非实测）；WASM 载体 ~49×（7.10 实测）追回一部分。因此自证闭环（三层）只跑小程序套件，全量 eval 不在三层验证范围内——规模边界在 8.4 实测后写进验收记录，不打肿脸充胖子。

## LLM-impact analysis

- **语言表面零变化**：无新语法/关键字/诊断码/内建。SPEC_FOR_AI 唯一可选变更是交付后加一节"自举资产"导航。
- **自举源码是 LLM 可读性的终极实证**：数千行真实编译器代码全程 Lom 写就，直接成为"LLM 生成 Lom"的最大训练参照物与修复闭环（`lom fix`）的最佳压力测试场。
- **最大长期风险是语义漂移**：一个语义三个实现（宿主树遍历、宿主 WASM、自举）。缓解就是退出标准 3/4——三层 golden 逐字比对进 CI，漂移当场爆炸（沿用 RFC-0002 的同一招）。

## Alternatives considered

- **直接做 L2 自举编译器（Lom emit WASM 字节）**：拒绝。被 `char_from_code` 硬阻塞——要么破冻结加内建，要么宿主中介写一半；且 L1 的前端与检查层是 L2 的必经前置，先做 L1 不浪费任何工作。
- **继续迷你语言路线加深（stmt_interp 6.0）**：拒绝。迷你语言与完整 Lom 的差距表（26 项缺失）证明"加深"等于重做，不如直接对标完整语言，验收标准也更硬。
- **不做全量自举，直接 v1.0 冻结**：拒绝（用户 2026-08-25 已裁决方向为 Phase 8）。
- **自举全量 typechecker 移植**：挂账。渐进式类型是 warning 层（不拦截执行），自举侧 ROI 低；见 Unresolved。
- **为自举加 `join`/`char_from_code` 内建**：拒绝（L1 范围内）。逐案论证均非阻塞，冻结倾向优先；L2 立项时重新裁决 `char_from_code`。

## Drawbacks

- 诚实的工作量账：这是**项目迄今第二大单项工程**（仅次于 Phase 7）——完整 parser 在 Lom 里预计数千行，8.1 一块就可能超过 Phase 5 全部自举工作量。
- 永久维护税加层：语言变更理论要同步三个实现。缓解：语言面冻结倾向（RFC-0001 已拒四个扩张），特性增速趋零。
- 验收基建成本：宿主 AST dump 输出、三层 golden CI、自证闭环的规模标定。
- 解释器套解释器的性能注定了自举解释器是"正确性工件"而非"生产运行时"——它不会替代任何现有执行路径。

## Unresolved questions

1. 自举静态检查的深度边界：NAM/arity/EFF/MAT 子集（倾向）vs 全量 typechecker——8.2 开工时最终定。
2. 42 个内建"自实现 vs 透传"的逐案清单：8.3 开工时定（原则已定：纯计算自实现，IO 透传）。
3. 宿主 AST dump 的形态：`--dump-ast` flag vs 独立调试子命令——8.1 开工时定。
4. 自证闭环的测试程序规模上限：8.4 实测三层常数因子后定。
5. L2（自举 WASM emitter）立项时机与 `char_from_code` 裁决：post-Phase-8。

---

### 附：2026-08-25 调研事实底账（迷你语言 vs 完整 Lom 差距，证据在 stmt_interp.lom / interpreter.rs / LANGUAGE_SPEC.md 对应行号）

- 迷你语言值系统：`VInt | VStr | VBool | VList`（stmt_interp.lom:103-108）；缺 Float/Map/Record/Tuple/Enum/Closure/Unit。
- 迷你语言语法：6 关键字（let/if/else/end/while/fn）、两级优先级、无 return/break/continue、无 for/range/闭包/管道/match/enum/效应/import/复合赋值/类型标注/字符串转义/注释/换行语义。
- 迷你语言静态检查：未定义变量/函数 + arity 三类（stmt_interp.lom:1183-1331），无类型检查、无效应检查。
- 宿主内建 42 个（interpreter.rs module_of 全表），自举迷你版 6 个（try_intrinsic）。
- 语言面缺口逐项裁决：`split(s,"")` 逐字符 + 字符串比较足够手写 lexer（stmt_interp 已实证）；file_read 失败即致命错误可用 file_exists 预检绕开；字符串拼接 O(n²) 用增量输出绕开；list_get O(n) 用顺序消费 token 流绕开（stmt_interp 的 `(value, rest)` 元组风格即正解）。
