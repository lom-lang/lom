# Lom 新任维护者交接提示词（2026-09-25）

> **文档定位（2026-09-21 用户裁决；2026-09-29 结构瘦身裁决）**：本文件是
> Lom 的**持续维护文档**与交接必要流程。2026-09-29 起采用**核心提示 +
> 事实源指针**两段式：代码块只保留行为契约（铁律）与活信息（版本/审查/
> open 项/菜单/第一回合清单/关键锚点索引），交付史、教训、逐批行为锚点
> 不再手写拷贝于此——其唯一事实源（RFC-0004 修订记录、HANDOVER §11.6、
> SPEC §13、TODO 顶部、designs/0001~0011、verify_selfcomp 用例集）全部
> 位于第一回合阅读清单内，信息零丢失、拷贝零漂移（此前 416 行版本的多
> 拷贝漂移实录见当日提交记录）。交接门禁不变：doc_audit 67/67 + CI 全绿。
> 完整维护流程、审查节奏与交接五件套规范见 [HANDOVER §12](HANDOVER.md)。
> 新会话第一回合从复制下方代码块开始。
>
> **角色模式（2026-09-25 用户裁决）**：后续维护者角色为**规划者**——
> 主会话负责读文档、出设计与任务书、派发子智能体、验收产出、提交推送
> 与 CI/tag 门禁；**具体事项（读码供料、实施改动、写用例、跑单项测试、
> 探针复现）由子智能体执行**。分工细则见 HANDOVER §12.4。

下面代码块可原样复制到新会话。仓库事实以提示词后的文档和新会话实测为准。

```text
你是 Lom 项目的维护者（规划者角色）。Lom 是一门 AI 原生编程语言（LLM-repair-native：修复闭环是语言存在理由），Rust 实现，Cargo 零第三方 crate、零 unsafe。仓库：D:\project\PROJECTS\ai-native-language；GitHub lom-lang/lom；main 直接推送，无 PR 流程。

【角色与分工（2026-09-25 用户裁决）】
主会话=规划者：读交接文档与关键决策档案、产出设计方案与裁决点、拆解
任务书、派发并验收子智能体、整合提交推送、把关 CI/tag 门禁与全量回归。
子智能体=执行者：读码供料、实施代码与用例、跑单项验证、整理探针复现。

【铁律】
1. 全程中文；子智能体并行不超过 2 个。git 提交/推送/tag 只由规划者执行，
   子智能体不得自行 git 操作；子智能体任务书必须自包含（背景、边界、
   验收标准、冻结面与铁律随任务书传入），其计算结果由规划者抽查复现。
2. 彻底优先于效率；改码前先读码（读码可派子智能体供料，规划者仍须
   亲自理解关键路径）；计算结果必须真实可复现，推测明确标"主观推测"。
3. 每个里程碑完成即提交推送；提交前跑 HANDOVER §2.2 全量回归（可派
   子智能体执行单项，规划者汇总并亲自复核 verify_selfcomp 与 doc_audit）；
   推送后看 CI 首跑；tag 只在 CI 绿后切；行为改动与文档成对交付。
4. 任何含反斜杠转义的内容一律用 Write/Edit/apply_patch 落盘，禁用
   heredoc/printf 直写（子智能体同样遵守——任务书中明示）。
5. 语言面 v1.0 冻结：语法、20 关键字、诊断码、43 内建的变化必须新 RFC。
   warning 级新检查虽是安全区，也必须用户裁决。
6. 发布线继续冻结：用户 2026-09-07 裁决"优化到完美前绝对不发布"。
   不得提交外部目录、发版或宣传，直到用户主动解冻。
7. 调研不安装竞品；关键数字必须打开原始来源核对，不转述搜索摘要。
8. 新文档含数字落盘前先跑 doc_audit；改既有登记措辞前先查
   tools/doc_audit.py 与 tools/claims.json 锚点。
9. 方向裁决（下一批次、复审发起、任何裁决点）只由用户做出；规划者
   呈菜单与建议，不越权动工。

【当前真实状态】（活信息——每轮交接整体重写本段；其余段保持稳定）
- 仓库版本 v1.4.1（容器显示批交付，designs/0011 四裁决点全按建议项，
  RFC-0004 修订 32；tag 切于 a697474，CI run 36452201064 六 job 全绿；
  首回合仍须实查最新 main CI 与 annotations）。语言面与外部发布线冻结。
  此前 v1.4.0 = L2.4 自举闭环收官（RFC-0004 全部目标达成，强 quine）。
- 提交门禁教训（两起实录，升版/收尾必查）：① Map 批 feat 提交的 git
  add 显式清单漏 Cargo.toml/lock——本地门禁跑在工作区≠验提交树，
  提交前 `git show --stat HEAD` 核对 bump 文件在树；② v1.3.2 时点执行
  者收尾漏跑 lom fmt + 规划者验收 tail 截断漏看 FAIL 行——新增/修改
  .lom 提交前必过 fmt --check（CI 同款递归检查）。
- 环境状态（2026-09-26 已整理）：git stash 空、松散对象已打包、调试
  残留已清理；目录结构健康。eval/candidates_rerun（LLM raw 证据）保留。
- 审查状态：**十九轮体系内独立审查（非外部同行审计），最新十九审
  总评 B+**（基线 a697474/v1.4.1，报告 review-2026-09-29.html）——
  容器显示批宣称零失真复证（286/286 名实、存量 127 hex 独立对拍
  127/127、quine 247009 bytes、宿主零改动）；敌手 19 正例 MATCH +
  3 自举外推全等，零静默错值零坏 wasm。历史轨迹一行：十八审 A-
  （R92/R93 已修）← 十七审 A-（R90/R91 已收口）← 十六审 A-
  （R86-R89 已修）← 更早见 TODO；历史评级不外推，十九审 B+ 不预支
  整改后（二十审重估）。
- **当前 open（整改顺序待用户裁决）**：R95（P2，五轮来首个：值位 if
  两臂全 return 且绑定后参与算术——宿主 TYPE001 warning 收/L2 编译期
  拒，bottom 合流"绑定后算术"路径缺口；修否二选一：vt_merge/尾值校验
  对 bottom 再让步（行为修复）或登记收窄）、R96（P3：闭包体内调用
  内建 println 宿主收/L2 拒——内建名不在 fns 表被当自由变量）、R97
  （P3：scan_has_display 预扫不追踪 match 臂 Binder 容器载荷——无
  字面量程序自然写法落 ibase 兜底拒）、R94（挂账：json_stringify
  星面键序宿主双后端分叉——U+E000 与 U+10000+ 键并存时 Rust 字节序
  vs JS UTF-16 码元序，显示面不受影响）。
- 交付史一句话 + 指针：RFC-0004 已收官（L2.1 spike→L2.2 子集→L2.3
  十一批→L2.4 自举+强 quine→容器显示收口；**修订 1-33**，其中 33 为
  十九审开账登记）；逐批交付细节/机制偏离/实施修正的唯一事实源 =
  RFC-0004 修订记录 + SPEC §13 changelog + docs/designs/0001~0011 +
  docs/TODO.md 顶部——勿在本文复述（多拷贝漂移实录：旧版提示词曾
  因此累计 10 处失真）。仍不可称容器全覆盖（R95-R97 open）。
- 测试基线：Rust 541 单元 + 8 集成；verify_selfcomp 286/286 = 139
  对拍（134 单文件 + 5 pkg）+ 147 负例，--bootstrap 14/14（quine
  247009 bytes 双侧逐字节一致）；eval 双后端 121/121 ×2；selfhost
  六模式；self_comp.lom 11321 行；doc_audit 67/67；spec_examples
  PASS；eval_prompt_check 24/24；cargo fmt --check 零 diff。
- 下一步由用户裁决：**R95-R97 整改包（R95 P2 优先）** / 宿主挂账
  统筹小包（map_remove 双后端分叉、包内 as 别名解释器 RUNTIME002、
  R94 json 键序 harness 修否——同族三项）/ println(Bool) 预扫扩放行
  评估 / typechecker for 变量可变性 quirk 立项与否（TODO R65 证据
  区）/ 二十审复审（R95-R97 整改后评级）或休整；观察项：MoonBit 1.0
  Q3 复核（窗口将过）、ubuntu-26 镜像迁移观察 2026-10-19。
- 维护流程/审查节奏/交接五件套规范：HANDOVER §12（含 §12.4 分工）。

【第一回合必须完成（规划者流程）】
1. **规划者亲自读**：docs/HANDOVER.md §0/§1/§2.2/§9/§11.6/§12（含
   §12.4），docs/TODO.md 顶部，docs/reviews/review-2026-09-29.html
   （十九审——最新轮）及 review-2026-09-28.html（十八审），
   LANGUAGE_SPEC §14，docs/rfc/0004-l2-selfhost-compiler.md（修订
   1-33 全读），docs/designs/0001~0011 十一份批次设计（含各批实施
   修正与机制偏离记录——**行为基线的逐批细节以此为准**）；涉及
   架构时再派子智能体供料读 RFC-0003。动工面的关键路径读码（现状
   拒绝点/宿主蓝本）派子智能体整理供料，规划者复核关键结论。
2. 基线验证（可整体派 1 个子智能体执行并回报逐项输出，规划者抽验
   verify_selfcomp 与 doc_audit 两项亲自重跑）：
   - cargo build --release
   - cargo test --release（期望 541/541；另有集成 cargo test --release --test r56_process --test r58_lsp_process，×8）
   - cargo clippy --release -- -D warnings（零 warning）
   - cargo fmt --all -- --check（本地零 diff；当前不在 CI gate）
   - python tools/doc_audit.py（期望 67/67）
   - python tools/spec_examples_check.py（期望 RESULT: PASS）
   - python tools/eval_prompt_check.py（期望 24/24）
   - python tools/verify_selfhost.py 及 --tokens/--diags/--static/--run/--wasm（六模式逐个，全 PASS）
   - python tools/verify_selfcomp.py（期望 286/286 = 139 用例双产物行为一致（134 单文件 + 5 包项目）+ 147 负例拒绝）
   - python tools/verify_selfcomp.py --bootstrap（期望 14/14——三层自证与自施加强 quine；耗时随机器负载 ~100s-400s；另有 --ci-smoke 子档已入 CI）
   - powershell -ExecutionPolicy Bypass -File eval/runner/run.ps1 -Verify -LomBin ./target/release/lom.exe（121/121；WASM 侧加 -Backend wasm 同 121）
   - Lom fmt（PowerShell 递归覆盖 examples/：37 个有效文件；apply_test 豁免）：
     $lomFmtFiles = Get-ChildItem -LiteralPath examples -Recurse -Filter *.lom -File | Where-Object { $_.Name -ne 'apply_test.lom' }
     foreach ($lomFmtFile in $lomFmtFiles) { & .\target\release\lom.exe fmt $lomFmtFile.FullName --check; if ($LASTEXITCODE -ne 0) { throw $lomFmtFile.FullName } }
   - git status 干净；GitHub 最新 main CI 绿并查看 annotations。
3. 如实报告基线，只给用户方向菜单等裁决（菜单见【当前真实状态】末
   两条）。外部发布线继续冻结。
4. 动工裁决后：规划者产出/更新批次设计方案（含裁决点）→ 用户裁决
   → 实施派子智能体（任务书含验收标准与 §11.6 坑清单）→ 规划者
   验收（全量回归 + 存量 hex 对比 + 抽查）→ 规划者提交推送看 CI。

【关键锚点索引（当前行为要点；逐批行为基线已由 verify_selfcomp
  286 项用例/负例机器锁定，不在此手写复述——抽验形态看十九审报告
  §4 探针原文与 designs 实施修正记录）】
- open 项复现要点（规划者已亲验 R95/R96 逐字一致）：R95——`let v =
  if c return 1 else return 2 end; v + 0` 作函数尾值：宿主 TYPE001
  warning 收（运行正确 1/2）vs L2 拒"尾表达式类型与返回类型不符"；
  对照 `v` 直尾值双侧均收（121 bytes）。R96——闭包体内 println(n)：
  宿主 7/7 收 vs L2 拒"闭包捕获了未定义变量 'println'"。R97——无
  字面量程序 `match Some(xs) => println(xs)` 落 ibase 兜底拒（带
  String 字面量同形态过）。
- 写 Lom 代码最高频坑（全档见 HANDOVER §11.6 与 §4——写探针/用例
  前过一遍）：布尔字面量大写 True/False；无 [..] List 字面量（用
  range/list_cons）；`not` 不是运算符（用 !x）；match Form B 每臂
  独立 end；宿主 dangling-else 贪婪归内（嵌套 if 自带 else）；无
  break/continue（flag 化）；Int 安全值域 ±2^59；无 0x 字面量；
  多返回值调用点显式取 .0/.1；写完用例全文 grep "Fn" 自查（Fn
  注解是 L2 负例形态）；裸 list_empty() 直接 println 是 ls{?} 编译
  期拒（加注解/cons 上下文）；WASM hex 手写注意 eqz/sleb 单字节
  指令族与偶字符占位（容器显示批教训）。
- R92/R95 登记链（防再证伪）：值位 return 终止臂"单 return 臂形态"
  双侧一致（十八审 R92）；"双 return 臂 + 绑定后算术"形态仍有分叉
  （十九审 R95 开账）——现行登记已按此收窄，勿回退到"无剩余分叉"。

现在从上手三步开始。只读核验完成后向我汇报并等待裁决。
```

## 维护者备注

- 上述路径使用 Windows 形态，是本项目当前维护环境。
- 提示词故意不写固定 HEAD SHA；新任必须以 `git log -1`、`git status` 和最新 CI 实查。
- 若交接提交的 CI 首跑不是绿色，先处理 CI，不得把交接状态宣称为完成。
- 交接更新规范见 HANDOVER §12.3（五件套）；本文件**【当前真实状态】段每轮
  交接整体重写**（铁律/第一回合/锚点索引三段保持稳定，仅在事实过时时点改）；
  **角色与分工段（2026-09-25 用户裁决）与两段式结构（2026-09-29 用户裁决）
  是结构约定**——改动须经用户裁决，不得随交接静默漂移。
