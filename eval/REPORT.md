# Lom Eval Report — Phase 2.8 LLM 实测结果

**测试日期**: 2026-08-03
**评测集版本**: lom-eval/v1 (100 tasks, 10 categories)
**被测 LLM**: 网页版专家模型 + 思考模式
**Lom 版本**: Phase 2.8 (v0.1.3)

---

## 总览

| 指标 | 结果 |
|---|---|
| **总通过率** | **99/100 (99%)** |
| 基线预期 | 60%+ |
| 实际 vs 预期 | **远超预期** |
| 完全通过分类 | 9/10 (90%) |
| 失败任务数 | 1 |
| 失败任务 ID | 078 (effects) |

---

## 按分类统计

| 分类 | 通过率 | 任务数 | 说明 |
|---|---|---|---|
| arithmetic | 10/10 (100%) | 10 | 基础算术、let、fn、println |
| control_flow | 10/10 (100%) | 10 | if/elif/else、while、for、return |
| types | 10/10 (100%) | 10 | Int/Float/Bool/String/Unit |
| closures | 10/10 (100%) | 10 | 一等公民闭包、高阶函数 |
| match_enum | 15/15 (100%) | 15 | match/enum/Result/Option |
| pipeline | 10/10 (100%) | 10 | `|>` 线性管道 |
| records_tuples | 10/10 (100%) | 10 | 结构记录、元组 |
| **effects** | **4/5 (80%)** | 5 | 显式效应系统 `! [IO, Clock]` |
| modules | 5/5 (100%) | 5 | from mod import、别名 |
| error_repair | 15/15 (100%) | 15 | **AI 原生核心**：基于 lom-diag/v1 修复错误代码 |
| **合计** | **99/100 (99%)** | 100 | |

---

## 失败任务分析

### 任务 078 — effects 分类

**Prompt 要求**: 写函数 `log_msg(msg: String) -> Unit ! [IO, Clock]` 先获取时间戳再 println。main 调用 `log_msg("boot")`。预期输出 `1700000000 boot`。

**Expected**: `1700000000 boot\n`

**LLM 生成代码**:
```lom
fn get_timestamp() -> Int ! [Clock]
    1700000000
end

fn log_msg(msg: String) -> Unit ! [IO, Clock]
    let ts = get_timestamp()
    println(ts)
    println(msg)
end

fn main() -> Unit
    log_msg("boot")
end
```

**Actual 输出**:
```
1700000000
boot
```

**失败原因**: 输出格式错误（任务理解偏差）
- LLM 用了两次 `println` 分别打印时间戳和消息，产生两行输出
- Prompt 隐含要求拼接成一行（expected 为 `1700000000 boot`）
- LLM 未使用 `int_to_string` 拼接字符串（如参考方案 `println(int_to_string(t) + " " + msg)`）

**错误类型**: 逻辑错误（任务理解偏差），非语法/导入/效应错误
- 语法正确 ✅
- 效应注解正确 ✅ (`! [IO, Clock]`)
- 函数间调用正确 ✅
- 仅输出格式理解错误 ❌

---

## 关键发现

### 1. Lom "AI 原生"特性得到验证

- **error_repair 15/15 (100%)**: LLM 能完美基于 `lom-diag/v1` 诊断信息修复错误代码。这是 Lom 区别于普通语言的核心价值——结构化诊断让 LLM 修复率极高。
- **effects 4/5 (80%)**: 唯一失败是输出格式理解，不是效应系统本身。LLM 正确理解了 `! [IO, Clock]` 注解、纯函数/效应函数组合规则。
- **match_enum 15/15 (100%)**: 复杂模式匹配全部正确，包括 enum 变体解构、Result/Option 处理。

### 2. 语法友好性得到验证

- **0 个语法错误**: 100 份代码中没有任何花括号、缺 `end`、缩进错误等常见 LLM 语法错误
- **0 个导入缺失**: LLM 正确记住了 `from string import` / `from math import` 规则
- **0 个类型错误**: 渐进式类型系统下无类型相关失败

### 3. 线性管道 `|>` 对 LLM 友好

- pipeline 10/10 (100%): LLM 完美掌握了 `x |> f |> g` 链式语法

### 4. 唯一失败类别分析

- 失败发生在 effects 分类（中等难度任务），但失败原因不是效应系统本身
- 而是输出格式理解（prompt 可表述更明确："拼接成一行打印"）
- 这提示评测集 prompt 质量也是影响因素——后续可优化 078 的 prompt 措辞

---

## 结论

**Lom 的 "AI 原生" 设计目标得到充分验证**：

1. **99% 通过率远超 60% 基线**——Lom 语法对 LLM 极度友好
2. **error_repair 100%**——结构化诊断 `lom-diag/v1` + 修复计划 `lom-fix/v1` 让 LLM 完美修复错误代码
3. **0 语法错误 / 0 导入缺失**——显式导入、`end` 闭合块、线性管道等设计有效降低了 LLM 犯错率
4. **唯一失败是输出格式理解**——与语言特性无关，属 prompt 措辞可改进范围

这为 Phase 2 的退出标准（"主流 LLM 生成通过率达标"）提供了硬证据：**Lom 的 AI 原生设计有效**。
