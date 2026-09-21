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
- 版本 v1.2.3（tag 于 CI 绿后切）；语言面与发布线冻结。
- 审查状态：R1-R64 全部关闭，open 项为零。第十轮独立复审
  （docs/reviews/review-2026-09-21-2.html，总评 B+，基线 v1.2.2）确认
  九审整改零失真；其新开 R62/R63 已于 v1.2.3 整改关闭（事实源
  docs/TODO.md 顶部）；R62/R63 整改后状态需下一轮独立复审重估，
  不得自行宣布评级回升。
- 活跃工作包：L2 自举编译器（用户 2026-09-21 裁决"执行 1，2"动工）。
  RFC-0004 方案 A：hex 文本落盘 + 宿主 ~40 行解码桩（零语言面，
  43 内建不动）；方案 B（file_write_bytes 44 号内建）属解冻菜单不作为
  起步。阶段：L2.1 编码器 spike（LEB128 除模 / f64 位模式纯算术拆解 /
  hex 发射通道三风险点先证，spike 失败即回 RFC 重议）→ L2.2 最小子集
  （fn/let/算术/println，与宿主双产物行为级对拍）→ L2.3 全语言面 →
  L2.4 自举闭环（三层自证，受 V8 栈深限制时按子集口径如实降级）。
  交付物：examples/selfhost/self_comp.lom + tools/verify_selfcomp.py +
  宿主解码桩。RFC-0004 状态需随动工补修订（draft→accepted + 方案 A 选定）。
- 测试基线 523 单元 + 5 集成（tests/：r56 CLI 退出码进程级 ×1、
  r58 LSP stdio e2e ×4——含 R63 传输边界对：超限 Content-Length 不
  abort + 畸形 JSON 回 -32700/-32600 后存活）；eval 双后端 121/121；
  selfhost 六模式；doc_audit 65/65；spec_examples PASS；
  eval_prompt_check 24/24；cargo fmt --check 零 diff。
- 两条最新教训（HANDOVER §11.6）：宿主 parser/诊断行为改动必须同步检查
  三实现（宿主/WASM/自举）且六模式逐个跑（R57 整改曾漏 --diags 致 CI 三连红）；
  含反斜杠路径的文档内容禁用 python 字符串直写（R64 曾致 §2.2 命令行坏字节，
  R15 同型第四次）。
- 待办窗口：MoonBit 1.0 Q3 复核等月底窗口；ubuntu-26 镜像迁移观察
  2026-10-19；doc_audit 命令行完整性锚（R64 根治方向，待下轮 gate 扩展）。
- 维护流程/审查节奏/交接五件套规范：HANDOVER §12（2026-09-21 用户裁决
  制度化；本文件是持续维护文档，交接必刷）。

【第一回合必须完成】
1. 读 docs/HANDOVER.md §0/§1/§2.2/§9/§11.6/§12，docs/TODO.md 顶部交接声明，
   docs/reviews/review-2026-09-21-2.html（十审）与 review-2026-09-21.html（九审），
   LANGUAGE_SPEC §14 与 docs/rfc/0004-l2-selfhost-compiler.md（L2 进行中，
   方案 A 已定）；涉及架构时再读 RFC-0003。
2. 顺序跑基线：
   - cargo build --release
   - cargo test --release（期望 523/523；另有集成 cargo test --release --test r56_process --test r58_lsp_process，×5）
   - cargo clippy --release -- -D warnings（零 warning）
   - cargo fmt --all -- --check（零 diff——R61 起机械格式化；若 rustfmt 版本更替出现新 diff，单独机械包处理，不混语义修复）
   - python tools/doc_audit.py（期望 65/65）
   - python tools/spec_examples_check.py（期望 RESULT: PASS）
   - python tools/eval_prompt_check.py（期望 24/24）
   - python tools/verify_selfhost.py 及 --tokens/--diags/--static/--run/--wasm（六模式逐个，全 PASS）
   - powershell -ExecutionPolicy Bypass -File eval/runner/run.ps1 -Verify -LomBin ./target/release/lom.exe（121/121；WASM 侧加 -Backend wasm 同 121）
   - git status 干净；GitHub 最新 main CI 绿并查看 annotations。
3. 如实报告基线，然后只给用户方向菜单，不自行修码。当前活跃工作包是
   L2（L2.1 spike 起步或按进行度继续）；菜单其余项：下一轮独立复审
   （R62/R63 整改后重估）、MoonBit 月底窗口、ubuntu-26 迁移观察；
   发布不应出现在任何菜单项内（冻结未解）。

【v1.2.3 修复要点（供复审探针参考）】
- R62/闭包遮蔽：外层参数 x 重赋值 + 函数内闭包 let x 遮蔽——v1.2.2 会把
  闭包内声明错改 let mut（ok:true）；v1.2.3 结构化定位后 applied=0、
  源码逐字不变、ok:false、参数 hint。
- R62/行内注释：x = x + 1 # let x = 0——注释文本不再被改写（同上四点验收）。
- R63：向 lom lsp 发 Content-Length: 999999999999 的 header——v1.2.2 为
  memory allocation failed abort（rc=0xC0000409）；v1.2.3 显式 exit 1 +
  stderr 说明。畸形 JSON payload 收到 -32700/-32600（id:null），服务器存活。

现在从上手三步开始。只读核验完成后向我汇报并等待裁决。
```

## 维护者备注

- 上述路径使用 Windows 形态，是本项目当前主维护环境。
- 提示词故意不写固定 HEAD SHA；新任必须以 `git log -1`、`git status` 和最新 CI 实查。
- 若交接提交的 CI 首跑不是绿色，先处理 CI，不得把交接状态宣称为完成。
- 交接更新规范见 HANDOVER §12.3（五件套）；本文件【当前真实状态】与
  【第一回合必须完成】两段在每次交接时整体重写，铁律段只在数字过时时点改。
