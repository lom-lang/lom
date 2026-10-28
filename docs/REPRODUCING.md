# REPRODUCING — reproducing the Lom results from a fresh clone

This guide is written for a **third party with zero context**: you cloned the
repository, you want to know which of its claims you can check yourself, and
what you need to bring to reproduce the rest. Everything below was verified on
a fresh working tree at the current release at the time of writing (v1.14.0);
the canonical, always-current list of expected counts is
[HANDOVER.md §2.2](HANDOVER.md) (internal working document, maintained in
Chinese — the counts are language-neutral).

Scope honesty up front, mirroring the registered boundary in
[eval/README.md](../eval/README.md): a fresh clone lets you reproduce **every
deterministic gate** (build, tests, self-hosting quine, benchmark integrity)
and **the LLM experiments themselves** with your own model budget. It does
**not** contain the raw model replies of the published multi-sample runs
(those archives are single-machine and gitignored), so you can re-run the
experiments but not re-audit the historical raw provenance. Section 4 draws
that line precisely.

## 1. Prerequisites

| Tool | Needed for | Notes |
|---|---|---|
| Rust (stable, edition-2024 capable) | everything | `cargo build --release`; zero third-party crates, so no network fetches beyond the toolchain |
| PowerShell | eval runner, prompt generation | Windows PowerShell 5.1+ (built into Windows) or `pwsh` 7 on Linux/macOS — this is the path CI uses on all three OSes |
| Python 3 | repo tools (`tools/*.py`, `eval/llm_eval.py`) | stdlib only |
| Node.js | WASM-backend eval, playground smoke | only for the dual-backend checks |
| bash 4 + `jq` | the alternative runner `eval/runner/run.sh` | optional; note macOS ships bash 3.2, which cannot run it — use `pwsh` there (this is why CI standardizes on `run.ps1`) |

Cross-platform note: the repository's CI (`.github/workflows/ci.yml`) runs a
three-OS matrix (ubuntu / macOS / Windows) plus selfhost, clippy, doc-gates
and playground jobs — seven jobs, every push. The eval verification step is
deliberately unified on `eval/runner/run.ps1` via `pwsh` on all platforms.

## 2. Reproduce the engineering baseline (deterministic, no LLM)

All commands from the repository root. Expected results are stated as of
v1.14.0; [HANDOVER.md §2.2](HANDOVER.md) is the single source of truth and is
machine-interlocked by `tools/doc_audit.py` in CI, so a cloned tag's expected
counts never silently drift from its code.

```bash
cargo build --release        # must finish with zero warnings (repo rule)
cargo test --release         # expected: 593 unit + 40 integration = 633 passing (v1.14.0)
```

The benchmark suite and its two independent backends (tree-walking
interpreter and WASM):

```bash
# native backend — expected 129/129
powershell -ExecutionPolicy Bypass -File eval/runner/run.ps1 -Verify -LomBin ./target/release/lom.exe
# WASM backend  — expected 129/129
powershell -ExecutionPolicy Bypass -File eval/runner/run.ps1 -Verify -LomBin ./target/release/lom.exe -Backend wasm
# error_repair prompts embed real compiler output — expected 32/32, exit 0
python tools/eval_prompt_check.py
```

Scoring criterion (identical in verify and candidate mode, both backends):
candidate stdout must equal `expected` byte-for-byte after CRLF→LF
normalization (case-sensitive), **and** the process exit code must be 0 —
"prints the right thing, then crashes" does not pass. Missing candidate
files count as failures. Diagnostics go to stderr and are never part of the
comparison.

The self-hosting chain (the "compiler compiles itself, provably" claim):

```bash
python tools/verify_selfhost.py      # expected 162/162 (dump mode; more modes via --tokens/--diags/--static/--run/--wasm)
python tools/verify_selfcomp.py      # expected 368/368 (196 single-file + 8 packages + 164 negatives)
python tools/verify_selfcomp.py --bootstrap   # expected 14/14; asserts the strong quine of 272925 bytes, byte-identical
```

The remaining regression surface (each with its expected result stated in
[HANDOVER.md §2.2](HANDOVER.md); most are wired into CI): the fmt gate over
the example corpus, the `stmt_interp` golden byte-compare, playground smoke,
`python tools/l2fix.py --self-check`, `python tools/doc_audit.py` (document
number interlock), `python tools/link_check.py` (broken-link gate),
`python tools/spec_examples_check.py`, `python tools/fuzz_smoke.py`, and the
differential-test trio `python tools/diff_test.py --probe | --rounds 20
--seed-base 1 --ci | --pkg --rounds 10 --seed-base 1 --ci`.

Everything in this section is deterministic: same tag in, same result out, on
all three CI operating systems.

## 3. Reproduce the LLM pass-rate experiments

### 3.1 What the benchmark is

RepairLoopBench: 129 tasks across 10 categories, of which 32 are
`error_repair` tasks (the repair loop is the point of the suite — the
category hands the model broken code plus verbatim `lom --check --json`
diagnostics captured from the real compiler). Task ids run 001–130 with one
known, accepted gap (no task 108). The machine-readable definition of record
is [eval/manifest.json](../eval/manifest.json); the benchmark card, including
how it differs from SWE-bench-family suites, is
[eval/README.md](../eval/README.md).

### 3.2 The published runs you are reproducing

| Run (report) | Date | Task set | Models | Sampling | Headline result |
|---|---|---|---|---|---|
| [Baseline](../eval/REPORT.md) | 2026-08-03 | 100 tasks | one web-hosted expert model, thinking mode (name/temperature not recorded — see its honesty section) | 1 sample | 99/100 |
| [Multi-model](../eval/REPORT-2026-08-31-multimodel.md) | 2026-08-31 | 113 tasks | deepseek-v4-pro+thinking, deepseek-v4-flash, glm-4.7, glm-5.3 | 1 sample each, provider-default temperature | 113/113 and 112/113 ×3 (sole failure: task 078) |
| [pass@k](../eval/REPORT-2026-09-07-passk.md) | 2026-09-07 | 116 tasks | deepseek-v4-pro+thinking, glm-5.3 | 10 samples each @ temperature 1.0 | pass@1 = pass@5 = pass@10 = 99.1%, both models |
| [Full-set pass@k (headline)](../eval/REPORT-2026-10-07-passk-128.md) | 2026-10-07 | 128 tasks | deepseek-v4-pro+thinking, glm-5.3 | 10 samples each @ temperature 1.0, max_tokens 16384 | pass@1 = 98.3% / 98.5%; pass@5 = 99.2% / 98.8%; pass@10 = 100.0% / 99.2% |

The 129th task (130, a runtime-diagnostic repair face) was added on
2026-10-13, after the headline run; it has no published multi-sample
measurement. Expectations for two specific tasks are pre-registered in the
reports and apply to any reproduction: **078** is the prompt-ambiguity anchor
(expect 0–1 of 10; its clarified twin 118 passes 10/10), and **127** is the
deliberately hard L2 cross-compiler repair (expect low single digits of 10).

### 3.3 The pipeline, step by step

**Step 0 — prompt regeneration (optional, proves the prompts are derived,
not hand-written).** The per-category prompt files are generated from the
task JSON plus a shared language-context header and output-format footer:

```bash
powershell -ExecutionPolicy Bypass -File eval/prompts/_generate.ps1
```

This rewrites `eval/prompts/01_arithmetic.md` … `10_error_repair.md` from
`eval/tasks/*.json`. At the time of writing the ten files total ~118 KiB
(measured 120,937 bytes; the error-repair prompt is the largest at 30,406
bytes). Each file is fully self-contained — it embeds a Lom language
overview, so a model needs no other context.

**Step 1 — collect candidates.** Two supported routes:

- *API route* (what the published runs used): `eval/llm_eval.py` speaks the
  OpenAI-compatible chat-completions protocol to DeepSeek and GLM endpoints.
  Keys come from `DEEPSEEK_API_KEY` / `GLM_API_KEY` / `GLM_CODING_API_KEY`
  environment variables or an `eval/.api_keys.json` file (gitignored; never
  committed). Example matching the headline run's shape:

  ```bash
  python eval/llm_eval.py --provider deepseek --model deepseek-v4-pro --thinking \
      --samples 10 --temperature 1.0 --out-name my_repro
  python eval/llm_eval.py --provider glm-coding --model glm-5.3 \
      --samples 10 --temperature 1.0 --out-name my_repro_glm
  ```

  Call-shape note (matters for budgeting): the pipeline sends **one whole
  category prompt file as a single user message** and expects all tasks of
  that category in one reply, fenced as `=== NNN.lom ===` blocks. A full
  one-model, 10-sample re-run is therefore **100 calls** (10 categories × 10
  samples), not per-task calls. `max_tokens` defaults to 16384. The script
  records model/temperature/thinking/timestamps/extraction stats in
  `run_meta.json` and supports resume (already-archived samples are skipped,
  zero duplicate cost).

- *Any-model route*: paste each `eval/prompts/NN_*.md` into any chat model
  (web UI included), save the full reply, and split it into per-task
  candidate files with `eval/prompts/extract.ps1`, or any tool that honors
  the `=== NNN.lom ===` convention.

**Step 2 — score.** Identical scorer for every route, the same one the
published runs used ("the scoring oracle is not duplicated"):

```bash
powershell -ExecutionPolicy Bypass -File eval/runner/run.ps1 \
    -CandidatesDir <dir-with-NNN.lom-files> -LomBin ./target/release/lom.exe
```

Overall and per-category pass rates; exit code 0 only if every task passes.

**Step 3 — pass@k.** For multi-sample directories (`s1/` … `s10/`
subdirectories as produced by `llm_eval.py --samples 10`):

```bash
python eval/passk_summarize.py --dir <run-dir> --ks 1 5 10 --lom-bin ./target/release/lom.exe
```

This drives the same runner per sample, builds the per-task pass matrix, and
computes the unbiased HumanEval-style estimator
`pass@k = 1 − C(n−c, k)/C(n, k)` (n = samples per task, c = passes; exact
integer arithmetic). The output `passk_summary.json` contains the full
per-task matrix — that is the artifact to attach when reporting.

### 3.4 The one raw LLM artifact that ships in the repository

The 2026-08-03 baseline's 100 candidate files and the 10 raw model replies
are tracked in-tree under `eval/candidates/` (plus the per-category reply
transcripts `output_01.txt` … `output_10.txt`). You can re-score them today
against the current toolchain:

```bash
powershell -ExecutionPolicy Bypass -File eval/runner/run.ps1 \
    -CandidatesDir eval/candidates -LomBin ./target/release/lom.exe -Verbose
```

Verified result on the current release: **99 passed of the 100 baseline
tasks, sole failure task 078** (the runner also reports the 29 tasks that did
not exist at baseline time as missing — they postdate the run). This is the
only byte-level LLM-provenance check available from the repository alone.

### 3.5 Cost expectations

Order of magnitude only — re-check provider pricing before budgeting. With
the whole-category call shape (100 calls per full one-model 10-sample re-run,
~12 KiB prompt in and a few KiB of code out per call), frontier per-token
prices put a full re-run in the **single-digit to ~15 USD band** for typical
frontier models (OpenAI gpt-5.x, Claude Sonnet 5.5, Gemini 4 Argon; public
per-token pricing checked 2026-10-20, typical-to-conservative). The
maintainer's published runs used a pay-per-use DeepSeek API key and a GLM
Coding Plan subscription endpoint respectively.

## 4. What you can and cannot verify from a fresh clone

**You can, deterministically:**

- the full engineering baseline of Section 2 (build, 633 tests, dual-backend
  129/129, prompt-diagnostic fidelity 32/32, self-hosting quine of 272925
  bytes) on any of the three CI operating systems;
- benchmark integrity: reference solutions pass, ids are unique, and every
  `error_repair` prompt's embedded diagnostics still match the live
  compiler's `--check --json` output (that is what `eval_prompt_check.py`
  proves);
- re-score the tracked 2026-08-03 baseline candidates (99/100, sole failure
  078) — Section 3.4;
- run the LLM experiments yourself, end to end, with any model and budget,
  using the same prompts, scorer, and estimator as the published runs.

**You cannot:**

- re-audit the raw provenance of the 2026-08-31 / 2026-09-07 / 2026-10-07
  runs: their raw model replies and per-task pass matrices live in
  `eval/candidates_rerun/`, which is gitignored by design (single-machine
  archives; the boundary is registered in [eval/README.md](../eval/README.md)
  and restated in each report's honesty section). Published numbers were
  transcribed from those archives' `passk_summary.json` outputs;
- obtain byte-identical numbers from the same model names: the evaluated
  models are commercial endpoint snapshots, not fixed weights, so a
  reproduction is a **same-pipeline behavioral comparison**, not a bit-exact
  replay — differences within the pre-registered expectation band (Section
  3.2) are model-version drift, not pipeline error;
- re-run the 2026-08-03 baseline model itself: it was a web-hosted session
  whose model name and temperature were not recorded (the report says so).

## 5. Reporting results

Open a GitHub issue on `lom-lang/lom` with:

1. **Setup** — model name, endpoint/provider, temperature, thinking on/off,
   sampling count, sampling date, and the repository tag you ran against.
2. **Commands** — the exact `llm_eval.py` / runner / `passk_summarize.py`
   invocations (or the any-model route description).
3. **Results** — overall pass rate, per-category table, and for multi-sample
   runs the pass@1/5/10 values plus the per-task matrix (attach or paste the
   `passk_summary.json` matrix section).
4. **Divergences** — anything outside the pre-registered expectation band
   (078 and 127 low by design; everything else should sit near the published
   values for frontier models). Model-version drift is the expected
   explanation domain for small deltas; scorer or task discrepancies are
   bugs and will be treated as such.

Reports following this format are the input the maintainers use to update
the evidence tables — negative results included.
