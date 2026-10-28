# 提案层执行波设计批：B3 迭代修复效率题型 + C4 信封 position 物理统一 + D5 `lom explain`

- **文档**：docs/designs/0027-iterative-repair-envelope-explain.md
- **性质**：动工前置设计（MAINTENANCE §2.1 第 2 条——涉评测面扩容（宣称面）、宿主工具面（check --json 信封 + 新 CLI 子命令）；先例 0001-0026）
- **基线**：HEAD `e79ec64` / v1.14.0（工作树干净；台账零 open，顺延号 R229 起）
- **状态**：**设计批产出（2026-10-28，用户裁决"执行 B2 B3 C4 D5"）——三件裁决点待用户裁决后动工**。B2 已先行交付（`e79ec64`，docs/REPRODUCING.md，纯文档批免设计）。本设计批零代码改动。
- **供料来源**：两路执行者行号级供料——`target/probes/c4d5_supply/report.md`（C4/D5：ACRI 条款原文/Diagnostic 构造点 diagnostics.rs L86-98 与序列化点 L399-400/消费面盘点/过渡选项破坏面/eval 25 题内嵌 line/col 风险量化/codes.rs 46 条注册表/数据源矛盾发现）+ `target/probes/b3_supply/report.md`（B3：runner 现状/协议三选/127 题 1/10 单轮低通过与取证档案/no-op 陷阱题 091/098/099/采样预算/类别落位两案）。关键结论已由规划者亲核（供料行号抽查 + 任务号缺口 108 复算）。
- **数字落盘前门禁**：写本文前后运行 `python tools/doc_audit.py`。

## 0. 总裁决点一览（详见各节）

| # | 裁决点 | 选项 | 规划者建议 |
|---|---|---|---|
| 0-1 | 三件实施切批 | 甲=C4/D5/B3 三批顺序 / 乙=C4+D5 合一升版批 + B3 独立批 | **乙**（v1.15.0 装 C4+D5；B3 后续 v1.16.0——评测采样另批） |
| 0-2 | B3 协议 | A 对话带历史 / B 单步反馈链 / C 带 diff | **B** |
| 0-3 | B3 主口径 | 轮数分布 / C(N) 曲线 / 双报 | **双报** |
| 0-4 | B3 轮数上限 R | 3 / 5 / 10 | **5** |
| 0-5 | B3 题集 | 缩幅 8-12 首发含 127、排除 no-op 三题 / 全 32 | **缩幅首发** |
| 0-6 | B3 采样预算 | glm-5.3 单模型零边际成本 / 双模型 ~$40-60 | **glm 单模型首发**（双模型留正式测量批） |
| 0-7 | B3 类别落位 | 并入 error_repair（仓库加题先例基因）/ 新类别 11_iterative_repair | **新类别**（锚面隔离——32 题宣称锚多处；eval_prompt_check 参数化 ~5 行） |
| 0-8 | C4 过渡形态 | A 双写 append-only / B deprecated 保留 / C 直切 | **A**（ACRI 前向兼容公理 + l2-fix v2 先例；eval 25 题内嵌诊断 verbatim 保持） |
| 0-9 | C4 §1.1 规范歧义处置 | a=随 ACRI v0.2 勘误（anchor:"none" 时 line/col 允许 0——与独立仓批联动）/ b=仅登记暂缓 | **a**（登记歧义随③独立仓批一并升 v0.2；Lom 侧本批不新造形态——LEX000 照实 0:0 + anchor:"none"） |
| 0-10 | C4 升版定性 | patch（行为兼容扩展）/ minor（用户可见新能力两读） | **随 0-1 乙案并入 minor 批**（若甲案则 patch） |
| 0-11 | D5 数据源 | A=codes.rs 单源内嵌文案 + doc_audit 源码计数锚 / B=SPEC_FOR_AI §11c 先改逐码表 / C=独立数据文件 | **A**（单源无漂移面；B 前置最重；C 引入第二数据文件） |
| 0-12 | D5 出口形态 | 双态（人类可读默认 + --json，与 codes 一致）/ 纯文本 | **双态** |
| 0-13 | D5 文案内容 | explain 专属新写（含一句修复方向）/ 复用 fix.rs 分派 hint | **新写**（三源文案粒度不同，复用会牵 fix 行为面） |

## 1. B3 迭代修复效率题型（多轮收敛度量）

### 1.1 动机与可测假设

RepairLoopBench 现测"一次修对"（pass@1 = 单轮特例）；repair-native 的核心主张"写错后高效修复"缺"**几轮修对**"的度量。供料实证的核心可测假设：**任务 127**（L2 跨编译器修复，双模型单轮 1/10——20 候选 18 个败于同一 RUNTIME000、缺同一 Option 解包步）——round 2 喂入运行时诊断后补齐率 10%→多少，就是修复闭环价值的直接量化。

### 1.2 协议（裁决 0-2；建议 B 单步反馈链）

每轮 = 单消息调用：round 1 发标准题 prompt（与 pass@1 完全同构——C(1)≡pass@1 可校准）；round N>1 发"代码 + 上轮实跑的真实诊断"（`lom <file> --check --json` 与运行时输出按现有双真实采集纪律），**模型每轮回写完整代码**（"代码即记忆"——如实命名，不称对话式）。早停：通过（stdout+rc0 同口径）即止，或达 R 上限记删失。备选 A（多消息带历史）最贴真实但轮间震荡干扰度量；C（带 diff）引入 diff 格式协议变量。

### 1.3 度量与题集（裁决 0-3/0-4/0-5）

- 双报：每题轮数分布（中位/均值/P90 + **删失率单列**）+ C(N) 收敛曲线（C(1)≡pass@1 锚点校准）。
- 题集缩幅首发 8-12 题：127 必选 + 现有 32 题 error_repair 中单轮非满通过/有修复深度的题 + **排除 091/098/099**（no-op 修复题——solution 嵌于 prompt，多轮下 round 1 即收敛零分辨力）+ 新题 2-3 道（面向"round 2 诊断驱动修复"设计——单轮信息不足、诊断补足后可解的形态）。
- 新类别 11_iterative_repair（裁决 0-7）：manifest/README/REPRODUCING 独立段，32 题锚面零触碰（B2 昨日刚写 "32 error_repair"，并入即连锁）。

### 1.4 工程与预算（裁决 0-6）

run.ps1/run.sh/_generate.ps1 **零改动**（评分口径不变）；新增 `eval/iter_eval.py`（逐题 prompt + 每轮实跑取诊断 + 轨迹布局 + 早停，~150-250 行）+ 汇总脚本（~80-120 行）。eval_prompt_check 新类别参数化 ~5 行。采样：整分类打包在多轮下不可用（round 2+ 逐题不同）→ 逐题调用；glm-5.3 = Coding Plan 存量通道**零边际成本**首发；缩幅首发 ~$0-10。升版 minor（v1.11.0 评测扩容先例）；正式测量报告另批（含双模型）。

## 2. C4 check --json 信封 position 物理统一

### 2.1 目标与现状

ACRI §1.1 要求嵌套 `position{anchor,line,col}`（anchor 三态 exact/name-first/none）；Lom 现状 = line/col 顶层字段（diagnostics.rs L86-98，序列化点 L399-400）——conformance 表登记 "Conformant by mapping"。物理统一后 Lom 变 native conformant，消"参考实现自己都要映射"的推广软肋，且是③ conformance suite 的直接可验证成果。

### 2.2 过渡形态（裁决 0-8；建议 A 双写 append-only）

信封序列化追加嵌套 `position` 对象（anchor + line + col），顶层 line/col **保留**（标注 deprecated 语义于 ACRI 映射表，不删字段）——ACRI 前向兼容公理（consumers MUST ignore unknown fields）+ l2-fix v1→v2 append-only 先例同款。**anchor 判定**：check 信封全部诊断通道（lex/parse/type/name/...）均带精确行列 → 统一 `"anchor":"exact"`；LEX000 输入不可读形态 0:0 照实 + `"anchor":"none"`（不伪造坐标——§7 负向锁定原则）。破坏面（供料量化）：Rust 消费全走结构体零影响；playground/l2fix.py/consumer.py 零 JSON line/col 消费；文档示例改 4 处；**eval 32 题中 25 题内嵌顶层 line/col 在 A 案下 verbatim 保持**（顶层字段不删）；C 案（直切）破 verbatim 一致性——弃。

### 2.3 规范文本联动（裁决 0-9）

供料两发现：① §1.1 Example 自称 "captured verbatim from the reference implementation"——Lom 升级后该示例须同步刷新（规范文本**不是**完全不动）；② §1.1 规范歧义：position 写死 `"line": ≥1` 但 anchor:"none" 语义未定 0:0 处置，而 Lom LEX000 现状即 0:0。处置：登记歧义 → 随③独立仓批升 **ACRI v0.2**（勘误：anchor:"none" 时 line/col 允许 0 + 刷新 Example + 增补"同版本号内容不可变"与 tag/changelog 版本载体条款——用户 2026-10-28 方向已认）；本批 Lom 侧照实输出不新造形态。§1.2 枚举 by-mapping 行**本批不消灭**（JSON 输出面 confidence/action 已小写——属文档澄清非代码缺口，随③ suite 批处置）。

### 2.4 边界与验收

quine 272925/368/368 构造性不变（零 codegen 触碰，verify_selfcomp 零 --json 消费实证）；新单测锁信封形态（position 在场 + anchor 判定 + 顶层字段保留）；eval_prompt_check 32/32 不动（A 案 verbatim）；lom-conformance.md 两处 by-mapping 行改写（§1.1 转 native、§1.2 标注移交③）；doc_audit 无信封锚、SPEC_FOR_AI 尺寸行锚随文档改动联动。

## 3. D5 `lom explain <code>`

### 3.1 目标与条款

ACRI §2.1 OPTIONAL："Long-form documentation for one code"——格式开放。现状 `lom explain` rc=2（被当文件名，cli.rs L352-356 多文件守卫）。兑现后 L1 符合性从 codes/explain 双出口齐备（explain 将成 codes.rs lookup 的首个生产消费方）。

### 3.2 数据源（裁决 0-11；建议 A codes.rs 单源）

供料证伪了"从 SPEC_FOR_AI 码表生成"预设：§11c 是范围式叙述非逐码表；逐码权威源 = codes.rs CODES（46 条，severity/分类/一句话齐备）。方案 A：explain 专属长文案（每码 2-4 句：语义/常见成因/修复方向）**内嵌 Rust 常量表与 CODES 同文件**（编译期计数一致性由单测锁：explain 文案条数 == CODES.len()），doc_audit 加源码计数锚（文案标记清点 → 文档宣称位对照——R27/R28 型钉子同款）；SPEC_FOR_AI §11c 增补指针句指向 `lom explain`（人读详表 = 工具出口，单一事实源）。方案 B（§11c 先改逐码表）前置最重且文档态与代码态双写；C（独立数据文件）引入第二份机器可读资产。

### 3.3 出口与文案（裁决 0-12/0-13）

双态：默认人类可读（码/族/严重级/长文案），`--json` 出 `lom-explain/v1` 信封（与 lom-codes/v1 同族字段 + doc 字段）；文案新写（不复用 fix.rs 分派 hint——两处文案粒度/受众不同，复用会把 explain 出口与 fix 行为面耦合成三源归一问题）；未知码 → 结构化错误 + 近邻建议（同族码列表）。冻结面零触碰（新子命令非语言面）；升 minor（新 CLI 子命令——决策表 L144 字面命中）。

## 4. 实施切批（裁决 0-1）与门禁

- **乙案（建议）**：实施批一 = C4+D5 合并升 **v1.15.0**（minor——D5 新子命令定性覆盖 C4 兼容扩展；一次锚同步/一次 CI 弧线/文档成对一次）→ 实施批二 = B3 工程面升 **v1.16.0**（题型+脚本+manifest）→ 采样报告批（零升版，评测报告落档）。
- 甲案：C4（patch v1.14.1）→ D5（minor v1.15.0）→ B3（minor v1.16.0）——弧线多两笔、门禁多两轮，收益仅是升版表更"纯"。
- 全批门禁：doc_audit（D5 加锚后项数 78→79 联动）/ link_check / HANDOVER §2.2 全量回归（C4/D5 触 src/ 加存量行为对拍：eval 双后端 129×2 + fix_corpus 六模式 + LSP e2e 不倒）/ 文档成对（README/SPEC_FOR_AI/SPEC §13/ACRI 映射/TODO）。
- 探针：C4 信封 A/B 对照（升级前后 --json 双跑 diff 恰增量 position 字段）；D5 全码逐条出口快照。
