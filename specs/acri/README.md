# ACRI 规范目录（specs/acri/）

> **ACRI = Agent-Compiler Repair Interface**——编译器/工具链与消费其诊断并执行修复的
> AI agent 之间的接口规范（v0.1 Draft，2026-10-12；裁决记录：三层全包 / 命名 ACRI /
> 顶层 specs/ 落位——designs/0023 三裁决点均按建议项）。
>
> **本目录是规范资产本体**（与项目文档 docs/ 分离——未来可整树迁出独立分发）。
> 实现方中立：符合 ACRI 不要求采用 Lom 语言或任何特定运行时。

## 目录文件

| 文件 | 内容 |
|---|---|
| [acri-v0.1.md](acri-v0.1.md) | 规范本体（英文，面向外部实现者）：L0 对象模型（Diagnostic/RepairSuggestion + 三条安全性公理）→ L1 CLI 协议（命令集/退出码矩阵/符合性级别）→ L2 修复基准方法论附录 → 参考消费器 → 变更协议与治理 |
| [lom-conformance.md](lom-conformance.md) | Lom 参考实现符合性映射：规范条款 ↔ 现状 ↔ 状态（β-2 后 L0/L1 conformant、L2 documented，两登记偏差） |
| [consumer.py](consumer.py) | **ACRI 参考消费器**（规范 §4 的可运行实装，~90 行纯标准库）：码表引导 → check 消费 → 公理 1 客户侧过滤 → plan/apply 闭环——证明规范可被对实现语言一无所知的第三方程序消费（β-3 交付，三闭环实跑通过） |

## 一分钟了解

- **它标准化什么**：结构化诊断对象、置信度分级的修复建议对象、
  "只自动应用 high 且非 hint"的安全性公理、check/fix/codes CLI 协议与退出码矩阵、
  防污染修复基准的构造方法论。
- **动机三事实**（规范 §0 可溯源）：GPT-5 修编译错误语法 73% vs 语义 41%
  （ComBench）；OpenAI 弃用 SWE-bench Verified（污染 + 测试缺陷）；跨厂商的
  agent 诊断 schema 截至 2026-10 不存在。
- **参考实现**：Lom（本仓库）；参考基准：RepairLoopBench（eval/README.md）。

## 维护纪律（仓内）

- 规范文档改动走既有门禁（doc_audit / link_check）；语义级变更按 §5 变更协议升版；
- β-2 工程批（`codes --json` 码表出口等）动工前后同步更新 lom-conformance.md。
