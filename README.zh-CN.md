# Lom（中文导读）

**Lom (Language of Machine)** — 一门围绕 **LLM 修复闭环**设计的编程语言：当模型写出坏代码时，语言本身负责精确诊断并机械化修复。修复闭环不是附加工具，是这门语言存在的理由。

[![CI](https://github.com/lom-lang/lom/actions/workflows/ci.yml/badge.svg)](https://github.com/lom-lang/lom/actions/workflows/ci.yml)
![deps](https://img.shields.io/badge/dependencies-0-2ea44f)
![unsafe](https://img.shields.io/badge/unsafe-0-blue)
![license](https://img.shields.io/badge/license-Apache--2.0-9cf)

## 三件硬证据

1. **修复闭环经真实模型验证**：31 道 error_repair 任务（含 warning 级修复与"宿主收/L2 拒"跨编译器修复），deepseek-v4-pro+thinking 与 glm-5.3 双模型 ×10 采样（t=1.0）pass@1 99.1–100%——[报告](eval/REPORT-2026-10-03-err-repair-31.md)；
2. **编译器自举可证明**：`self_comp.lom`（12,591 行 Lom）把自身源码编译为 WASM，双侧产物逐字节一致（272,293 字节强 quine，CI 每次运行复核）；
3. **可信内核**：零第三方 crate、零 `unsafe`、561 单元 + 31 集成测试、三平台 CI；语言面（语法 / 20 关键字 / 诊断码 / 43 内建）自 v1.0 [冻结](LANGUAGE_SPEC.md)，变化须 RFC。

## 60 秒体验修复闭环

`hello.lom`——你（或你的模型）忘了导入：

```lom
fn main() -> Unit
    println(len("hello"))
end
```

运行、修、再运行：

```
$ lom hello.lom
[runtime] error (0:0): [RUNTIME002] 符号 'len' 未导入。需在文件顶部声明：from string import {len}

$ lom fix hello.lom --apply
lom apply: hello.lom（迭代 2 轮）
  round 1: applied 1, skipped 0
    [1:1] insert (NAM005) — 在文件顶部插入 'from string import {len}'
  最终诊断（修复后源码）: 0 错误 / 0 警告

$ lom hello.lom
5
```

该插入是 `high` 置信度的机械动作；歧义形态只给 hint 不做静默改写。`lom fix` 的计划格式（`lom-fix/v1`）就是 LLM agent 消费的接口——见 [SPEC_FOR_AI.md](SPEC_FOR_AI.md)。

## 快速开始

```powershell
cargo build --release
./target/release/lom examples/fib.lom
```

## 状态与边界（如实）

- 当前 **v1.6.1**（2026-10-03）；语言面自 2026-09-02 冻结后零变化——其后每个版本都是整改与证据工作。逐版记录见[英文 README 的 Release log](README.md#release-log)；
- **L2 自举子集编译器**（`examples/selfhost/self_comp.lom`）明确标注**实验性**：编译 Lom 严格子集到 WASM，宿主收而 L2 拒的每个面都带 `[L2xxx]` 码与修复建议（[tools/l2fix.py](tools/l2fix.py)）；
- 26 轮审查为**体系内敌手式审查**（agent 执行、以证据为准）——是良好的工程卫生，**不是外部同行评审**；评级不跨基线外推；
- **[在线 playground](https://lom-lang.github.io/lom/playground/)**——浏览器里跑 Lom：结构化诊断 + `lom fix --apply` 自动修复（带行级 diff），全客户端（宿主工具链 wasm 化，零第三方 JS）。

## 延伸阅读

- [中文教程](docs/lom-tutorial.html) · [语言规范 LANGUAGE_SPEC](LANGUAGE_SPEC.md) · [面向 LLM 的精简规范 SPEC_FOR_AI](SPEC_FOR_AI.md)
- [设计取舍 DESIGN_RATIONALE](DESIGN_RATIONALE.md) · [定位叙事 positioning](docs/positioning.html)
- [英文 README](README.md)（含完整里程碑史与逐版横幅）

许可证：Apache-2.0。贡献见 [CONTRIBUTING.md](CONTRIBUTING.md)。
