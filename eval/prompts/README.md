# Lom Eval Prompts (Phase 2.8 LLM 实测)

本目录包含 10 个提示词文件，用于在网页版 LLM 中批量生成 Lom 代码，然后通过 runner 评估通过率。

## 文件说明

| 文件 | 说明 |
|---|---|
| `01_arithmetic.md` … `10_error_repair.md` | 10 个分类提示词（每个自包含，含 Lom 上下文 + 任务列表 + 输出格式） |
| `_context.md` | 共享的 Lom 语言上下文（被各提示词引用） |
| `_footer.md` | 共享的输出格式要求（被各提示词引用） |
| `_generate.ps1` | 生成脚本：从 tasks/*.json + _context.md + _footer.md 生成 10 个提示词 |
| `extract.ps1` | 提取脚本：从 LLM 输出拆分到 candidates/<id>.lom |

## 使用流程

### 第 1 步：发送提示词到 LLM

1. 打开网页版 LLM（推荐专家模型 + 思考模式）。
2. 打开 `eval/prompts/01_arithmetic.md`，**复制全部内容**，粘贴到 LLM 对话框，发送。
3. LLM 会按格式输出 10 份 `.lom` 代码（用 `=== 001.lom ===` 分隔）。
4. **保存 LLM 的完整回复**到一个文本文件，如 `eval/candidates/output_01.txt`。
5. 对其余 9 个提示词文件重复此步骤（02-10），分别保存为 `output_02.txt` … `output_10.txt`。

> 每个提示词自包含（含 Lom 语言速览），LLM 不需要额外上下文即可生成代码。

### 第 2 步：提取 candidates

对每个保存的 LLM 输出文件，运行提取脚本：

```powershell
# 提取一个文件
powershell -ExecutionPolicy Bypass -File eval/prompts/extract.ps1 -InputFile eval/candidates/output_01.txt

# 提取所有文件（循环）
foreach ($i in 1..10) {
    $num = "{0:D2}" -f $i
    $file = "eval/candidates/output_$num.txt"
    if (Test-Path $file) {
        powershell -ExecutionPolicy Bypass -File eval/prompts/extract.ps1 -InputFile $file
    }
}
```

提取后的文件会保存到 `eval/candidates/001.lom`、`002.lom`、…、`100.lom`。

### 第 3 步：运行评测

```powershell
# 确保 lom 已构建
cargo build --release

# 运行评测
.\eval\runner\run.ps1 -CandidatesDir eval/candidates -LomBin target\release\lom.exe

# 查看详细结果（每任务 PASS/FAIL 明细）
.\eval\runner\run.ps1 -CandidatesDir eval/candidates -LomBin target\release\lom.exe -Verbose
```

Runner 会输出：
- 总通过率（如 `Rate: 87%`）
- 按分类的通过率（如 `arithmetic 9/10 (90%)`）
- 失败退出码 1（CI 友好）

### 第 4 步：分析结果

失败任务的原因可能是：
- **语法错误**：LLM 用了花括号、忘记 `end` 等 → 查看 `lom <file> --json` 诊断
- **导入缺失**：LLM 忘记 `from string import { len }` 等 → 查看运行时错误
- **输出不匹配**：Float 格式（`12.0` vs `12`）、大小写、多/少换行等 → 查看详细输出

## 10 个分类的任务数

| 提示词文件 | 分类 | 任务数 |
|---|---|---|
| `01_arithmetic.md` | 算术与基础函数 | 10 |
| `02_control_flow.md` | 控制流 | 10 |
| `03_types.md` | 类型与推断 | 10 |
| `04_closures.md` | 闭包与高阶函数 | 10 |
| `05_match_enum.md` | match/enum/Result/Option | 15 |
| `06_pipeline.md` | 管道 `|>` | 10 |
| `07_records_tuples.md` | 记录与元组 | 10 |
| `08_effects.md` | 显式效应系统 | 5 |
| `09_modules.md` | 模块与导入 | 5 |
| `10_error_repair.md` | 错误修复（AI 原生核心） | 15 |
| **合计** | | **100** |

## 重新生成提示词

如果任务文件（`eval/tasks/*.json`）有更新，重新生成提示词：

```powershell
powershell -ExecutionPolicy Bypass -File eval/prompts/_generate.ps1
```

## 注意事项

1. **提示词语言**：任务 prompt 为中文，Lom 上下文为中文+英文代码示例。LLM 输出的代码应为纯 `.lom` 代码（无中文注释要求）。
2. **输出格式**：LLM 必须按 `=== <id>.lom ===` 格式输出，否则 extract.ps1 无法提取。如果 LLM 不遵守格式，可在发送提示词时强调格式要求。
3. **错误修复任务**（`10_error_repair.md`）：prompt 中包含错误代码和 `lom-diag/v1` JSON，LLM 需要输出修复后的完整代码。
4. **多次运行**：可以对不同 LLM（DeepSeek/Claude/GPT 等）分别运行，结果保存到不同目录（如 `eval/candidates_deepseek/`、`eval/candidates_claude/`），对比通过率。
