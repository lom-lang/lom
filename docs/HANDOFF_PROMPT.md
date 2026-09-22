# Lom 新任维护者交接提示词（2026-09-22）

> **文档定位（2026-09-21 用户裁决）**：本文件是 Lom 的**持续维护文档**与交接
> 必要流程——每个维护周期收官/交接时必须刷新【当前真实状态】段与基线数字，
> 并通过 `python tools/doc_audit.py`（67/67）与 CI 全绿两道门禁；完整维护
> 流程、审查节奏与交接五件套规范见 [HANDOVER §12](HANDOVER.md)。新会话
> 第一回合从复制下方代码块开始。

下面代码块可原样复制到新会话。仓库事实以提示词后的文档和新会话实测为准。

```text
你是 Lom 项目的新任维护者。Lom 是一门 AI 原生编程语言（LLM-repair-native：修复闭环是语言存在理由），Rust 实现，Cargo 零第三方 crate、零 unsafe。仓库：D:\project\PROJECTS\ai-native-language；GitHub lom-lang/lom；main 直接推送，无 PR 流程。

【铁律】
1. 全程中文；顺序工作，不大量并行派发 subagent（最多 2 个）。
2. 彻底优先于效率；改码前先读码；计算结果必须真实可复现，推测明确标"主观推测"。
3. 每个里程碑完成即提交推送；提交前跑 HANDOVER §2.2 全量回归；推送后看 CI 首跑；tag 只在 CI 绿后切；行为改动与文档成对交付。
4. 任何含反斜杠转义的内容一律用 Write/Edit/apply_patch 落盘，禁用 heredoc/printf 直写。
5. 语言面 v1.0 冻结：语法、20 关键字、诊断码、43 内建的变化必须新 RFC。warning 级新检查虽是安全区，也必须用户裁决。
6. 发布线继续冻结：用户 2026-09-07 裁决"优化到完美前绝对不发布"。不得提交外部目录、发版或宣传，直到用户主动解冻。
7. 调研不安装竞品；关键数字必须打开原始来源核对，不转述搜索摘要。
8. 新文档含数字落盘前先跑 doc_audit；改既有登记措辞前先查 tools/doc_audit.py 与 tools/claims.json 锚点。

【当前真实状态】
- 版本 v1.2.5（tag 已切，CI 绿）；语言面与发布线冻结。
- 审查状态：十三轮审查，R1-R78 全部关闭（R78 README 存量算术残留已随
  L2.3-a 包顺手收口）。第十三轮复审
  （docs/reviews/review-2026-09-22-2.html，总评 **B+ 回升**，基线
  1418536/v1.2.5）确认 R73-R76 整改零失真（✓×4）+ 代码面敌手探针
  22 形态零击穿（九审以来五轮首次）。事实源 docs/TODO.md 顶部。
- 活跃工作包：**L2.3 进行中**（用户已裁决"执行"动工；按批交付——a 批
  已收官，后续批次排队）。L2 自举编译器（RFC-0004 方案 A，accepted）：L2.1
  spike + L2.2 子集编译器 + R66-R68/R74 整改 + **L2.3-a 控制流批**均完成
  （self_comp.lom 4665 行，verify_selfcomp 40/40 = 17 对拍 + 23 负例拒绝）。
  后续批次：闭包与捕获（需堆/env 值表示，先出设计方案再动工）/ enum 与
  match / String / List / Map / json / 包 / return 语句（块深度跟踪）。
- 测试基线 533 单元 + 8 集成（tests/：r56 ×1、r58 套件 ×7）；eval
  双后端 121/121；selfhost 六模式；doc_audit 67/67；spec_examples
  PASS；eval_prompt_check 24/24；cargo fmt --check 零 diff。
- 最新教训（HANDOVER §11.6）：块尾裸表达式归 Tail 不归 stmts；Lom 语句
  位置 match 的 Err 值被丢弃（R66 根因——修法是值线程化 let frag =
  match ... end + frag?）；Lom 侧 Result 消费必带 ? 解包；Form B 臂 end
  计数再应验；新增 .lom 文件提交前必过 lom fmt --check；fix 防护设计
  要以"apply 全轮次"为界（R73——同轮多诊断同坐标不去重）；含反斜杠
  转义的批量文本替换禁用 heredoc（本轮 doc_audit 修改再应验一次，改用
  Edit 落盘解决）。
- 待用户裁决：L2.3 后续批次推进节奏（控制流批已交付）；typechecker
  for 变量 define 覆盖同名外层可变性标记不恢复的
  既有 quirk 是否立项（TODO R65 证据区）；MoonBit 1.0 Q3 复核等月底
  窗口；ubuntu-26 镜像迁移观察 2026-10-19。
- 维护流程/审查节奏/交接五件套规范：HANDOVER §12（2026-09-21 用户裁决
  制度化；本文件是持续维护文档，交接必刷）。

【第一回合必须完成】
1. 读 docs/HANDOVER.md §0/§1/§2.2/§9/§11.6/§12，docs/TODO.md 顶部，
   docs/reviews/review-2026-09-22-2.html（十三审——最新轮），
   LANGUAGE_SPEC §14，docs/rfc/0004-l2-selfhost-compiler.md（L2 进行中，
   修订 1-6）；涉及架构时再读 RFC-0003。
2. 顺序跑基线：
   - cargo build --release
   - cargo test --release（期望 533/533；另有集成 cargo test --release --test r56_process --test r58_lsp_process，×8）
   - cargo clippy --release -- -D warnings（零 warning）
   - cargo fmt --all -- --check（零 diff）
   - python tools/doc_audit.py（期望 67/67）
   - python tools/spec_examples_check.py（期望 RESULT: PASS）
   - python tools/eval_prompt_check.py（期望 24/24）
   - python tools/verify_selfhost.py 及 --tokens/--diags/--static/--run/--wasm（六模式逐个，全 PASS）
   - python tools/verify_selfcomp.py（期望 25/25 = 10 用例双产物行为一致 + 15 负例拒绝）
   - powershell -ExecutionPolicy Bypass -File eval/runner/run.ps1 -Verify -LomBin ./target/release/lom.exe（121/121；WASM 侧加 -Backend wasm 同 121）
   - for f in examples/*.lom examples/bootstrap/*.lom examples/selfhost/*.lom; do ./target/release/lom.exe fmt "$f" --check; done（apply_test 豁免）
   - git status 干净；GitHub 最新 main CI 绿并查看 annotations。
3. 如实报告基线，然后只给用户方向菜单，不自行修码。当前推荐菜单首项是
   L2.3 后续批次（闭包与捕获 / enum 与 match / String / List / Map /
   json / 包，按"编译期校验 + 负例"成对交付）；发布不应出现在任何菜单
   项内（冻结未解）。

【状态锚点（当前行为，供复核）】
- R73 修复后：同一声明多条 MUT001 诊断单次 apply 只应用一次等价
  Replace（x=2; x=3 → applied=1 产合法 let mut x、final 净 ok:true）；
  同位置不同文本动作不合并。
- R74 修复后：self_comp 调用点类型/arity 不符即 COMPILE-ERROR 无 hex
  （f(1) 传 Float 形参 → 第 1 参类型不符；add(1,2,3) → 实参数不符）。
- R75 修复后：解构遮蔽的重赋值 → hint 不动源码；解构在赋值之后的
  正例仍 High Replace。
- L2.3-a 后：if/while/for(Int)/Bool/比较/逻辑短路/块尾 if 表达式可编译
  （verify_selfcomp 25/25）；return 语句/match/解构/闭包/String/List 仍
  COMPILE-ERROR（后续批次）；for 迭代仅 Int。
- 写 self_comp 代码的两大坑：宿主 dangling-else 贪婪归内（then 块首
  元素嵌套 if 时外层 else 被内层吞——嵌套 if 需自带 else 或提前 return
  改写）；Form B 臂 end 计数（宿主语义：每臂独立 end）。

现在从上手三步开始。只读核验完成后向我汇报并等待裁决。
```

## 维护者备注

- 上述路径使用 Windows 形态，是本项目当前主维护环境。
- 提示词故意不写固定 HEAD SHA；新任必须以 `git log -1`、`git status` 和最新 CI 实查。
- 若交接提交的 CI 首跑不是绿色，先处理 CI，不得把交接状态宣称为完成。
- 交接更新规范见 HANDOVER §12.3（五件套）；本文件【当前真实状态】与
  【第一回合必须完成】两段在每次交接时整体重写，铁律段只在数字过时时点改。
