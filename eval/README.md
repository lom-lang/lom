# Lom Eval Suite (Phase 2.8)

100-task benchmark for measuring **LLM generation pass-rate** on Lom code.

> This is the **core deliverable of Phase 2** — the hard metric that backs Lom's "AI-native" claim. An LLM is given a prompt; it generates `.lom` code; the runner executes the code and compares stdout to the expected output.

## Design goals

1. **Coverage** — 100 tasks across 10 categories (arithmetic, control flow, types, closures, match/enum, pipeline, records/tuples, effects, modules, error repair).
2. **Reproducibility** — every task has a verified reference solution and expected output; the runner self-checks the eval set before any LLM run.
3. **LLM-coding-native focus** — tasks are designed to probe Lom's AI-friendly features (tolerant parse, structured diagnostics, `|>` linearity, structural types, `Result`/`match`).
4. **Multi-model comparability** — same prompts, same runner, same expected outputs across DeepSeek / Claude / GPT / Kimi / GLM / Gemini.

## Directory layout

```
eval/
  README.md                  # this file
  manifest.json              # category metadata + task counts
  tasks/
    01_arithmetic.json       # 10 tasks — basic math, let, fn
    02_control_flow.json     # 10 tasks — if/elif/else, while, for, return
    03_types.json            # 10 tasks — annotations, inference, casts
    04_closures.json         # 10 tasks — closures, HOF, capture
    05_match_enum.json       # 15 tasks — match, enum, Result, Option
    06_pipeline.json         # 10 tasks — |> operator
    07_records_tuples.json   # 10 tasks — records, tuples, field access
    08_effects.json          #  5 tasks — ! [IO, Clock] annotations
    09_modules.json          #  5 tasks — from ... import
    10_error_repair.json     # 15 tasks — fix broken code (lom fix flow)
  runner/
    run.ps1                  # PowerShell runner (Windows, no deps)
    run.sh                   # Bash runner (requires jq + lom on PATH)
    README.md                # runner usage
```

## Task format

Each `tasks/NN_<category>.json` file is an array of task objects:

```json
[
  {
    "id": "001",
    "category": "arithmetic",
    "difficulty": "easy",
    "prompt": "写一个函数 add(a: Int, b: Int) -> Int 返回两数之和。main 调用 println(add(3, 4))。",
    "solution": "fn add(a: Int, b: Int) -> Int\n    a + b\nend\n\nfn main() -> Unit\n    println(add(3, 4))\nend\n",
    "expected": "7\n",
    "notes": "考察 fn/let/println 基础语法；常见错误：忘记 end、用 return 而非尾表达式"
  }
]
```

Fields:
- **`id`** — zero-padded 3-digit task id (001-100), globally unique
- **`category`** — one of the 10 categories (matches filename suffix)
- **`difficulty`** — `easy` / `medium` / `hard`
- **`prompt`** — Chinese natural-language description of what the LLM should generate. Mentions required functions, expected behavior, and any constraints. **The LLM only sees this field** (plus `SPEC_FOR_AI.md`); it does not see `solution` or `expected`.
- **`solution`** — reference `.lom` source code (verified to produce `expected` when run through `lom`)
- **`expected`** — exact stdout when `lom solution.lom` is run (including trailing newline)
- **`notes`** — optional, documents what the task probes and common LLM failure modes

## Runner usage

### Verify the eval set itself (no LLM needed)

```powershell
# PowerShell
cargo build --quiet
eval/runner/run.ps1 -Verify
```

```bash
# Bash
cargo build --quiet
eval/runner/run.sh --verify
```

This runs every reference `solution` through `lom` and compares stdout to `expected`. Should report **100/100 pass**. Use this to catch regressions in the interpreter or the eval set.

### Evaluate LLM-generated candidates

1. Run your LLM on each prompt, save outputs as `eval/candidates/<id>.lom`.
2. Run:

```powershell
eval/runner/run.ps1 -CandidatesDir eval/candidates
```

```bash
eval/runner/run.sh --candidates-dir eval/candidates
```

The runner runs `lom eval/candidates/<id>.lom` for each task, compares stdout to `expected`, and reports pass-rate by category and overall.

### Error-repair category (15)

Tasks in `10_error_repair.json` have a different flow:
1. The prompt contains **broken `.lom` code** and the **`lom-diag/v1` JSON** for that code.
2. The LLM is asked to produce **fixed** `.lom` code.
3. The runner treats the LLM output the same way: run through `lom`, compare stdout to `expected`.

This tests the full LLM-coding-native loop: **LLM generates → Lom diagnoses → LLM repairs → Lom runs**.

## Adding tasks

1. Pick the category file in `tasks/`.
2. Append a task object with the next `id`.
3. Run `eval/runner/run.ps1 -Verify` to confirm your `solution` produces `expected`.
4. Update `manifest.json` task count.

## Baseline results

Phase 2.8 only ships the framework + reference solutions. Baseline LLM pass-rates will be measured separately and recorded in `eval/results/` (TBD).
