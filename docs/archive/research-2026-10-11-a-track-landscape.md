> **归档说明**：本文件为第五/六轮外部调研原始报告归档（2026-10-11 执行、2026-10-12 归档）。
> 原始位置 target/probes/survey*/report_*.md（gitignored）；原文零改动照档，仅加本头。
> 规划者复核：关键事实抽验六项吻合（Zero diagnostics.md 原文/Vera README 逐字/MoonBit SeekMoon/agentlanguages.dev/OpenAI 弃榜文存在性/GitHub API 数字）；
> 未核项以原文内 [未核] 标记为准。铁律 7 纪律：未安装任何竞品（文档级深读）。
> 调查背景与结论消费见 TODO 顶部状态块（2026-10-12 裁决：γ 评测资产先行 + β 规范升维主轴）。

# 调研报告 A：AI 原生编程语言赛道全景（2026-10 时点）

- 执行轮次：第五轮外部调研（survey5），执行日 2026-10-11
- 方法：文档级深读（未安装任何竞品）；关键数字均打开原始来源核对（GitHub repo/API、官方博客、arXiv、HN Algolia API、雷峰网原文）
- 标记约定：`已核` = 打开原始来源核对过；`未核` = 仅搜索摘要/二手转述，未经原文核对；`主观推测` = 执行者观点
- 未能核实的项：Pinecone "/dev/pi"（官网博客页与猜测 URL 均 404，HN 无帖——判断为本轮执行者记忆混淆，不列入清单）；IBM "DocLang"（仅一条搜索摘要提及，找不到原始来源，不采信）；akitaonrails 文章正文（URL 404，仅存搜索摘要引句，已标注）

---

## 一、赛道清单：宣称"为 LLM/AI 设计"的语言与 DSL

按热度和主体量级排序。GitHub stars/日期均为打开 repo 或 API 核对值。

| # | 名称 | 一句话定位 | 提出时间 | 状态 | stars / 最近推送 | 主体 |
|---|------|-----------|---------|------|----------------|------|
| 1 | **MoonBit** | AI 原生通用语言（工具链全家桶 + "把 AI 当用户"） | 2022-09-19 立项 | 活跃（1.0 前夜） | core 1,200（org 91 repos，多个 repo 当日仍在推送） | 深圳 IDEA 研究院（张宏波，ReScript 作者） |
| 2 | **Zero (zerolang)** | "The Programming Language for Agents"，图原生语言：语义图即程序数据库，agent 提交受检编辑而非改文本 | repo 创建 2026-05-15 | 活跃（实验性 v0） | 5,384 / 2026-09-16 | Vercel Labs |
| 3 | **NanoLang** | "tiny experimental language designed to be targeted by coding LLMs"——为机器编写、人类阅读；无歧义语法 + 强制测试 + Coq 形式化语义 | HN 首发热帖 2026-01-19 | 非常活跃（v5.0.0 已发，v5.1 规划中，7,200 commits） | 628 / 活跃（repo 页核） | Jordan Hubbard（FreeBSD 联合创始人、前 Apple Darwin 主管）个人 |
| 4 | **Mog** | "statically typed Lua" for LLM：spec 仅 3200 tokens、无运算符优先级、capability I/O、编译为原生码 | repo 创建 2026-02-01，HN 帖 2026-03-09（163 分/83 评） | 疑似发布后停滞（最后推送 2026-03-09，即 HN 帖当日） | 144 / 2026-03-09 | Voltropy（公司）；第一版由其 Volt agent 用 Claude Opus 4.6 / Kimi k2.5 / GLM-4.7 生成 |
| 5 | **GlyphLang** | "An AI-Optimized Programming Language"，宣称比 Python 省约 45% token | repo 创建 2025-12-16，HN 2026-01-10（44 分） | 活跃 | 65 / 2026-09-14 | 个人 |
| 6 | **Valea** | AI 原生系统语言：结构化 JSON 诊断（错误码 E001）+ agent"生成→编译→修复"闭环 | HN 帖 2026-03-12 | 早期实验（Rust MVP 编译器，24 commits） | 4 | 个人（hvoetsch） |
| 7 | **Agentis** | "LLM 即标准库"的实验语言（一切皆 prompt、认知预算、内容寻址代码 DAG） | HN 帖 2026-03-09 | 实验中，已转向"agent 平台"定位 | 1 / 2026-09-14 | 个人（Replikanti） |
| 8 | At | "agent-first、explicit、greppable"语言 | HN 2026-02-26（1 分） | 无人关注 | — | 个人 |
| 9 | PromptL / Pipelex / SudoLang | 提示模板/AI workflow DSL 一族（非通用语言） | PromptL 2024-11；Pipelex 2025-10（HN 122 分）；SudoLang 2023（HN 84 分） | 各自小规模存续 | — | 公司/个人 |

事实来源（每行均已开原文）：
- MoonBit 版本线与生态 | https://www.moonbitlang.com/blog/ 及 /blog/roadmap 、/blog/seekmoon-from-ide-to-ade | 已核
- moonbitlang org 91 repos、core 1.2k、openseek 98、moon 424 | https://github.com/orgs/moonbitlang/repositories | 已核
- Zero: 5,384 stars / 342 forks / created 2026-05-15 / pushed 2026-09-16 / desc "The Programming Language for Agents" | https://api.github.com/repos/vercel-labs/zerolang | 已核
- NanoLang: 628 stars、v5.0.0、7,200 commits、Coq 验证、spec.json 机器可读规范 | https://github.com/jordanhubbard/nanolang | 已核；HN 帖 232 分/202 评论（2026-01-19） | https://hn.algolia.com/api/v1/search?query=nanolang | 已核
- Mog: 144 stars / created 2026-02-01 / pushed 2026-03-09 | https://api.github.com/repos/voltropy/mog | 已核；官网 spec≈3200 tokens、rqbe 后端、无运算符优先级 | https://moglang.org | 已核
- GlyphLang: 65 stars / created 2025-12-16 / pushed 2026-09-14 / "An AI-Optimized Programming Language" | https://api.github.com/repos/GlyphLang/GlyphLang | 已核
- Valea: 4 stars、五原则、E001 JSON 诊断、修复闭环工作流 | https://github.com/hvoetsch/valea | 已核
- Agentis: 1 star / created 2026-04-07 / pushed 2026-09-14 / 描述已改为 "AI-native platform for autonomous agent emergence" | https://api.github.com/repos/Replikanti/agentis + https://hn.algolia.com/api/v1/items/47317010 | 已核
- At / PromptL / Pipelex / SudoLang 帖目数据 | https://hn.algolia.com/api/v1/search?query=programming%20language%20for%20LLMs 等 | 已核（帖子元数据）

观察（主观推测）：2025H2-2026 出现集中爆发——清单中 8/9 个通用语言项目诞生或首发于这 12 个月内；但除 Zero（公司级投入）与 NanoLang（资深个人长期投入）外，多数在 HN 首发后热度与开发强度同步衰减（Mog 最后一推即 HN 帖当日；Agentis 转型；Valea/At 停留在玩具规模）。

---

## 二、重点对象深读

### 2.1 MoonBit delta（相对上轮调研）

- 版本线：Beta 2025-06-18 → 1.0 Roadmap 2025-07-24 → 0.8.0 2026-02-09 → 0.9（引入一等形式化验证）2026-04-08 | https://www.moonbitlang.com/blog/ | 已核
- 1.0 目标："officially launch version 1.0 in the first half of 2026"（路线图原文）；截至本调研日（2026-10-11）**1.0 尚未发布**，最新版本线停在 0.9（2026-04）——已比路线图延期约一个季度 | https://www.moonbitlang.com/blog/roadmap | 已核（延期判断为主观推测）
- 1.0 内容方向：值类型原生支持、轻量异步运行时（结构化并发、defer）、正则/view 模式匹配、标准库稳定化 | 同上 | 已核
- 工具链进展（2025-2026）：LLVM 后端（2025-03）、LLDB 调试器（2025-10）、Wasm 版工具链（2025-10）、async（2025-09）、开源 Wasm 运行时（2025-12）、属性测试（2026-08） | https://www.moonbitlang.com/blog/ | 已核
- **AI 原生叙事显著加码**：MoonBit Pilot 代码智能体（2025-07-30）→ async 写 code agent 教程（2025-12-03）→ **SeekMoon："From IDE to ADE（Agent Development Environment）"**（2026-09-01，Token/Syntax-Tree Diff、语法树搜索直接喂 agent 上下文，开源在 moonbitlang/openseek）→ "Building Agents, MoonBit is all you need"（2026-10-10，调研前一天） | https://www.moonbitlang.com/blog/seekmoon-from-ide-to-ade 及博客列表 | 已核
- 社区/生态规模：mooncakes.io 首页统计 Packages 26,422 / Modules 2,940 / Downloads 8,234,501（首页数字，两字段对应关系可能互换，保守读作"约 3k 独立模块、820 万+ 下载"）；GitHub org 91 repos，core 1.2k stars，openseek 98 stars | https://mooncakes.io 与 https://github.com/orgs/moonbitlang/repositories | 已核（mooncakes 字段口径存疑已注明）
- 商业化（关键 delta）：雷峰网采访（2025-10-31，张宏波）——"如果不能商业化或者商业化不成功，那 MoonBit 就是一锤子买卖"；路径先 ToC 后 ToB，"未来我们想做成一个端到端的软件 deliver"（Devin 方向）；自称用户近十万；代码量到"一亿行以上"后做自有 post-training；2025-09-19 三周年 | https://m.leiphone.com/category/industrynews/aY43oiCDjnFqsKQM.html | 已核（"用户近十万"为创始人自述）
- 融资：未检索到已完成融资轮公告，项目仍在 IDEA 研究院体系内 | 搜索多轮无果 | 未核（负结论）
- 国际学术露出：SPLASH/PLSS 2026 报告 "MoonBit: Safe Wasm-Backed Skills for AI Agents" | 搜索摘要提及，署名 International Digital Economy Academy | 未核（未开会议原文）

### 2.2 Zero（Vercel Labs）——本轮最重要新对象

- 定位：图原生实验语言。"语义图本身就是程序数据库"；人只提目标，agent 查询图、提交 checked patch、证明结果，取代"写文本→报错→改文本" | https://github.com/vercel-labs/zerolang | 已核
- 修复闭环机制（与 Lom 命题直接可比）：① `zero patch` 受检编辑——stale 图哈希/非法形状/类型错误在写入 store **之前**被拒绝；② 显式语义句柄（symbols、node IDs、graph hashes、types、effects、ownership facts、capabilities、call edges），编辑针对语义结构而非行号；③ `.0` 文件只是人类可读投影，真源是 `zero.graph`，`zero verify-projection` 防分叉；④ `zero query/inspect/check` 把编译器事实以 agent 友好命令暴露 | 同上 | 已核
- 注意：README **没有**展开"结构化诊断码"式的错误输出格式——Zero 的修复保障走"受检编辑+图查询"，与 Lom 的"诊断码+fix 引擎"是同命题下的不同技术路线 | 同上 | 已核（差异判断为主观推测）
- 规模：5,384 stars / 342 forks / 47 open issues / 82 open PRs / 1,188 commits；Apache-2.0；官网 zerolang.ai | GitHub repo 页与 API | 已核
- 发布细节（v0、sub-10KiB 二进制、Chris Tate 介绍）转自 MarkTechPost/HackerNoon | https://www.marktechpost.com（搜索摘要） | 未核（仓库本体已核）

### 2.3 Valea——与 Lom 最同构的小项目

- 五原则：确定性语法（one obvious way）、显式语义（无隐藏分配/异常）、**机器可读诊断**、规范化格式（唯一风格）、小语言表面（"便于自动修复循环"） | https://github.com/hvoetsch/valea | 已核
- 诊断 API：`valea check examples/type_error.va --json` 输出含**错误码（如 E001）**、消息、行、列的 JSON；另有 `valea ast --json`、`valea fmt`、C 后端 `valea emit-c`；典型工作流即"agent 写码→编译器回结构化诊断→agent 修复→编译运行"；仓库还带 AGENTS.md / CLAUDE.md / MANIFESTO.md | 同上 | 已核
- 规模与热度：4 stars、24 commits、Rust MVP——**设计蓝图与 Lom 高度重合但几乎无人知晓**（HN 帖 1 分） | repo 页 + https://hn.algolia.com/api/v1/items/47355138 | 已核

### 2.4 NanoLang 与 Mog（补充深读）

- NanoLang：prefix/infix 双记法（prefix 无歧义）、shadow 测试块、**Coq 机械证明**（类型健全/进展/确定/大小步语义等价，零 Axiom）、NanoISA 161 opcodes 栈式 VM、FFI 隔离进程、转译 C + 实验 PTX/RISC-V、`MEMORY.md` 与机器可读 `spec.json` 供 LLM 训练/参考、`-pg` JSON profile 供 agent 读取 | https://github.com/jordanhubbard/nanolang | 已核
- Mog：无运算符优先级、无隐式转换、无 truthy/falsy；capability 系统（程序自身零 I/O，宿主授权）；rqbe（QBE 的安全 Rust 重写）编译原生码、毫秒级编译；spec≈3200 tokens；明确声明未安全审计 | https://moglang.org | 已核
- 两者对照（主观推测）：NanoLang 走"工程完备+形式化"重路线且持续迭代，Mog 走"最小规范+安全模型"轻路线但 3 月后无推送——同一命题下投入深度决定存续。

### 2.5 与"修复闭环/诊断设计"命题直接相关的规范与学术线

- **ComBench**（arXiv 2603.27333，2026-03-28，Jia Li / Michael R. Lyu 等）：首个 C/C++ 编译错误修复的 repo 级真实基准（从 GitHub CI 挖掘真实编译失败、执行验证）；12 个 LLM 评测——**GPT-5 语法正确率 73% 但语义有效补丁仅 41%**，鸿沟显著；编译器提供 error codes/locations/messages 结构化诊断作为修复输入 | https://arxiv.org/abs/2603.27333 | 已核
- **"AI Coders Are among Us"**（arXiv 2404.16333，ISSTA'24，Zhensu Sun 等）：提出 SimPy（面向 AI 的 Python 语法变体），CodeLlama/GPT-4 token 用量分别降 13.5%/10.4%，性能保持或提升——"面向 LLM 的语法"学术开山作 | https://arxiv.org/abs/2404.16333 | 已核
- Token Sugar: Making Source Code Sweeter for LLMs（2025-12）——SimPy 后续 | 搜索摘要 | 未核
- Elm 社区讨论 "Is Elm a great language for AI to code in?"（2026-03）：既有语言靠"好编译器错误 + LSP 稳定机器可读诊断（含 error codes、JSON）"被重新评估为 AI 友好——**"诊断友好"资产被存量语言兑现** | https://discourse.elm-lang.org（搜索摘要） | 未核
- MoonBit SeekMoon 与 Vercel Zero 均为"修复闭环"命题的产业侧实现（见 2.1/2.2） | 已核

---

## 三、对照组：小生态动态语言/WASM 系的生存形态（非 AI 原生）

| 语言 | stars | 最新版本 | 活跃度 | 定位与演化 |
|------|-------|---------|--------|-----------|
| **Gleam** | 21,987 | v1.19.1（2026-10-07） | pushed 2026-10-08，高频发版 | "friendly language for type-safe, scalable systems"，BEAM + JS 双目标；1.x 后进入稳定小步迭代，卖点是 BEAM 生态复用与内置全家桶工具链 | 
| **Roc** | 6,104 | alpha4-rolling（2025-08-26） | pushed 2026-10-11（当日仍有提交） | "fast, friendly, functional"，机器码/WASM；**多年仍卡 alpha**，官网正为全职贡献者募捐每月 $4,000（Roc Foundation，501(c)(3)） |
| **Grain** | 3,466 | v0.7.2（2026-02-08） | pushed 2026-09-25 | "modern web staple"，WASM 优先的 ML 系语言；0.x 缓慢推进 |

- Gleam 数据 | https://api.github.com/repos/gleam-lang/gleam 及 /releases/latest、https://gleam.run | 已核
- Roc 数据 | https://api.github.com/repos/roc-lang/roc 及 /releases/latest、https://www.roc-lang.org | 已核
- Grain 数据 | https://api.github.com/repos/grain-lang/grain 及 /releases/latest | 已核
- 生存形态小结（主观推测）：小语言的三种活法——Gleam（借宿主生态 BEAM，1.x 后"无聊地稳定"）；Roc（理想主义长期 alpha，靠基金会化续命）；Grain（0.x 慢火）。共同点：都在 8-10 年尺度上经营，无一靠"AI 原生"叙事；stars 与发版节奏在此尺度上比叙事更诚实。

---

## 四、信号面：命题热度与代表观点

热度锚点（均已核）：NanoLang HN 232 分/202 评论（2026-01）；Mog 163 分/83 评论（2026-03）；Pipelex 122 分（2025-10）；Zero 未走 HN 首发但 5 个月 5.4k stars。同时段 Valea 1 分、Agentis 2 分、At 1 分——**头部热、长尾冷**。

支持方引文：
1. reitzensteinm（Mog 帖）："编码是可验证领域，可以合成训练数据；相比说服人类群体学习新语言，为 LLM 造语言实际上 things have gotten easier, not harder" | https://news.ycombinator.com/item?id=47312728（经 Algolia items API） | 已核
2. rtfeldman（Roc 作者，Mog 帖）："The permission model is almost identical to Roc's"——存量语言作者将其视为路线交汇 | 同上 | 已核
3. 张宏波（雷峰采访）："Python 在 90 年代就开始开发了……它们优化的点和 AI 时代需要的完全不一样"；"我们把 AI 当成用户看待" | https://m.leiphone.com/category/industrynews/aY43oiCDjnFqsKQM.html | 已核

质疑方引文：
1. saithound（Mog 帖）："LLMs very much do care"（反驳"LLM 不在乎表达力"）——LLM 在非标准运算符优先级语言中表现可测量地更差 | https://news.ycombinator.com/item?id=47312728 | 已核
2. andreybaskov / steve_adams_86（Mog 帖）：新语言没有语料库，任何既有语言在 AI 辅助下 "would work better"——训练数据缺失是采纳死结 | 同上 | 已核
3. TheDong / olekrk（Mog 帖）：真正该做这件事的是 AI 实验室自己（海量语料+RL）；独立新语言陷入"模型写不好它→不流行→模型更写不好"的鸡生蛋循环 | 同上 | 已核
4. dana321（Mog 帖）："Why didn't you just call it 'bad rust copy'?" | 同上 | 已核
- akitaonrails（2026-02-09，"What Would Be the Best Programming Language for LLMs?"）："The idea of 'readable syntax' is a human concern. Lisp S-expressions or even raw AST notation would be enough"——-syntax 优化是伪需求、modularization 与 structured tracing 才是一等公民 | https://www.akitaonrails.com/2026%2F02%2F09 | 未核（原文 404，引句转自搜索摘要，谨慎使用）

---

## 三段式浓缩结论（执行者视角，供规划者复核）

**赛道现状**：命题已从 2024-2025 的"想法级"（SudoLang/PromptL/学术 SimPy）进入 2026 的"产品级"——三股力量同时落地：大厂实验（Vercel Zero，5 个月 5.4k stars，把"agent 编辑程序图+受检补丁"做成核心）、资深个人长期工程（Jordan Hubbard 的 NanoLang，v5.0+Coq 形式化）、研究机构商业化（MoonBit 0.9、SeekMoon ADE、雷峰口径"正式开启商业化"）。但长尾（Valea/Agentis/At/GlyphLang）热度与存续极弱；学术线（SimPy、ComBench）恰好给出命题的双重证据——token 层面可省 10-13%，而"编译器反馈驱动的修复"在 GPT-5 上仍有 73% 语法 vs 41% 语义的巨大鸿沟，说明修复闭环是真问题、且现有工具没解决。

**Lom 的差异化位置**：本轮最直接的"撞型"是 Valea（结构化诊断码 E001+JSON 诊断+修复闭环+Rust 实现+确定性格式化，蓝图几乎逐条重合）——但它只有 4 stars，说明**这条路线本身还没有被任何有分发能力的主体占据**；Zero 与 MoonBit 走的是"图/ADE+agent 工具链"上层路线，均未把"诊断码+fix 引擎"做成语言的存在理由本身（Zero README 无诊断码格式展开，MoonBit 靠 IDE/Pilot 层）。因此 Lom 的"LLM-repair-native：诊断码为一等公民+fix 引擎内置+修复闭环是语言存在理由"仍是**空位**，且 ComBench 的 73/41 鸿沟可作为其问题陈述的硬引用。差异化代价也明确：HN 质疑方的主流论点（无训练语料→模型写不好→不流行）对 Lom 同样成立，"零第三方 crate+树遍历+WASM 双后端+自举"的工程姿态更接近 NanoLang 路线，需要同等的多年级投入预期。

**空白与威胁**：① 最大威胁是 Zero——Vercel 有分发、有 agent 生态，一旦其 `zero check`/诊断层补上结构化错误码与修复建议（其 conformance/evals 目录暗示在布局），"修复闭环"叙事将被其覆盖；② 第二威胁是 MoonBit 的 SeekMoon/ADE——它把"agent 拿编译器事实"做进 IDE 层而非语言层，若成为国内事实标准，Lom 的语言层方案会被"框架层方案"截胡；③ 空白在于：没有任何项目把"诊断码注册表+fix 引擎+修复闭环"作为**规范**提出（Zero/Valea 都是私有实现）——Lom 若把诊断协议做成可被其他工具消费的稳定规范（哪怕语言本身小），有机会从"又一门新语言"升维成"agent-编译器接口标准"的卡位者；④ 对照组启示：Gleam/Roc/Grain 的 8-10 年存活依赖宿主生态或基金会化，AI 原生叙事只影响冷启动，不影响长跑——Lom 的 WASM 双后端+自举路线与 Roc/Grain 同类，需以年为单位设定里程碑预期（以上 ①-④ 均为主观推测）。
