# L2 码进修复闭环通道设计（第二前置件：l2fix 工具）

- **状态**：**已交付（2026-10-02，RFC-0004 修订 53；提交 `d665d11` CI run `36891640634` #253 六 job 绿；纯工具批不升版无 tag）**——四点均按甲。执行者实施（377 行，含 44 名映射表与七族完整文案）+ 规划者验收（l2fix 三题亲跑、--self-check 6/6 亲跑、映射表头登记抽读）。
- **设计基线**：HEAD `384c6f8` / v1.6.0。语言面 v1.0 冻结与发布线继续冻结；**裁决点 1 甲形态下零宿主 src/ 改动**。
- **供料来源**：执行者供料六节（链路成本实测/fix 入口结构/七族码建议映射/三题联动/通道形态对比/影响面）+ 规划者亲读（run_fix L755 管线、generate_plan L590 签名——Diagnostic 全 pub 字段可直接流过 fix 链，未知码落 fallback）。
- **数字落盘前门禁**：写本文前后运行 `python tools/doc_audit.py`。

## 1. 供料结论（浓缩）

1. **链路成本可用（实测）**：宿主跑 self_comp 编译单文件——eval 规模（12-74 行）**0.2-0.45s**、千行级 2.5-8.5s、自举规模 142s（与闭环无关）。COMPILE-ERROR 恒 rc=0，判定靠 stdout 码行。
2. **七族码建议面（141 文案提炼）**：无行号硬约束下——**唯一高置信度可 apply 动作 = L2U×名字→模块表**（文案 `未定义变量 'len'` 提取名字 ∈ module_of 八模块约 30 名 → Insert import @1:1，NAM005 同构、位置无关；名字不在表 → 盲补是错的，降 Low）；L2P/L2T/LC → Medium hint（文案可机提取重名标识符/期望实得类型与数量；"缺 main"可给骨架 hint-with-text）；L2S/L2V → 族级静态模板 hint（文案多自带方向；127 正解 map_get 不在文案需模板携带；**128"该表达式形态"零细节——模板需列常见触发物（管道等）**）；L2E → 独立"内部错误请报维护者"（非用户错误）。**注意 io 口径差**：L2 中 println 需显式 import（宿主 prelude 恒可用），L2U 表须含 io。
3. **三题联动（实测+手推）**：127 宿主仅 TYPE001 Low hint（l2fix 模板有增量）；**128 宿主 fix 零诊断零 plans——l2fix 是唯一指导源（通道价值最强论据与建议质量最弱点并存）**；129 宿主 NAM005 已能 High 修（l2fix 等价重复覆盖）。
4. **通道形态约束**：甲（宿主 CLI 子命令）要动 src/、处置 Stage 枚举/lom-diag schema 值域（L2 诊断借 Stage::Type 不准确、扩枚举是 schema 变更）、部署耦合 self_comp.lom 路径（发布形态 examples/ 不随行）、生产路径无子进程先例；乙（tools/ 包装）零宿主改动、同形态探针 collect_errors.py 已验证（git-ignore 可照搬入库）、需自带映射表（双源，内建冻结漂移风险低）。
5. **影响面**：L2 码进 fix.rs 分派与既定裁决一致（L2 工具码非宿主面，§14 不涉）；run_fix 只跑宿主管线不会自发产 L2 诊断（无冲突面）；apply 层只应用 High 非 hint——L2 hint 永不自动改写。

## 2. 设计（裁决点 1 甲形态展开）

### A. `tools/l2fix.py`（新工具，~250 行）

- **用法**：`python tools/l2fix.py <file.lom> [--json] [--self-check] [--lom-bin PATH] [--l2 PATH]`（后两项默认 repo 相对 `target/release/lom` 与 `examples/selfhost/self_comp.lom`，参数化解除路径耦合）。
- **管线**：读文件 → 子进程跑 `<lom> <self_comp> -- <tmp-in> <tmp-out>`（temp 目录，用后清理）→ 解析 stdout 的 `codegen error: \[(L2[A-Z]\d{3}|LEX\d{3})\]` 行（LEX 形态透传原样）→ 七族建议映射 → 输出。
- **七族映射**（建议文案从供料 §3 提炼，实施时逐族实测定稿）：
  - L2U：正则提取 `'([^']+)'` → 查内置 名字→模块 表（**含 io**；表头注释登记源 = `interpreter::module_of`（interpreter.rs L2202）单一事实源 + §14.4 内建冻结依据——裁决点 2 甲）→ 命中给 High 建议动作（insert `from M import {name}` at 1:1，附"确认后手动应用或经 lom fix 的 NAM005 通道"）；miss → Low hint（可能是真未定义/拼错——不建议盲补）。
  - L2P：Medium hint（提取重名标识符给重命名方向；"缺 main"给骨架模板）。
  - L2T/L2C：Medium hint（提取期望/实得类型与数量，给"改注解或改值/核对该调用"方向）。
  - L2S/L2V：族级静态模板（L2V 含"Map 用 map_get 取值后运算"等正解模板；L2S 含"常见触发：管道 x |> f 改写为 f(x)、match/解构语句、复杂表达式逐个简化"清单）。
  - L2E：独立文案"内部错误——请报维护者（附该输出）"，不给修复建议。
- **输出**：默认人类可读（码/文案/建议/置信度）；`--json` 输出 `l2-fix/v1` 轻量 JSON（file/code/message/suggestion/confidence/action——**独立 schema，不撞 lom-fix/v1**，L2 工具码不进宿主协议）。

### B. 质量锁定（`--self-check` 模式）

- 对 tools/selfcomp/negative/ 全部 163 负例 + 三题 broken 跑全链：断言码解析 100%、建议非空（L2E 除外为专属文案）、129 得 High import 建议、127/128 得对应族模板（含 map_get/管道关键词）。
- 不入 CI 常驻（首次交付自验 + 报告留档；是否入 CI doc-gates 待后续按需裁决——避免 CI 时长膨胀）。

### C. 文档成对

- eval/README 的 L2 面任务生成纪律节补一句（l2fix 可作为 L2 拒绝文本的辅助采集/建议工具）；HANDOVER §2.2 工具清单 + HANDOFF_PROMPT 状态段登记；SPEC/SPEC_FOR_AI **不动**（L2 工具码非宿主面，v1.6.0 既定口径）；RFC-0004 修订 53 登记。
- **不升版**（纯工具+文档零 Rust 先例——D 包/工具治理批）。

## 3. 冻结面与不做面

- 零宿主 src/ 改动；零 self_comp 改动（quine 271989/verify_selfcomp 367 不变）；零 eval 任务改动（三题联动只是验证面）；语言面/发布线冻结不变。
- 不做（登记）：宿主 CLI 子命令（甲形态——留 L2 转正后的升级路径；届时需一并裁决 Stage 枚举/lom-diag schema 值域与部署耦合）；行号（designs/0017 §4 既定后续批）；l2fix 进 fix.rs 分派（宿主 fix 不产 L2 诊断，无分派需求——通道自身输出建议）。

## 4. 裁决点（呈用户）

1. **通道形态**：甲 = **乙先行**（tools/l2fix.py 纯工具面，零宿主改动不升版；宿主 CLI 子命令留 L2 转正后）【建议——与 L2 实验性定位相称、快速验证价值】/ 乙 = 直接做宿主 CLI 子命令（l2check 进 fix 分派，minor 升版）/ 丙 = 同批乙+甲全做。
2. **L2U 名字→模块映射表**：甲 = 工具自带 ~30 项双源（表头注释指向 interpreter::module_of 单一事实源 + 冻结依据；内建冻结漂移风险低）【建议】/ 乙 = 加对账机制（生成脚本或校验断言——内建冻结下过度工程）。
3. **输出格式**：甲 = 人类可读 + `--json`（独立 l2-fix/v1 轻量 schema）【建议】/ 乙 = 仅人类可读文本。
4. **升版**：甲 = **不升版**（纯工具+文档先例）【建议】/ 乙 = patch v1.6.1。
