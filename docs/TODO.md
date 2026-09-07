# docs/TODO.md — post-1.0 整改待办台账

> **职责**：跨会话的可执行待办唯一事实源。任何会话领任务/交付任务以本文件为准；
> 完成一项就把状态改为 `done` 并附一行证据（命令输出/测试名），由维护会话复核后提交。
> **来源**：独立审查报告 [review-2026-09-03.html](reviews/review-2026-09-03.html)（基线 v1.0.0）
> 的逐条裁决，见文末"驳回/挂起登记"。
> **创建**：2026-09-03（v1.0.0 + 1 docs 提交之后）。**当前活跃**：无——M 工作包已收官
> （2026-09-07，三线优化全落地，见下档案）；L 工作包已收官（2026-09-07，见下档案）；
> W 工作包已收官（2026-09-07，v1.1.1，见下档案）。
> R1-R8 与 T1-T7 均已关闭（档案保留下文）。
> **纪律**：语言面冻结（LANGUAGE_SPEC §14）不破——只允许新增 warning 级诊断码；
> 其余行为修复均为 bug 修复性质且不动语法/保留字/内建表。

## M 工作包：三线优化（2026-09-07 开立，进行中）

**来源**：用户裁决（2026-09-07"还有什么优化空间"盘点后的 6→1→3 排序）。三项
互不依赖但顺序执行（协作偏好），全部不动语言面（冻结 §14 安全区：文档/工具/
诊断定位/测试语料）。

**用户裁决（2026-09-07）**：M1 = spec_examples_check 扩展到 LANGUAGE_SPEC +
tutorial（"文档不撒谎"从 SPEC_FOR_AI 一份扩到三份，R7 制度红利延伸）；
M2 = Pattern 无 span 补齐（诊断定位最后一个死角，fix 整词扫描兜底退役）；
M3 = fix_corpus 扩充（repair-native 回归网 4 例 → 全诊断码家族覆盖）。

### M1｜spec_examples_check 扩展 LANGUAGE_SPEC + tutorial ✅ done 2026-09-07

**M1 证据区（2026-09-07 实测）**：

- 工具升级：`--spec` 多值 + 默认三文档（SPEC_FOR_AI / LANGUAGE_SPEC /
  tutorial HTML）；HTML `<pre>` 块提取（实体反转 + 剥内嵌标签）；三条机械
  skip 规则（skip-ebnf 产生式形态强特征 / skip-keywords 全词 ∈ 20 保留字 /
  skip-cli 行首 `lom `）——skip 仅限"非 Lom 代码块"，不吞断言失败。
- **存量腐坏 7 处实测修复**（预期成真——双文档同病只修一半的实锤）：
  ① LANGUAGE_SPEC §2.4.1 管道示例缺 `from string import {trim, upper}`
  （照抄即 RUNTIME002）；② §10.4 distance 示例缺 `from math import {sqrt}`；
  ③ §6.7 效应签名块无函数体（补最小体，print→print_msg 避内置 NAM002）；
  ④⑤⑥ type alias / trait-impl / pub 三个"rejected sketch"块加 ❌ 反例标记
  （R1/R2 在 SPEC_FOR_AI 侧加过、LANGUAGE_SPEC 侧漏了——正是"同病只修一半"）；
  ⑦ **tutorial L422 闭包教学示例用嵌套具名 fn——Lom 不支持（PARSE001），
  教程在教非法语法**；改 `let step = fn() -> Int` 闭包字面量 + 顺带加 MUT002
  教学旁注（mut 捕获双后端分歧，语言主动警告——repair-native 活示范）。
- 9 处非 Lom 块 marker（概念性 enum 重定义 / lom info 输出 / 诊断消息 /
  eval 目录树 + tutorial 架构图/解析链/执行流程/Rust 代码/TOML 清单）。
- 全绿：81 块（34+40+7）= 正例 51 + 反例 5 + skip 25（json 8/diag 1/
  marker 10/ebnf 4/keywords 1/cli 1）；ci.yml 无参调用自动继承三文档默认
  （零 CI 配置变更）；doc_audit 16/16、cargo test 459/459。
- **锁定测试三组全红**：①type alias 去❌→大写名外来语法检测 FAIL；
  ②教程闭包改回嵌套 fn→PARSE001 FAIL；③删 import→运行层退出码 1 FAIL；
  各恢复后复绿。工具统计 bug 一枚（跨文档累计重复累加）已修。
- 过程坑：Read 工具显示吞 markdown 反引号围栏内闭合花括号的坑（§3.1 变体）
  再次防御性核对——本轮经 python assert 替换（不落地中间态）绕开。

### M2｜Pattern 无 span 补齐 ✅ done 2026-09-07

**M2 证据区（2026-09-07 实测）**：

- 方案：`Pattern::Variant` 加 `name_span: Span`（诊断需求仅在变体名——
  Lit 内含 Expr 自带 span、Binder/Wildcard 无名；未做 Pattern→PatternKind
  全面外置重构，改动面收敛）。消费点六处：parser 构造 1 + typechecker
  解构用 name_span + dump/interpreter/wasm 解构加 `..`（wasm_codegen 4493
  原有 `..` 不动）。
- 实测：`Circl(r)` 拼错 → `[NAM004] (5:9)` 定位变体名 token（此前硬编码
  (0,0)）；`fix --plan` 产出 `replace at 5:9..5:14 text="Circle"` 单点精确
  替换（fix.rs 零改动——precise_occurrence 对非零位置自动生效，扫描兜底
  保留为内容不符回退）。`--apply` 对 Medium 置信度仍不自动应用（既有裁决）。
- +1 测试 `nam004_variant_diag_locates_name_token`（460/460）；全量回归
  电池全绿：clippy -D warnings 零告警、stmt_interp golden 逐字（dump 格式
  契约不含 span、AST 变更零输出变化——149 文件 dump 验证）、fmt 幂等、
  eval 双后端 116/116、selfhost dump 149/static 对齐 149、doc_audit 16/16。
- 文档同步：README 测试数 456→460（**顺手修两代陈旧**——W 工作包升 459
  时漏改了 README，文档腐坏复发实例）；HANDOVER §1/§2.2/§9 三处 459→460；
  §1 遗留挂账移除"Pattern 无 span"条目（就此关闭）。
- 已知边界（如实记录）：无参数变体拼错（`Grean`）在 parser 层就是 Binder
  模式、不触发 NAM004——M1 时代记录过的既有模式语义，非本项范围。

### M3｜fix_corpus 扩充 ✅ done 2026-09-07

**M3 证据区（2026-09-07 实测）**：

- **4 对 → 8 对**，覆盖全部"有修复动作"的诊断规则族（LEX003/004 与
  TYPE003 是 hint_only 无动作，不在语料范围）：
  | # | 规则 | 形态 | 行为 |
  |---|---|---|---|
  | 01 | PARSE001 缺 ')'（既有） | 行首 | High 自动修 |
  | 02 | LEX005 + EFF001 单效应（既有） | 两轮迭代 | 自动修 |
  | 03 | LEX001 未闭合字符串（既有） | 已知限制 | 钉行为 |
  | 04 | NAM003 拼写（既有） | Medium | 钉不自动应用 |
  | 05 | **MAT001 内置变体补臂（新）** | Result 缺 Err 臂 | High：`insert (MAT001) — 在 match 的 end 前插入缺失分支: Err(_) => ()` |
  | 06 | **EFF001 多效应合并（新）** | 声明 [IO] 调 [Clock] 函数 | High：`[5:34] insert — 在现有效应列表中添加缺失效应: Clock` → `! [IO, Clock]` |
  | 07 | **PARSE001 缺 end（新）** | EOF 截断 match | Medium：计划 `insert at 3:28 text="\nend\n"`（钉不自动应用） |
  | 08 | **NAM004 字段拼写（新）** | p.xx → x | Medium：`replace at 3:15..3:17 text="x"`（M2 位置增益的同族呈现） |
- 全部 fixed 文件由 `lom fix --apply` 实跑产出（05/06 取自动修复产物，
  07/08 fixed==bad 钉 Medium 不应用语义——对齐 04 的既有设计）。
- fix_corpus_end_to_end 测试 8/8 通过（460/460 总）；全量回归电池全绿：
  clippy 零告警、golden 逐字、eval 双后端 116/116、doc_audit 16/16、
  spec_examples_check 三文档 PASS。
- 构造过程坑两枚：① `now()` 不是内建（LANGUAGE_SPEC §6.7 是示意签名）
  ——Clock 效应用用户声明函数触发；② match 缺 end 的报错形态依赖
  容错解析路径，`println` 行会被当作新臂模式先报"期望 '=>'"——
  EOF 截断形态才是"期望 'end' 闭合 match"的可靠触发。
- 文档计数联动：README:130/HANDOVER §11 的"4 例"为 M4 时点历史快照，
  按惯例不回改；无现值计数声称需要更新。

### 纪律

顺序执行（M1→M2→M3，每项完成跑全量回归电池 + doc_audit 双件才进入下一项）；
语言面冻结不破；严禁虚构数据；每项 feat+docs 成对提交推送看 CI 首跑。

---

## L 工作包：LLM 复测升级 pass@k（2026-09-07 开立 → 2026-09-07 收官）✅ done

**结局：修复+完成出口**——上轮复测诚实挂账第一条（单采样）关闭；「LLM 写 Lom
低错误率」证据链升级为三层（基线 99/100 → 四模型单采样 → 双模型 × 10 采样
pass@k）。报告 eval/REPORT-2026-09-07-passk.md。

**目标**：把"LLM 写 Lom"证据链从单采样升级为多采样 pass@k，关闭上轮复测
（REPORT-2026-08-31-multimodel.md）诚实挂账第一条（**单采样**）；评测集从 113
升到现行 116 任务（115-117 三任务的首次 LLM 实测）。

**用户裁决（2026-09-07）**：2 模型（deepseek-v4-pro+thinking / glm-5.3 Coding Plan
端点）× **10 采样** × temperature=1.0，报告 pass@1 / pass@5 / pass@10（无偏估计）；
跨语言对照组（挂账第二条）**另立项，本包不含**。API 走 eval/.api_keys.json 已配置 key。

### L0｜管线升级（纯本地改造，零 API 成本）✅ done 2026-09-07

- `eval/llm_eval.py` 加 `--samples N`：每分类 prompt 调用 N 次，raw 留档
  `raw/<cat>_s<k>.md`，候选写 `s<k>/<id>.lom` 子目录；不带 `--samples` 时行为
  与现状完全一致（向后兼容）。
- **断点续跑**：某采样的 raw 文件已存在即跳过该次调用（10 采样长跑必然中断，零成本恢复）。
- 新 `eval/passk_summarize.py`：驱动 run.ps1 逐采样评分（评分事实源不二设）→
  逐任务通过矩阵 → pass@k 无偏估计（HumanEval 式 `1 - C(n-c,k)/C(n,k)`，
  整数运算）+ 分类汇总。
- 验收：`--from-raw` 对上轮四模型存量目录复算零行为变化；`--samples 2 --only
  <单分类>` 端到端实测通过。

**L0 证据区（2026-09-07 实测）**：

- 公式对账：pass_at_k 已知值全对（n=10 c=2 k=5 = 0.7778 = 1−C(8,5)/C(10,5)
  手算双验证；Fraction 整数精确无浮点漂移）。
- passk_summarize 端到端（零 API 成本的假双采样目录：s1=上轮 glm-4.7 候选、
  s2=上轮 v4-pro 候选）：s1=112/116、s2=113/116 与上轮报告一致；078 = 1/2
  （glm 挂 pro 过，吻合）；115/116/117 = 0/2（上轮候选是 113 任务集时代产物，
  MISSING 计不通过的正确语义）；总体 pass@1 = 97.0%（112.5/116 手算吻合）、
  pass@2 = 97.4%（113/116 吻合）。
- `--from-raw` 兼容：deepseek-v4-pro 存量目录重提取 113/116（缺 115-117 为
  上述同因），113 个候选文件前后 md5 逐字节一致（幂等）。
- live 冒烟（glm-5.3 Coding Plan 端点，--samples 2 --only 08_effects）：
  raw/08_effects_s1.md + _s2.md、s1//s2/ 各 5 候选、run_meta 记录 samples=2；
  passk_summarize 评分联调通过（局部采集警告路径正确）；同命令重跑断点续跑
  "跳过已有调用 2 次"零 API 消耗。
- 过程坑（留档）：① Python subprocess 读 PowerShell 输出须显式
  `encoding="utf-8"`（默认 GBK 解码直接 UnicodeDecodeError，§3.3 家族坑）；
  ② **Read 工具显示会吞 f-string 的闭合花括号**（§3.1 坑新变体：git 原版
  `{len(want.get(cat, []))}"` 显示为 `{len(want.get(cat, []))"`）——转抄长段
  代码必须 git diff 逐行核对。
- key 状态：上轮两个 key 均已失效（deepseek `****a34f invalid` / glm 401），
  用户当日更新后双端点复通（deepseek 标准端点 + glm Coding Plan 端点）。

### L1｜正式采集（2 模型 × 10 采样 × 116 任务）✅ done 2026-09-07

- deepseek-v4-pro --thinking、glm-5.3（glm-coding 端点）各 10 采样；
  全部参数（温度/采样数/起止时间/逐分类统计）记录 run_meta.json。
- 预计每模型 100 次大 prompt 调用，thinking 模式总时长可能数小时——后台跑 +
  断点续跑，严禁虚构：所有数字来自实测留档。

**L1 证据区（2026-09-07 实测）**：

- 采集完成：两模型各 **1160/1160 提取零缺失**（116 任务 × 10 采样 × 2 模型）；
  200 个 raw 回复全留档（`candidates_rerun/<model>_passk10/raw/`）。
- **两次服务端断连崩溃 + 断点续跑实战**：glm 74/100 时 RemoteDisconnected、
  deepseek 80/100 时 IncompleteRead——均非 URLError 子类、穿透了上轮管线的
  catch 列表。修复：`http.client.HTTPException` + `ConnectionError` 进重试分支
  （llm_eval.py call_api）；续跑分别跳过 74/80 次已有调用零成本补齐。断点
  续跑机制（L0 新增）两次实战通过。
- 过程偏差如实记录：run_meta 起止时间为末段进程时段（续跑覆盖语义）；
  API key 中途全部失效（deepseek ****a34f invalid / glm 401），用户当日更新
  后双端点复通（glm 新 key 支持 Coding Plan 端点，与上轮 glm-5.3 同路径）。

### L2｜评分汇总 + 报告 + 文档同步 ✅ done 2026-09-07

**L2 证据区（2026-09-07 实测，passk_summarize × 2 模型，评分器 run.ps1 不二设）**：

- **deepseek-v4-pro+thinking**：pass@1 = pass@5 = pass@10 = **99.1%**
  （1149/1160 通过单元；s1-s5/s7-s10 各 115/116、s6 114/116）；非满分任务
  078 = 0/10、051 = 9/10（温度方差，pass@5 覆盖）。
- **glm-5.3**：pass@1 = pass@5 = pass@10 = **99.1%**（1149/1160；s3 = 114、
  其余 115）；非满分任务 078 = 0/10、104 = 9/10。
- **078 证据链闭环**：跨时间（2026-08-03 基线挂）、跨模型（上轮 4 模型 3 挂
  1 过）、跨采样（本轮 20/20 全挂）——prompt 歧义锚点题结论坐实。失败形态
  三种实测留档：两行拆分 / 丢时间戳 / 自创 `boot @ N` 格式。
- **认知修正（如实记录）**：上轮"deepseek-v4-pro+thinking 是首个通过 078 的
  模型——思考模式能处理歧义"被推翻——本轮同模型 temperature=1.0 下 0/10，
  上轮通过是默认温度下的边缘事件。报告与 HANDOVER 均已改口径。
- error_repair 20 任务双模型 20 采样全过（200/200）——repair-native 闭环在
  temperature=1.0 下依然有效。
- 交付：eval/REPORT-2026-09-07-passk.md（总览/分类/失败分析/方法论与过程
  偏差）；README 现状段、eval/README、HANDOVER §1（LLM 实测行三层证据 +
  下一步菜单改"对照组另立项"）+ 顶部横幅同步；doc_audit 复跑 PASS（见提交）。
- 无 CLI/语言面变化，不升版。

### 纪律

顺序工作；严禁虚构数据（所有通过率/统计来自 run.ps1 实测与 raw 留档）；
每阶段完成在本文件标进度附证据；完成后自行提交推送（conventional commits）
并看 CI 首跑——维护会话随后独立复核。

---

## W 工作包：wasm 越界深挖（2026-09-05 开立 → 2026-09-07 收官，v1.1.1）✅ done

**结局：修复出口**（非降级）——两处 codegen bug 根治，8.4 改判完成，升版 1.1.1，
wasm 层回流 CI。完整技术档案 RFC-0003 修订 25-29；工具沉淀 tools/wasm_debug/。

**目标**：根治 8.4 挂账的未定位非确定性内存越界（RFC-0003 修订 20/21 完整档案），
使 wasm 载体的三层自证成为可能；攻不下时允许"如实降级"出口（见下）。

**必读档案（动手前全读）**：
- docs/rfc/0003-phase8-full-selfhosting.md 修订记录 20/21/22（症状、六类已排除假设、
  怀疑面收窄结论）；HANDOVER §11.0（复现口径与诊断工具遗产）。
- tools/verify_selfhost.py 的 mode_wasm 与 WASM_LAYER2_FILES（限内清单）；
  eval/runner/run_wasm.mjs（harness：LOM_PRE_GROW 已在；LOM_HP_TRACE、trap 时内存页数）。

**现状基线（2026-09-05 维护会话实测）**：self_interp.lom 已从 4890 行长到 5703 行
（v0.27.0 Part H json 自实现），wasm 产物 238,597 字节（v0.26.0 时点约 220KB）——
**修订 21 的 ">~6.7KB 必崩 / 限内间歇" 阈值口径基于旧模块，必须先重新标定**；
本轮限内 19/19 通过（单轮）。

### W0｜复现基线重建（必做前置）✅ done 2026-09-07（证据区见下，后续一切定位以此新基线为准）

- 编译 self_interp → wasm；确认限内 19 文件多轮稳定性（≥5 轮，记录间歇率）；
  构造递增规模的目标程序（重复函数声明 / 长表达式链 / 深嵌套）二分出**当前**必崩阈值
  与间歇窗口；同一命令重复 ≥10 轮记录非确定性率；
  记录 trap 形态（栈顶函数、报错、内存页数）是否仍是 tok_disc 载荷野值口径。
- **产出**：新阈值与非确定性数据写回本节证据区——后续一切定位以新基线为准。

**W0 证据区（2026-09-07 实测，node v24.15.0，Windows）**：

- 产物与编译确定性：`self_interp.wasm` = **238,597 B**（与开立记录一致）；同源编译
  5 次 md5 全同（`6c4a47bd…df03`）——编译侧完全确定，旧"非确定性"不在编译产物。
- 官方口径（verify 同款 temp 路径，argv0=44 字符）：限内 19 文件 × 5 轮全 **19/19**；
  项目内短路径（argv0=21 字符）同清单 10 轮 **18/19**——`string_demo.lom`（2091B，
  远低于旧 6.7KB 口径）10/10 稳定 trap（`wasm trap: memory access out of bounds`，
  stdout 0 字节 = 死在 parse 早期，trap 时 mem pages 3）。
- **决定性新发现——argv[0] 长度控制触发**（函数索引映射：funcidx = 13 导入 + 64 helper
  + 用户函数序号）：梯度实测 argv0 12-25 字符，**≤22 必崩、≥23 必过**（一字节翻转）。
  argv0 经 `lom_env_args` 物化进堆（4+len 字节），其长度平移后续所有分配的绝对地址
  ——**bug 是堆布局敏感的；给定 (wasm 字节 + argv + 目标文件) 结果完全确定**。
- 规模标定（temp 路径，fn/expr/nest 三形态 × 10 档，fn=重复函数声明 / expr=长加法链 /
  nest=深括号嵌套）：fn 形态**非单调**——2.2KB OK / 4.4-8.8KB TRAP / 11-13KB OK /
  17.6KB TRAP / 26KB OK / 35KB TRAP；expr ≥4KB 全 TRAP；nest 多数撞 V8 栈深
  （深递归 parse，已知限制非本 bug），8.2/12.3KB 撞 OOB。**旧 ">~6.7KB 必崩" 阈值口径
  废弃——不存在单调阈值，是布局抽签**。
- 非确定性率：边界复测 fn@4384B trap **10/10**、fn@10978B pass **10/10**——同命令
  零漂移；旧"间歇性"（两轮 19/21 vs 15/19）重新定性为**跨调用形式的布局差异**
  （当时未控 argv/路径等前置分配变量）。
- trap 形态：仍是 **tok_disc**（funcidx 103 = 13+64+26，用户函数序 #26）且两个独立
  复现（string_demo 短路径 / fn@4384B）撞在**同一指令偏移 0x776d**；调用链 Pratt 族
  （parse_or→and→comparison→pipeline→addition→multiplication→unary→question→
  postfix→tok_disc；fn 形态为 comparison→or→range→expr→…）；mem pages at trap 3-9
  （非耗尽）——与修订 21 的"Tok 枚举载荷野值"口径一致且更精确。

### W1｜定位 ✅ done 2026-09-07

- 遗产工具直接复用：run_wasm.mjs 的 LOM_HP_TRACE / trap 内存页数 / LOM_PRE_GROW；
  git 历史中的 lom_dbg_alloc 探针与 LOM_WASM_DUMP_FN=1 函数索引表（源码级探针当时
  已回滚，可从历史复取）。
- **六类已排除假设不要重做**（Map rehash / JS 栈深 / 内存耗尽 / build_alloc 字节级 /
  静态串去重 / 独立最小复现）——新假设沿修订 21 的收窄方向：大模块特有的堆对象指针链
  或静态数据布局交互。
- 方法论（项目沉淀，遵守）：一次只加一针，DBG 链按调用链逐级；崩溃 vs 通过的同型运行
  做 wasm 内存快照 diff（harness 导出 memory，反推野值写入源）；从 trap 栈顶的野值
  反向定位是哪次分配/哪段静态数据被错误引用。
- 假设起点（非限制）：hp 增长撞入静态数据段边界；variant_idx/闭包 table 索引在大模块
  下越界；数据段布局与 4 位 tag 的高位交互；Tok 枚举载荷的野值注入路径。

**W1 证据区（2026-09-07 实测）**：

- 探针重建：codegen 加 `lom_dbg_alloc` import（rt_alloc 尾部上报 size/new_hp，
  harness handler 本就在）→ trap/pass 双侧全量分配日志 13,417/25,757 条——
  **尺寸序列除 argv0 外逐条一致**（排除错误尺寸分配；hp 全程不降）。
- **同刻快照 + 语义图 diff**（工具沉淀 tools/wasm_debug/heapdiff.py）：harness 第
  N 次分配时导出内存，N 从 13400 二分到 13416（trap 前最后一次分配）——按分配日志
  逐对象解码、指针归一化为 owner 序号后，**全堆语义级零差异**：排除存储腐蚀，
  野值是运行时算出的（wasm 栈/局部变量直接传递）。
- **野值现场探针** `lom_dbg_wild`（变体臂载荷提取前 tag/边界检查，异常即现场）抓到
  实值：`value=0x2fff46 tag=6 ptr=196596 mem=196608 hp=196604`——scrutinee 是
  **合法的 0 参枚举**（8 字节头对象）躺在线性内存末尾；其 +8 载荷读
  [196604,196612) 越过页界 4 字节 = trap（字节级解码 0x267 处 `29 00 08` =
  i64.load offset=8，与 W0 的 0x776d 同一指令）。
- **根因**：match 变体臂降级（wasm_codegen.rs `Pattern::Variant`）把载荷提取与臂
  测试平铺 AND——WASM 急切求值下 `i64.load(s>>4+8)` 不等 tag==6 且 idx 匹配就执行；
  `emit_variant_test` 的 idx 读同样 tag 盲（大 Int scrutinee 会 OOB）。六类旧排除
  全部兼容（无重做）：bug 不在分配/存储侧，在求值侧的读时机。
- **顺带抓获第二个独立 bug**（W2 扩量验证时暴露）：eval 020（return-in-if）在 wasm
  自举层输出 -1/-1/-1 → 直编最小复现（`for x in xs if x>1 return x end end` → -1）
  确认 **for 体内 return/? 被吞**：`Stmt::For` 三个迭代分派 if（Int/Str/List）未压
  `Label::If` → br $ret 深度少算 2-3 层、br 落到分派 if 出口。v0.12.0（7.6）起潜伏，
  eval 116 任务与示例无 "?/return 在 for 体内" 形态所以从未被拦。

### W2｜修复 + 验收 + 收尾 ✅ done 2026-09-07（升版 1.1.1）

- 修复在宿主侧（wasm_codegen.rs / run_wasm.mjs），**不动语言面（冻结）**。
- 验收阶梯：① W0 标定的必崩阈值解除（原必崩程序稳定通过 ≥10 轮）；② verify_selfhost
  --wasm 限内全绿且稳定；③ 逐步解除 WASM_LAYER2_FILES 规模限制，扩到全 examples + eval
  参考解（重标第三层：自举跑自身 5703 行的可行性，如实记录结果）；④ 稳定性观察 ≥5 轮
  后决定 wasm 层是否回流 CI（不回流也可，写明理由）。
- 收尾：RFC-0003 修订 25（发现/修复/或排除档案）；HANDOVER 8.4 行与 §11.0 增补；
  guide 条目；教程 §2.14 的 8.4 表述更新；**升版 1.1.1**（wasm 后端 bug 修复，patch），
  CI 绿后打 tag v1.1.1。
- **全量回归电池不倒**（HANDOVER §2.2 + doc-gates 双件）。

**W2 证据区（2026-09-07 实测）**：

- **修复三处（仅 wasm_codegen.rs，零语言面）**：① `emit_variant_test` idx 读改
  tag 守卫条件读（tag≠6 给不可能 idx -1）；② `Pattern::Variant` 载荷提取推迟到
  变体测试通过后（`if (result i32)` 内，else 0）；③ `Stmt::For` 三个分派 if 补
  push/pop `Label::If`。+3 e2e 回归测试（459/459）。
- **阶梯①解除**：fn 形态 2.2KB-211KB 全过（原 4.4-8.8/17.6/35KB 必崩）；string_demo
  短路径 10/10（原 10/10 必崩）；expr ≥4KB 的 OOB 消失；nest 8.2/12.3KB OOB 消失。
  残余失败全部为 `Maximum call stack size exceeded`——深链 parse 递归撞 V8 默认栈，
  `--stack-size` 可调的**已知限制**（RFC-0002 退出标准 4 口径），非本 bug。
- **阶梯②③达成**：第二层全量 = examples+bootstrap（RUN_EXCLUDE 豁免）+ eval 116
  参考解 = 147 文件 × 5 轮稳定 PASS（有状态示例按 §11.0 教训两侧前清理）；file_demo
  曾因陈旧产物假 FAIL，清理后过。**第三层**：wasm 自举跑 stmt_interp，39 条 golden
  逐字一致（`--stack-size=60000`）。**自施加（重标第三层）**：wasm 自举解释器解析
  **自身 5703 行源码**，`--dump-ast` 655,890 字节与宿主原生 dump 逐字一致。
- **阶梯④回流决策：回流 CI**——5 轮稳定 + 给定 (wasm 字节+argv+目标文件) 完全确定
  （旧"随机红"根源已除）；verify_selfhost `--wasm` 升级三段验收（layer2 全量 / layer3
  golden / 自施加），WASM_LAYER2_FILES 限内清单废除；ci.yml selfhost job 新 step。
- 收尾齐：RFC-0003 修订 25-29、HANDOVER（§0 横幅/§1 8.4 行/§2.2/§9/§11.0 重写为
  已解决档案）、guide 条目 + 8.4 条目改判、教程 5 处、LANGUAGE_SPEC §13 v1.1.1、
  README 横幅 v1.1.1 行、工具沉淀 tools/wasm_debug/（gen_scale / dbg_run /
  heapdiff + README 探针补丁说明）。
- 全量回归电池：cargo test **459/459**、clippy -D warnings 零告警、stmt_interp
  golden 逐字、fmt 幂等、eval 双后端 **116/116**、selfhost 六模式全 PASS（dump/tokens
  149、diags 5、static 15/149、run 31、wasm 三段）、`lom --version` → 1.1.1。
  doc-gates 双件复跑 PASS。tag v1.1.1 在 CI 绿后打（见下方登记）。
- **CI 首跑 + tag 登记（2026-09-07）**：commit `edd0c5e` run #34058172473 全绿——
  三平台矩阵 / selfhost 六模式（含新 step "Selfhost wasm (8.4 三段验收)" ubuntu 首跑
  success）/ clippy / doc-gates 全部 success；**tag `v1.1.1` 已打并推送**（CI 绿后）。
  提交对：`eb0ce7e`（fix: 修复+验收基建+升版）+ `edd0c5e`（docs: 档案）。

### 出口条件（攻不下不算失败）

多轮仍无法定位时：把新基线数据、新排除假设、新增诊断工具沉淀为 RFC-0003 修订 25 的
增补档案后关账——诚实档案与修复同权重（项目纪律）。此时只更新文档不升版本。
**（本次走修复出口，本节未启用）**

### 纪律

顺序工作；一次一针；严禁虚构数据（trap 输出原样留档）；每阶段在本文件标进度附证据；
完成后自行提交推送（conventional commits）并看 CI 首跑——维护会话随后独立复核。

---

## 第二轮审查工作包（2026-09-05 开立，已关闭）

**来源**：第二轮独立审查 [review-2026-09-05.html](reviews/review-2026-09-05.html)（总评 A-，无 P0；基线 470670d / v1.1.0）。
**验证状态**：12 条发现已由维护会话逐条亲手复现（两条行为性发现实测：`type UserId = Int` → PARSE001、`from math import {sin}` → RUNTIME005；十条文档定位 grep/wc 证实），零失实，全部采纳。交接关键项（HANDOVER §9/§2.2 的 456/1.1.0/149、§9-5 的 5703、guide 两条目补录）已由维护会话即时修复；其余按 R1-R6 执行，R7/R8 为采纳审查结构性建议的加强项。
**通用纪律**：顺序执行（R1→R8）；每项完成在本文件标 done 附一行证据并跑全量回归电池（同 T7 清单）；语言面冻结不破；**无需升版**（纯文档 + 仓库工具 + CI 配置，无 CLI 面变化）。

### R1｜P1：SPEC_FOR_AI §5 Type alias 假特性清除 ✅ done 2026-09-05

- **证据**：小节改写为负面示例（❌ `type UserId = Int` 标注 PARSE001 verified + 冻结非目标
  RFC-0001/LANGUAGE_SPEC §14 + 结构类型正面替代示例）；实测复现 `type UserId = Int` → 
  `[PARSE001] 期望函数声明 'fn'…得到 标识符 'type'` 退出 1；`grep "type [A-Z][A-Za-z]* = " SPEC_FOR_AI.md`
  仅剩 167/168 两行且均带 ❌（只以反知识形态出现）；新正面替代示例包 main 后 `--check`
  零诊断、运行输出 42；回归电池 8/8 PASS（456/golden/eval 双后端 116/selfhost 五模式）。

- 事实：SPEC_FOR_AI.md:166-170 存在 "Type alias (Phase 2)" 小节，把从未实现的 `type UserId = Int` 当特性教——实测 PARSE001；LANGUAGE_SPEC:235 已标 never implemented、§14 冻结非目标重申 no type aliases；教程 :744 还宣称此类谎言已清扫（清在了人读的 spec，漏了 LLM 读的这份）。
- 范围：删除该小节，或（推荐）改写为负面示例——"**不要写** `type X = Y`（未实现且 v1.x 冻结非目标，实测 PARSE001；类型语义用结构类型直接表达）"。修后教程 :744 的宣称恢复成立。
- 验收：SPEC_FOR_AI 中 `type` 别名只以反知识形态出现；R7 工具上线后 §5 零失败示例。

### R2｜P2/P3：SPEC_FOR_AI 其余三处内容修正 ✅ done 2026-09-05

- **证据**：三处落实——①:15 核心规则 6 改 `from math import {sqrt, abs}` 并注明 math 恰好
  导出四符号（实测 `sqrt(16.0)`/`abs(-5)` 输出 4.0/5；`grep sin\|cos` 仅剩 since/enclosing
  等子串误匹配，无真实残留）；②§10-4 改 MUT001 **warning** 口径（实测 `let x = 3; x = 4`
  → 1 条 MUT001 warning、输出 4、退出 0）；③§8 pub 块加 ❌ + "Do not write" + PARSE001
  verified 标注、块内 `# private` 注释改"# NOT private"（实测 `pub fn` → PARSE001 退出 1）。
  **顺手（台账外，按 T5"顺手陈旧计数"惯例）**：结尾段 "the 114-task eval suite" → 116-task
  （审查方法局限声明过的漏网同类项，grep 时暴露，现值实测 116/116）。
  回归电池 8/8 PASS。

- :15 核心规则 6 示例 `from math import {sin, cos}` → 改为真实符号（如 `{sqrt, abs}`；math 仅导出 sqrt/abs/min/max，§11b/§8 已是正确口径）——实测 `sin` 报 RUNTIME005；
- :363 §10-4 "Reassigning immutable … is an error" → 改为 MUT001 **warning** 口径（实测 `let x = 3; x = 4` 照常运行输出 4、退出 0；LANGUAGE_SPEC §5.1 已准确）；
- :331-340 pub 示例块加 ❌/反例标记（文字已声明 rejected，但代码块无标记、照抄即 PARSE001——对齐 §10 错误示例的排版风格）。

### R3｜P2：LANGUAGE_SPEC §12 同节自相矛盾旧计数 ✅ done 2026-09-05

- **证据**：§12.4 "(15 tasks, 15% of the suite)" → "(20 tasks, ~17%)"（实测
  `len(eval/tasks/10_error_repair.json)` = 20；20/116 = 17.2%）；§12.5
  "Reference solutions: 100/100 pass" → "116/116 pass on both backends"
  （实测 run.ps1 双后端各 116/116）；`grep "15 tasks\|100/100" LANGUAGE_SPEC.md`
  零命中（§12.3 原本就正确）；回归电池 8/8 PASS。

- :1308 "`10_error_repair.json` (15 tasks, 15% …)" → 20；:1312 "Reference solutions: 100/100 pass" → 116/116（§12.1/§12.3 同节已写正确值）。

### R4｜P2：eval/README.md 三处陈旧 ✅ done 2026-09-05

- **证据**：布局注释 03_types 11→13、04_closures 12→13（实测 tasks JSON 逐文件计数
  10/13/13/13/16/10/10/5/6/20，合计 116 与 manifest total_tasks=116 一致）；
  "001-115" → "001-117, 116 unique ids — 108 is a known, accepted gap"（实测 ID
  min=001 max=117 unique=116）；"Error-repair category (15)" → (20)；
  `grep "001-115\|(15)\|11 tasks\|12 tasks" eval/README.md` 零命中；
  回归电池 8/8 PASS。

- :97 "Error-repair category (15)" → (20)；:56 "001-115" → 001-117；:23-24 布局注释 03_types=11/04_closures=12 → 13/13（合计与 116 对齐）。

### R5｜P3：README 两处 ✅ done 2026-09-05

- **证据**：:66 拆分改 "29 examples + 4 bootstrap + 2 in the pkg_demo package + 1
  self-hosted interpreter examples/selfhost/self_interp.lom"（实测 git ls-files 逐目录
  计数 29/4/2/1，合计 36 不变）；状态横幅追加 v1.1.0 行（FROZEN 事实保持原文，
  列 MUT002 warning / NAM003 假阳性修复 / inf-NaN 双后端统一三项，链接 spec §13）；
  `grep "1\.1\.0" README.md` 命中 :8（全文此前零命中）；回归电池 8/8 PASS。

- :66 文件拆分 "3 in the pkg_demo package" → 2（main.lom + mathlib/math.lom），并补 selfhost 1（29+4+2+1=36，总数原本碰巧正确）；
- 状态横幅（:7，保持 FROZEN 事实）追加一行 v1.1.0 整改说明（MUT002 warning / NAM003 递归闭包假阳性修复 / inf-NaN 显示统一三项用户可见变更；全文现无任何 1.1.0 痕迹）。

### R6｜P2（半）：ci.yml 步骤名自举计数 ✅ done 2026-09-05

- **证据**：ci.yml:116 步骤名 "Selfhost dump (147 文件逐字, Phase 8.1)" → 149
  （149 = 29 examples + 4 bootstrap + 116 eval 参考解，基线实测 verify_selfhost dump
  PASS 149/FAIL 0）；`grep "147" .github/workflows/ci.yml` 零命中；
  回归电池 8/8 PASS。

- .github/workflows/ci.yml:116 "Selfhost dump (147 文件逐字, Phase 8.1)" → 149（HANDOVER §2.2 的另一半已由维护会话即时修复）。

### R7｜加强：SPEC_FOR_AI 代码示例实测对账工具（审查发现二的建议）✅ done 2026-09-05

- **证据**：新 tools/spec_examples_check.py（零第三方依赖，纯标准库）——34 个 fenced 块
  全量对账：正例 27（解析层 `--check --json` 零 parse Error + 导入层实跑无 RUNTIME005 +
  自含块运行退出 0；教学片段仅 NAM003 散文名放行）/ 反例 2（pub PARSE001、字段赋值
  PARSE000——均实测产诊断）/ skip 5（3 JSON schema + 1 诊断输出示例 + 1 marker），
  RESULT: PASS。**锁定测试四组全红**：①改回 type alias 假特性 → "值位置出现大写名
  [Float, Int, Point, UserId]" FAIL；②插入 `from math import {sin}` → RUNTIME005 FAIL；
  ③字段赋值还原成正例教法 → PARSE000 FAIL；④pub 反例去 ❌ 变正例 → PARSE001 FAIL。
  回归电池 8/8 PASS。CI 接入见 R8（同一 doc-gates job，首跑结果 R8 登记）。
- **顺手发现并修复（台账外，R7 调查中工具暴露——审查漏网的第三个假特性）**：
  **记录字段赋值 `p.x = 5` 从未实现**（parser.rs:642 赋值目标必须是普通变量；实测
  PARSE000 退出 1），但 SPEC_FOR_AI §5 "Field mutation requires `let mut`" 与
  LANGUAGE_SPEC:384 "mutation (if `mut`): `p.x = 5`" 都当特性教（v0.1.1 起带病至今，
  spec §11 "for immutable structured data use records" 与之自相矛盾）。修法镜像 R1：
  SPEC_FOR_AI 改 ❌ 反例（PARSE000 verified + 重建新记录替代 + map 指路，工具锁定）；
  LANGUAGE_SPEC:384 改如实口径（不可变、重建、或用 map）。另：§6/§9 两处无函数体
  签名示例块补最小函数体使其可实测（print 撞内置改名 print_msg——实测 NAM002）；
  §11 `enum Result` 概念性定义块加 `spec-check: skip` 标记（真实代码重定义是 NAM002）；
  §7 管道示例 main 的 `-> Unit` 注解去掉（实测 TYPE010 warning，示例 stderr 应干净）。

- 事实：历次文档清扫以 LANGUAGE_SPEC/README 为主战场，SPEC_FOR_AI 只在评审点名时被扫——本次 P1 正是漏网结果。
- 范围：新 tools/spec_examples_check.py（零第三方依赖）——抽取 SPEC_FOR_AI 的 fenced code 块逐个写临时 .lom 跑 `lom --check`；反例（❌/「不要写」标注的块）断言其确实产诊断，正例断言零 Error。存量反例先手工标注（R1/R2 完成后应只剩 pub 一处）。接入 CI（可与 R8 同一 step）。
- 验收：脚本对全量示例跑通且分类全符合；R1/R2 的修复被工具锁定（故意改回假特性能红）。

### R8｜加强：文档数字自动对账 gate（审查发现三的建议——根治第三次复发）✅ done 2026-09-05

- **证据**：新 tools/doc_audit.py（零第三方依赖）——五类真值自动计算（eval 总数=任务
  JSON 求和 116；dump=29+4+116=149；.lom 拆分 glob 实数 36=29+4+2+1；self_interp
  行数 wc 口径 5703；版本 Cargo.toml 1.1.0+lock 一致），16 项文档现值逐处比对
  **16/16 PASS**；**锁定测试五组全红**（eval/README 116→115 / ci.yml 149→147 /
  README 拆分 36→35 / HANDOVER 5703→5696 / HANDOVER 版本 1.1.0→1.0.0，各改一处
  即 FAIL，恢复后复绿）。CI 新增 doc-gates job（ubuntu，R7/R8 同 job：build +
  spec_examples_check + doc_audit，YAML 本地校验合法）；回归电池 8/8 PASS。
  **CI 首跑：绿（2026-09-05 实测观测，commit 31fd26d，run #33963153689——doc-gates job
  的 "SPEC_FOR_AI example check (R7)" 与 "Doc numbers audit (R8)" 两 step 均 success；
  同 run 三平台矩阵/selfhost 五模式/clippy 全绿）。**
- 设计口径：模式找不到也算 FAIL（措辞重构必须同步更新对账清单）；历史时点值
  （changelog 的 v1.0.0 "eval 114/114" 等带日期快照）不在监控范围——spec 现值位置
  是 §12.3/§12.5；HANDOVER §2.2 的 eval 行锚定 run.ps1 上下文（同文件 "期望 456/456"
  是测试行，测试数不在本轮五类监控清单内）。

- 事实：本次陈旧数字（§9 的 454/147、spec §12.4/12.5、eval/README 三处、README 拆分）呈现"总账更新、边角漏网"模式，其中两处是 HANDOVER:355 白纸黑字记录过的复发教训——人工簿记清单天然漏项。
- 范围：新 tools/doc_audit.py（零第三方依赖）——机械可核的现值数字自动对账：eval 总数（manifest vs README/HANDOVER/spec §12/eval-README 四处）、自举 dump 计数（由 examples/bootstrap+manifest 计算出的期望值 vs HANDOVER §2.2 与 ci.yml 步骤名）、.lom 文件拆分（find vs README）、self_interp 行数（wc vs HANDOVER §9）、版本号（Cargo.toml vs HANDOVER §1/§9）。CI 加 gate step（ubuntu 即可）。
- 验收：当前仓库（R3-R6 完成后）跑通全绿；故意改错任一被监控数字能红；gate 进 CI 后首跑绿。

### 本轮驳回/挂起

无驳回项。审查方法局限声明的未验证项（LLM 评测未重跑、性能表未系统复测、WASM OOB 未深挖）与既有方向级挂账一致，不重复立项。

---

## 执行顺序与依赖

T1（纯文档）→ T2 → T3 → T4 → T5（含两个新 eval 任务，依赖 T2/T4 完成）→ T6 → T7（收尾升版）。
每完成一项：跑全量回归电池（见 T7 后的清单）全绿后才进入下一项。

---

## T1｜P0：SECURITY.md "checked arithmetic" 失实修正（纯文档）✅ done 2026-09-03

- **证据**：`grep -n -i checked SECURITY.md` 仅剩否定/更正语境（"not checked"）；
  溢出/除零/模零/grep 计数均实测后写入（`println(9223372036854775807 + 1)` → `-9223372036854775808`；
  `1/0`→RUNTIME000 整数除以零；`5%0`→RUNTIME000 整数取模零；
  `grep -rn "checked_add\|checked_sub\|checked_mul\|checked_div" src/` 零命中——
  注意裸 `grep -c "checked_"` 会命中 wasm_codegen.rs:4586 的测试名 `arity_checked_at_compile_time`，故用精确模式）；
  回归电池 7/7 PASS（cargo test 454、clippy 0 告警、golden 逐字、fmt 幂等、eval 双后端 114/114、selfhost 五模式）。

- **审查出处**：P0-1。SECURITY.md:17 声称 "all arithmetic on Lom Int is checked (i64)"，
  实现是裸 `a + b`（interpreter.rs eval_arith），release 下 `i64::MAX + 1` 静默回绕；
  README:105 又把该条列入 "grep-verified" 加固清单——一个 grep 无法验证的语义声称。
- **范围**：
  1. SECURITY.md:17 重写为如实口径：Int 算术是 i64 语义——release 构建溢出静默回绕、
     无诊断；**仅**除零/模零是运行时诊断（RUNTIME000）。checked 算术**未实现**。
     该条从 "Interpreter hardening already in place" 移入 "Known limitations (accepted risks)"
     （受信程序威胁模型下危害有限，接受回绕语义）。
  2. README:105（Phase 6.6 历史条目）——删除 "checked arithmetic" 子句或加更正标注
     （历史记录不改写事实，但虚假子句必须被指出：加 "(更正 2026-09-03：checked 算术
     从未实现，见 SECURITY.md)" 之类）。
  3. SECURITY.md "Audit procedure" 节推广验证程序：**每条加固声称必须附可执行验证命令**
     （现有条目补齐——如溢出回绕：`println(9223372036854775807 + 1)` 输出 -9223372036854775808；
     除零：输出 RUNTIME000；总数器：`grep -c "checked_" src/*.rs` 为 0）。
- **验收**：SECURITY.md 中不再存在无限定的 "checked" 声称；README 的 "grep-verified"
  范围与事实相符；audit 节每条有命令。
- **明确不做**：不改实现上 checked_add（运行时行为变化，按冻结 §14 须 RFC——见驳回登记）。

## T2｜P1：let 绑定递归闭包的 NAM003 假阳性修复（双侧）✅ done 2026-09-03

- **证据**：复现程序 `--check` 退出码 0、输出"诊断通过，无错误"；
  对照组（`let x = x + 1`）宿主与自举各仍报 1 条 NAM003；
  新增测试 `let_closure_self_reference_no_nam003` / `let_non_closure_self_reference_still_nam003` 通过（456/456）；
  `verify_selfhost.py --static`："坏文件 PASS 15 / FAIL 0；干净集 ALIGNED 147 / DIFF 0"；
  回归电池 7/7 PASS（eval 双后端 114/114）。

- **审查出处**：P1-2。`let f = fn ... f(x-1) ... end` 静态报 Error 级 NAM003、--check 退出码 1，
  但两后端运行动态均正确（递归闭包是被设计支持的功能）。根因：typechecker.rs `Stmt::Let`
  先 check_expr(value) 后 env.define(name)——闭包体内自引用时名字尚不在作用域。
- **范围**：
  1. 宿主 typechecker.rs：`Stmt::Let` 的初始化器是**闭包字面量**时，先以 Unknown 预绑名字
     再查体（镜像运行时的 pre-bind + env 槽位补丁语义；非闭包初始化器如 `let x = x + 1`
     仍必须报 NAM003）。
  2. **自举侧同步修**：self_interp.lom Part E 的检查器（SLet 同样先查后绑）。不修则
     verify_selfhost --static 干净集会因两侧产出不再一致而红（当前两侧"一致地误报"所以对齐）。
  3. 回归测试：宿主 +2（闭包 let 自引用 --check 零诊断；非闭包自引用仍报）。
- **验收**：审查报告的复现程序 `--check` 退出码 0、ok:true；全量回归电池绿（含
  verify_selfhost --static 147 干净集对齐）。

## T3｜P1：闭包捕获 mut 绑定的 warning 诊断（新增 MUT002，warning-only 合法）✅ done 2026-09-03

- **证据**：复现程序 `--check` 退出码 0、恰好 1 条 `[MUT002] 闭包捕获了可变绑定 'x'`（4:9）；
  `--json` severity=warning、ok:true（与 MUT001 同发射路径）；examples 全部 +
  eval 下 560 个 .lom 扫描零 MUT002 触发（干净程序零新增诊断）；
  边界用例：不可变捕获/闭包内同名遮蔽不报，嵌套闭包捕获外层闭包 mut 局部在引用点报；
  `fix --plan` 对 MUT002 走未知码 hint-only 兜底（不自动应用）；
  自举侧未动（FOUR_CODES 过滤不受影响）；回归电池 7/7 PASS。

- **审查出处**：P1-3。`let mut x = 1; let f = fn() -> Int { x } end; x = 2; f()` 解释器输出 2、
  WASM 输出 1——分歧本身已文档化，但 --check 零诊断。MUT001 已建 TypeEnv.mutables 跟踪，
  机制现成。
- **范围**：typechecker 在闭包体作用域内引用已知 mut 绑定时发 **MUT002 warning**
  （stderr、不拦截、不置 ok:false——对齐 MUT001 的渐进式口径）。消息写明双后端语义相反
  （解释器=共享作用域 / WASM=创建时值拷贝），建议避免依赖。
- **注意**：MUT002 是宿主侧产出；自举检查器（8.2 子集）不产 MUT 家族，
  verify_selfhost --static 两侧都按 FOUR_CODES 过滤——不受影响，勿动自举侧。
- **验收**：复现程序 --check 退出码 0、恰好 1 条 MUT002 warning；干净程序零新增诊断；
  --json 的 severity=warning 与 MUT001 同形。
- **明确不做**：不统一两后端捕获语义（设计取舍，已文档化）。

## T4｜P1：浮点 inf/NaN 显示统一（修 bug）+ 大数分歧入清单（只记录）✅ done 2026-09-03

- **证据**：双后端实测 `println(1.0/0.0)`/`(0.0/0.0)`/`(-1.0/0.0)` 逐字一致输出
  `inf`/`NaN`/`-inf`（修前为 `inf.0` vs `Infinity.0`）；有限值 4.0/3.14/-0.5 双后端不变；
  SPEC_FOR_AI §11f 分歧清单 5 条全部当日实测（除零：RUNTIME000 vs `wasm trap: divide by zero`；
  trim：NBSP 解释器 trim→1 / WASM 不 trim→3；大数：`1000000000000000200000000000000.0` vs
  `1.0000000000000002e+30`）；字面量行已加"无科学计数法字面量"警示；
  回归电池 7/7 PASS（golden/eval 零变化，114/114 双后端）。

- **审查出处**：P1-1。①显示 bug：`println(1.0/0.0)` 解释器打 `inf.0`（to_display 给
  "inf" 补 ".0"）、WASM 打 `Infinity.0`（JS String）——双后不一致且都难看。
  ②大数：解释器全展开（Rust Display）vs WASM 科学计数法（JS），未列入 SPEC_FOR_AI 已知分歧。
- **范围**：
  1. 宿主 to_display（interpreter.rs:248-255）：Float 的 to_string 无小数点**且为非有限值**
     （"inf"/"-inf"/"NaN"）时原样输出，不补 ".0"。有限值行为不变（x.0 口径保持）。
  2. WASM 侧对齐：浮点打印路径（lom_ftoa/lom_print_float 链）把 JS 的 Infinity/-Infinity/NaN
     映射为 `inf`/`-inf`/`NaN`，与解释器逐字一致。
  3. SPEC_FOR_AI:675 已知分歧清单同步补全：现有清单漏了 HANDOVER 记录过的除零 trap 消息
     与 trim ASCII/Unicode 两条，一并补齐；**新增**"大数显示：解释器全展开 vs WASM 科学
     计数法（JS String），如 1e30 量级"——此条**只记录不统一**（harness 侧格式化工作量
     大、收益低，见驳回登记）。
  4. 顺手（同文件）：SPEC_FOR_AI 字面量行加一句"无科学计数法字面量——写 1000000000.0
     不写 1e9"（审查 P2-2 的文档警示部分）。
- **验收**：inf/-inf/NaN 三值双后端 stdout 逐字一致（`inf`/`-inf`/`NaN`）；有限值显示与
  全部存量 golden/eval 零变化；SPEC_FOR_AI 分歧清单含 5 条且与实现相符。

## T5｜eval 新任务 ×2 + 全量计数簿记（依赖 T2/T4）✅ done 2026-09-03

- **证据**：任务 116（closures）/117（types）solution 双后端实跑逐字一致后定稿
  （116：`120/3628800`；117：`inf/NaN/-inf/2.5/4.0`）；
  run.ps1 双后端 **116/116**（Total: 116 / Passed: 116 / Failed: 0）；
  `tests::eval_task_ids_globally_unique` ok（456/456 通过）；
  簿记逐一更新：manifest（total 116 + types/closures 13）＋ prompts 重跑（仅 03/04 变更，
  其余 8 文件字节不变）＋ eval/README 三处＋README 状态段（114/114→116/116，
  同段 454→456 一并如实更新）＋LANGUAGE_SPEC §12（116/116 + 布局注释 11/12→13/13
  顺手修陈旧值）＋HANDOVER §1 快照/§2.2 基线/§11.1 操作注记＋tutorial 量化面板/gate
  ⑤⑥/进度答案/871 差异清单 4→5 处＋ci.yml:87（113→116）＋SPEC_FOR_AI §11f 残留
  "all 108"（陈旧计数顺手修 116）；grep 复查现值声称无旧计数残留；
  回归电池 7/7 PASS（eval 双后端 116/116）。

- **审查出处**：发现 4（"边界值不进集合"）的**部分采纳**：只把修复后收敛的行为做成
  parity 任务；设计性分歧（mut 捕获）走 T3 诊断不走 parity——否则 gate 永久红。
- **范围**：
  1. 新任务 **116**（category: closures，difficulty: easy）：let 绑定递归闭包求值
     （如 factorial/fib 递归闭包，expected 双后端一致）。依赖 T2。
  2. 新任务 **117**（category: types，difficulty: easy）：println(1.0/0.0)、(0.0/0.0)、
     (-1.0/0.0) 及有限值对照（expected：inf/NaN/-inf 双后端一致）。依赖 T4。
     两个任务的 solution 必须在解释器与 WASM 双后端实跑通过后才算定稿。
  3. **计数簿记（114→116，一处不漏）**：
     - eval/manifest.json（total_tasks + closures/types 分类计数）
     - eval/prompts/_generate.ps1 重跑（03_types.md / 04_closures.md）
     - eval/README.md（总数两处 + 分类描述如涉及）
     - README.md 当前状态段落（114/114→116/116）
     - LANGUAGE_SPEC.md §12（114→116）
     - docs/HANDOVER.md §1 eval 行 + §2.2 基线
     - docs/lom-tutorial.html（量化面板 + gate 表 ⑤⑥ + "项目现在什么进度"答案）
     - .github/workflows/ci.yml:87 步骤名 "113 tasks"→"116 tasks"（审查 P2-3 顺手清）
- **验收**：run.ps1 双后端 116/116；cargo test 的 eval ID 唯一性测试绿；
  上列簿记位置逐一 grep 无残留旧计数。

## T6｜P3 杂项清理 ✅ done 2026-09-03

- **证据**：六项逐一落实——①verify_selfhost.py mode_run 注释改写为"防御网（正常 0 计数）"
  口径（与文件头 13-14 行一致，行为不变）；②SECURITY.md TUnknown 句改写为宿主实际机制
  （实测 `@` → LEX005 + 容错续解析；src/lexer.rs 无 TUnknown——那是自举侧机制）；
  ③Cargo.lock 句改为 "the lockfile is in-tree and contains no third-party packages"；
  ④README Roadmap Phase 7 行加"（时点值，2026-08-23；现值见 eval/README）"；
  ⑤`_t_fib.wasm` 已删除（未跟踪+gitignore，`ls` 确认不存在）；
  ⑥README Phase 1 段加"（历史里程碑快照，非现状……）"（执行者判断：加）；
  回归电池 7/7 PASS。

- tools/verify_selfhost.py:362-363：mode_run 内旧注释（"宿主 lexer 按字节 Latin-1 展开"）
  与文件头新注释矛盾——改写为"防御网（v1.0.0 修复后正常 0 计数）"口径（审查 P2-3）。
- SECURITY.md:16 "TUnknown tokens" 机制归属写串层（那是自举 mini 语言机制；宿主是
  LexError 容错路径）——改写为宿主实际机制，行为描述不变（审查 P3）。
- SECURITY.md:9 "There is no Cargo.lock dependency surface" 句面易误读——改为
  "the lockfile is in-tree and contains no third-party packages"（审查 P3）。
- README Roadmap Phase 7 行 "eval 113/113×2" 加"（时点值）"可读性标注（审查 P2-3 第三条）。
- 删除工作区杂物 `_t_fib.wasm`（未跟踪、已 gitignore，本地清理即可）。
- README "Phase 1" 段落加半句"（历史里程碑快照，非现状）"（审查 P3，可选项——执行者判断）。

## T7｜收尾：升版 1.1.0 ✅ done 2026-09-03

- **证据**：Cargo.toml/lock 升 1.1.0（`cargo build --release` 刷新），`lom --version` → `lom 1.1.0`；
  T7 后全量回归电池重跑 8 项全绿：cargo test 456/456、clippy -D warnings 零告警、
  stmt_interp golden 逐字、fmt --check 幂等、eval 双后端 116/116、verify_selfhost 五模式 PASS、
  --version 显示 1.1.0。未打 tag、未 commit、未 push（全部改动留工作区待维护会话验证）。
  配套登记：LANGUAGE_SPEC §13 v1.1.0 变更记录 + §7.3 MUT 行登记 MUT002 +
  SPEC_FOR_AI 码表/家族行登记 MUT002 + HANDOVER 版本行/tutorial 进度答案同步 1.1.0。

- 依据：MUT002（新 warning 诊断能力）+ NAM003 假阳性修复 + inf/NaN 显示修复 = 用户可见
  变更，minor 语义版本纪律。
- 范围：Cargo.toml + Cargo.lock 升 1.1.0（`cargo build --release` 刷新 lock）。
  **不打 tag、不提交、不推送**——验证与提交由维护会话执行。
- 全量回归电池（T1-T7 每步后跑，T7 后必须再跑一遍）：
  1. `cargo test --release`（全绿）
  2. `cargo clippy --release -- -D warnings`（零告警）
  3. `./target/release/lom.exe examples/bootstrap/stmt_interp.lom` diff golden（逐字）
  4. `./target/release/lom.exe fmt --check examples/selfhost/self_interp.lom`（幂等）
  5. `powershell -ExecutionPolicy Bypass -File eval/runner/run.ps1 -Verify -LomBin ./target/release/lom.exe`（T5 后应为 116/116）
  6. 同上 + `-Backend wasm`
  7. `python tools/verify_selfhost.py` 五稳定模式（dump/tokens/diags/static/run 全 PASS）
  8. `lom --version` 显示 1.1.0（T7 后）

---

## 维护会话复核记录（2026-09-03，交付验收）

整改会话交付后由维护会话独立复核：① 逐行审阅全部代码/文档 diff（typechecker 的
边界判定次序、嵌套闭包与非 mut 捕获语义均推演确认）；② 亲自复跑全量电池——
456/456、clippy 零告警、golden 逐字、fmt 幂等、eval 116/116 双后端、selfhost
五模式全 PASS（dump/tokens 149）；③ T2/T3/T4 复现程序逐个实测（递归闭包 --check
零诊断 / `let x = x + 1` 仍报 NAM003 / MUT002 恰 1 条且 ok:true / inf-NaN 双后端
逐字一致）；④ 整改会话的五项超出台账裁量项**全部接受**（T1 计数器口径修正是
对台账原始建议缺陷的如实纠偏——原 grep 模式会误命中测试函数名；456 计数连带、
顺手陈旧计数、v1.1.0 配套登记均符合项目惯例；历史快照不动正确）。

本工作包就此关闭；后续新待办重新登记于本文件。

## 驳回/挂起登记（2026-09-03 维护会话裁决，勿重新翻案）

| 审查建议 | 裁决 | 理由 |
|---|---|---|
| P0-1 走"改实现"路线（checked_add/sub/mul） | **驳回** | 运行时行为变化，冻结 §14 语义上需 RFC；受信程序威胁模型下回绕危害有限。文档路线（T1）已闭环失实问题。将来若真实需求出现可开 RFC。 |
| P1-1 大数显示统一（全展开 vs 科学计数法） | **挂起** | 统一需重写 harness 侧 JS 浮点格式化为 Rust Display 语义，工作量大、收益低；入 SPEC_FOR_AI 已知分歧清单（T4-3）即可守住"不撒谎"。 |
| P2-1 栈溢出结构化诊断 | **挂起（原状）** | 本就在 HANDOVER §1 post-1.0 挂账清单，不重复立项。 |
| P2-2 科学计数法字面量的 fix 高置信度改写规则 | **挂起** | 文档警示（T4-4）先落地；fix 规则的边界情况（1e300 的等价十进制形态超长）需要设计，收益待验证。 |
| P2-4 补 eval ID 空洞 108 | **驳回** | 审查自己也结论"改号成本大于收益"；ID 唯一性已有回归测试钉住。 |
| 发现 4 的 backend-parity edge 分类全量采纳 | **部分采纳** | 只把收敛行为做成 parity 任务（T5 的 116/117）；设计性分歧（mut 捕获、大数显示）进 parity 会永久红一侧——它们走 T3 诊断 + T4 文档。 |
| README Phase 1 历史快照段落 | **可选** | 项目惯例是时点快照记录；加一句标注即可（T6），不重写。 |

## 与既有挂账清单的关系

HANDOVER §1 的 post-1.0 挂账（wasm 越界深挖、L2 自举编译器、Pattern 无 span、包注册中心、
调试器、概率类型）是**方向级**长期清单，不属本台账范围；本台账是审查驱动的**整改级**
 bounded 工作包。两者勿混。
