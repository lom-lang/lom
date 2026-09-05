# docs/TODO.md — post-1.0 整改待办台账

> **职责**：跨会话的可执行待办唯一事实源。任何会话领任务/交付任务以本文件为准；
> 完成一项就把状态改为 `done` 并附一行证据（命令输出/测试名），由维护会话复核后提交。
> **来源**：独立审查报告 [review-2026-09-03.html](reviews/review-2026-09-03.html)（基线 v1.0.0）
> 的逐条裁决，见文末"驳回/挂起登记"。
> **创建**：2026-09-03（v1.0.0 + 1 docs 提交之后）。**当前活跃**：第二轮审查工作包 R1-R8（2026-09-05 开立，见下）；T1-T7 已全部关闭（档案保留下文）。
> **纪律**：语言面冻结（LANGUAGE_SPEC §14）不破——只允许新增 warning 级诊断码；
> 其余行为修复均为 bug 修复性质且不动语法/保留字/内建表。

## 第二轮审查工作包（2026-09-05 开立，活跃）

**来源**：第二轮独立审查 [review-2026-09-05.html](reviews/review-2026-09-05.html)（总评 A-，无 P0；基线 470670d / v1.1.0）。
**验证状态**：12 条发现已由维护会话逐条亲手复现（两条行为性发现实测：`type UserId = Int` → PARSE001、`from math import {sin}` → RUNTIME005；十条文档定位 grep/wc 证实），零失实，全部采纳。交接关键项（HANDOVER §9/§2.2 的 456/1.1.0/149、§9-5 的 5703、guide 两条目补录）已由维护会话即时修复；其余按 R1-R6 执行，R7/R8 为采纳审查结构性建议的加强项。
**通用纪律**：顺序执行（R1→R8）；每项完成在本文件标 done 附一行证据并跑全量回归电池（同 T7 清单）；语言面冻结不破；**无需升版**（纯文档 + 仓库工具 + CI 配置，无 CLI 面变化）。

### R1｜P1：SPEC_FOR_AI §5 Type alias 假特性清除

- 事实：SPEC_FOR_AI.md:166-170 存在 "Type alias (Phase 2)" 小节，把从未实现的 `type UserId = Int` 当特性教——实测 PARSE001；LANGUAGE_SPEC:235 已标 never implemented、§14 冻结非目标重申 no type aliases；教程 :744 还宣称此类谎言已清扫（清在了人读的 spec，漏了 LLM 读的这份）。
- 范围：删除该小节，或（推荐）改写为负面示例——"**不要写** `type X = Y`（未实现且 v1.x 冻结非目标，实测 PARSE001；类型语义用结构类型直接表达）"。修后教程 :744 的宣称恢复成立。
- 验收：SPEC_FOR_AI 中 `type` 别名只以反知识形态出现；R7 工具上线后 §5 零失败示例。

### R2｜P2/P3：SPEC_FOR_AI 其余三处内容修正

- :15 核心规则 6 示例 `from math import {sin, cos}` → 改为真实符号（如 `{sqrt, abs}`；math 仅导出 sqrt/abs/min/max，§11b/§8 已是正确口径）——实测 `sin` 报 RUNTIME005；
- :363 §10-4 "Reassigning immutable … is an error" → 改为 MUT001 **warning** 口径（实测 `let x = 3; x = 4` 照常运行输出 4、退出 0；LANGUAGE_SPEC §5.1 已准确）；
- :331-340 pub 示例块加 ❌/反例标记（文字已声明 rejected，但代码块无标记、照抄即 PARSE001——对齐 §10 错误示例的排版风格）。

### R3｜P2：LANGUAGE_SPEC §12 同节自相矛盾旧计数

- :1308 "`10_error_repair.json` (15 tasks, 15% …)" → 20；:1312 "Reference solutions: 100/100 pass" → 116/116（§12.1/§12.3 同节已写正确值）。

### R4｜P2：eval/README.md 三处陈旧

- :97 "Error-repair category (15)" → (20)；:56 "001-115" → 001-117；:23-24 布局注释 03_types=11/04_closures=12 → 13/13（合计与 116 对齐）。

### R5｜P3：README 两处

- :66 文件拆分 "3 in the pkg_demo package" → 2（main.lom + mathlib/math.lom），并补 selfhost 1（29+4+2+1=36，总数原本碰巧正确）；
- 状态横幅（:7，保持 FROZEN 事实）追加一行 v1.1.0 整改说明（MUT002 warning / NAM003 递归闭包假阳性修复 / inf-NaN 显示统一三项用户可见变更；全文现无任何 1.1.0 痕迹）。

### R6｜P2（半）：ci.yml 步骤名自举计数

- .github/workflows/ci.yml:116 "Selfhost dump (147 文件逐字, Phase 8.1)" → 149（HANDOVER §2.2 的另一半已由维护会话即时修复）。

### R7｜加强：SPEC_FOR_AI 代码示例实测对账工具（审查发现二的建议）

- 事实：历次文档清扫以 LANGUAGE_SPEC/README 为主战场，SPEC_FOR_AI 只在评审点名时被扫——本次 P1 正是漏网结果。
- 范围：新 tools/spec_examples_check.py（零第三方依赖）——抽取 SPEC_FOR_AI 的 fenced code 块逐个写临时 .lom 跑 `lom --check`；反例（❌/「不要写」标注的块）断言其确实产诊断，正例断言零 Error。存量反例先手工标注（R1/R2 完成后应只剩 pub 一处）。接入 CI（可与 R8 同一 step）。
- 验收：脚本对全量示例跑通且分类全符合；R1/R2 的修复被工具锁定（故意改回假特性能红）。

### R8｜加强：文档数字自动对账 gate（审查发现三的建议——根治第三次复发）

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
