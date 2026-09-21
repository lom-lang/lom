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
- 版本 v1.2.2（tag 已切，CI 绿）；语言面与发布线冻结。
- 审查状态：R1-R61 全部关闭。第九轮（B）整改经第十轮独立复审
  （docs/reviews/review-2026-09-21-2.html，总评 B+，基线 v1.2.2）确认
  七项验收零失真；复审新开 R62（P2）/R63（P3）/R64（P3，已开账即修），
  当前 open 仅 R62/R63，事实源 docs/TODO.md 顶部。
- 测试基线 513 单元 + 3 集成（tests/：r56 CLI 退出码进程级、r58 LSP stdio
  e2e ×2）；eval 双后端 121/121；selfhost 六模式；doc_audit 65/65；
  spec_examples PASS；eval_prompt_check 24/24；cargo fmt --check 零 diff。
- 两条最新教训（HANDOVER §11.6）：宿主 parser/诊断行为改动必须同步检查
  三实现（宿主/WASM/自举）且六模式逐个跑（R57 整改曾漏 --diags 致 CI 三连红）；
  含反斜杠路径的文档内容禁用 python 字符串直写（R64 曾致 §2.2 命令行坏字节，
  R15 同型第四次）。
- 待用户裁决：R62/R63 整改包（十审建议 R62 前置/并行、R63 顺手收口）；
  L2 动工菜单（十审裁定九审条件已满足，可重新呈现）；MoonBit 1.0 Q3 复核
  等月底窗口；ubuntu-26 镜像迁移观察 2026-10-19。
- 维护流程/审查节奏/交接五件套规范：HANDOVER §12（2026-09-21 用户裁决
  制度化；本文件是持续维护文档，交接必刷）。

【第一回合必须完成】
1. 读 docs/HANDOVER.md §0/§1/§2.2/§9/§11.6/§12，docs/TODO.md 顶部 R62-R63，
   docs/reviews/review-2026-09-21-2.html（十审）与 review-2026-09-21.html（九审），
   LANGUAGE_SPEC §14；涉及架构时再读 RFC-0003，涉及 L2 时读 RFC-0004。
2. 顺序跑基线：
   - cargo build --release
   - cargo test --release（期望 513/513；另有集成 cargo test --release --test r56_process --test r58_lsp_process，×3）
   - cargo clippy --release -- -D warnings（零 warning）
   - cargo fmt --all -- --check（零 diff——R61 起机械格式化；若 rustfmt 版本更替出现新 diff，单独机械包处理，不混语义修复）
   - python tools/doc_audit.py（期望 65/65）
   - python tools/spec_examples_check.py（期望 RESULT: PASS）
   - python tools/eval_prompt_check.py（期望 24/24）
   - python tools/verify_selfhost.py 及 --tokens/--diags/--static/--run/--wasm（六模式逐个，全 PASS）
   - powershell -ExecutionPolicy Bypass -File eval/runner/run.ps1 -Verify -LomBin ./target/release/lom.exe（121/121；WASM 侧加 -Backend wasm 同 121）
   - git status 干净；GitHub 最新 main CI 绿并查看 annotations。
3. 如实报告基线，然后只给用户方向菜单，不自行修码。当前推荐菜单首项是 R62/R63
   整改包；L2 动工菜单可呈现（十审裁定条件已满足，建议 R62 前置/并行）；
   发布不应出现在任何菜单项内（冻结未解）。

【当前 open 项（R62/R63）的最小复现要点】
- R62/闭包遮蔽：外层参数 x 重赋值 + 函数内闭包 let x = 10 遮蔽；apply 会把闭包内
  声明改成 let mut（原诊断未治、final 仍 1 warning 而 ok:true）。
- R62/行内注释：x = x + 1 # let x = 0；注释文本被当声明命中改写。
- R63：向 lom lsp 发 Content-Length: 999999999999 的 header 即
  memory allocation failed abort（rc=0xC0000409）；畸形 JSON payload 被静默
  丢弃（无 -32700 响应）。

现在从上手三步开始。只读核验完成后向我汇报并等待裁决。
```

## 维护者备注

- 上述路径使用 Windows 形态，是本项目当前主维护环境。
- 提示词故意不写固定 HEAD SHA；新任必须以 `git log -1`、`git status` 和最新 CI 实查。
- 若交接提交的 CI 首跑不是绿色，先处理 CI，不得把交接状态宣称为完成。
- 交接更新规范见 HANDOVER §12.3（五件套）；本文件【当前真实状态】与
  【第一回合必须完成】两段在每次交接时整体重写，铁律段只在数字过时时点改。
