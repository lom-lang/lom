# RepairLoopBench — the Lom 129-task repair-suite benchmark card

> **Asset name: RepairLoopBench** (adjudicated 2026-10-12). The name names what the
> suite measures — the repair loop — not the language that hosts it. Lom is the
> reference implementation and host; the benchmark's methodology (contamination-safe
> task synthesis, dual-backend verification, advisory-loop scoring) is designed to be
> reusable by any language/toolchain that exposes structured diagnostics to agents.

## How to re-run (no dependencies beyond the toolchain)

```powershell
cargo build --release
powershell -ExecutionPolicy Bypass -File eval/runner/run.ps1 -Verify -LomBin ./target/release/lom.exe           # 129/129 expected, native backend
powershell -ExecutionPolicy Bypass -File eval/runner/run.ps1 -Verify -LomBin ./target/release/lom.exe -Backend wasm  # 129/129 expected, WASM backend
python tools/eval_prompt_check.py    # 32/32: error_repair prompts embed real host output (--check --json; task 130 embeds the runtime stderr channel), machine-checked against the live binary
```

The suite self-checks (reference solutions verified, eval IDs unique) before any LLM
run. Headline LLM results: [full-set pass@k, 2026-10-07](REPORT-2026-10-07-passk-128.md)
(pass@1 = 98.3% / 98.5% across two frontier models, 10 samples each, unbiased estimator).

## How this benchmark differs from SWE-bench-family suites (scope statement)

Scope honesty first: this is a **small, purpose-built suite (129 tasks; 32 repair
tasks)** anchored to one language's toolchain. It does not compete with SWE-bench-class
repo-scale leaderboards on scale or realism, and its numbers are not comparable to
them. It occupies a different, complementary slot, with three deliberate differences:

1. **Advisory repair loop vs generative patching.** SWE-bench-family tasks give an
   issue and ask for a patch against a real repo. RepairLoopBench's error_repair tasks
   hand the model **real structured diagnostics** (verbatim `--check --json` output
   captured from the live compiler, plus the L2 cross-compiler's `[L2xxx]` channel for
   3 tasks) and ask it to converge on a passing program through the repair loop. This
   measures the loop the language exists to serve, not repo archaeology.
2. **Contamination-safe by construction.** Tasks are synthetically authored (plus
   verbatim diagnostic captures), reference solutions are verified on three chains
   (host interpreter / host WASM / the L2 self-hosted compiler chain), and expected
   outputs are locked. The suite was never scraped from public issue trackers. This is
   a direct answer to the contamination and test-defect findings that led OpenAI to
   stop reporting SWE-bench Verified ([why we no longer evaluate SWE-bench
   Verified](https://openai.com/index/why-we-no-longer-evaluate-swe-bench-verified/),
   2026-02-23: controlled SOTA 80.9%, 59.4% of 138 unsolved problems had substantive
   test defects).
3. **Dual-backend determinism.** Every reference solution passes identically on two
   independent backends (tree-walking interpreter and WASM) before entering the set —
   the suite measures model behavior, not backend drift.

The problem this suite is built for is externally documented: on repo-level compilation
error repair, GPT-5 reaches 73% syntactic correctness but only 41% semantically valid
patches (ComBench, [arXiv:2603.27333](https://arxiv.org/abs/2603.27333)) — the gap
between "compiles" and "correct" is exactly what a repair-native language surface
targets. Agent-tooling practice is converging on the same thesis: error responses are
prompt-engineering objects for agents (Anthropic,
[Writing effective tools for AI agents](https://www.anthropic.com/engineering/writing-tools-for-agents), 2025-09).

---

# Lom Eval Suite (Phase 2.8)

129-task benchmark for measuring **LLM generation pass-rate** on Lom code.

> This is the **core deliverable of Phase 2** — the hard metric that backs Lom's "AI-native" claim. An LLM is given a prompt; it generates `.lom` code; the runner executes the code and compares stdout to the expected output.

## Design goals

1. **Coverage** — 129 tasks across 10 categories (arithmetic, control flow, types, closures, match/enum, pipeline, records/tuples, effects, modules, error repair).
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
    02_control_flow.json     # 13 tasks — if/elif/else, while, for, return
    03_types.json            # 13 tasks — annotations, inference, casts
    04_closures.json         # 13 tasks — closures, HOF, capture
    05_match_enum.json       # 16 tasks — match, enum, Result, Option
    06_pipeline.json         # 10 tasks — |> operator
    07_records_tuples.json   # 10 tasks — records, tuples, field access
    08_effects.json          #  6 tasks — ! [IO, Clock] annotations
    09_modules.json          #  6 tasks — from ... import
    10_error_repair.json     # 32 tasks — fix broken code / warning-guided repair (incl. L2-subset face)
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
- **`id`** — zero-padded 3-digit task id (001-130, 129 unique ids — 108 is a known, accepted gap), globally unique
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

This runs every reference `solution` through `lom` and compares stdout to `expected`. Should report **129/129 pass**. Use this to catch regressions in the interpreter or the eval set.

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

### Error-repair category (32)

Tasks in `10_error_repair.json` have a different flow:
1. The prompt contains **broken `.lom` code** and the **`lom-diag/v1` JSON** for that code.
2. The LLM is asked to produce **fixed** `.lom` code.
3. The runner treats the LLM output the same way: run through `lom`, compare stdout to `expected`.

This tests the full LLM-coding-native loop: **LLM generates → Lom diagnoses → LLM repairs → Lom runs**.

> **Audit caveat (2026-09-21, R60):** the runner validates candidate stdout + exit code; it does not validate that diagnostic JSON embedded in a prompt is still emitted by the current compiler. Tasks 089/090 claim a missing-function-`end` PARSE001, while the current parser silently accepts EOF as the block terminator. These are historical repair prompts, not a current diagnostic-conformance corpus, until R57/R60 refresh them from real `--check --json` output. New diagnostic-bearing tasks must continue to be generated from real output, never hand-written.

#### L2-face task generation discipline (tasks 127-129, 2026-10-01)

Tasks 127-129 add an **L2 face** on top of the host-diag flow: the broken code is also rejected by the L2 subset compiler (`examples/selfhost/self_comp.lom`, run as `lom examples/selfhost/self_comp.lom -- <input.lom> <out.hex>`; "experimental" at capture time — the L2 face was promoted to supported in v1.8.0, 2026-10-08, but the prompts keep their verbatim capture-time wording). Their prompts carry **two separately collected real outputs**:

1. The host `lom --check --json` diagnostic (same R60 discipline as every other task; embedded as the `诊断 JSON` block, `file` normalized to `bad.lom`).
2. The L2 rejection text, captured verbatim from the L2 compiler's real stdout (`codegen error: ...` + `COMPILE-ERROR`), introduced by an explicit source line （"实验性 L2 子集编译器（examples/selfhost/self_comp.lom）拒绝如下："）. It is never mixed into the `lom-diag/v1` JSON block.

Generation rules: the reference solution must keep host behavior identical to `expected` (verified on both backends: direct run + `build --target wasm` via `node eval/runner/run_wasm.mjs`) **and** pass the full L2 chain (`self_comp` → `tools/hex2wasm.py` → `node tools/selfcomp/run_selfcomp.mjs`) with stdout matching `expected`. Note the runner (`run.ps1`) itself only exercises host behavior — the L2-chain guarantee is a task-generation-time gate, not a runner gate; likewise `tools/eval_prompt_check.py` re-validates only the embedded host diagnostic JSON (the L2 text block sits outside its extraction anchors and is out of its check face). When collecting L2 rejection text (or sanity-checking a repair direction for it), `python tools/l2fix.py <file.lom>` can help: it runs the same L2 chain, surfaces the `[L2xxx]` code lines, and offers per-family repair suggestions (designs/0018).

#### Runtime-diagnostic task generation discipline (task 130, 2026-10-13)

Task 130 adds a **runtime face**: the broken code is statically clean (`--check --json` returns zero diagnostics) and fails only at run time (the v1.10.3 structured deep-recursion guard). Its prompt carries **two separately collected real outputs**:

1. The host `lom --check --json` block — genuinely empty diagnostics, same R60 discipline, `file` normalized to `bad.lom`.
2. The runtime stderr text, captured verbatim from `lom bad.lom` (the `[RUNTIME000]` three-part diagnostic plus the summary line, with the exit-code note), introduced by an explicit source line. **Ordering is a hard constraint**: the runtime block must come **after** the `诊断 JSON` block — `tools/eval_prompt_check.py` extracts code up to the first `诊断 JSON` anchor and takes the first balanced JSON object after it, so a runtime block placed earlier would bleed into the code block and false-positive (probe-verified both orderings).

Generation rules mirror the L2-face discipline: reference solution verified on both backends; the runner only exercises host behavior. Registered product boundary, stated in the task notes: `lom fix --plan` produces no suggestion for this broken code — the fix pipeline is driven by static diagnostics and runtime errors do not enter it (v1.10.3's message-aware RUNTIME000 hint exists in the dispatcher table but has no CLI arrival path for runtime-raised diagnostics).

## Adding tasks

1. Pick the category file in `tasks/`.
2. Append a task object with the next `id`.
3. Run `eval/runner/run.ps1 -Verify` to confirm your `solution` produces `expected`.
4. Update `manifest.json` task count.

## Baseline results

Framework + reference solutions ship in-tree. The 2026-08-03 LLM baseline (99/100 on the original 100-task set) is recorded in `REPORT.md`; raw candidates are preserved in `eval/candidates/`.

**2026-08-31 multi-model retest** (`REPORT-2026-08-31-multimodel.md`, via `llm_eval.py`): deepseek-v4-pro+thinking **113/113 (100%)**, deepseek-v4-flash 112/113, glm-4.7 112/113 — the project's "≥3 LLMs" bar (guide §3.3) is now met, and tasks 101-113 have their first LLM measurements. Only task 078 fails (same prompt-ambiguity failure as the 2026-08-03 baseline).

**2026-09-07 pass@k retest** (`REPORT-2026-09-07-passk.md`, via `llm_eval.py --samples 10 --temperature 1.0` + `passk_summarize.py`): deepseek-v4-pro+thinking and glm-5.3, 10 samples each on the full 116-task set (first LLM measurements for tasks 115-117). **pass@1 = pass@5 = pass@10 = 99.1% for both models** (unbiased estimator `1 - C(n-c,k)/C(n,k)`; scorer unchanged: `run.ps1 -CandidatesDir`). Only 078 fails systematically (0/10 both models) — the single-sample pass-rate is robust to sampling variance, closing the "single sample" honesty caveat of the 2026-08-31 report. Candidates + per-task pass matrices: `eval/candidates_rerun/<model>_passk10/`.

**2026-10-03 error_repair 31-task resample** (`REPORT-2026-10-03-err-repair-31.md`): both models × 10 samples on all 31 error-repair tasks — the 24-task overlap subset passed 480/480; new tasks 123-125/128/129 passed 100/100; the two failures were attributed (126 = prompt-level stdout ambiguity, fixed by tightening the task the same lineage as 078→118; 127 = genuine difficulty, see below).

**2026-10-07 full-set pass@k — the current headline evidence** (`REPORT-2026-10-07-passk-128.md`): both models × 10 samples @ t=1.0 on the **full 128-task set** (first multi-sample measurement for the v1.5.0 additions; 126/127 tightened the same day at `e1702ba`). **pass@1 = 98.3% (deepseek-v4-pro+thinking) / 98.5% (glm-5.3)**, pass@5 = 99.2% / 98.8%, pass@10 = 100.0% / 99.2%. Sub-100 tasks fully attributed: 078 (prompt-ambiguity anchor; its explicit twin 118 passes 10/10 both models) and 127 (deliberately hard L2 cross-compiler repair: after tightening, all 20 candidates take the `map_get` route — zero shortcut solutions — but only 2 add the required Option-unwrapping step). The tightened 126 passes 10/10 on both models (was 0/10 and 8/10 under the ambiguous wording). Candidates + matrices: `eval/candidates_rerun/<model>_passk10_128/`.

The multi-sample `candidates_rerun/` raw replies and summaries are intentionally gitignored. On the handoff machine they were present and the ninth review independently recomputed 1149/1160 for each pass@k model and 240/240 per model for the 24-task repair patch. A fresh clone cannot verify raw provenance from the repository alone; published claims must retain that qualification or archive a privacy-reviewed evidence bundle later.

## Citing

RepairLoopBench is version-anchored to the repository's tags — **cite the tag,
not `main`** (`main` drifts with each task-set change; e.g. the 129-task form
first exists at `v1.11.0`).

Recommended citation form:

> wytyKen (2026). *RepairLoopBench: a purpose-built repair-loop benchmark for
> an LLM-repair-native language* (129 tasks; contamination-safe task
> synthesis, dual-backend verification). GitHub repository, version anchored
> at tag v1.11.0+. https://github.com/lom-lang/lom

Notes: authored under sustained LLM co-authorship (as is the whole repository);
licensed Apache-2.0 with the rest of the repository. A DOI (automated
archive-per-tag) is planned once the external-catalogue submission settles —
until then the tag anchor above is the stable reference.
