# lom-lang 工作区治理规则（WORKSPACE-GOVERNANCE）

> **定位**：本文件界定 **lom-lang 工作区**（D:\lom-lang\{lom,acri,lom-ssg} 三仓）各层的规则关系，分四部分：§1 lom-lang（维护者本体）· §2 lom（主仓）· §3 lom-ssg · §4 acri。冲突时：仓内事务以各仓规则为准，跨仓事务以本文件为准。
> **沿革**：三仓工作区 2026-10-28 成立；工作区规则此前只存在于 HANDOFF_PROMPT 携带镜像，2026-10-29 用户裁决正式入册并细化为本结构。
> **变更协议**：本文件的任何修改 = 用户裁决（与 MAINTENANCE §6 同级）。

## 总览

| 层 | 角色 | 规则源 | 门禁 | 版本载体 |
|---|---|---|---|---|
| **lom-lang**（工作区） | 维护者本体：三仓之上的协作层 | **本文件** | 交接核验（三仓） | — |
| **lom** | 语言/工具链/评测/治理规则源与主写位 | docs/MAINTENANCE.md | HANDOVER §2.2 全量回归 + CI 七 job | Cargo.toml/lock + tag |
| **lom-ssg** | 第一个 Lom 真实应用（SSG） | 本文件 §3 + 仓内 README | 主仓 lom.exe 构建 + fmt + 断言集 | README 版本提及 + tag |
| **acri** | ACRI 规范之家（spec + 套件 + 登记） | 本文件 §4 + 仓内 README/CHANGELOG | 轻量自查 + 套件 runner 自证 | spec 版本 + tag + CHANGELOG（双载体） |

---

## §1 lom-lang（维护者本体）

工作区 = 维护协作层，不是一个仓库。以下规则适用全部三仓：

### 1.1 三仓隔离铁律
1. **执行者任务单仓绑定**：每个子智能体任务绑定且仅绑定一个仓；执行者全部读写（含探针、临时物）不得越出该仓文件夹。发现需要动另一仓 = 立即停止上报规划者，由规划者拆该仓独立任务。
2. **禁止跨仓耦合改动**：同一任务/执行者/改动不横跨两仓；跨仓联动需求（规范升版牵动主仓映射、主仓改语言面牵动 lom-ssg）由规划者拆成各仓独立批次顺序执行，各仓各自提交、各自门禁。
3. **唯一允许的跨仓接触 = 运行时只读工具引用**：acri / lom-ssg 任务可调用主仓 `target/release/lom.exe` 二进制——子进程调用，不是源文件读写。
4. **探针落各自 gitignored 区**（主仓 `target/probes/`、acri 与 lom-ssg 的 `.tmp/` 等）；**D:\lom-lang 根目录零新文件**（硬约束）。

### 1.2 工作区铁律（三仓同守）
- 全程中文；执行者并行 ≤ 2（主仓独立审查编成例外按 MAINTENANCE §4.5，仅主仓）。
- **git 写操作（add/commit/push/tag）三仓均只由规划者执行**；三仓身份统一 `wytyKen <204019711+wytyKen@users.noreply.github.com>`；三仓均 main 直接推送无 PR 流程。
- **单写者纪律**：同一时刻只允许一个活跃会话写仓库（2026-10-29 并发会话事件后成文）；多会话并存时由用户指定唯一写者。
- 对外动作（发布/发声/外部目录提交，无论哪个仓）逐项呈批；调研不安装竞品、关键数字开原始来源；含反斜杠转义内容用 Write/Edit 落盘；失败如实报告。

### 1.3 交接
唯一交接入口 = 主仓 `docs/HANDOFF_PROMPT.md`（随仓走）；其【工作区形态】段与第一回合清单（三仓核验：status/log/身份/无分叉/根目录干净）对三仓生效。

---

## §2 lom（主仓）

**角色**：语言/工具链/评测/治理的全部规则源与主写位（MAINTENANCE/TODO/HANDOVER/SPEC/designs 均在此）。

**规则源**：docs/MAINTENANCE.md（唯一规范写位）——铁律十条、裁决分级表（§1.4 L1 预授权五条款）、批次循环（§2）、执行者派发与验收（§3）、审查与常设机制三件（§4：独立审查 §4.5 / 外部观察 §4.6 / 头脑风暴 §4.7）、交接协议（§5）；全量回归命令 = HANDOVER §2.2；语言冻结面 = LANGUAGE_SPEC §14。

**门禁**：提交前 HANDOVER §2.2 全量回归（规划者必亲验 verify_selfcomp 与 doc_audit）+ doc_audit/link_check → 推送后看 CI 首跑 → 绿后切 tag；升版按 MAINTENANCE §2.2 决策表。

**活信息写位**：TODO 顶部状态块（唯一主写位）；HANDOFF_PROMPT【当前真实状态】= 交接镜像。

**边界（不自动外推到其他仓）**：L1 预授权条款、doc_audit 数字锚、CI 七 job、独立审查机制、外部观察/头脑风暴、RFC/设计批前置、语言面冻结——这些都是主仓机制；其他仓引入同类机制须该仓自行成文 + 用户裁决。

**审查形态三分**（防混淆）：
1. **主仓独立审查**（§4.5）= 主仓常态化机制：敌手复核仓库宣称、双层覆盖、开账号段（R 编号）；
2. **各仓常规维护审查** = lom-ssg / acri 的 code-review 式审查（发现直接修，不开账、不占主仓号段）；
3. **模拟审稿**（如论文发表前评审）= 一次性第三方操作，与上述两类均无关，不常态化。

---

## §3 lom-ssg

**角色**：第一个 Lom 真实应用——零依赖静态站点生成器（build.lom 单文件，Lom 语言写就），证明语言可承载真实项目。

**规则源**：本节 + 仓内 README。

**门禁**（改动必过）：① 输出目录前置（`mkdir -p example/dist`）② 主仓 `lom.exe build.lom -- example/site.json` 构建（3 页、rc=0）③ `lom fmt --check build.lom` ④ 断言集全过（71 项，仓内 `.tmp/assert.py`；入库与否见登记）。

**版本语义**：行为修复/纠偏 = patch；新功能 = minor；版本提及在 README，tag 由规划者切（无 CI，门禁绿即 tag）。当前 **v0.1.1**（`a2fd241`）。

**既定边界**：pages 显式清单为冻结面边界的如实登记（主仓 file 模块无目录列举原语，不绕过）；CI 未建（构建依赖主仓二进制——双仓 CI 耦合成本 > 收益；主仓有 release 制品分发后可再裁）。

**登记待办**：断言集入库（tests/）与否、R2-R9 维护项（lom-ssg/.tmp/review-v0.1.0.md）。

---

## §4 acri

**角色**：ACRI 规范之家——spec 权威源 + conformance 套件 + 实现符合性登记，独立于任何实现项目演进。

**规则源**：本节 + 仓内 README/CHANGELOG。

**核心纪律**：
- **spec 同版不可变**：编号 cut 后任何文字改动（含 typo）必须升版；勘误以新 draft/cut 交付（v0.1 已冻结、v0.2-draft 待裁）。
- **升版双载体**：git tag + CHANGELOG 条目，缺一不算 cut。
- 双许可：spec 文本 CC-BY-4.0 / 代码 Apache-2.0。
- **conformance 套件**（Batch 2+3+4 已交付）：改套件须跑 runner 自证（对 adapters/lom.json 首跑三态全过）；探测项/条款对齐变更须与 implementations/ 登记一致。
- 中立性条款（spec §6）：实现中立、基金会捐赠预留。

**门禁**：轻量——文本批自查链接；套件批 runner 自证；spec 零改动除非升版三件套。

**登记待办**：v0.2 切版批（含 §3 参考实例计数 129→138 刷新——0.x 草案未 cut 可改）。

---

## §5 与既有规则的关系
- 本文件不改变 MAINTENANCE.md 任何内容；MAINTENANCE 的铁律/验收/审查等机制继续仅约束主仓，其跨仓适用的子集（中文/git 独占/诚实/调研纪律等）已收编入 §1.2。
- HANDOFF_PROMPT 的工作区段此后指向本文件（携带镜像 → 规范源）。
- 速查表新增一行：工作区/三仓规则 = 本文件（docs/WORKSPACE-GOVERNANCE.md），MAINTENANCE §附。
