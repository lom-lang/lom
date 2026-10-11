> **归档说明**：本文件为第五/六轮外部调研原始报告归档（2026-10-11 执行、2026-10-12 归档）。
> 原始位置 target/probes/survey*/report_*.md（gitignored）；原文零改动照档，仅加本头。
> 规划者复核：关键事实抽验六项吻合（Zero diagnostics.md 原文/Vera README 逐字/MoonBit SeekMoon/agentlanguages.dev/OpenAI 弃榜文存在性/GitHub API 数字）；
> 未核项以原文内 [未核] 标记为准。铁律 7 纪律：未安装任何竞品（文档级深读）。
> 调查背景与结论消费见 TODO 顶部状态块（2026-10-12 裁决：γ 评测资产先行 + β 规范升维主轴）。

# 第五轮外部调研（生态面）：修复闭环生态与大事件调查

- 调研时点：2026-10-11（检索与原文核对均在本日执行）
- 方式：公开网络检索 + 官方原文打开核对（WebFetch / webReader / GitHub API 只读）；未安装任何竞品（文档级深读，遵守用户裁决）
- 上轮对照：docs/archive/ 内 research-2026-09-15（第1轮生态位）、research-2026-09-15-repair-landscape（第3轮修复基建）、research-2026-09-15-zero-vera-doc-deepdive（第2轮）、research-2026-09-16-broad-scan（第4轮广域扫描）
- 核对标记约定：`已核`=本轮亲自打开原文核对；`部分核`=原文页未能直开但多家独立来源一致；`未核`=仅搜索摘要转述或负面检索结论
- git 全程未动；本文件为唯一产出

---

## 一、诊断/修复生态标准化动向（LSP 之外的 LLM 导向诊断格式）

**总体判定：没有形成中的统一"LLM 诊断 JSON schema"标准；实践收敛在两条线上——MCP 生态的 LSP 桥接层、工具厂商的"错误回喂"框架能力。**

- 未检出任何以 "LLM-friendly error reporting" 名义的正式规范/工作组/提案（多轮英文检索，含 SARIF 扩展方向均无直接结果） | 检索无果（负面事实） | 未核（负面结论，只能标注检索范围）
- MCP 生态出现"LSP→MCP 桥"层：isaacphi/mcp-language-server，Go 编写，"This is an MCP server that runs and exposes a language server to LLMs"，提供 definition/references/**diagnostics**/hover/rename_symbol/edit_file 六工具；diagnostics 工具描述原文 "Provides diagnostic information for a specific file, including warnings and errors"；1.6k stars、104 commits、BSD-3；已测 gopls/rust-analyzer/pyright/typescript-language-server/clangd | https://github.com/isaacphi/mcp-language-server | 已核
- 该桥的诊断输出无独立 JSON schema——README 未给出 diagnostics 字段结构，底层沿用 LSP 协议类型（internal/protocol/tsprotocol.go 借自 gopls） | 同上 | 已核
- agent-lsp：mcpservers.org 收录的"stateful LSP runtime for AI agents"（warm 会话 + 50+ 工具，含诊断） | https://mcpservers.org/Servers/Development | 未核（目录转述）
- VS Code Marketplace 有 "Diagnostics MCP Server"，把 VS Code 诊断（TS/ESLint/Prettier 及全部扩展）经 HTTP MCP 暴露给 AI agent | https://marketplace.visualstudio.com/items | 未核（市场列表转述，未定位到具体条目页）
- Anthropic 官方工程博客《Writing effective tools for AI agents》（2025-09-11）：错误响应应 "prompt-engineer your error responses to clearly communicate specific and actionable improvements"，反对 "opaque error codes or tracebacks"；截断响应需给 agent 引导指令 | https://www.anthropic.com/engineering/writing-tools-for-agents | 已核（原文）
- Claude Code 官方 hooks 文档：PostToolUse hook 以 exit code 2 退出时 "Shows stderr to Claude; the tool already ran"；"Claude never sees stderr from a hook that exits 0"；文档含 lint 场景配方（matcher "Edit|Write" 跑 lint-check.sh）；另有 hookSpecificOutput.additionalContext 把任意文本插到工具结果旁 | https://code.claude.com/docs/en/hooks | 已核（原文）
- Claude 平台 Skill authoring best practices："Error messages point to specific problems"，校验脚本应输出如 "Field 'signature_date' not found" 的具体错误 | https://platform.claude.com（skill authoring best practices） | 未核（搜索摘要，原文页未直开）
- Armin Ronacher《A Language For Agents》（2026-02-09）：理想世界 "the agent has one command, that lints and compiles"，且 "there is mechanical fixing for as many linting failures as possible so that the agent does not have to do it by hand"——有影响力的语言设计者公开主张"机械修复进工具链" | https://lucumr.pocoo.org/2026/2/9/a-language-for-agents/ | 已核（原文深读，本轮新核）
- 同文批评多路径失败："That can gaslight the agent"（指 TS 类型检查失败仍能运行）；主张 "Agents struggle with exceptions, they are afraid of them" | 同上 | 已核
- 学术线：arXiv 2607.09979《Using LLMs to Adjudicate Static-Analysis Alerts with Error Reduction Techniques》（Klieber/Svoboda/Flynn/Martins，CMU+SEI，2026-07-10 提交）——研究 LLM 依据错误信息裁定静态分析警报真伪 | https://arxiv.org/abs/2607.09979 | 已核（摘要页定位）
- 协议层治理标准化（与诊断格式标准化是两回事）：MCP 于 2025-12-09 捐入 Agentic AI Foundation（Linux Foundation directed fund），成创始项目；Anthropic 官宣与 MCP 官方博客、GitHub 博客、Wikipedia 口径一致 | https://www.anthropic.com（Dec 9, 2025 公告）+ https://blog.modelcontextprotocol.io | 部分核（官方原文页两次直开 404/404，多源一致）
- SARIF（OASIS 静态分析交换格式）未见面向 LLM/agent 消费的扩展动向 | 检索无果 | 未核（负面）

**小结（事实层）**：诊断格式仍以 LSP Diagnostic 结构为事实底座；MCP 层只做"搬运"不做"再定义"；错误回喂的约定散落在各家工具的 hooks/lint 文档里，以"stderr + 非零退出码"为主要通道——尚无任何跨厂商的结构化诊断码 schema（Lom 的 lom-diag/v1 在这个坐标系里仍无对应标准竞品，也无标准可挂靠）。

---

## 二、agentic 编码工具消费编译器诊断的实践形态

**总体判定：主流工具统一走"终端文本回喂"（stdout/stderr + exit code）路线；结构化 JSON 诊断消费只存在于 MCP 语言服务桥这类外挂层；官方"什么样的错误对 agent 有效"话语已经成型（具体、可操作、含修复线索）。**

- Claude Code：官方机制为 hooks——PostToolUse exit 2 回喂 stderr、additionalContext 注入、lint hook 配方；这是 Claude Code 消费诊断/错误的一等通道 | https://code.claude.com/docs/en/hooks | 已核（原文）
- Anthropic 官方对"有效错误"的立场："prompt-engineer your error responses to clearly communicate specific and actionable improvements"；错误响应可含 "examples of correctly formatted tool inputs"（用错误示例教 agent 正确输入） | https://www.anthropic.com/engineering/writing-tools-for-agents | 已核（原文）
- Aider：官方文档《Linting and testing》——"By default, aider will lint any files which it edits"，目的是 "identify and repair any problems introduced by the AI edits"；--lint-cmd 指定 linter（约定：错误打到 stdout/stderr + 非零退出码）；--auto-test/--test-cmd 每次编辑后自动跑测试，非零退出则 "Aider will try and fix any errors"；/run 手动把命令输出分享给 LLM | https://aider.chat/docs/usage/lint-test.html | 已核（原文）
- Cline：官网能力表述 "Executes in your terminal and reacts to output as it appears"（实时对终端输出作出反应）；编辑能力含 "linter-aware fixes, diffs, checkpoints" | https://cline.bot/ | 已核（原文）
- OpenAI Codex：社区通行实践是把 CI/lint/build 失败输出直接 pipe 给 Codex 迭代修复（如 Nimbalyst 2026-02、SitePoint 2026-05 教程）；Codex 自身约定入口为 AGENTS.md | https://shipyard.build（cheatsheet，二手） | 未核（二手实践文，官方文档未直开核对）
- GitHub issue（2025-06）：Codex 在非零退出码但无输出时行为异常——反向印证"错误输出质量直接决定 agent 行为" | github.com/openai/codex issues | 未核（检索摘要）
- Cursor 社区在讨论"agent 实时读编译器诊断、自纠后再交付"的循环 | https://forum.cursor.com | 未核（论坛线索）

**公开讨论"什么样的错误信息对 agent 最有效"的一手来源清单**（每问各 1-2 个，已在上文）：Anthropic 工程博客（错误响应=提示工程对象）+ Claude Code hooks 文档（exit 2/stderr 协议）；Aider lint-test 文档（linter 约定）；Ronacher 博文（机械修复/单命令循环/异常恐惧）。

---

## 三、模型能力面：修复类基准的公开态势（2026-10）

**核心事件：OpenAI 官宣弃用 SWE-bench Verified。**

- OpenAI《Why we no longer evaluate SWE-bench Verified》：受控口径下 SOTA 六个月内从 74.9% 升至 **80.9%** 后放缓（"improving from 74.9% to 80.9% in the last 6 months"） | https://openai.com/index/why-we-no-longer-evaluate-swe-bench-verified/ | 已核（原文全文）
- 同文：审计 138 道 o3 未持续解出的题，**59.4% 存在实质缺陷**（35.5% 窄测试、18.8% 宽测试、5.1% 其他）——基准的测试质量本身是问题 | 同上 | 已核
- 同文：污染确认——所测前沿模型（GPT-5.2-Chat、Claude Opus 4.5、Gemini 3 Flash Preview）均能复现 gold patch 或逐字复述题目细节（训练数据泄漏） | 同上 | 已核
- 同文：**"we have stopped reporting SWE-bench Verified scores, and we recommend that other model developers do so too"**；建议改报 SWE-bench Pro（public split）；Pro 中污染少见（"no model was able to produce a complete verbatim gold patch"） | 同上 | 已核
- 同文：GPT-5.2 解出 31 道"几乎不可能"（all frontier fail）任务 | 同上 | 已核
- 发表日期：页面元数据一次读取为 2026-02-23T09:00:08+00:00，另一次为 2026-10-06T12:00（疑为原文+更新时间混读）；第三方引用（jdav.is、Cursor Composer 2 技术报告、arXiv 2610 审计论文）均按 2026-02-23 引用 | 同上 | 已核（日期矛盾如实标注）
- SWE-bench Pro 官方榜（Scale AI）：页面 2026-10-10 更新，当前对比的前沿为 **GPT-5.4、Muse Spark、Claude Opus 4.6、Gemini 3.1 Pro**；榜述"top models score around 23% on the SWE-Bench Pro public set, compared to 70%+ on SWE-bench Verified"（基准发布时口径） | https://labs.scale.com/leaderboard/swe_bench_pro | 已核（页面元数据与导语；榜单表格为动态渲染，**当期具体名次百分比未取到**）
- SWE-bench Pro 第三方数字：GPT-5.4（xHigh）2026-04 时 59.10% public 榜首（paddo.dev）；另有"Claude Opus 4.8 69.2%"等说法 | paddo.dev / 搜索摘要 | 未核（未能打开可核原文）
- SWE-bench Verified tracker 口径：vals.ai "Seven of the 86 models reach 95% or better, the leader Claude Opus 5 at 97.00%"；benchlm.ai（2026-10）"Claude Opus 5 leads at 96%" | https://www.vals.ai / https://benchlm.ai | 未核（搜索摘要；vals.ai 直链 404）
- 官方 swebench.com：榜单动态加载未取到名次；页面新闻最新至 2026-05 | https://www.swebench.com/ | 已核（负面：具体名次未取到）

**对"修复闭环作为语言卖点"命题的两面证据（执行者归纳）**：
- 印印侧（模型越强越能利用好诊断）：OpenAI 官方认定的三大问题——污染、测试窄/宽、长程未解（Pro 远未饱和）——全部指向"反馈质量决定修复"，正是语言级诊断协议可改善的维度；Anthropic/Ronacher 的"错误要具体可操作"话语与模型消费能力同步升级；模型强 → 更能消费更细粒度的结构化修复建议（lom-fix/v1 的 High/Medium/Low 分级与迭代 apply）。
- 稀释侧（模型强到不需要语言配合）：tracker 自报 Verified 已 95%+（即便有污染折扣）；GPT-5.2 解出 31 道"几乎不可能"任务；Pro 榜 6 个月内从 ~23% 快速抬升——通用模型在既有语言上的自我纠错闭环（终端输出→猜→试）确实在变强，弱化"专为修复设计语言"的必要性感知。
- 主观推测：当前证据净方向偏"印证"——因为弃榜理由（测试质量/污染/长程）恰好是确定性诊断协议的主场；但 Lom 对外叙事应主动区分自己的 128 任务评测与 SWE-bench 系口径（防污染设计、建议型 vs 生成型），避免被"基准不可信"的时代情绪波及。

---

## 四、上轮调研对象 2026-09 → 10 delta 跟踪

上轮覆盖对象清单（来源：第 1/2/3/4 轮报告）：Zero、Vera、Mojo、MoonBit、agentlanguages.dev 目录（35 家）、Codong、X07、B-IR/Loom（同名无关）、BirdDisk、AINL、Ronacher 博文、spec-driven 工具六家（Augment Cosmos/Kiro/GitHub Spec Kit/OpenSpec/BMAD/Cursor）、rustc-rustfix-cargo fix、TS code fixes、APR 学术线。

- MoonBit：**1.0 未发布**；官方 updates 页最新为 v0.10.14（2026-09-21，含 `T::_` 子错误模式匹配、`moon search` 等工具链命令）；Q3（9 月末）窗口已过——上轮挂账的"Q3 目标"落空，1.0 二度滑期 | https://www.moonbitlang.com/updates/ | 已核
- Zero（Vercel Labs）：官网仍标 "Experimental / Expect breaking changes"，定位口号 "A programming language where the graph is the program"（agent 查询程序图、提交 checked edits、证明结果）；GitHub vercel-labs/zerolang **5,384 stars、最后 push 2026-09-16、open issues 129**；**最后 release tag 为 v0.3.4（2026-06-13）——自 6 月起无新版本发布** | https://zerolang.ai/ + https://api.github.com/repos/vercel-labs/zerolang（releases） | 已核
- Vera：版本线 0.1.8（VeraBench 口径）→ 0.1.13 → **0.2.0**（"The checker and verifier are stricter in 0.2.0"，新增 E137 递归缺少 decreases 报错、E502 度量下溢报错；VeraBench v0.0.18）；定位原文不变 "A programming language designed for LLMs to write, not humans." | https://veralang.dev/ | 已核（内容）；0.2.0 具体发布日期页面未标注（时间线未核）
- Mojo：1.0 + 开源已于 2026-08-12 发生（上轮已记，本轮检索口径一致：Wikipedia/Techzine/The Register）；2026-09→10 未检出新的大公告；"秋季开源余下编译器组件"为待观察项 | https://en.wikipedia.org/wiki/Mojo_(programming_language) 等 | 部分核（旧事实多源一致；新 delta 为检索无果的负面结论）
- agentlanguages.dev：**35 家（2026-06-14 快照）→ 42 家（last updated 2026-10-05，star 同日刷新）**；Lom 仍不在目录（名字最接近的是 Loom 与 Lume，均非本项目）；修复元素条目无变化（Zero "typed repair plans"、Codong "structured JSON errors with fix/retry fields"、X07 "quickfixes the toolchain applies deterministically"）——**未出现新的 repair-native 直接竞品** | https://agentlanguages.dev/ | 已核
- BirdDisk：3 stars、最后 push **2026-03-01**——停滞（上轮即定性个人 POC，本轮确认无进展） | https://api.github.com/repos/Jawbreaker1/bird_disk_code | 已核
- Armin Ronacher《A Language For Agents》（2026-02-09）：上轮仅线索，本轮已打开原文深读（见第 1/2 节引文）；后续博文 "Absurd In Production"（2026-04-04）与语言议题无直接关系（检索摘要） | https://lucumr.pocoo.org/2026/2/9/a-language-for-agents/ | 已核（原文）/ 未核（后续博文）
- Codong、X07、AINL：本轮未逐一复核（上轮已判低优先级/外围） | — | 未核（如实标注）
- spec-driven 工具六家（Augment Cosmos/Kiro/Spec Kit/OpenSpec/BMAD/Cursor）：本轮未逐一复核版本 delta | — | 未核（如实标注）
- rustc/rustfix/cargo fix、TS code fixes、APR 学术线（第 3 轮对象）：传统基建与学术线，月度级变化概率低，本轮未复查 | — | 未核（如实标注）

---

## 五、大厂/官方"为 AI 设计语言或 DSL"动向

**总体判定：大厂语言级动作缺席，官方能量全部投在"协议与约定层"（AGENTS.md、MCP、Agent Skills）；语言级实验仍由 Vercel Labs（Zero）这类实验室与社区承担。**

- AGENTS.md：定位原文 "a simple, open format for guiding coding agents"；官方仓库 24.9k stars、38 commits、MIT、含 Technical_Charter.pdf（治理章程）；2025-12-09 OpenAI 宣布共同创立 Agentic AI Foundation（Linux Foundation 旗下）并捐出 AGENTS.md | https://github.com/agentsmd/agents.md（已核）+ https://openai.com/index/openai-co-founds-the-agentic-ai-foundation/（403 未直开，标题与多家来源一致） | 部分核
- MCP：同日（2025-12-09）Anthropic 将 MCP 捐入同一基金会，成创始项目（Anthropic 官宣/MCP 官方博客/GitHub 博客/Wikipedia 口径一致） | https://www.anthropic.com（Dec 9, 2025） | 部分核（原文页未直开，多源一致）
- Anthropic Agent Skills：2025-10-16 发布（skills=文件夹+SKILL.md+脚本的开放打包格式，模型可加载）；后续被表述为"开放标准"并获 OpenAI、Google DeepMind 等采纳（mindstudio 2026-03 转述） | https://www.anthropic.com/news/skills（未直开）+ 二手 | 未核（二手链）
- Google/Microsoft/Meta：未检出任何"为 AI agent 设计新编程语言/DSL"的官方动作；三家最接近的公开物是协议层——Google A2A（与 Microsoft 合作，2025-05）、MCP、AGENTS.md；Microsoft 侧为 Copilot 生态（GitHub博客参与 AAIF 报道） | 检索无果（负面） | 未核
- Vercel Labs Zero 是本轮唯一"大厂系语言级"对象，其 09→10 delta 见第四节（无新 release、渠道仍在） | https://zerolang.ai/ | 已核
- 观点层（非厂商）：Ronacher 呼吁"外行作品"式语言实验并认为 agent 表现可量化使语言设计首次可实测（"make sure the agent is happy and extend from there to the human"） | https://lucumr.pocoo.org/2026/2/9/a-language-for-agents/ | 已核

---

## 浓缩三段（执行者视角，供规划者复核）

**生态标准化**：LLM 导向的诊断格式没有形成任何统一标准或社区约定草案——MCP 生态的作法是把 LSP 原样搬给 agent（isaacphi 桥 1.6k 星，无再定义 schema）；各家 agentic 工具把"错误回喂"做成了框架一等能力（Claude Code hooks exit 2/stderr、Aider auto-lint/auto-test、Cline react-to-output），通道统一是**终端文本**而非结构化 JSON；"错误信息要对 agent 具体可操作"已从个体经验升格为 Anthropic 官方工程立场与 Ronacher 的语言设计主张（mechanical fixing）。协议层治理在标准化（AAIF 收编 MCP+AGENTS.md），但那与诊断格式标准化是两条线——后者仍是空白，意味着 lom-diag/v1 既无标准可挂靠、也无标准竞品挤压，跨厂商采纳只能靠自身证据链。

**模型能力对命题的含义**：印证与稀释并存、当前证据净偏印证。稀释侧：Verified tracker 自报 95%+、GPT-5.2 破解 31 道"全前沿失败"任务、Pro 榜半年内快速抬升——通用模型的"输出→猜→试"自纠环在变强。印证侧：OpenAI 官方弃用 Verified 的三条理由（污染、测试窄/宽、长程未解）全部指向"反馈质量决定修复"，且明确承认受控口径 SOTA 仅 80.9%（约 1/5 修复任务仍败）——这正是确定性诊断协议的主场；模型越强，消费结构化建议（置信度分级、迭代 apply）的杠杆越大。主观推测：对外叙事应把 Lom 的 128 任务评测与 SWE-bench 系口径明确区隔（建议型 vs 生成型、防污染设计），并可直接引用 OpenAI 弃榜事件作为"反馈质量=瓶颈"的时代注脚。

**上轮对象 delta 总结**：赛道表面热度上升（agentlanguages.dev 35→42 家，2026-10-05 刷新）但无新 repair-native 竞品；Zero 渠道与声量仍在、工程节奏放缓（最后 release 2026-06-13，最后 push 2026-09-16）；MoonBit 1.0 二度滑期（Q3 已过，最新 v0.10.14）；Vera 小步演进（0.2.0，checker 更严）；Mojo 无新月度动作；BirdDisk 停滞；Ronacher 博文已深读归档（其 mechanical fixing 主张与 Lom 同向）。Lom 仍不在目录站收录中（符合收录标准、冻结裁决下不提交——维持上轮挂账状态）。本轮未复核项已如实列出：Codong/X07/AINL、spec-driven 工具六家、rustc/TS/APR 传统线。
