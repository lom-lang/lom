> **归档说明**：本文件为第五/六轮外部调研原始报告归档（2026-10-11 执行、2026-10-12 归档）。
> 原始位置 target/probes/survey*/report_*.md（gitignored）；原文零改动照档，仅加本头。
> 规划者复核：关键事实抽验六项吻合（Zero diagnostics.md 原文/Vera README 逐字/MoonBit SeekMoon/agentlanguages.dev/OpenAI 弃榜文存在性/GitHub API 数字）；
> 未核项以原文内 [未核] 标记为准。铁律 7 纪律：未安装任何竞品（文档级深读）。
> 调查背景与结论消费见 TODO 顶部状态块（2026-10-12 裁决：γ 评测资产先行 + β 规范升维主轴）。

# survey6 · 7 对象 × 10 维横向评定（文档级深读）

执行者：Lom 第六轮调研横向测评。调研日：2026-10-11。
方法：纯文档级（官方 README / docs / 博客 / GitHub API / HN Algolia API / arXiv / WebSearch），未安装任何竞品，未用 git。
标记规则：**[已核]** = 执行者打开原文（curl raw / API / WebFetch）；**[未核]** = 仅来自搜索摘要或二手转述。
评分：P3=产品级完备；P2=可用但初级；P1=宣称/实验；P0=无。

---

## ① 总表（7 对象 × 10 维）

| 维度 | Vercel Zero | NanoLang | MoonBit | Valea | Mog | Vera | Gleam(参照) |
|---|---|---|---|---|---|---|---|
| D1 修复闭环 | **P3** 码+fix计划+evals | P1 无码表,trace兜底 | P2 304码表+CI校验 | P1 E001 JSON | P1 无诊断体系 | **P3** 186码+fix指令+bench | P2 友好错误+code action |
| D2 agent接口 | **P3** query/patch/skills | P2 LSP/DAP/-pg JSON | P2 LSP+SWE-AGI | P1 check/ast --json | P1 编译器嵌入+context.md | **P3** 全--json+LSP4法 | P2 LSP成熟 |
| D3 语义保证 | P2 类型/效应/所有权 | **P3** Coq完备证明 | P2 强类型+实验prove | P1 静态类型MVP | P2 类型+能力沙箱 | P2 Z3合同强制 | P2 类型+穷尽 |
| D4 工程完备 | P2 C后端+conformance | P2 C+VM+自举中 | **P3** 4后端+全工具链 | P1 MVP | P2 纯Rust+1332测试 | P2 Python+WASM | **P3** 10年+BEAM/JS |
| D5 编辑模型 | **P3** 语义图+受检patch | P1 纯文本 | P1 纯文本 | P1 纯文本 | P1 纯文本 | P2 LSP受检编辑 | P1 纯文本 |
| D6 token/语法效率 | P1 宣称无数字 | P1 宣称被批不可证伪 | P2 KV-cache设计+约束采样 | P1 宣称 | **P2** spec=3200token实测 | P2 bench 98.7% | P0 无 |
| D7 运行时/性能 | P2 native+自测指标 | P2 C转译+PGO | **P3** 15x Java(FFT) | P1 C后端无数据 | P2 QBE native宣称 | P1 WASM无数据 | P2 BEAM+JS |
| D8 生态分发 | P2 5384★+Vercel | P2 628★+名人 | **P3** 公司IDEA+生态 | P0 4★停更 | P1 144★ | P1 418★单人 | **P3** 21987★+19公司 |
| D9 开放治理 | P2 Apache-2.0开放 | P2 Apache-2.0+spec | P2 Apache-2.0公司治 | P1 README称MIT无LICENSE | P1 MIT | P2 MIT+llms.txt | **P3** 社区+大会 |
| D10 使用侧 | P1 媒体多,项目无 | P1 Willison实验 | P2 真实项目+链上 | P0 无 | P1 无用户 | P1 仅第一方bench | **P3** 19+公司生产 |

---

## ② 逐对象逐维证据

### ① Vercel Zero（vercel-labs/zerolang）
- 基本盘：2026-05-15 建 repo，最近 push 2026-09-16，5384★/342 fork，C 实现，Apache-2.0，最新 release v0.3.4（2026-06-13）。[已核] https://api.github.com/repos/vercel-labs/zerolang
- D1 **P3**：诊断文档标题即 "Read The Error As A Repair Contract"；错误码族 NAM003/BLD002/RGP003/GRF013/CIMP003…，每条含 rule/expected/actual/fix(local-edit)；`zero explain NAM003`、`zero fix --plan --json`（返回修复计划而非自动改写）；conformance/diagnostics 有 unknown-name.expected.json 机器校验；evals/ 是真实修复闭环评测（Vercel Sandbox 内跑 Claude Opus-4.7/Sonnet-4.6，Rosetta 用例+agent-scale 套件，"eval system prompt 刻意不给语法示例，模型须从 version-matched skills + 编译器反馈学习"）。[已核] https://raw.githubusercontent.com/vercel-labs/zerolang/main/docs/articles/diagnostics.md ；…/evals/README.md ；…/conformance/diagnostics/
- D2 **P3**：agent 面 = `zero init/query/patch/check/test/run/explain/fix/doctor/import/export/skills`，`zero check --json`、`zero fix --plan --json`、`zero doctor --json`；patch 以 op 序列提交受检图编辑（"invalid or stale 在写入前失败"）；编译器捆绑 version-matched skills（agent/graph/language/stdlib），`npx skills add vercel-labs/zerolang` 分发；conformance/agent-surface/ 独立目录。[已核] README + AGENTS.md + repo contents
- D3 **P2**：图节点携带 "types, effects, ownership facts, capabilities"，补丁先验证后落盘；无形式化验证。[已核] README
- D4 **P2**：native C 后端（native/zero-c）、format/lexer/packages 等 11 类 conformance、command-contracts、benchmarks（buildMs/runMs/artifactBytes/peakRssBytes）；pre-1 实验自认 "Expect breaking changes... security issues"。[已核] benchmarks.md + AGENTS.md
- D5 **P3**：`.0` 文件只是 `zero.graph` 的文本投影，"Agents should normally keep using zero query and zero patch"；human 文本编辑须经 `zero import` 回图并 `zero verify-projection`。[已核] README
- D6 **P1**：运行时目标列 "Token-efficient inspection"，无 token 数字；"sub-10 KiB binaries" 仅见于 agentlanguages.dev 条目。[未核] https://agentlanguages.dev/
- D7 **P2**：native C 编译 + 交叉编译文档 + 自有 bench 指标体系；无对第三方语言的公开对比数。官网自述 "regression signals... not broad marketing claims"。[已核] benchmarks.md
- D8 **P2**：5384★/4 个月 + Vercel Labs 背书 + 一键安装脚本 + skills 生态；无商业化。[已核] GitHub API
- D9 **P2**：Apache-2.0，docs 全开（zerolang.ai），但定位 "Vercel Labs Experiment" 徽章、单一公司治理。[已核] README 徽章 + zerolang.ai
- D10 **P1**：InfoQ（2026-08-06 "Vercel Labs Ships Zero: a Graph-First Language Built So AI Can Read, Repair, and Ship"，未核）、MarkTechPost/HackerNoon/Product Hunt 报道（未核）、coddy.tech 第三方 playground（未核）；HN 无大帖（最高 20 分/7 评）。[已核] https://hn.algolia.com/api/v1/search?query=zerolang ；第三方项目未发现。

### ② NanoLang（jordanhubbard/nanolang）
- 基本盘：2025-09-30 建，push 至今日（活跃），628★/24 fork，C，Apache-2.0，v5.0.0。作者 Jordan Hubbard（FreeBSD 联合创始人、前 Apple、NVIDIA GPU 计算软件高级总监，未核身份细节，核自 HN/媒体转述）。[已核] GitHub API + README
- D1 **P1**：无错误码注册表、无 --json 诊断、无修复评测集；闭环资产是"精确行号 + built-in trace mode"——作者在 HN 自述（jkh99）："This is why I made nanolang always report line numbers accurately and also have a built-in 'trace mode'... The LLM uses this aggressively for debugging!"。[已核] https://hn.algolia.com/api/v1/items/46684958
- D2 **P2**：`-pg` 输出 agent 可读采样 JSON（--profile-output）、`--profile`/`--trace` 插桩、`--pgo` 轮廓引导内联；LSP + DAP + VS Code 扩展 + Web Playground；~1400 行 MEMORY.md（LLM 消费文档）。[已核] docs/PERFORMANCE_MONITORING.md + README + HN(topspin)
- D3 **P3**：全组唯一 Coq/Rocq 完备证明——"type soundness, progress, determinism... complete and Admitted-free"，rocqchk 独立复核 12 个编译库、43 份假设报告全 "Closed under the global context"，CI 每次推送跑证明门；另有 shadow 测试政策（项目策略强制有用测试）。[已核] formal/README.md + README
- D4 **P2**：C 转译（生产路径）+ NanoISA VM（161 opcode、字节码验证、FFI 隔离进程）、自举进行中徽章、CI、六语言消息目录。[已核] README
- D5 **P1**：纯文本编辑；双记法（前缀/中缀）宣称无歧义。[已核] README
- D6 **P1**："unambiguous syntax" 宣称；HN 批评（forgotpwd16）：前缀记法理由 "is based on vibes really"，LLM 优化主张 "unvalidatable and unfalsifiable"。[已核] HN 46684958
- D7 **P2**：native C + PGO；"measured dispatch" 等自测数据；无公开跨语言对比。[已核] README/docs
- D8 **P2**：628★ + FreeBSD/Apple/NVIDIA 名人效应 + 每日推送。[已核] GitHub API
- D9 **P2**：Apache-2.0；SPECIFICATION.md/QUICK_REFERENCE 等全量开放文档。[已核] README
- D10 **P1**：Simon Willison 实测（2026-01-19）：MEMORY.md 单发 Claude Opus 4.5 写 Mandelbrot 失败；Claude Code 进 repo 后 "it worked!"，结论 "LLMs and coding agents might significantly reduce the friction involved in launching a new language"；HN 232 分/202 评；无第三方项目。[已核] https://simonwillison.net/2026/Jan/19/nanolang/ + HN

### ③ MoonBit（moonbitlang）
- 基本盘：2023-08 发布（官方博客自述）；moonbitlang/core 1225★、moon 424★（均 Apache-2.0）、官网 star 徽章 2457；运营方 IDEA（深圳，"focuses on the development of an AI-native general-purpose programming language"）。[已核] GitHub API + moonbitlang.com
- D1 **P2**：`next/language/error_codes/` 共 304 个文件（E0001–E4228），配套 `check_error_docs.py` 在 CI 里对每个码跑受检示例工程、跳过码需注释理由——产品级错误码注册表；但面向 agent 的 --json 诊断输出未见于文档（未发现）。[已核] https://api.github.com/repos/moonbitlang/moonbit-docs/contents/next/language/error_codes + check_error_docs.py
- D2 **P2**：VS Code/LSP IDE、moon 构建/测试/bench、mooncakes 包管理；独有 "MoonBit AI" 约束采样（见 D6）；SWE-AGI 提供完整 agent 评测接口（docker 化 client/server、公开/私有测试拆分、仅对隐藏私有测试计分）。[已核] docs + moonbitlang/SWE-AGI README
- D3 **P2**：现代类型系统（结构化接口、代数类型、值类型）；`moon prove` 实验性 SMT 形式验证（.mbtp 谓词/引理、循环不变式）。[已核] next/language/verification.md
- D4 **P3**：WASM/JS/Native/LLVM 四后端（"all three backends share most of the foundational framework"）；moon（构建+包管理，Apache-2.0）、moonfmt、Web IDE、Online Judge、Tour、教程课程；核心库用 MoonBit 自举（core repo 语言=MoonBit）。[已核] GitHub API + /blog/native/
- D5 **P1**：纯文本编辑模型，无受检编辑。[已核] 文档体系未见相反证据
- D6 **P2**：官方博客 "Flattened Design, Adapted for Transformer Architecture"——无嵌套+顶层强制签名 = "KV-cache friendly... minimizing the KV cache misses"；"MoonBit AI" 本地+全局采样、speculation buffer 回退防坏 token；无 token 节省数字。[已核] https://www.moonbitlang.com/blog/moonbit-ai/
- D7 **P3**：native 博客（2024-10-17）："15x faster than Java"（FFT）、"over 4x compared to GraalVM"；Perceus 系基准内存超 Java/Swift；多后端全公开数据。[已核] https://www.moonbitlang.com/blog/native/
- D8 **P3**：公司全职团队 + 季度黑客松（奖池至 RMB 63,000）+ mooncakes 生态 + ESP32 等落地博客 + 学术产出（SWE-AGI 论文，ICSE 发表说法未核）。[已核] moonbitlang.com 首页横幅
- D9 **P2**：core/moon 均 Apache-2.0、文档/博客全开放；但语言演进由 IDEA 公司主导（商标与治理归公司，未见基金会/标准组织参与）。[已核] GitHub API + 官网 About Us
- D10 **P2**：见使用侧专节。

### ④ Valea（hvoetsch/valea）
- 基本盘：2026-03-12 建，最后 push 2026-03-13（**隔日停更**），4★/0 fork，Rust，GitHub license 字段为 null（README 尾部自称 MIT）。作者 Holger Vötsch（toolbox Systemberatung GmbH，德国咨询公司；GitHub 账号 2025-11 新建）。[已核] GitHub API（repo+user）
- D1 **P1**：`valea check --json` 输出 `[{"code":"E001","message":"type mismatch","line":2,"col":5}]`——机器可读诊断 MVP；无码表规模、无 fix 建议、无评测集。[已核] README
- D2 **P1**：`valea ast --json`、`valea fmt`、`valea emit-c`；asciinema 演示。无 LSP/无 agent 协议。[已核] README + https://asciinema.org/a/834560
- D3 **P1**：静态类型（i32 等）MVP；无验证。[已核] README 示例
- D4 **P1**：Rust MVP 编译器（deterministic lexer/parser/formatter、简单 C 后端），cargo test 存在；无 CI/工具链广度证据。[已核] README "Project Status"
- D5 **P1**：纯文本 + "Canonical formatting: one stable style" 宣称。[已核] README
- D6 **P1**："Deterministic syntax / fewer edge cases and easier automated repair loops" 宣称，无实测。[已核] README
- D7 **P1**：C 后端 MVP，无性能数据。[已核] README
- D8 **P0**：4★、0 fork、一天生命、无分发渠道。[已核] GitHub API
- D9 **P1**：README 称 MIT + SPEC.md 开放；但仓库无 LICENSE 文件（不一致）。[已核] GitHub license:null + README
- D10 **P0**：未发现任何使用痕迹（fork 为 0）。agentlanguages.dev 将其收录为 "Valea (AI-native systems language from a Google engineer...)"——"Google 工程师" 说法与其 GitHub 档案（toolbox Systemberatung）矛盾，判 [未核] 存疑。

### ⑤ Mog（voltropy/mog）
- 基本盘：2026-02-01 建，最后 push 2026-03-09，144★/9 fork，Rust，MIT；作者 "Ted"（HN: belisarius222）。[已核] GitHub API + HN
- D1 **P1**：README/docs 未见错误码体系、--json 诊断或修复建议；修复闭环依赖 host 能力与编译失败信号。[已核] README
- D2 **P1**：编译器是 Rust 库 + C API（cdylib/staticlib + mog_compiler.h），可进程内嵌入；docs/context.md "Compact reference designed to fit in an LLM's context window"；showcase.mog 755 行全特性演示。无诊断 JSON/AST 导出。[已核] README
- D3 **P2**：静态类型 + GC + 边界检查 + no null/no 指针；能力模型使脚本 "no way to escape sandbox... or crash the process"；无形式化。[已核] README
- D4 **P2**：纯 Rust 工具链（mogc + rqbe 安全 Rust 版 QBE ~15K 行 + runtime ~6K 行），编译器测试 1,146+ / rqbe 186；插件（.so/.dylib）编译与链接。[已核] README
- D5 **P1**：纯文本；无运算符优先级（非结合必须括号）是唯一"编辑安全"设计。[已核] gist/essay + HN
- D6 **P2**：**全组唯一 token 硬数字**——"the full spec fits in 3200 tokens"（essay tl;dr）+ 哲学第一条 "the entire language should fit in an LLM's context window"。[已核] https://gist.github.com/belisarius222/203ac5edbc3306c34bf0481f451d4003
- D7 **P2**：QBE 编 native（"tight loops run at C speed"）、进程内 AOT 加载无 IPC；无公开基准数据。[已核] README + HN（Retr0id 问询）
- D8 **P1**：144★；voltropy 小团队；HN Show 帖 163 分/83 评。[已核] GitHub API + HN 47312728
- D9 **P1**：MIT；单一 org 治理，无规范组织。[已核] gist + repo
- D10 **P1**：HN 无人报告使用；rtfeldman（Roc 作者）指出其权限模型 "almost identical to Roc's platforms approach"；作者自述 OpenClaw 集成 "I have not written... but it should be doable"。[已核] HN 47312728

### ⑥ Vera（aallan/vera，veralang.dev）
- 基本盘：2026-02-22 建，push 至今日（活跃），418★/27 fork/191 open issues，Python，MIT，PyPI veralang 当前 0.2.1；作者 Alasdair Allan（veralang.dev 页脚 "© 2026 Alasdair Allan"；亦维护 agentlanguages.dev 目录，存在利益相关）。[已核] GitHub API + PyPI JSON + veralang.dev
- D1 **P3**：错误码注册表实测 **186 个**（E001–E703，llms-full.txt 枚举）；`vera errors` 列码、"Diagnostics carry stable codes, which you can pin tooling to"；每错含 what/why/"Fix:" 具体示例/spec 章节引用（"Errors are instructions"）；全诊断命令 --json；VeraBench 修复/生成评测（60 题×5 难度×9 模型，含 spec-from-nl 模式）；repo 挂 mutation score 徽章。[已核] https://veralang.dev/llms-full.txt + TOOLCHAIN.md + README + vera-bench README
- D2 **P3**：`vera check/verify/test/compile/run/fmt/serve/lsp` 全部 --json；自省命令 `vera errors/builtins/effects --json`、`vera check --explain-slots`；LSP 带 warm Z3 会话 + **4 个 agent 专用自定义方法**（vera/speculativeEdit、proposeEdit、strengthenContract、addEffect），返回 proof_delta（newly_discharged/newly_undischarged/proof_regressions），编辑 "applies only through the verification gate"；SKILL.md/AGENTS.md/CLAUDE.md/llms.txt 分发。[已核] LSP_SERVER.md + TOOLCHAIN.md + veralang.dev
- D3 **P2**：每函数强制 requires/ensures/effects，Z3 静态 discharge + 分级（证明/反例/运行时检查 E532）+ 运行时陷阱兜底；SQL 注入 E207 类型期拒绝；非全语言机械化证明（对比 FAQ 自述 Dafny/Lean 定位）。[已核] README + FAQ 链接
- D4 **P2**：编译到 WASM（browser bundle / wasi-p2）、fmt 确定性幂等、VS Code 市场/Vim/TextMate、CI+codecov；但实现语言 Python、编译器本身非产品级性能。[已核] veralang.dev + TOOLCHAIN.md + CI 徽章
- D5 **P2**：纯文本 + 唯一 canonical form + LSP 版本护栏受检编辑（version-guarded edits + documentChanges）。[已核] LSP_SERVER.md
- D6 **P2**：无 token 数字；VeraBench 实测："Six of the nine models solve every problem in Vera, a language absent from every training set... Vera averages 98.7%, ahead of Python's 96.7%"，零训练数据对比 Vera 98.2%/AILANG 96.8%/Aver 92.4%。[已核] vera-bench README
- D7 **P1**：WASM/wasmtime 运行；无任何性能数据（设计优先正确性）。[已核] README
- D8 **P1**：418★ + PyPI/VS Code 市场分发 + 每日提交；无公司/机构背书、无第三方贡献生态。[已核] GitHub API
- D9 **P2**：MIT + llms.txt/开放 FAQ/CHANGELOG/ROADMAP；单人维护。[已核] repo + veralang.dev
- D10 **P1**：见使用侧专节。

### ⑦ Gleam（gleam-lang/gleam，参照系）
- 基本盘：2016-06-30 建，21987★/1034 fork，Rust 实现，Apache-2.0，v1.x（v1.0 于 2024 年初，精确日期未核）。[已核] GitHub API
- D1 **P2**：产品级友好错误（"Unknown record field... Did you mean `name`?"）+ LSP code actions/rename/find-refs/auto-imports；但无错误码注册表、无 --json 机器诊断、无修复评测集——作为"非 AI 原生参照系"恰为对照。[已核] gleam.run 首页 + news 列表
- D2 **P2**：内置编译器/构建/格式化/包管理/编辑器集成（gleam new/add/test/docs）；LSP 能力多年迭代（auto-imports、convenient-code-actions、global-rename）；无 agent 专用接口。[已核] gleam.run + /news/ 索引
- D3 **P2**："No null values, no exceptions" + 穷尽匹配 + 实用类型系统；无形式化验证、无强制测试。[已核] gleam.run
- D4 **P3**：BEAM + JavaScript 双目标（v0.16 起 JS、JS 性能 +30% 博客）、增量编译、monorepo、escript 单文件、GitHub Security Lab 审计合作；十年迭代节奏（news 列表 60+ 篇）。[已核] gleam.run/news/ + v0.16/v0.26 条目
- D5 **P1**：纯文本 + gleam format 规范格式。[已核] gleam.run
- D6 **P0**：无任何 LLM/token 优化宣称（符合参照系定位）。[已核] 官网/文档未见
- D7 **P2**：BEAM（"powers planet-scale systems such as WhatsApp"）+ JS；无 native 后端。[已核] gleam.run
- D8 **P3**：21987★、官方问卷 841 人（632 人在用、52 人已上生产）、社区生产公司清单、Gleam Gathering 2027 伦敦（已官宣）。[已核] GitHub API + survey + news
- D9 **P3**：Apache-2.0、社区治理、公开 roadmap、公开问卷与安全流程；最接近"开放语言项目"标准形态。[已核] /news/introducing-the-gleam-roadmap 等
- D10 **P3**：见使用侧专节。

---

## ③ 使用侧证据专节（本轮重点）

**Gleam —— P3，唯一"真生产"**
- 社区维护生产公司清单（19+ 家）：ArcStone、CroCoder、Fostrom（IoT 云）、Goaty Football、HayHay（AI 马匹护理）、Kalvad（附工程博客）、Medfin（英国牙科数据层）、Nambi Biolabs、Nestful（全站 Gleam 重写博客）、nexum、Numenon、Ottimizza、Questel（知识产权平台）、Santomot、Steerlab、Strand Communications（官方案例研究）、Surtoget、CMU Simon Initiative（附 LTI 示例工具）等。[已核] https://github.com/isaacharrisholt/gleam-in-production
- 官方案例研究页 + 2024 问卷：841 受访、52 家生产（"Around 8%... despite the language being less than a year old"）。[已核] https://gleam.run/news/developer-survey-2024-results/
- 个人实践报告：Lukas Werner "Shipping Gleam to Production"（2025-02，URL shortener，未核）。HN 上 rapind 称赞 Gleam 适合 agent 写代码（Mog 帖内，已核）。

**MoonBit —— P2，公司推动的真实使用**
- awesome-moonbit：wasm5（MoonBit 写的 WASM 虚拟机）、mbtcc（MoonBit 写的 C 编译器，官方博客 moonbit-c-compiler 佐证）等。[已核] awesome-moonbit 存在（搜索）+ moonbitlang.com/blog 索引含 moonbit-c-compiler [已核]
- 链上部署：Dfinity 论坛（2025-04）报告 Internet Computer Canister CDK 全 MoonBit 上链。[未核] forum.dfinity.org
- 嵌入式：官方 ESP32-C6 实机博客。[已核] /blog 索引 moonbit-esp32
- **元使用（对 Lom 最相关）**：SWE-AGI（moonbitlang 官方，2026-02，20★）——"first open-source benchmark for evaluating end-to-end, specification-driven construction of production-scale software systems in MoonBit"，任务=从 RFC/标准实现解析器/解释器/二进制解码器/SAT 求解器，"retrieval-resistant"（MoonBit 语料污染少），公开/私有测试拆分仅对隐藏测试计分。MoonBit 正把自己变成 agent 能力基准的宿主语言。[已核] https://github.com/moonbitlang/SWE-AGI README
- 知名第三方独立项目/公司生产系统：未发现（黑客松/课程/OJ 为主）。

**Zero —— P1，声量与使用脱节**
- 第三方媒体报道密集：InfoQ（2026-08-06）、MarkTechPost（2026-05-17）、HackerNoon（2026-05-18）、Product Hunt（2026-08-22）[均未核，来自搜索]；第三方 playground coddy.tech [未核]。
- HN 无成型讨论（最高 20 分/7 评，另有多个 1–4 分小帖）。[已核] HN Algolia
- 用 Zero 写的第三方项目：未发现（repo 4 个月、evals 全部第一方）。

**NanoLang —— P1，一个高质量外部实验**
- Simon Willison（2026-01-19）：单发失败 → Claude Code + repo 内编译器闭环成功；引申"agent 大幅降低新语言启动摩擦"。[已核]
- HN 46684958：232 分/202 评；simonw 实测 + cmrdporcupine/vidarh 报告 LLM 在其他小众自造语言上也能工作（反馈回路 > 训练语料）。[已核]
- 第三方项目：未发现。

**Vera —— P1，闭环只在第一方**
- VeraBench（作者自建）：9 模型×3 供应商，Vera 平均 98.7% > Python 96.7%（差 TS 99.7% 约 1 点）；HN 讨论中 kgeist 指其用 "single-run results" 与较旧模型，方法学存疑。[已核] vera-bench + HN 47955118
- HN 47955118（111 分/95 评）：无人报告使用 Vera；对 De Bruijn 无名设计的批评占主导（"names carry useful semantic info the design discards"—danpalmer；grep 不可用、插变量要重编号—Dragon-Hatcher）；正面锚点："The strongest idea here, IMO, is not the syntax but the feedback loop"（MarceliusK）。[已核]
- HN 引述："No LLM has ever been trained on Vera. There are no Vera examples on GitHub, no Stack Overflow answers" [未核，HN 转述]。第三方 Vera 项目：未发现。

**Mog —— P1，零使用**
- HN Show 47312728（163 分/83 评）：rtfeldman 认可权限模型；主流质疑=训练语料缺口与 "Deno+TS 已解决"；无人使用报告。[已核]
- 作者自述目标场景（OpenClaw 插件/hooks）尚未落地集成。[已核] gist

**Valea —— P0**
- 0 fork、4★、一天停更。除 agentlanguages.dev 收录外无任何外部痕迹。[已核] GitHub API

**横切参照**：agentlanguages.dev（Alasdair Allan 维护）收录 42 门 agent 语言（Syntactic/Verification/Orchestration 三营），Zero/Vera/NanoLang/Mog/MoonBit/Valea 均在列——该赛道已拥挤，且目录维护者即 Vera 作者。[已核] https://agentlanguages.dev/

---

## ④ 执行者视角三段浓缩

**一、谁在哪个维度最强。** 工程与生态维度（D4/D7/D8/D9）由两家"机构级"选手统治：Gleam（十年、21987★、19+ 生产公司、社区治理+年会）与 MoonBit（四后端、15x Java 实测、公司 IDEA 全职投入、把自家语言做成 SWE-AGI 基准宿主）。但这两个维度不是 AI 原生的胜负手。真正的分水岭在 D5（编辑模型）与 D6（token 效率）：Zero 的"语义图即程序数据库 + 受检 patch + 文本仅投影"是全组唯一把编辑模型从纯文本升维的对象（P3）；Mog 拿出全组唯一 token 硬数字（spec=3200 tokens）；MoonBit 的 KV-cache 友好扁平设计 + 约束采样是唯一"从 Transformer 机制出发"的语言侧设计；NanoLang 则以 Coq 完备证明（Admitted-free + rocqchk CI）独占 D3 顶格。Vera 在 D1/D2 双 P3，但 D7/D8 是全组垫底档（Python 实现、无性能数据、单人、无第三方使用者）。

**二、修复闭环（D1+D2）格局：两极已成型，中间地带空缺。** 第一极是 Vera 的"验证即闭环"：186 个稳定错误码 + 每错带 Fix: 指令与 spec 引用 + 全命令 --json + LSP proof-delta 自定义方法（speculativeEdit 的 newly_undischarged/proof_regressions 就是给 agent 的差分反馈信号）——它把"诊断→修复建议→再验证"做成了产品级契约，代价是语言本身（De Bruijn 无名）在 HN 被专业读者几乎一面倒地否决。第二极是 Zero 的"图即闭环"：错误即修复契约（rule/expected/actual/fix 四段）、zero fix --plan --json、受检图补丁在写入前拒绝 stale/invalid、conformance/diagnostics 用 expected.json 机器校验、evals 在真实 sandbox 里用 Claude 跑修复闭环——工程严肃性最高，且有 Vercel 渠道。中间几乎无人：MoonBit 坐拥 304 个 CI 校验的错误码文档却没有（文档可见的）agent 侧 JSON 诊断出口；NanoLang 只有行号+trace；Mog/Valea 近乎空白。对 Lom 的直接含义：**"诊断码注册表 + 机器可读输出 + 分级 fix + 修复评测集"四件套，目前只有 Vera 与 Zero 凑齐，且二者一个牺牲了人类可读性、一个牺牲了文本编辑模型——同时保住"人类可读文本 + 机器修复闭环"的位置仍然空着。**

**三、使用侧证据揭示的真相：没有一门 AI 原生语言有真实第三方使用；反馈回路正在赢，语言不是。** D10 全景：Gleam（非 AI 原生参照系）是唯一 P3——19+ 具名公司生产使用、841 人问卷、案例研究；MoonBit P2 靠公司推动（awesome 项目、链上、ESP32）加一个关键元现象——它作为"低污染语料"被选为 SWE-AGI 基准语言，即"被 agent 考试使用"而非"被 agent 生产使用"。五门 AI 原生新语言全部 P0–P1：Zero 媒体声量与 HN 冷淡（最高 20 分）形成剪刀差；Vera 的 98.7% 是第一方基准且方法学被质疑；NanoLang 最好的使用证据恰是 Simon Willison 的失败-成功二段实验——单发不行，**进 repo、拿编译器、走闭环就行**；Mog 连作者自己的 OpenClaw 集成都还没写；Valea 一天死。HN 三大帖（NanoLang 232 分、Mog 163 分、Vera 111 分）的社区共识收敛为一句话（MarceliusK）："最强的想法不是语法，而是反馈回路"——以及 DonHopkins/offbyone42 一派的反命题：LLM 在语料丰富的语言上最强。这是 Lom 最应吸收的市场判定：**卖点不该是"为 LLM 设计的语法"，而是"任何语言都能插上的修复闭环"——而闭环四件套目前只有两个对手做全，且各留有明显破绽（Vera：人类不可读+无性能；Zero：图数据库+公司实验治理+无第三方使用者）。**

---

### 附：关键来源索引（除文中已注外）
- Zero repo/文档：https://github.com/vercel-labs/zerolang （README/AGENTS.md/docs/articles/{diagnostics,benchmarks}.md/evals/README.md 均 [已核]）
- NanoLang：https://github.com/jordanhubbard/nanolang （README/docs/PERFORMANCE_MONITORING.md/formal/README.md [已核]）
- MoonBit：https://www.moonbitlang.com/ 、/blog/moonbit-ai/ 、/blog/native/ 、moonbitlang/moonbit-docs next/language/error_codes（304 文件）[已核]
- Valea：https://github.com/hvoetsch/valea （README [已核]）
- Mog：https://github.com/voltropy/mog 、https://moglang.org/ 、https://gist.github.com/belisarius222/203ac5edbc3306c34bf0481f451d4003 [已核]
- Vera：https://veralang.dev/ 、https://github.com/aallan/vera （README/TOOLCHAIN.md/LSP_SERVER.md）、https://github.com/aallan/vera-bench 、llms-full.txt [已核]
- Gleam：https://gleam.run/ 、/news/developer-survey-2024-results/ 、https://github.com/isaacharrisholt/gleam-in-production [已核]
- HN：items 46684958（NanoLang）/47312728（Mog）/47955118（Vera）、search zerolang [已核，经 hn.algolia.com API]
- 第三方目录：https://agentlanguages.dev/ [已核]；媒体报道（InfoQ/MarkTechPost/HackerNoon/coddy.tech/Dfinity 论坛/lindbakk.com/lukaswerner.com）均 [未核]
