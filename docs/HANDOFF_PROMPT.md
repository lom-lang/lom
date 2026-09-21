# Lom 新任维护者交接提示词（2026-09-21）

> **文档定位（2026-09-21 用户裁决）**：本文件是 Lom 的**持续维护文档**与交接
> 必要流程——每个维护周期收官/交接时必须刷新【当前真实状态】
- 版本 v1.2.3（tag 已切，CI 绿）；语言面与发布线冻结。
- 审查状态：十一轮审查，R1-R64 全部关闭、**R65-R72 open**（第十一轮
  独立复审新开：R65/R66 为 P1，R67-R69 为 P2，R70-R72 为 P3——R70 的
  HANDOVER 陈旧句已随交接刷新收口）。事实源 docs/TODO.md 顶部。
- 第十一轮复审（docs/reviews/review-2026-09-21-3.html，总评 B，基线
  65e51dc）确认：R62/R63 整改在自列验收面上零失真；L2.1 双载体 24
  向量 + L2.2 验收 5/5 复证。头条两 P1 已由维护会话亲手复现确认。
- 活跃工作包：无。L2 自举编译器（RFC-0004 方案 A，accepted）：L2.1
  编码器 spike ✓、L2.2 最小子集编译器 ✓（self_comp.lom 2895 行，
  对拍 5/5——但 R66-R68 缺陷在案，整改前 self_comp 不得当可靠编译器
  使用）；十一审建议 L2.3 暂缓，先收口 R65-R72。
- 测试基线 523 单元 + 5 集成（tests/：r56 ×1、r58 套件 ×4）；eval
  双后端 121/121；selfhost 六模式；doc_audit 65/65；spec_examples
  PASS；eval_prompt_check 24/24；cargo fmt --check 零 diff。
- 最新教训（HANDOVER §11.6，本轮新增三条）：块尾裸表达式归 Tail 不
  归 stmts（void 尾副作用曾静默丢失）；Lom 语句位置 match 的 Err 值被
  丢弃（R66 根因——与"裸语句杀尾"同族）；新增 .lom 文件提交前必过
  lom fmt --check（CI fmt gate 覆盖 examples/ 全部）。
- 待用户裁决：R65-R72 整改包（十一审建议 R65/R66-R68 优先，收口后
  升版 v1.2.4）；L2.3 是否继续（建议整改后）；MoonBit 1.0 Q3 复核等
  月底窗口；ubuntu-26 镜像迁移观察 2026-10-19。
- 维护流程/审查节奏/交接五件套规范：HANDOVER §12（2026-09-21 用户裁决
  制度化；本文件是持续维护文档，交接必刷）。

【第一回合必须完成】
1. 读 docs/HANDOVER.md §0/§1/§2.2/§9/§11.6/§12，docs/TODO.md 顶部
   R65-R72，docs/reviews/review-2026-09-21-3.html（十一审），
   LANGUAGE_SPEC §14，docs/rfc/0004-l2-selfhost-compiler.md（L2 进行中）；
   涉及架构时再读 RFC-0003。
2. 顺序跑基线：
   - cargo build --release
   - cargo test --release（期望 523/523；另有集成 cargo test --release --test r56_process --test r58_lsp_process，×5）
   - cargo clippy --release -- -D warnings（零 warning）
   - cargo fmt --all -- --check（零 diff）
   - python tools/doc_audit.py（期望 65/65）
   - python tools/spec_examples_check.py（期望 RESULT: PASS）
   - python tools/eval_prompt_check.py（期望 24/24）
   - python tools/verify_selfhost.py 及 --tokens/--diags/--static/--run/--wasm（六模式逐个，全 PASS）
   - python tools/verify_selfcomp.py（期望 5/5 双产物行为一致）
   - powershell -ExecutionPolicy Bypass -File eval/runner/run.ps1 -Verify -LomBin ./target/release/lom.exe（121/121；WASM 侧加 -Backend wasm 同 121）
   - for f in examples/*.lom examples/bootstrap/*.lom examples/selfhost/*.lom; do ./target/release/lom.exe fmt "$f" --check; done（apply_test 豁免）
   - git status 干净；GitHub 最新 main CI 绿并查看 annotations。
3. 如实报告基线，然后只给用户方向菜单，不自行修码。当前推荐菜单首项是
   R65-R72 整改包（十一审建议：R65 与 R66-R68 优先，收官升版 v1.2.4）；
   L2.3 建议整改后继续（复审裁定暂缓）；发布不应出现在任何菜单项内
   （冻结未解）。

【当前 open 项的最小复现要点（R65/R66 头条 P1）】
- R65/MUT001 错目标+非幂等：外层 fn 内 let x = 1，同函数闭包内
  let x = y + 1 且闭包内 x = x + 1——fix --apply 会把外层声明两轮改成
  let mut mut x（applied=2，PARSE001 损坏源码，ok:false 但文件已坏）。
- R66/self_comp 语句蒸发：含 while/if/for/return 语句的程序经
  lom examples/selfhost/self_comp.lom -- <in> <out.hex> 仍报 COMPILED，
  产出的 wasm 行为错误（如 while 计数宿主输出 3 / L2 输出 0）——
  语句级子集拒绝的 Err 值在语句位置被丢弃（死臂）。

现在从上手三步开始。只读核验完成后向我汇报并等待裁决。
```

## 维护者备注

- 上述路径使用 Windows 形态，是本项目当前主维护环境。
- 提示词故意不写固定 HEAD SHA；新任必须以 `git log -1`、`git status` 和最新 CI 实查。
- 若交接提交的 CI 首跑不是绿色，先处理 CI，不得把交接状态宣称为完成。
- 交接更新规范见 HANDOVER §12.3（五件套）；本文件【当前真实状态】与
  【第一回合必须完成】两段在每次交接时整体重写，铁律段只在数字过时时点改。
