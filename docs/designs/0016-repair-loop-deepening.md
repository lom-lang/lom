# repair 闭环深化首批设计（fix 分派表补齐 + SPEC_FOR_AI 欠账 + fix_corpus/error_repair 扩容 + L2 子集边界修复题）

- **状态**：设计方案产出（动工前置，待用户裁决三个裁决点）。用户已裁方向 ①（2026-10-01"按建议执行"）；复审时机已裁（二十五审与本批合并发起）。
- **设计基线**：HEAD `dba69d6` / v1.4.15。语言面 v1.0 冻结与外部发布线继续冻结；本批全部为宿主 fix/文档/评测资产面（零语言面变化——NAM006/MUT002 既有码仅补 fix 处理与文档，不新增诊断码）。
- **供料来源**：执行者供料报告五节（fix 引擎全图 / error_repair 24 题 / L2 诊断面 / SPEC_FOR_AI 修复面 / manifest 与 fix_corpus）+ 规划者亲读（fix_for_diagnostic 分派表 L754-854、fix_mut001 动作族、cargo fix_corpus_end_to_end 消费面）+ 规划者补核（SPEC_FOR_AI L187/L547/L675-677 码表与修复表现值）。
- **数字落盘前门禁**：写本文前后运行 `python tools/doc_audit.py`。

## 1. 供料结论（浓缩；关键行号为 v1.4.15 时点）

1. **fix 分派表空位**：`fix_for_diagnostic`（src/fix.rs L754-854）无 NAM006/MUT002 条目——两者均落 `_ =>` "未知错误码，参考诊断消息手动修复"（Low）。NAM006 是 typechecker 实际产出的 warning（v1.4.10 designs/0014 加入，import 别名/包符号撞本地定义——本地赢、行为确定）；MUT002 是 v1.1.0 加入的 warning（闭包捕获 mut，解释器共享作用域 vs WASM 值拷贝双后端分歧旗标，SPEC_FOR_AI L549/L705 已登记语义）。**两者都是"实际产出但 fix 面兜底误导"的欠账**。
2. **PKG007 不进 fix 面**：PKG 族诊断（PKG001-007）不进 `Diagnostics` 管线（package.rs 的 PkgError/eprint 直报），fix 引擎结构性接触不到——登记不修。
3. **SPEC_FOR_AI 欠账**：§11c 码表（L547）NAM 族止于 NAM005，**NAM006 缺**；§11e 修复表（L664-678）**NAM006/MUT002 双缺**。PKG 族整体不在 SPEC_FOR_AI（历史口径——该文档面向单文件编码任务，不翻）。
4. **error_repair 24 题覆盖缺口**：无 NAM006/MUT002/TYPE001/NAM002 形态题；无任何 L2 面。runner 验证流 = 宿主 stdout 逐字比对 + rc==0（无"诊断清零/L2 编译"判据）；任务生成纪律（R60/③ 包）= **prompt 内嵌诊断必须从真实 `--check --json` 输出采集，不得手写**。
5. **L2 诊断面现状**：self_comp 的 `subset_err`（L2481-2483，264 处调用）+ 40 处裸 `Err(...)` 输出 `codegen error: <文案>` 纯文本——**无码、无行号、无 span、COMPILE-ERROR 恒 rc=0**；fix 引擎吃 L2 诊断缺三层前置件（结构化出口/文案解析器/宿主-L2 拒绝面不对齐）——**形态"fix 引擎直接吃 L2 诊断"本批不做**（需先给 L2 诊断结构化，属后续独立批）。161 负例的拒绝文案聚类 19+ 族（145 枚锁定文案）是现成的"L2 子集边界知识库"。
6. **L2 拒绝 × 宿主实跑对比**（供料三例 + 补一例）：宿主 warning 过/L2 硬拒（Map 算术、pipe 语法）；宿主 NAM005 warning/L2 裸文案拒（未导入内建）；双侧都拒但文案/退出码全不同（println 重名）。**"宿主可跑、L2 拒"的形态真实存在且成族**——是 L2 面修复题的天然素材。
7. **fix_corpus 消费面**：11 对 `.bad/.fixed`，cargo 单测 `fix_corpus_end_to_end` 逐字比对（04 的 fixed==bad 锁定 Medium 不自动改）；`*.bad.lom` 同时被 verify_selfhost 当坏文件集（L202-203/265）——**新增对需验证自举对拍兼容**（NAM006/MUT002 不在自举 8.2 检查器四码面内，预期自举侧"干净"，以 verify_selfhost 实跑为准）。

## 2. 设计（四子面 + 顺带，批次 v1.5.0 候选）

### A. fix 分派表补齐（src/fix.rs，+2 条目）

- **NAM006** → 认知型 hint（Medium）：`撞名告知型 warning——行为已确定（本地定义优先 / 同别名取后写声明 / 两包同名按包根路径序取后者），无需修复；如需消除歧义可重命名本地定义或 import 别名`。**不是修复动作**：NAM006 语义是撞名告知（行为正确），自动改名/删 import 都属语义越权。
- **MUT002** → hint（Medium）：`闭包捕获了 mut 绑定——解释器（共享作用域）与 WASM（创建时值拷贝）行为不同；修复：不依赖捕获的 mut 状态（先 let 局部副本再捕获，或改用函数参数传递新值）`——对齐 SPEC_FOR_AI L705 既有语义描述。
- 两者均 hint 级、永不自动 apply（--apply 零改动）。单元测试：两形态 `fix --json` 输出新 hint 不再"未知错误码"；负向：`--apply` 后源码逐字不变、applied=0。
- 不动：NAM005/EFF001/MUT001 等既有条目；PKG007（结构性不可达，登记）。

### B. SPEC_FOR_AI 欠账补齐（纯文档）

- §11c 码表 NAM 族补 NAM006（v1.4.10，warning——import alias/包符号撞本地定义，本地赢，行为确定，无需修复）；§11e 修复表补两行：NAM006 → 认知 hint（medium）/ MUT002 → hint（medium）。
- I 类尺寸锚同步（doc_audit）；PKG 族不进（既有口径，设计登记）。

### C. fix_corpus 扩容（+2 对）

- `12_nam006_alias_clash`（import 别名撞本地 fn，--check 出 NAM006）：fixed==bad 锁定（沿 04 先例——认知 warning 不自动改）。
- `13_mut002_closure_capture`（闭包捕获 mut，出 MUT002）：fixed==bad 同款锁定。
- 实施须实跑 verify_selfhost 六模式确认自举坏文件集兼容（供料结论 7 的风险点）；cargo fix_corpus 测试自动覆盖新对。

### D. error_repair 任务扩展（+6~8 题，全部从真实输出生成）

- **宿主面新题（4~5 题）**：NAM006 消歧题（重命名使 warning 清零且行为不变）/ MUT002 改写题（捕获 mut 改为参数或副本传递，两后端行为一致化）/ TYPE001 注解不符 / NAM002 真重复定义（error 级形态补缺）。
- **L2 面新形态题（2~3 题，首批核心新意）**："宿主可跑、L2 拒"的代码（如 Map 参与 ==/!= 外算术、pipe 语法、未导入内建的裸引用），任务 = 修复为**L2 子集内且宿主行为不变**。prompt 内嵌：宿主真实 `--check --json`（warning 形态如实）+ **L2 真实 `codegen error` 文本**（从 self_comp 实跑采集，作为"L2 编译器诊断"信息段）——双真实采集守住 R60 纪律。runner 零改动（验宿主 stdout+rc）；**参考解额外经 L2 全链验证**（self_comp → hex2wasm → run_selfcomp 实跑通过）作为任务生成侧纪律写入 eval/README（L2 判据不进 runner，理由：L2 COMPILE-ERROR 恒 rc=0、串 python/node 工具链超 runner 现面）。
- manifest task_count、README 计数、prompts 重生成（_generate.ps1）、eval ID 唯一性测试同步；L2 面任务的 expected 全部实跑核定。

### E. 顺带（方向 ③，用户已裁"随批顺带"）

- 发布解冻条件盘点：TODO 新增一节（不发布、不改代码）——列"优化到完美"的可验收项检查单草案（复审轨迹/已知边界清单/评测覆盖/pass@k 基线/文档门禁），作为将来解冻决策的输入。纯登记，不设完成时点。

## 3. 冻结面与边界

- 语言面零变化（既有码补处理，不新增码/不改文案语义）；--apply 行为零变化（新条目均 hint 级）；L2 面（self_comp/codegen）零改动——quine 269555 与 verify_selfcomp 365/365 构造性不变。
- runner（run.ps1/run.sh）零改动；eval 既有 24 题不动（089/090 历史 caveat 维持 R60 登记口径）。
- 不做：fix 引擎吃 L2 诊断（三层前置件缺失，后续独立批候选）；PKG007 进 fix/SPEC_FOR_AI（结构性不可达/既有口径）。

## 4. 验收门禁（实施后全量）

- cargo test（新单元 +2~4，计数实算）+ fix_corpus 新对逐字比对 + clippy/fmt 零。
- 新任务全链：每题参考解宿主双后端实跑 expected 吻合；L2 面题参考解经 self_comp→hex→wasm 实跑通过；`run.ps1 -Verify` 双后端 121+121 → 新计数全过；prompt 内嵌诊断与当前编译器实跑逐字一致（eval_prompt_check 若涉及则同步）。
- 全量回归：六模式 / verify_selfcomp 365/365 / --bootstrap 14/14 quine 269555 / doc_audit（锚同步后，I 类尺寸与任务数锚）/ spec_examples / fmt gates。
- 升版与门禁：裁决点 3 定版本号；提交推送看 CI 首跑，绿后切 tag；RFC-0004 修订 49 登记 + SPEC_FOR_AI/README/TODO/HANDOVER/HANDOFF_PROMPT/positioning 文档成对。

## 5. 裁决点（呈用户）

1. **批次范围**：甲 = A+B+C+D+E 全包【建议】/ 乙 = A+B+C 宿主面先做（D/E 下批）/ 丙 = 仅 A+B 最小欠账。
2. **L2 面任务形态**（D 内）：甲 = 宿主 runner 验证 + 参考解 L2 全链纪律锁定【建议——runner 零改动、R60 纪律可守】/ 乙 = runner 集成 L2 编译判据（重，不建议首批）。
3. **升版**：minor **v1.5.0**【建议——fix 面新条目 + 评测任务集扩容均为用户可见功能，"加了用户可见功能就升 minor"§10 教训】/ patch v1.4.16。
