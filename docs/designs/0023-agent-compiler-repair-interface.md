# β 规范升维设计：agent-编译器修复接口开放规范（ACRI 候选名）+ Lom 参考实现

- **文档**：docs/designs/0023-agent-compiler-repair-interface.md
- **性质**：动工前置设计（MAINTENANCE §2.1 第 2 条——涉宿主工具面与规范文档面；先例 0001-0022）
- **基线**：HEAD `6d46162` / v1.9.5（交接就绪态，台账零 open）
- **状态**：**β-1 与 β-2 已交付（2026-10-12，三裁决点均按建议项：三层全包 / ACRI / 顶层 specs/）。β-1 = specs/acri/ 三件套（规范本体 v0.1 Draft + Lom 符合性映射 + 中文导读，提交 `936a37b` CI 七 job 绿）；β-2 = v1.10.0 工程批（用户裁决"按序执行"：`lom codes [--json]` 码表出口 46 条 + l2-fix/v2 位置字段双实现同步对拍 30/30 + 退出码带 20 路径 1→≥2——Rust 573→578+37→40，quine 272925 逐字节不变，**Lom 达成 ACRI L1-conformant**；specs/acri/lom-conformance.md 随批更新含两登记偏差）。β-3（参考消费器实装 + 实跑闭环）既裁按序待执行。**
- **供料来源**：执行者供料五节（A lom-diag/v1 对象模型 / B 修复建议对象与安全性模型 / C CLI 交互约定 / D RepairLoopBench 方法论 / E 外部对照提取自归档调查）+ 规划者亲验三处关键行号（Diagnostic 结构体 diagnostics.rs:86-98 十字段逐字、fix_for_diagnostic fix.rs:754 起按码分派、--apply 只 High 非 Hint 的安全性注释 :12/:3344）。供料留存 target/probes/beta_supply/report.md（gitignored）。
- **数字落盘前门禁**：写本文前后运行 `python tools/doc_audit.py`，结果 `RESULT: PASS（73/73 项通过）`（本文档不含 doc_audit 锚位新增）。

## 1. 背景与目标（为什么是规范升维）

2026-10 大调查（docs/archive/research-2026-10-11-*）三个事实构成本设计的动机基座：

1. **修复闭环四件套的 P3 玩家是 Zero、Vera、Lom 三强**——独有窗口已关，但**没有任何项目把修复闭环做成开放规范**（全是私有实现；MCP-LSP 桥沿用 LSP 结构不再定义；诊断格式无统一标准——两条均为归档调查的已核负面事实）。
2. **使用侧全赛道零真实第三方用户**——"语言被生产采用"受三死结（语料悖论/context 存量依赖/问责锚）约束；**"被引用/被用于评测/被作为参考实现"是现实生态位**（MoonBit 被 SWE-AGI 选为低污染基准宿主的先例）。
3. **中性命名先例**（LSP 改名/MCP 中立命名/AGENTS.md 捐基金会）——规范/基准层用中性名，语言名属实现层（2026-10-12 用户裁决 RepairLoopBench 命名时的论证，同源适用）。

**目标**：把 Lom 已验证的修复闭环资产提炼为一份**可被任何语言/工具链实现的开放接口规范**，Lom 作为首个参考实现——从"又一门小语言"升维为"agent-编译器修复接口"的卡位者。

## 2. 现状盘点（供料浓缩；行号 v1.9.5）

### 2.1 已规范级的资产（四 schema）

| 资产 | 形态 | 关键事实 |
|---|---|---|
| lom-diag/v1 | `--check --json` 信封 | Diagnostic 10 字段（severity/stage/code/message/file/line/col/source_line/is_hole/hint，diagnostics.rs:86-98）；信封 schema/file/ok/summary/diagnostics（:365-419）；**位置为单点 line+col 非 range** |
| lom-fix/v1 | `fix --plan/--apply` | FixAction = 4 动作（insert/replace/delete/hint）× 3 置信度（fix.rs:540/:562）；分派按码 match（:754-867）；**--apply 只自动应用 High 且非 hint**（:12/:3344 安全性注释）——安全性模型有 corpus 锁定（fix_corpus 13 对端到端，其中 04/12/13 三对 fixed==bad 锁"认知型 warning 不自动改"先例） |
| l2-fix/v1 | `l2fix --json` | 五字段（code/message/confidence/action/suggestion）；**无位置字段**（位置在 message 前缀） |
| RepairLoopBench | 基准卡 + 机器 gate | 双真实采集纪律（诊断 verbatim 采集 never hand-written）、三链参考解验证、期望锁定、stdout+rc 评分、eval_prompt_check 码多重集比对 gate |

### 2.2 与开放规范的差距（供料 8 条浓缩为主攻面）

1. **码体系私有耦合**：9 命名空间格式约定分散在四处头注（diagnostics.rs:34-38 / typechecker/mod.rs:21-24 / package.rs:15-22 / EFF、MUT 散见发射点）；且宿主码由消息中文字符串**事后分类**产生而非源头产码；**无机器可读码表出口**（Zero 有 `zero explain NAM003`、Vera 有 `vera errors --json`，Lom 缺位）。
2. **位置三套表示**：lex/parse 通道行前缀、codegen 通道 ln:cl 名字锚（miss 不伪造 0:0——v1.7.2 纪律）、L2 通道 message 内嵌——三套未统一为一种位置对象。
3. **置信度/动作枚举双体系**：fix 的 High/Medium/Low + 4 动作 vs l2fix 的 confidence/action 同义不同形。
4. **退出码未成文为协议**：实读现状（--json/--check error=1 warning=0；fix 恒 0；l2fix 分析完成恒 0）无文档承诺。
5. LSP 推送转 0-based 单字符宽 range 且丢弃 stage/source_line/is_hole/hint 四字段——适配层信息损失（规范化的附带收益位）。

### 2.3 外部对照（规范设计的坐标系）

- **Zero**：四段诊断契约 rule/expected/actual/fix + `zero fix --plan --json`（返回计划不自动改写）+ version-matched skills + conformance expected.json + evals（真实修复闭环评测）。
- **Vera**：186 稳定码 + 每错带 Fix: 指令与 spec 章节引用 + `--json`/`vera errors --json`。
- **MoonBit**：304 错误码文档但面向 agent 的 JSON 出口未见于文档（中间地带空缺证据）。
- **MCP-LSP 桥（isaacphi）**："无独立 JSON schema……底层沿用 LSP 协议类型"——搬运不再定义。

**结论**：Lom 的对象模型字段比 Zero 四段更细（stage/is_hole/hint/置信度分级/安全性语义）、比 Vera 更通用（不绑 De Bruijn）；差距集中在**出口与成文**（码表/位置统一/退出码/评测纪律），不在模型本身——升维基础好。

## 3. 规范设计（候选名见 §4；下文以"该规范"指称）

### 3.1 分层（L0/L1/L2）

- **L0 对象模型（核心，必须）**：
  - `Diagnostic` 对象：`code`（稳定字符串，格式 `<NS><NNN>`，命名空间注册于实现方码表）、`severity`（error/warning）、`message`、`file`、`position`（`{line, col, anchor}`——anchor ∈ exact/name-first/none，**允许实现只提供单点**；锚语义显式化吸收 Lom 三通道现状）、`stage`、`hint`（可选）、扩展字段开放（Lom 的 source_line/is_hole 即扩展）。
  - `RepairSuggestion` 对象：`code`（关联诊断码）、`confidence`（high/medium/low——统一枚举，吸收 fix 与 l2fix 双体系）、`action`（insert/replace/delete/hint）、`payload`（动作参数）、`position`（可选——修复 l2-fix/v1 无位置缺口）。
  - **安全性公理（规范级）**：`--apply` 类自动应用**只允许 high 且 action ≠ hint**；认知型（行为已确定、修复仅为消除歧义）建议永不自动应用——以 Lom fix_corpus 先例为规范语义来源。
- **L1 交互协议（CLI 约定）**：
  - `check`（含 `--json` 信封：schema/ok/summary/diagnostics）；`fix --plan` / `fix --apply`（plan 返回建议数组，apply 遵守安全性公理）；`codes --json`（**机器可读码表出口**——新增要求）；`explain <code>`（可选能力）。
  - **退出码矩阵**：check error=1 / warning=0 / clean=0；fix 恒 0（分析完成语义）；分析失败（工具自身错误）≥2——以 Lom 现状成文。
- **L2 评测方法论（附录级）**：修复闭环基准的防污染构造纪律（双真实采集/合成声明、参考解多链验证、期望锁定、评分口径、码多重集机器 gate）——从 RepairLoopBench 提炼为可复用清单。

### 3.2 Lom 参考实现符合性映射（规范 ↔ 现状）

| 规范要求 | Lom 现状 | 符合性 |
|---|---|---|
| Diagnostic 对象 | lom-diag/v1 十字段 | **已符合**（position 三通道映射 anchor 语义） |
| RepairSuggestion + 安全性公理 | lom-fix/v1 + fix_corpus 锁定 | **已符合**（l2-fix/v1 补位置字段后符合） |
| check/fix/退出码矩阵 | 实读现状 | **成文即符合** |
| codes --json 码表出口 | **缺位** | 需 β-2 工程（触 src/ CLI 面） |
| explain（可选） | 缺位 | 可选实施 |
| L2 评测方法论 | RepairLoopBench | 提炼成文 |

### 3.3 治理

- 规范自身 semver（v0.1 draft 起）；变更协议（append-only 字段、breaking 需 major）；维护主体现阶段 = Lom 项目；**未来捐赠路径预留**（先例 MCP/AGENTS.md → 基金会）——文档内显式声明"实现方中立"条款（规范不要求实现方采用 Lom 语言或任何特定运行时）。

## 4. 命名候选（裁决点 2）

| 候选 | 形态 | 利弊 |
|---|---|---|
| **ACRI（Agent-Compiler Repair Interface）**（规划者建议） | 三词缩写 | 与 LSP/MCP 命名法同构（三词、实现方中立、可念）；无撞名 |
| Repair Contract Protocol（RCP） | 用赛道流通词 | 概念对齐度高，但 "Repair Contract" 是 Zero 文档标题词——跟随者观感 |
| 不命名 | 文档名即规范名 | 最保守，引用不便 |

## 5. 实施分期（β 批次拆解）

- **β-1（纯文档批）**：规范本体 v0.1 draft（L0/L1/L2 按裁决范围）+ Lom 符合性映射表 + 中立条款与治理段——落位按裁决点 3。
- **β-2（工程批，触 src/ → minor）**：`codes --json` 码表出口（含 9 命名空间 + L2xxx 七族注册表化——顺带根治"码由消息事后分类"的源头产码差距位评估）+ l2-fix/v1 补位置字段（schema v2，消费面同步）。
- **β-3（验证批）**：第三方消费示例——一个不依赖 Lom 语言的极简 ACRI JSON 消费器（如 50 行 Python：读 check --json → 按 confidence 过滤 → 调 fix --plan 展示闭环），证明规范可脱离 Lom 语言被消费。
- 每批独立全量回归 + 提交推送 + CI；β-2 升 minor（用户可见新 CLI 面）。

## 6. 边界与不做

- 本设计**不改变** Lom 任何现有诊断码/消息/行为（规范吸收现状而非改造现状——三通道位置以 anchor 语义显式化，不改实现）；
- 不进 CI 新 gate（规范文档过 doc_audit/link_check 既有门禁即可）；
- 对外发布（规范独立仓库/目录站登记/发帖宣传）**不在 β 批内**——届时逐项呈批（铁律 6）；
- 语言面 v1.0 冻结零触碰。

## 7. 裁决点（三个，待用户）

1. **规范范围**：三层全包（L0+L1+L2，规划者建议——L2 是差异化资本且零工程成本）/ 仅 L0+L1 / 仅 L0 对象模型。
2. **命名**：ACRI（规划者建议）/ Repair Contract Protocol / 不命名。
3. **落位与治理**：顶层 `specs/` 目录（规范本体 specs/<name>/ + Lom 符合性映射，规划者建议——资产本体与项目文档分离，未来可整树迁出）/ docs/specs/ / 暂仅本设计文档不出规范本体（β-1 缓议）。
