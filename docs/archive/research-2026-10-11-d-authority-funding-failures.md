> **归档说明**：本文件为第五/六轮外部调研原始报告归档（2026-10-11 执行、2026-10-12 归档）。
> 原始位置 target/probes/survey*/report_*.md（gitignored）；原文零改动照档，仅加本头。
> 规划者复核：关键事实抽验六项吻合（Zero diagnostics.md 原文/Vera README 逐字/MoonBit SeekMoon/agentlanguages.dev/OpenAI 弃榜文存在性/GitHub API 数字）；
> 未核项以原文内 [未核] 标记为准。铁律 7 纪律：未安装任何竞品（文档级深读）。
> 调查背景与结论消费见 TODO 顶部状态块（2026-10-12 裁决：γ 评测资产先行 + β 规范升维主轴）。

# survey6 扩大面报告 D（四问）｜执行者：扩大面调查线程｜2026-10-11

方法说明：关键事实尽量打开原文核对，标【已核】；原文 403/404 或仅有搜索快照/二手引句的标【间接核】或【未核】并如实注明。GitHub/HN 数据经 api.github.com 与 hn.algolia.com API 核对（2026-10-11 时点）。引用数为 Semantic Scholar 口径。全程未安装任何竞品。

---

## 一、语言设计权威观点战（2025-2026 公开表态，除 Armin Ronacher 2026-02 外）

1. **Anders Hejlsberg（TypeScript/C# 之父）——质疑"为 LLM 造新语言"，拥抱 AI 辅助**【已核，原文访谈】
   - 来源：Ryan Peterman 访谈，developing.dev，2026-08-17（https://www.developing.dev/p/creator-of-typescript-10x-faster，标题 "Creator of TypeScript: 10x Faster TypeScript, Why AI Won't Replace SWEs"）。
   - 关键原句（已开原文核对）："AI is best at the languages it's seen the most of in its training set"；"incumbent languages are actually, if anything, going to flourish because of AI"；"We're there to make you more productive as a developer or make AI more productive as a developer"；"sometimes you don't ask AI for the answer, you ask it for a program that computes the answer"。
   - 事实佐证：AI 工具几乎都输出 TS 而非 JS——类型系统是 LLM 的静态护栏。
   - 倾向判定：**质疑**（存量语言马太效应论：训练语料最多的语言最受益，新语言天然吃亏）。

2. **Guido van Rossum（Python 之父）——直接否定"为 AI 改语言"必要性**【已核，原文访谈】
   - 来源：Roberto V. Zicari 采访，ODBMS Industry Watch，2025-10-10（https://www.odbms.org/blog/2025/10/beyond-the-ai-hype-guido-van-rossum-on-pythons-philosophy-simplicity-and-the-future-of-programming/）。
   - 关键原句（已开原文核对）：被问 Python 是否要为 AI 添加什么时答 "Nothing comes to mind. AI is over-hyped. It's still software."；"I am definitely not looking forward to an AI-driven future"（担忧的是无伦理者借 AI 作恶）；观察到模型在 Python 这类"人文主义"哲学的语言中编程最自如——编程语言本就为人类设计，LLM 擅长人类语言结构；代码仍须人类审查，否则"完全失去对自身存在的控制"；自家团队不做 vibe coding，架构与 API 设计保持人类掌控。
   - 倾向判定：**质疑**（"human-friendly 即 LLM-friendly"，与本赛道核心命题直接对冲）。另 GitHub Blog 2025-11-25 有同主题访谈（Python 年增 49%），未开原文【未核】。

3. **Graydon Hoare（Rust 原作者）——承认变革量级，未对"语言为 LLM 设计"表态**【间接核】
   - 博文 "LLM time"，2026-03-15，https://graydon2.dreamwidth.org/322732.html（原文 403，无法直开）。关键句经 Lobsters 讨论页（https://lobste.rs/s/4jgfcx/llm_time，已核该讨论页）中 simonw 原文引用核对："This is the fastest and most violent change to working conditions and assumptions I've witnessed in my career"，互联网/开源/云计算的到来的冲击"Nothing else is in the same ballpark"。
   - 其 2014 年旧文 "always bet on text"（2025-07-30 重发，https://graydon2.dreamwidth.org/193447.html）在 LLM 讨论中被反复引用为"文本是最佳通用接口"论据【间接核，经 Lobsters 域列表核对重发事实】。
   - 倾向判定：**中性偏警觉**（承认剧变，无任何"应为 LLM 重设计语言"的主张；帖子重心在安全漏洞激增与行业冲击）。

4. **Richard Feldman（Roc 作者）——LLM 降新语言使用成本，但不改变采纳逻辑**【间接核 + HN 现场已核】
   - 访谈 "This might be the BEST time to create a NEW programming language"（2025-06-27），原始视频未能定位【未核原文】；观点经 Lobsters 讨论帖（2025-06-27，标题 "Richard Feldman on new language adoption in the LLM age"）与 gist 记录核对【间接核】："LLMs make bespoke code cheap"（LLM 让小众语言写码变便宜）、LLM 不改变采纳的"人类部分"（社区/工具链/生态）、对 AI agent 改变语言采纳持怀疑。
   - HN 现场：2026-03 在 Mog 帖（item 47312728，已核 Algolia API）以 rtfeldman 身份指出 Mog 的能力权限模型与 Roc platforms 概念几乎一致——capability 设计出现跨项目趋同。
   - 倾向判定：**中性偏乐观**（对新语言在 LLM 时代的机会），但明确冷启动坎仍在。

5. **Gilad Bracha（Strongtalk/Newspeak/Dart）——比"为 LLM 设计语言"更激进：语言之后的阶段是与 AI 交互**【未核】
   - 线索：论文 "Substrates: Thru the Looking Glass"，搜索快照显示存在于 https://software-substrates.github.io（快照中可见 "AUTHORS: Gilad Bracha, TITLE: Substrates: Thru the Looking Glass"，疑为合集/评审材料中的 PAPER 1），页面现 404 打不开【未核，如实报告】。快照转述其主张：语言之后编程的下一阶段是 "programming by interacting with AI"。
   - 该站正式可查论文为 Klokmose 等 "Substrates: From Webstrates and Beyond"（2025）【已核存在性】——Bracha 与 Webstrates 社区有交集，但正式版论文未检出。
   - 倾向判定：**支持且更激进**（绕过"语言文本"本身，走向与 AI 交互的软件基片）。此条证据等级最低，仅作方向参考。

6. **Mitchell Hashimoto——重度 agent 用户但无语言设计表态；任务提示有误需纠正**【间接核】
   - Gleam 的作者是 **Louis Pilfold**，不是 Mitchell Hashimoto（Hashimoto 的代表语言实践是 Zig/Ghostty）。Hashimoto 2025 年公开实践：Pragmatic Engineer 访谈（约 2025-10）"always have an agent running in the background"（https://newsletter.pragmaticengineer.com，未开全文【间接核】）。
   - 反向证据：Mog HN 帖中评论者 rapind 把 **Gleam** 当作"现成即用的 LLM 友好语言"推荐——小型、静态类型、无 null、BEAM 之上，LLM 可靠生成且人类可审【已核，Algolia 评论】。Gleam 作者本人对 LLM 的直接表态**未检出**。

7. 其他有分量者：Jonathan Blow（Jai）2025-11-15 上 Software Unscripted 谈语言设计【存在性已核 https://shows.acast.com/software-unscripted，内容未核】；Chris Lattner 早期在 Feldman 播客谈过语言设计（nondot.org 简历提及）【未核】。两者与"LLM 语言"命题的直接表态未检出。

小结：第一问共成条 6 组（Hejlsberg/Guido/Graydon/Feldman/Bracha/Hashimoto-Gleam），其中 3 条已核原文、1 条间接核、1 条 HN 现场核、1 条未核线索。**质疑派（Hejlsberg、Guido）的论据高度一致：训练语料马太效应 + 人类友好即模型友好；支持派最有力者（Bracha）反而主张跳过语言层。没有任何一位主流权威公开主张"应当为 LLM 设计一门新语言"。**

---

## 二、学术接受度

1. **SimPy 论文**（arXiv 2404.16333，"AI Coders Are Among Us: Rethinking Programming Language Grammar Towards Efficient Code Generation"，Sun/Du/Yang/Li/Lo，ISSTA'24）【已核 arXiv 摘要页 + S2 API】
   - 内容：提出"AI 面向语法"，SimPy 为首个面向 AI 的 Python 精简语法（同 AST、可互转）；CodeLlama/GPT-4 token 降 13.5%/10.4%，性能持平或更好。
   - 引用（S2，2026-10-11）：**30 次，其中 influential 2 次**。判断：被顶会接收、有一定扩散，但两年 30 引属"正常可见度"，未成引用热点；未见以 SimPy 语法为基础的后续系统论文（代表性后续未检出）。

2. **ComBench**（arXiv 2603.27333，"A Repo-level Real-world Benchmark for Compilation Error Repair"，Li/Zhuang/Chen/Su/Meng/Lyu，2026-03-28 v1）【已核 arXiv + S2 API】
   - 引用：**1 次，influential 0**（发表仅半年，符合预期）。
   - 口径提醒：它是 C/C++ 编译错误修复的仓库级基准，**不是**语言设计论文；与赛道的关联在于其核心发现——GPT-5 语法正确率 73% vs 语义正确率 41%——被用来论证"编译器诊断是 LLM 反馈回路"（agentlanguages.dev 引用同类 ICSE 2026 "24.5% token 是格式"研究佐证语言设计动机【间接核】）。

3. **论文量级**（arXiv cs.PL 检索，export.arxiv.org API，已核）
   - "为 LLM/agent 设计语言或 DSL"的直接相关论文约**月均 1-3 篇、年 10-20 篇**量级——小而快速增长的子领域。更大的一块是"PL 技术服务 LLM"（agent 编排 DSL、验证、约束解码），而非"AI-native 通用语言"本身。
   - 代表性 5 篇（均已核 arXiv 元数据）：Djinnlang（2609.23954，2026-09，"this makes specification languages the new programming languages"）；Turn（2603.08755，2026-03，agent 计算的静态类型语言，"认知类型安全"）；LLMON（2603.22519，2026-03，LLM 原生标记语言）；XFlow（2606.14790，2026-06，多 agent 工作流协议语言）；声明式 Agent DSL（2512.19769，2025-12）。另 Oracular Programming（2502.05310，v5 更新至 2026-03）持续活跃。
   - 未检出专门的 "LLM-oriented language design" 综述。

4. **Workshop/track**【已核 CfP 页面】
   - **LMPL**（International Workshop on Language Models and Programming Languages）是本方向专门阵地：第 1 届 2025-10-15 新加坡；**第 2 届 LMPL 2026 于 2026-10-09 在 Oakland 与 SPLASH/ISSTA 同地举办**，组织者 Nada Amin（Harvard）/Qingkai Shi（南大）/Guannan Wei（Tufts）/Zhuo Zhang（Columbia）（https://conf.researchr.org/home/lmpl-2026）。
   - 关键观察：LMPL CfP 主题全是 "PL techniques for X"（harness 工程/agent 设计/模型训练/幻觉抑制）——**PL 服务 LLM，而非语言为 LLM 重设计**；"LLM-oriented language design" 未列入专题条目。邻近：PAgE@PLDI 2026、VeriCodeGen@NeurIPS 2026【存在性已核，未细读】。

---

## 三、资金与机构投入

1. **MoonBit（已知基线补充）**：IDEA 研究院（深圳福田，政府背景新型研发机构）基础软件中心，2022-10 立项，张宏波主导；2025-06-18 发布 Beta，深圳政府网站（htcz.sz.gov.cn）刊文报道【间接核，快照】——政府背书证据。**纠错**：与香港 2025-26 预算的 AIRDI（10 亿港元）无关联，网传"香港注资 MoonBit"不成立。

2. **Zero（Vercel Labs）**：企业内部实验室背书的公开项目，★5.4k+【已核 agentlanguages.dev 条目】。除 IDEA 与 Vercel Labs 外，**未检出其他大厂正式立项**；Valea 作者 Hans Voetsch 任职 Google 但 repo 未声明机构隶属——属大厂员工个人项目而非 Google 立项【已核 agentlanguages.dev 原文注明】。

3. **融资事件**：未检出任何以"新语法语言"为主业的公司融资。最接近的资本事件：
   - **Wordware**：$30M 种子轮（2024-11，Spark Capital 领投），创立前提即 "natural language is the next programming language"（https://www.eu-startups.com 播客与 theventurecodex.com 快照）【间接核，未开原文】——本主题最大单笔，但标的是自然语言编程/agent 编排工具，非编译型新语言。
   - CodeWords：$9M 种子（2026-05-08，thesaasnews）【间接核】，AI 自动化平台，边缘相关。
   - VC 观点层：Karpathy "Software 3.0"（英语即新编程层）构成投资叙事底座【常识性引用，未单独核】。

4. **VC 系统性研究的唯一公开证据**：**Negroni Venture Studios** 2026-05 发文 "Three camps alike in dignity"，提出 syntactic/verification/orchestration 三阵营分类法，并衍生出社区维护目录 agentlanguages.dev（维护者 Alasdair Allan，42 门语言，内容 CC BY 4.0）【已核 github.com/aallan/agentlanguages README：目录源自 Negroni 的研究但非其产品】。即：**有 VC 在做赛道测绘，但停留在研究/目录层，未见其下注语言项目本身**。

判断：本赛道资金形态是"研究院 + 企业实验室 + 个人"三元结构，纯商业融资为零；资本把"AI 原生编程"主题的钱投在了自然语言工具层（Wordware 们），刻意绕开了新语法语言。

---

## 四、失败/停滞案例解剖（冷启动死结证据核心）

### 4.1 Mog：发布日即巅峰的"一日流星"
- 发布渠道：HN "Show HN: The Mog Programming Language"（moglang.org），2026-03-09 17:57 UTC，**163 分 / 83 评论**，作者 belisarius222【已核 Algolia API，item 47312728】。
- 仓库实况：github.com/voltropy/mog，created 2026-02-01，**pushed_at 2026-03-09T17:55:47Z——最后一次推送距 HN 发帖仅 73 秒，此后 7 个月零提交**；144★、9 fork、无 tagged release【已核 GitHub API】。
- 作者投入时长：约 5 周（02-01 建仓至 03-09 发帖）；目录站记载其起源为 "Volt coding agent 三周连续会话（Claude Opus 4.6、Kimi k2.5、GLM-4.7）完成"，128 commits、1146+ 编译器测试、规范压缩至 3200 token【已核 agentlanguages.dev/mog 条目】。
- HN 关键批评（均已核 Algolia 评论原文）：
  - steve_adams_86（训练数据缺失）：新语言缺 LLM 训练语料，Deno+TS 已有安全模型/类型/沙箱，问 Mog 在 agent 场景的实质优势何在。
  - TheDong（冷启动悖论）：只有 AI 实验室自建语言才可能成——需向训练集塞入 "several terrabytes of valid useful 'mog' programs" 并做 RL。
  - embedding-shape（上下文经济）：冷门语言须永久占用 context 放 stdlib 与语法说明，挤占用户代码空间。
  - verdverm（生态缺失）：无 GitHub 高亮/LSP/包管理；结论句 "I don't see an agent first language becoming a thing while humans are still ultimately responsible."
  - mkl + saithound（反直觉设计反噬）：无运算符优先级是 foot-gun，"They are measurably worse when writing code in languages with non-standard or non-existent operator precedence."
- 死因（主观推测）：**热度没有转化，因为批评全部命中结构性问题而非可修复缺陷**——训练语料、context 经济、工具链生态三样都不在作者控制内；项目本身是"单人+agent 会话的一次性交付"，发布即完成，无预留持续投入与路线图沟通。163 分买来的是围观，不是采用。

### 4.2 Valea：蓝图最完整、寿命最短
- 仓库实况：github.com/hvoetsch/valea，created 2026-03-12 17:35，**pushed_at 2026-03-13 13:06——从建仓到停更约 37 小时**；**4★**、24 commits、未归档【已核 GitHub API——"4★"即 star 数】。
- 蓝图完整度：SPEC.md/MANIFESTO.md/ROADMAP.md/AGENTS.md/CLAUDE.md 五件套 + asciinema 演示；Rust 实现编译到 C；五原则（确定性语法/显式语义/机器可读诊断/规范化格式/小语言表面）【已核 agentlanguages.dev/languages/valea 条目】。
- 目录站评语（已核）："this much intent, this little code, and this much manifesto"——并指出 "AI-native systems programming language" 在 HN 已成有固定形态的类别（Rust 编译器 + JSON 诊断 + agent 导向 README），此类项目"已很常见"。
- 死因（主观推测）：**文档先行、无分发、无反馈回路**——没有自己的 HN 发布记录（目录仅记"首见于 2026-03 HN"但无帖子），没有外部使用者，五份宣言与 24 次提交的工程量反差过大；蓝图完整度恰恰暴露了"设计词表已稳定、实现纵深无人买单"的赛道通病。

### 4.3 Agentis：从语言漂移到平台叙事
- 仓库实况：github.com/Replikanti/agentis，created 2026-04-07，pushed_at 2026-09-14（低频维持），**1★**【已核 GitHub API】。
- 现描述："AI-native platform for autonomous agent emergence. Runtime, language, evolution engine."——语言已降格为平台三要素之一；README 自称 "not an LLM wrapper"，主打可审计自治【已核 repo 描述字段；README 全文未读，转型时间线未核】。
- 死因（主观推测）：语言零采用（1★）后，**靠扩大叙事范围（runtime/evolution engine）维持项目正当性**——这是无用户语言项目的典型存活策略：不解决冷启动，改为重新定义问题。

### 4.4 更早/旁证案例
- **SudoLang**（2023）：HN "a programming language designed to collaborate with AI language models"，2023-10-06，61 分【已核 Algolia，item 37791060】；VSCode 支持仓库 1357★，最后 push 2026-01-15，此后 9 个月无更新【已核 GitHub API】。形态：停留在 prompt 约定/文档层，从未进入实现语言行列——**边缘维持三年，是该命题最早也最"未死透"的样本**。
- **Marsha**（agentlanguages.dev 编目）：维护者最后活动 2023-08 初，目录标注基本停滞【已核目录首页】。
- **死亡谷全景**（已核 agentlanguages.dev，stars 2026-10-05 刷新）：42 门语言中 ★≤5 者至少 8 门（Lume/Faber/Pact 各 1、Laze/Tacit/reqlan/Valea 各 4、Boruna 5），Koru "只在单机编译过"，Sever/B-IR/Spec 被标思想实验；★≥100 者仅 Zero(5.4k)/NanoLang(628)/Vera(417)/Mog(144)/AILANG(34)——**长尾极深、腰部缺失**。

---

## 结尾三段浓缩（执行者视角，供规划者复核）

**权威观点格局**：2025-2026 没有出现"为 LLM 设计语言"的权威代言人——恰恰相反，两位最大在役主力（Hejlsberg、Guido）给出了方向一致的反命题：训练语料的马太效应让存量语言更受益（Hejlsberg），人类友好天然即模型友好（Guido，"AI is over-hyped. It's still software."）；Graydon 承认这是职业生涯最猛烈的变革但不提语言重设计；Feldman 承认 LLM 降低小众语言成本但坚持采纳逻辑（人）不变；唯一方向性支持者 Bracha 走得更远——主张跳过语言文本层直接进入"与 AI 交互"的基片，等于釜底抽薪。该命题目前的智力合法性主要来自学术侧（SimPy 等）与实干者社区，而非语言设计权威。

**学术与资金温度**：学术温度"温而不热"——SimPy 两年 30 引属正常可见度，ComBench 半年 1 引；直接相关论文月均 1-3 篇且向"PL 服务 LLM"（编排 DSL/验证/约束解码）而非"语言为 LLM 重生"倾斜；专门 workshop LMPL 已办两届（2025 新加坡、2026-10-09 Oakland），但 CfP 无语言设计专题。资金温度"测绘有、下注无"——唯一系统性 VC 研究（Negroni 2026-05 三阵营分类法 → agentlanguages.dev 目录）停在目录层；真正的钱（Wordware $30M 种子）流向自然语言编程工具层；机构投入仅 IDEA（政府背景研究院）与 Vercel Labs（企业实验室）两条公开证据，无大厂正式立项。

**失败案例对新语言冷启动死结的共性启示（主观推测）**：Mog（163 分热度、发帖时刻即最后提交）、Valea（蓝图五件套、37 小时寿命）、Agentis（零采用后漂移为平台叙事）、SudoLang（三年边缘维持）指向同三个死结：其一，**训练语料悖论**——LLM 写不好没见过的语言，而写好所需的"数 TB 级语料 + RL"只有 AI 实验室能供给（TheDong 论证，Mog 帖现场）；其二，**context 经济与生态工具链的存量依赖**——冷门语言的语法/stdlib 常驻挤占上下文，且 LSP/高亮/包管理缺一环 agent 就难用，这些是十年尺度的人类基础设施，不是声明式蓝图能替代的；其三，**"人类仍负最终责任"的问责锚**——只要人还要审查代码，可读性与既有生态就压倒 token 效率（verdverm 论证）。对扩大生态面的直接推论：新语言的生死不取决于语法优劣或 HN 分数，而取决于能否绑定一个"模型侧供给侧"（自建语料/RL 或深度绑定某 agent 平台）加一个"人类侧问责出口"（可读子集/成熟工具链），三者缺一即为死亡谷长尾。

（报告完，约 260 行）
