# R110 邻接面：块内 LetDestruct 同名遮蔽快照/恢复设计（同族第三步推广）

- **状态**：**已交付（2026-10-01，RFC-0004 修订 48；仓库版本 v1.4.15，tag/CI 门禁回填后切）**——用户裁决"继续执行"（规划者菜单建议项 1）。执行者实施（mod.rs +18/-6 + 新单元 ×5）、规划者验收（diff 亲读、cargo test 559、verify_selfcomp 365/365 亲跑、p13 正/负原形三路径亲验、fix 交互抽查）。验收数字：Rust **554→559**、全仓 4300 文件 --check 对拍 MUT001 面 **DIFF=0**（改前/改后各 10 条全为真不可变；"9 条"按文件口径随批统一按行）、365/365 与 quine 269555 不变（self_comp 零改动双跑）。设计"预期 DIFF=0"与实测一致。
- **设计基线**：HEAD `70e97ab` / v1.4.14（tag `e8b099e`，CI #237 绿）。语言面 v1.0 冻结与外部发布线继续冻结；本批为纯 typecheck 诊断面 patch，无新语义选边（快照/恢复既定修法的自然推广），升 **v1.4.15**。
- **问题登记来源**：二十四审报告 §3.1 边界发现 + §3.4 p13 实测（误报 1 条 MUT001 维持、运行正确）；RFC-0004 修订 47 邻接面登记；TODO 顶部"登记在案不修边界"首项。
- **数字落盘前门禁**：写本文前后运行 `python tools/doc_audit.py`。

## 1. 供料结论（规划者亲读，src/typechecker/mod.rs @ 70e97ab）

1. **check_block（L427-449，v1.4.14 R110 交付）**：进入块时建 `saved: HashMap<String, Option<(TypeOrUnknown, bool)>>`；逐 stmt 遍历，快照条件 `if let Stmt::Let { name, .. } = stmt && !saved.contains_key(name)` **只匹配 Stmt::Let**；全部语句与尾表达式检查完后按 saved 恢复（同名恢复原条目、无同名 entry=None 不弹出）。
2. **Stmt::LetDestruct（L487-516）**：两分支均 `env.define(name, ..., false)`——元组匹配分支（L505-507）带元素类型、未知类型分支（L510-514）绑 Unknown，**恒不可变、不进快照**。块内 `let (x, y) = …` 同名遮蔽外层 `let mut x` 后，可变性条目被覆盖为 false 且块退出不恢复 → 块外 `x = 5` 误报 MUT001（二十四审 p13：1 条 MUT001 (8:5)、运行 10/20/5 正确按块作用域）。
3. **漏报方向不适用**：解构绑定恒 false——不存在"块内解构把不可变外层变成可变"的路径；外层 `let x`（不可变）+ 块内解构遮蔽 + 块外赋值现状即正确报 MUT001（负向，修后必须不倒）。
4. **for var 快照（L620-625）独立叠加先例**：体恢复在前、var 恢复在后；本批不动它，for 体内解构遮蔽经 check_block 统一受益。
5. **match 臂走 env.child()（check_match L1158）**：臂内 MatchArmBody::Block 调 check_block 但同名条目落 child 层，不经父层恢复面——解构同此，无新面。
6. **fix 引擎（R65/R75）不动**：flat_binds 的"最近同名绑定是解构 → 降级 hint"是 AST 词法链逻辑，与 typechecker 的块级恢复语义分层。修复后"外层 let mut + 块内解构 + 块外赋值"不再报 MUT001（fix 无触发点）；"外层 let（不可变）+ 块内解构 + 块外赋值"仍报真 MUT001，fix 对其维持保守降级（二十四审 p13_fixcopy 实测形态，不误改）。fix 交互只验证不倒，不改 fix 代码。

## 2. 设计（最小面）

**改动点唯一**：check_block 的快照条件从只匹配 `Stmt::Let` 扩为也匹配 `Stmt::LetDestruct { names, .. }` 的每个 name（同样仅首个同名快照、恢复到进块前条目）。预计 +6/-1 行。

- 恢复语义沿 R110 四不变量：逐名首现快照（快照点在 define 前即进块前条目）、尾表达式之后恢复（尾表达式可见块内绑定）、同名恢复原条目/无同名保留不弹出（泄漏 divergence 维持 v1.4.13/14 先例）、嵌套各层各自 saved（内层先恢复）。
- 解构内同名（如 `let (x, y)` 后续又 `let x` 或再解构含 x）：沿"仅首个同名快照"——恢复到进块前条目（与 StLet 链式同构，r110 p7/p17 先例）。
- 类型恢复维度（R110 p18/p19 先例）：恢复的是 `(TypeOrUnknown, bool)` 成对条目，解构遮蔽外层 String/Int 类型同样恢复。

## 3. 测试规划（src/typechecker/tests.rs 新单元，沿 r110 族命名）

1. `r110_destruct_shadow_mut_outer_postblock_reassign_no_mut001`（**修复目标**）：外层 `let mut x` + if/while/for 三块型体内 `let (x, y) = (10, 20)` + 块外 `x = 5` → 零 MUT001（修复前各 1 条误报）。
2. `r110_destruct_shadow_immutable_outer_postblock_reassign_mut001`（**负向不倒**）：外层 `let x`（不可变）+ 块内解构遮蔽 + 块外 `x = 5` → 仍 1 条 MUT001、定位块外赋值行。
3. `r110_destruct_partial_name_restore`：外层 `let mut y` + 块内 `let (x, y) = …` → 块外 `y = 5` 零 MUT001（部分同名恢复）；块外读/赋块内新名 x 维持既有 divergence（读放行零 NAM003、赋值报 MUT001——无同名不弹出先例）。
4. `r110_destruct_unknown_ty_branch_same_restore`：解构值为未知类型（如无注解函数调用返回）分支同样恢复（两分支都走 define false）。
5. `r110_destruct_nested_and_chain`：嵌套块内解构遮蔽（内先外后恢复）+ 同块解构后再 `let x` 链式（恢复进块前条目）。

维持面（不新设测试、跑既有全绿即可）：MUT001 族 8 + R65 族 6 + for quirk 5 + r110 族 6 全量；泄漏 divergence 两形态维持。

## 4. 验收门禁（v1.4.15）

- 新单元全绿 + 既有测试全绿：cargo test --release **554→554+N**（N=新单元数，J 锚同步）。
- **全仓 .lom 新旧 --check 对拍**（R110 先例）：4300 文件，预期 MUT001 面 DIFF=0（二十四审全仓扫描 9 条全为真不可变，无一例块内解构遮蔽形态——预期零差异，实测为准、如实记录）。
- 全量回归：六模式 / verify_selfcomp **365/365**（typechecker 改动不触 codegen，构造性不变）/ --bootstrap **14/14** quine **269555** 不变 / eval 双后端 121+121 / clippy·fmt 零 / doc_audit **71/71**（J 锚 Rust 计数同步后）/ spec_examples / eval_prompt / fmt gates。
- 探针抽查（规划者亲验）：p13 原形（外层 let mut + 块内 `let (x, y)` + 块外 x=5）三路径（--check / run / build wasm）零 MUT001 且运行三侧一致；负向原形仍报。
- fix 交互抽查：p13 变形（外层 let 不可变 + 块内解构 + 块外赋值）`lom fix --dry-run` 维持保守降级 hint 不误改（R75 路径不倒）。

## 5. 文档成对与顺刷（随本批一并交付）

1. RFC-0004 修订 48（交付记录，沿整改链 41-47 先例）；LANGUAGE_SPEC §13 v1.4.15 条目。
2. **SPEC §4 L344 surfacing 归属改写**（二十四审精度备注 2，建议随本批）：句尾 "allows it with NAM006/PKG007 surfacing" 的归属歧义——改写为 import/包碰撞面归 NAM006/PKG007（§8.1）、块级遮蔽静默但块退出恢复可变性标志的如实口径。
3. TODO 顶部：交付记录段 + 登记边界行去掉"块内 LetDestruct 同名遮蔽"项 + **无文件 build 不检查主文件一句话登记**（二十三审备注 2 维持面）。
4. HANDOVER 顶部条目 + §1 快照 + §2.2 基线 + §9-5；HANDOFF_PROMPT【当前真实状态】整体重写，顺刷 **clippy --all-targets 登记值**（v1.4.9 时点"9 条"→ 当前实测 ~4 条，二十四审备注 1）与 **L67 历史快照段"当前 open"→"当时 open"**（二十四审备注 3）。
5. README Current release 段 + positioning 数字口径段（J 锚位随实算）。
6. 升版 v1.4.15（Cargo.toml/lock 同步）；提交推送看 CI 首跑，绿后切 tag。

## 6. 冻结面与边界

- 语言面 v1.0（语法/20 关键字/诊断码/43 内建）零变化；MUT001 既有 warning 级别与文案不动（只修判定面）；发布线继续冻结。
- 不触碰：fix 引擎、code 生成器、self_comp.lom、解释器、匹配臂 child 层语义、泄漏 divergence（维持）、循环外读 for 变量 divergence（维持）。
- 实施纪律：改前查 doc_audit/claims.json 锚；探针证据留 target/probes/（gitignored）；全量回归 + 全仓对拍后规划者验收；git 提交/推送/tag 归规划者。
