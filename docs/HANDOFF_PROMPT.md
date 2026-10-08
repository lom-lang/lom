# Lom 新任维护者交接提示词

> **文档定位（2026-09-21/25/29 用户裁决；2026-10-06 维护规程 2.0 重构）**：
> 本文件是 Lom 的**持续维护文档**与交接必要流程、新会话入口。三段式：
> ①角色与铁律（行为契约，规范源为 [MAINTENANCE.md](MAINTENANCE.md) §1——
> 本代码块为携带式镜像）②【当前真实状态】（活信息镜像——每轮交接整体重写本段；主写位 TODO 顶部）
- **状态（2026-10-09，v1.9.1 build 三缺口收口 + 三十审 B+ 二连后交接就绪）**：
  最新已提交基线 **v1.9.1**（tag `7c2403c`，CI run `37767137349` 七 job
  绿；HEAD 以 git log 与 CI 首跑为准）——build 三缺口（多文件包假阳性/
  PKG007/主文件诊断面）已收口销账（designs/0021）；**L2 已转正**
  （v1.8.0：supported face）+ **l2fix CLI 子命令**（v1.9.0：宿主二进制
  一等公民）+ **L2 诊断行号**（v1.7.2：`ln:cl` 位置前缀三通道统一）。
  playground 线上常绿（footer 已刷转正措辞）；发布面：README 门面
  （v1.9.1 Current release）+ Release v1.7.0（pre-release）+ Release
  v1.6.1（pre-release）。**台账零 open**（R119-R126 全关）。
- **本轮交付链（2026-10-08~09，用户裁决"执行"/"按建议执行"/"按照你的想法执行"）**：
  **④L2 行号批 v1.7.2**（`ca31691`）→ **二十八审 A-（四审连平）**（`047659c`）
  → R119 收口 → **⑤L2 转正批 v1.8.0**（`88d91dc`：宣称面三处更新+审查轮数
  26→28）→ playground footer 重部署（`889a43f`）→ **⑥l2fix CLI 子命令
  v1.9.0**（`9751011`：828 行零 regex、python↔rust 对拍零差异、12 单测
  561→573）→ **⑧build 三缺口收口 v1.9.1**（`7c2403c`：减法修法+PKG007+
  主文件对称补全，集成 31→35）→ **二十九审 B+**（`29f34ea`：R120 CI 接线
  落空/R121 位置过度宣称——两笔宣称失真跌档；32 探针零击穿）→ R120-R123
  收口（`7803afe`）→ **三十审 B+ 二连**（`82e8ed1`：R124 计数全组同滞/
  R125 根减法缺位/R126 头注反向——技术面全绿但"宣称零失真"二连被打破）
  → R124-R126 收口（`b614037`：含根路径减法行为修复——与缺口 A 对称）。
- **审查轮次与评级（均不外推）**：……→二十七审 A-→二十八审 A-（四审连平）
  →**二十九审 B+（宣称失真×2）→三十审 B+ 二连（计数同滞+根减法缺位）**
  ——B+ 二连的根因均为文档精度/宣称纪律，非技术面缺陷（两轮 51 项探针
  零击穿、基线矩阵 38 项全绿）。三十一审若复核 R124-R126 修复零失真，
  技术面无新 P1/P2 → A- 可期。最新报告
  [review-2026-10-09-2.html](reviews/review-2026-10-09-2.html)。
- **已提交基线**：verify_selfcomp **368/368 = 196 单文件 + 8 包 + 164 负例**、
  bootstrap **14/14**（强 quine 272925 bytes 双侧一致）、
  self_comp.lom 12671 行、Rust **573 单元 + 35 集成**
  （r56×1 + r58×7 + r94×2 + r104×6 + r107×4 + r108×5 + closure×3 +
  r112×3 + build_gaps×4）、eval 双后端各 128/128、fix_corpus 13 对、
  doc_audit **71/71**、六模式全 PASS（dump 161）、playground node 冒烟
  ALL PASS、link_check PASS + l2fix self-check 6/6（CI doc-gates 常驻）。
  Cargo.toml/lock 均为 **1.9.1**。
- **L2 现行面（v1.8.0 转正后）**：supported face（strict subset / [L2xxx]
  码 / ln:cl 位置 / l2fix 修复闭环 / `lom l2fix` CLI 子命令 / l2fix.py
  CI 对拍基准）。
- **playground 维护要点（v1.9.1 面）**：构建
  `cargo build --target wasm32-wasip1 --profile wasm-release`；冒烟
  `node playground/smoke.mjs`；部署 = playground/ 五源件 + 产物 wasm 推
  gh-pages 孤儿分支（worktree 方式）；改 src 后同步重建 wasm 再部署。
- **现行语义要点**：撞名/遮蔽族三侧一致（本地定义优先；两包同名按包根
  路径序取后者；同别名取后写声明；NAM006/PKG007 warning 不拦截）；
  build 三缺口收口后无文件 build 诊断面 = 包源逐文件（减法 externals）
  + PKG007 + 根目录主文件对称检查（减法 externals）；根文件 fn 撞包名
  不发 NAM006 遮蔽告警（减法消除——NAM006 检测仅在带文件路径合并单元）。
- **登记在案不修边界（非 open 项，已从六项减至三项）**：块内 let 无同名
  泄漏与循环外读 for 变量两 divergence；clippy --all-targets ~4 条工具
  链漂移存量；~~多文件包假阳性/无文件 PKG007/主文件不检查~~（v1.9.1
  销账）；playground 四边界；gh-pages 独立构建流；归档 HTML assets 未随档。
- 下一步：**交接就绪，方向待用户裁决**。菜单：⑦性能工程 / 三十一审
  （B+ 二连后复评——若 R124-R126 修复零失真可回 A-）/ Release v1.7.0
  转正（等外部反馈）。观察项：Ubuntu 26 迁移（**2026-10-19**，盯 CI
  首跑——doc-gates 含断链+l2fix 两新步骤）。对外动作逐项呈批（铁律 6）。

【第一回合必须完成（规划者流程；分层阅读协议 MAINTENANCE §5.2）】
1. **L0 现行必读（规划者亲自）**：docs/MAINTENANCE.md 全文（规则）；
   docs/TODO.md 顶部状态块（台账/菜单/顺延号主写位）；docs/reviews/
   最新一轮及上一轮审查报告；LANGUAGE_SPEC §14（冻结）；RFC-0004 修订
   记录**最近 10 条全文 + 全部修订标题行**（grep "修订"）；docs/designs/
   全部**状态行**（各文件头部）+ 预计动工面设计全文；docs/HANDOVER.md
   §1/§2.2/§9 + §0/§4/§11.6 按索引入口。历史细节（RFC 早期修订全文、
   旧报告、archive 年表）属 L2 考古层——复审宣称复核时强制下钻，冷启动
   不必全读。动工面关键路径读码派执行者供料，规划者复核关键结论。
2. 基线验证（可整体派 1 个子智能体执行并回报逐项输出——按
   EXECUTOR_TEMPLATE 输出格式，计数配对逐项单独执行；规划者抽验
   verify_selfcomp 与 doc_audit 两项亲自重跑、计数抽查复跑）：
   - cargo build --release
   - cargo test --release（期望 573/573；另有集成 cargo test --release --test r56_process --test r58_lsp_process --test r94_pkg_alias --test r104_dedup_order --test r108_variant_externals --test r107_alias_clash --test build_closure_externals --test r112_closure_assign --test build_gaps_remediation，×35）
   - cargo clippy --release -- -D warnings（零 warning）
   - cargo fmt --all -- --check（本地零 diff；当前不在 CI gate）
   - python tools/doc_audit.py（期望 71/71）
   - python tools/spec_examples_check.py（期望 RESULT: PASS）
   - python tools/eval_prompt_check.py（期望 31/31；v1.5.0 起 error_repair 31 题）
   - python tools/verify_selfhost.py 及 --tokens/--diags/--static/--run/--wasm（六模式逐个，全 PASS）
   - python tools/verify_selfcomp.py（已提交基线 368/368 = 196 单文件 + 8 包 + 164 负例）
   - python tools/verify_selfcomp.py --bootstrap（期望 14/14——三层自证与自施加强 quine 272925 bytes；耗时随机器负载 ~100s-500s；另有 --ci-smoke 子档已入 CI）
   - powershell -ExecutionPolicy Bypass -File eval/runner/run.ps1 -Verify -LomBin ./target/release/lom.exe（128/128；WASM 侧加 -Backend wasm 同 128）
   - Lom fmt（PowerShell 递归覆盖 examples/：37 个有效文件；apply_test 豁免）：
     $lomFmtFiles = Get-ChildItem -LiteralPath examples -Recurse -Filter *.lom -File | Where-Object { $_.Name -ne 'apply_test.lom' }
     foreach ($lomFmtFile in $lomFmtFiles) { & .\target\release\lom.exe fmt $lomFmtFile.FullName --check; if ($LASTEXITCODE -ne 0) { throw $lomFmtFile.FullName } }
   - 新增/迁移的 tools/selfcomp 用例逐个 fmt --check（词法合法的用例应全过 rc=0）；v1.6.0 的 lex 负例 2 枚（neg_l2_lex_*）按构造含词法错误、不适用 fmt --check（锁点在 verify_selfcomp 的 lex 诊断格式+码）；examples 原 37 个有效文件不变。
   - `git status` 应基本干净（交接刷新的文档回填除外）；实查最新已提交 main CI 与 annotations，不能外推至未提交工作区。
3. 如实报告 tag 基线核验边界。新方向只呈菜单待裁；对外发布动作逐项
   呈批后执行（铁律 6）。
4. 已裁批次：规划者更新方案与自包含任务书（EXECUTOR_TEMPLATE + 批次段）
   → 派执行者实施 → 规划者验收（全量回归 + 存量 hex 对比 + 抽查复跑）→
   数字锚按 MAINTENANCE §2.3 SOP 同步 → 提交推送并看 CI 首跑；CI 绿后
   才切 tag。新增裁决点仍由用户决定。

【行为要点与坑索引（现行行为速查；已提交基线由 verify_selfcomp
  368 项用例/负例机器锁定，不在提示词复述——复现形态查 designs 实施
  修正记录与对应审查报告探针原文；完整坑索引按触发面见
  EXECUTOR_TEMPLATE 第四节）】
- 写 Lom 代码最高频坑（全档 HANDOVER §4/§4.5b/§11.6——写探针/用例前
  过一遍）：布尔字面量大写 True/False；无 [..] List 字面量（用
  range/list_cons）；`not` 不是运算符（用 !x）；match Form B 每臂独立
  end；宿主 dangling-else 贪婪归内（嵌套 if 自带 else）；无
  break/continue（flag 化）；Int 安全值域 ±2^59；无 0x 字面量；多返回
  值调用点显式取 .0/.1；写完用例全文 grep "Fn" 自查；裸 list_empty()
  直接 println 是 ls{?} 编译期拒（加注解/cons 上下文）；无注解构造的
  Bool/嵌套容器迭代显示保守拒。
- **L2 面坑（v1.6.0 起，写 L2 用例/负例前过 l2fix 七族映射）**：L2 中
  println 与全部内建需显式 import（宿主 prelude 恒可用——口径差）；
  管道/解构语句 L2 拒（宿主收）；L2 拒绝输出带 [L2xxx] 码（tools/
  l2fix.py 可跑单文件取码与修复建议）；同文件用户 fn 重名 v1.6.1 起
  单文件模式明确拒（[L2P001]——包展开单元不查，跨包同名走 NAM006
  确定序语义）。
- WASM hex 手写/对拍坑（改 self_comp 才相关）：eqz/sleb 单字节指令族、
  偶字符占位、local 基址、section 序——全档 §11.6。

现在从上手三步开始。先报告只读核验，再按已裁批次推进并呈报；
新方向仍待用户裁决。
```

## 维护者备注

- 上述路径使用 Windows 形态，是本项目当前维护环境。
- 提示词故意不写固定 HEAD SHA；新任必须以 `git log -1`、`git status` 和最新 CI 实查。
- 若交接提交的 CI 首跑不是绿色，先处理 CI，不得把交接状态宣称为完成。
- 交接五件套与分层阅读协议见 MAINTENANCE §5；本文件**【当前真实状态】段
  每轮交接整体重写**（主写位为 TODO 顶部状态块，本段为镜像，两处不一致以
  TODO 为准并随交接批修正）；**三段式结构与铁律镜像段是 MAINTENANCE §6
  用户裁决事项**——改动须经用户裁决，不得随交接静默漂移。
