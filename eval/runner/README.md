# Lom Eval Runner

Two scripts, same behavior:

| Script | Platform | Dependencies |
|---|---|---|
| `run.ps1` | Windows | PowerShell 5.1+ (built-in), `lom.exe` on PATH |
| `run.sh`  | Linux / macOS | `bash 4+`, `jq`, `lom` on PATH |

## Quick start

```powershell
# Build lom first
cargo build

# Verify the eval set itself (reference solutions)
eval/runner/run.ps1 -Verify

# Evaluate LLM-generated candidates (saved as eval/candidates/<id>.lom)
eval/runner/run.ps1 -CandidatesDir eval/candidates
```

```bash
cargo build
eval/runner/run.sh --verify
eval/runner/run.sh --candidates-dir eval/candidates
```

## Modes

### `--verify` / `-Verify`

Runs every reference `solution` through `lom` and compares stdout to `expected`. Should report **100/100 pass**. Use this to:
- Catch regressions in the interpreter
- Validate new tasks you add to the eval set
- Confirm `cargo build` didn't break anything

### `--candidates-dir <dir>` / `-CandidatesDir <dir>`

For each task `<id>`, looks for `<dir>/<id>.lom`, runs it through `lom`, compares stdout to `expected`. Use this to score an LLM run:
1. Generate candidates from prompts (one `.lom` file per task, named `<id>.lom`)
2. Run the script
3. Read the pass-rate report (overall + by category)

## Output

Both scripts print:
- Per-task PASS/FAIL (with `--verbose`)
- Summary: total / passed / failed / rate
- By-category breakdown

Exit code: `0` if all pass, `1` if any fail.

## Error-repair category

Tasks 086-100 (in `tasks/10_error_repair.json`) have a different prompt format: the prompt contains **broken `.lom` code** and the **`lom-diag/v1` JSON**. The LLM is expected to produce **fixed** `.lom` code. The runner treats the LLM output the same way as other categories — run through `lom`, compare stdout to `expected`.
