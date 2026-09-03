# docs/TODO.md — post-1.0 整改待办台账

> **职责**：跨会话的可执行待办唯一事实源。任何会话领任务/交付任务以本文件为准；
> 完成一项就把状态改为 `done` 并附一行证据（命令输出/测试名），由维护会话复核后提交。
> **来源**：独立审查报告 [review-2026-09-03.html](reviews/review-2026-09-03.html)（基线 v1.0.0）
> 的逐条裁决，见文末"驳回/挂起登记"。
> **创建**：2026-09-03（v1.0.0 + 1 docs 提交之后）。
> **纪律**：语言面冻结（LANGUAGE_SPEC §14）不破——只允许新增 warning 级诊断码；
> 其余行为修复均为 bug 修复性质且不动语法/保留字/内建表。

## 执行顺序与依赖

T1（纯文档）→ T2 → T3 → T4 → T5（含两个新 eval 任务，依赖 T2/T4 完成）→ T6 → T7（收尾升版）。
每完成一项：跑全量回归电池（见 T7 后的清单）全绿后才进入下一项。

---

## T1｜P0：SECURITY.md "checked arithmetic" 失实修正（纯文档）

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

## T2｜P1：let 绑定递归闭包的 NAM003 假阳性修复（双侧）

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

## T3｜P1：闭包捕获 mut 绑定的 warning 诊断（新增 MUT002，warning-only 合法）

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

## T4｜P1：浮点 inf/NaN 显示统一（修 bug）+ 大数分歧入清单（只记录）

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

## T5｜eval 新任务 ×2 + 全量计数簿记（依赖 T2/T4）

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

## T6｜P3 杂项清理

- tools/verify_selfhost.py:362-363：mode_run 内旧注释（"宿主 lexer 按字节 Latin-1 展开"）
  与文件头新注释矛盾——改写为"防御网（v1.0.0 修复后正常 0 计数）"口径（审查 P2-3）。
- SECURITY.md:16 "TUnknown tokens" 机制归属写串层（那是自举 mini 语言机制；宿主是
  LexError 容错路径）——改写为宿主实际机制，行为描述不变（审查 P3）。
- SECURITY.md:9 "There is no Cargo.lock dependency surface" 句面易误读——改为
  "the lockfile is in-tree and contains no third-party packages"（审查 P3）。
- README Roadmap Phase 7 行 "eval 113/113×2" 加"（时点值）"可读性标注（审查 P2-3 第三条）。
- 删除工作区杂物 `_t_fib.wasm`（未跟踪、已 gitignore，本地清理即可）。
- README "Phase 1" 段落加半句"（历史里程碑快照，非现状）"（审查 P3，可选项——执行者判断）。

## T7｜收尾：升版 1.1.0

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
