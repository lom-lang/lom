# Lom 新任维护者交接提示词（2026-09-25）

> **文档定位（2026-09-21 用户裁决；2026-09-29 结构瘦身裁决）**：本文件是
> Lom 的**持续维护文档**与交接必要流程。2026-09-29 起采用**核心提示 +
> 事实源指针**两段式：代码块只保留行为契约（铁律）与活信息（版本/审查/
> open 项/菜单/第一回合清单/关键锚点索引），交付史、教训、逐批行为锚点
> 不再手写拷贝于此——其唯一事实源（RFC-0004 修订记录、HANDOVER §11.6、
> SPEC §13、TODO 顶部、designs/0001~0012、verify_selfcomp 用例集）全部
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
- **R94 挂账族两批全部交付（v1.4.8/v1.4.9），R94 全部关闭、
  台账无 open 项（2026-09-30）**：用户裁决"按你的提议执行"——
  统筹 R94 宿主挂账族 + 二十审三条精度备注 + 规划者只读核验新
  发现 R103 开账即修，设计 [designs/0013](designs/0013-r94-ledger-remediation.md)。
  批 1（v1.4.8，零 src/ 改动）：run_wasm.mjs 唯一 Map 排序点
  JS 码元序 → Buffer.compare UTF-8 字节序（json 星面键序面关闭）；
  二十审 §6-①② 负例两枚 + §6-③ 清单句；R103 四处时点残留
  开账即修；顺带更正 R102 漏网（README 横幅 #209→#210）。批 2
  （v1.4.9）：**map_remove 统一 Bool**——对齐 SPEC §9.7 冻结宣称
  （翻转 v1.2.13 裁决 2 甲的 L2 选边）：宿主 build_map_remove
  V_UNIT → probe 命中 Bool tag（蓝本 build_map_has，e2e 锁定）、
  L2 "void" 裁决/R89 文案/签名同步 i32，三侧 println 形态
  true/false/true 一致，存量 198 恒等 + 2 有意变化（81/84）；
  **包内 as 别名**——解释器 load_packages 注册补齐（顺带打通
  包内 stdlib 别名暗坑）+ build 路径 externals，105 用例三侧
  18/18、3 条 NAM003 假阳性清零（十八审"2 条"系笔误随批更正）。
  R95-R103 全关；**台账无 open 项**。登记在案不修边界：`lom
  build` 包管理流程逐文件视图下包源内别名仍 NAM003；clippy
  --all-targets 有 9 条存量测试 lint（rust-1.97.0 漂移、CI
  口径零输出不受影响）。二十审 A-（review-2026-09-30.html，
  v1.4.2→v1.4.7 六批，宣称零失真 + 22 探针零击穿）不外推。
- **交付链**：…→ v1.4.6 批1 E甲扩围+R100（`96c26b6`，#207）→
  v1.4.7 批2 R101 宿主（`22a37a6`，#210）→ 二十审 A- + R102 修正
  （`257e726`，#213）→ v1.4.8 R94 批 1（`8c86a60`，#214）→
  v1.4.9 R94 批 2（本轮）。**已提交基线**：verify_selfcomp
  **362/362 = 196 单文件 + 5 包 + 161 负例**、bootstrap **14/14**
  （强 quine 269521 bytes 双侧一致）、self_comp.lom 12529 行、
  Rust **543 单元 + 10 集成**（r56×1 + r58×7 + r94_pkg_alias×2）、
  eval 双后端各 121/121；存量 hex 对比 198 恒等 + 2 有意变化
  （批 2a）。Cargo.toml/lock 均为 1.4.9。
- R101 修复后**三实现短路 return 全对齐**（0/9/1/8 rc0）；
  map_remove 三侧 true/false/true 对齐；bottom 家族与 Map 家族
  均不再有已知宿主-L2 分叉。
- 下一步：**交接就绪，无在制工作**。待裁菜单：typechecker for
  可变性 quirk 等登记项处置、`lom build` 逐文件视图 NAM003 边界
  修否、二十一审发起时机、下一阶段方向（发布线继续冻结直至
  用户解冻）。
- 交付史与机制事实源：RFC-0004 修订 1–42、SPEC §13、
  designs/0001–0013、TODO 顶部、HANDOVER §11.6 和 selfcomp
  用例集。审查评级不外推，历史数字不回写。
- 维护流程、审查节奏与交接五件套规范：HANDOVER §12（含 §12.4）。
【第一回合必须完成（规划者流程）】
1. **规划者亲自读**：docs/HANDOVER.md §0/§1/§2.2/§9/§11.6/§12（含
   §12.4），docs/TODO.md 顶部，docs/reviews/review-2026-09-30.html
   （二十审——最新轮）及 review-2026-09-29.html（十九审），
   LANGUAGE_SPEC §14，docs/rfc/0004-l2-selfhost-compiler.md（修订
   1-41 全读），docs/designs/0001~0013 十三份批次设计（含各批实施
   修正与机制偏离记录——**行为基线的逐批细节以此为准**）；涉及
   架构时再派子智能体供料读 RFC-0003。动工面的关键路径读码（现状
   拒绝点/宿主蓝本）派子智能体整理供料，规划者复核关键结论。
2. 基线验证（可整体派 1 个子智能体执行并回报逐项输出，规划者抽验
   verify_selfcomp 与 doc_audit 两项亲自重跑）：
   - cargo build --release
   - cargo test --release（期望 542/542；另有集成 cargo test --release --test r56_process --test r58_lsp_process，×8）
   - cargo clippy --release -- -D warnings（零 warning）
   - cargo fmt --all -- --check（本地零 diff；当前不在 CI gate）
   - python tools/doc_audit.py（期望 67/67）
   - python tools/spec_examples_check.py（期望 RESULT: PASS）
   - python tools/eval_prompt_check.py（期望 24/24）
   - python tools/verify_selfhost.py 及 --tokens/--diags/--static/--run/--wasm（六模式逐个，全 PASS）
   - python tools/verify_selfcomp.py（已提交基线 362/362 = 196 单文件 + 5 包 + 161 负例）
   - python tools/verify_selfcomp.py --bootstrap（期望 14/14——三层自证与自施加强 quine 269521 bytes；耗时随机器负载 ~100s-500s；另有 --ci-smoke 子档已入 CI）
   - powershell -ExecutionPolicy Bypass -File eval/runner/run.ps1 -Verify -LomBin ./target/release/lom.exe（121/121；WASM 侧加 -Backend wasm 同 121）
   - Lom fmt（PowerShell 递归覆盖 examples/：37 个有效文件；apply_test 豁免）：
     $lomFmtFiles = Get-ChildItem -LiteralPath examples -Recurse -Filter *.lom -File | Where-Object { $_.Name -ne 'apply_test.lom' }
     foreach ($lomFmtFile in $lomFmtFiles) { & .\target\release\lom.exe fmt $lomFmtFile.FullName --check; if ($LASTEXITCODE -ne 0) { throw $lomFmtFile.FullName } }
   - 新增/迁移的 tools/selfcomp 用例逐个 fmt --check：C甲 12 + D甲 12 + 批1 8 枚（正例 194–200 + 负例 neg_r97e_* 1 枚）已入 v1.4.4–v1.4.6；examples 原 37 个有效文件不变。
   - `git status` 应基本干净（交接刷新的文档回填除外）；实查最新已提交 main CI 与 annotations，不能外推至未提交工作区。
3. 如实报告 tag 基线核验边界。R94 挂账族两批均已交付；新方向
   只呈菜单待裁。外部发布线继续冻结。
4. 已裁批次：规划者更新方案与自包含任务书 → 派执行者实施 → 规划者
   验收（全量回归 + 存量 hex 对比 + 抽查）→ 提交推送并看 CI 首跑；
   CI 绿后才切 tag。新增裁决点仍由用户决定。

【关键锚点索引（当前行为要点；已提交基线由 verify_selfcomp
  362 项用例/负例机器锁定，不在此手写复述——抽验形态看十九审报告
  §4 探针原文与事后勘误、designs 实施修正记录）】
- 复现锚点：R95 原形态 `let v = if c return 1 else return 2
  end; v + 0` 作函数尾值，宿主 TYPE001 warning 收（运行正确
  1/2），A甲 L2 拒；B甲已于 `a5dac74`/v1.4.3 经 CI 绿后修复
  并关闭 R95。原 `v` 直尾旧 L2 COMPILED 121B 后栈下溢（R99），
  A甲安全拒，B甲转为可验证且行为一致。R96——闭包体内
  println(n)：宿主 7/7 收，v1.4.3 的 L2 拒"闭包捕获了未定义
  变量 'println'"；C甲已在 v1.4.4 放行（父 env 优先 + 仅豁免
  两名 prelude）并关闭 R96；裸值位引用仍拒（负例锁定）。R97——
  无字面量程序 `match Some(xs) => println(xs)` 曾落 ibase 兜底拒
  （带 String 字面量同形态过）；D甲已在 v1.4.5 经预扫追踪 Binder
  三流入路放行并关闭 R97，跨函数容器不经 match 直接 println 仍拒
  （负例锁定）。
- 写 Lom 代码最高频坑（全档见 HANDOVER §11.6 与 §4——写探针/用例
  前过一遍）：布尔字面量大写 True/False；无 [..] List 字面量（用
  range/list_cons）；`not` 不是运算符（用 !x）；match Form B 每臂
  独立 end；宿主 dangling-else 贪婪归内（嵌套 if 自带 else）；无
  break/continue（flag 化）；Int 安全值域 ±2^59；无 0x 字面量；
  多返回值调用点显式取 .0/.1；写完用例全文 grep "Fn" 自查（Fn
  注解是 L2 负例形态）；裸 list_empty() 直接 println 是 ls{?} 编译
  期拒（加注解/cons 上下文）；无注解构造的 Bool/嵌套容器迭代显示
  保守拒（仅注解三形态播种，修订 38）；WASM hex 手写注意 eqz/sleb
  单字节指令族与偶字符占位（容器显示批教训）。
- R92/R95/R99 登记链：单 return 臂历史双侧一致；双 return
  臂绑定后算术十九审 R95 误拒、绑定直尾值 R99 坏 WASM；A甲
  安全拒后 B甲在 v1.4.3 正确发射。R101 短路右侧实际 return 的宿主
  WASM 分叉已于 v1.4.7 修复（三侧对齐 0/9/1/8）。

现在从上手三步开始。先报告只读核验，再按已裁批次推进并呈报；
新方向仍待用户裁决。
```

## 维护者备注

- 上述路径使用 Windows 形态，是本项目当前维护环境。
- 提示词故意不写固定 HEAD SHA；新任必须以 `git log -1`、`git status` 和最新 CI 实查。
- 若交接提交的 CI 首跑不是绿色，先处理 CI，不得把交接状态宣称为完成。
- 交接更新规范见 HANDOVER §12.3（五件套）；本文件**【当前真实状态】段每轮
  交接整体重写**（铁律/第一回合/锚点索引三段保持稳定，仅在事实过时时点改）；
  **角色与分工段（2026-09-25 用户裁决）与两段式结构（2026-09-29 用户裁决）
  是结构约定**——改动须经用户裁决，不得随交接静默漂移。
