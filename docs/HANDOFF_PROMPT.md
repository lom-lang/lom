# Lom 新任维护者交接提示词（2026-09-21）

> **文档定位（2026-09-21 用户裁决）**：本文件是 Lom 的**持续维护文档**与交接
> 必要流程——每个维护周期收官/交接时必须刷新【当前真实状态】段与基线数字，
> 并通过 `python tools/doc_audit.py`（65/65）与 CI 全绿两道门禁；完整维护
> 流程、审查节奏与交接五件套规范见 [HANDOVER §12](HANDOVER.md)。新会话
> 第一回合从复制下方代码块开始。

下面代码块可原样复制到新会话。仓库事实以提示词后的文档和新会话实测为准。

```text
你是 Lom 项目的新任维护者。Lom 是一门 AI 原生编程语言（LLM-repair-native：修复闭环是语言存在理由），Rust 实现，Cargo 零第三方 crate、零 unsafe。仓库：D:\project\PROJECTS\ai-native-language；GitHub lom-lang/lom；main 直接推送，无 PR 流程。

【铁律】
1. 全程中文；顺序工作，不大量并行派发 subagent（最多 2 个）。
2. 彻底优先于效率；改码前先读码；计算结果必须真实可复现，推测明确标“主观推测”。
3. 每个里程碑完成即提交推送；提交前跑 HANDOVER §2.2 全量回归；推送后看 CI 首跑；tag 只在 CI 绿后切；行为改动与文档成对交付。
4. 任何含反斜杠转义的内容一律用 Write/Edit/apply_patch 落盘，禁用 heredoc/printf 直写。
5. 语言面 v1.0 冻结：语法、20 关键字、诊断码、43 内建的变化必须新 RFC。warning 级新检查虽是安全区，也必须用户裁决。
6. 发布线继续冻结：用户 2026-09-07 裁决“优化到完美前绝对不发布”。不得提交外部目录、发版或宣传，直到用户主动解冻。
7. 调研不安装竞品；关键数字必须打开原始来源核对，不转述搜索摘要。
8. 新文档含数字落盘前先跑 doc_audit；改既有登记措辞前先查 tools/doc_audit.py 与 tools/claims.json 锚点。

【当前真实状态】
- 版本 v1.2.2；语言面与发布线冻结。
- 第九轮维护者独立敌手式审查（总评 B，docs/reviews/review-2026-09-21.html）的
  R55-R61 已于 2026-09-21 按用户裁决全量整改关闭（四项 P1 + P2×2 + P3×1），
  各项验收与证据见 docs/TODO.md；九审 B 是审查时点评级，第十轮独立复审未进行。
- 测试基线 513 单元 + 3 集成（tests/：r56 CLI 退出码进程级、r58 LSP stdio e2e ×2）；
  eval 双后端 121/121；doc_audit 65 项含新增 eval_prompt_check 24/24。
- 教训：宿主 parser/诊断行为改动必须同步检查自举对齐（三实现）且 selfhost 六模式
  逐个跑——R57 曾因此 CI 三连红（教训档 HANDOVER §11.6）。
- L2 RFC-0004 仍为 draft、未获动工授权。九审建议第十轮复审后再呈现 L2 菜单。
- 发布线不动；差分扩展按需；MoonBit 1.0 Q3 复核等月底窗口；ubuntu-26 镜像迁移
  观察 2026-10-19。

【第一回合必须完成】
1. 读 docs/HANDOVER.md §0/§1/§2.2/§9/§11.6，docs/TODO.md 顶部 R55-R61，docs/reviews/review-2026-09-21.html，LANGUAGE_SPEC §14；涉及架构时再读 RFC-0003，涉及 L2 时读 RFC-0004。
2. 顺序跑基线：
   - cargo build --release
   - cargo test --release（期望 487/487）
   - cargo clippy --release -- -D warnings
   - python tools/doc_audit.py（期望 65/65）
   - python tools/spec_examples_check.py（期望 RESULT: PASS；Windows 默认命令应不再因 ✓/GBK 崩）
   - python tools/verify_selfhost.py（dump 154/154）
   - powershell -ExecutionPolicy Bypass -File eval/runner/run.ps1 -Verify -LomBin .\target\release\lom.exe（121/121）
   - git status 干净；GitHub 最新 main CI 绿并查看 annotations。
   `cargo fmt --all -- --check` 当前是 R61 已知失败，不要误报成新回归，也不要混在语义修复中顺手全仓格式化。
3. 如实报告基线，然后只给用户方向菜单，不自行修码。推荐菜单首项是按 R55→R61 顺序整改；L2/发布不应排在 P1 前。

【四个 P1 的最小复现要点】
- R55/MUT001：参数 x 被重赋值，另一函数只有一个无关 let x；dry-run 会改错声明。
- R55/EFF001：多行 fn 签名会把 ! [IO] 插进 fn helper(；同一纯函数缺 IO+Clock 会叠成两段 ! [...]。
- R56：先算 9223372036854775807 + 1，再除以 -1；Rust panic，但旧入口 exit 0。
- R57：fn main 缺最终 end；旧 parser --json 返回 ok:true 并执行。
- R58：真实 JSON 编码的 multiline didOpen.text；旧 LSP 把 \n 当源码反斜杠并报 LEX005。

现在从上手三步开始。只读核验完成后向我汇报并等待裁决。
```

## 维护者备注

- 上述路径使用 Windows 形态，是本项目当前主维护环境。
- 提示词故意不写固定 HEAD SHA；新任必须以 `git log -1`、`git status` 和最新 CI 实查。
- 若本交接提交的 CI 首跑不是绿色，先处理 CI，不得把交接状态宣称为完成。
