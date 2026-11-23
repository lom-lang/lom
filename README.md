# Lom

**Lom (Language of Machine)** — an AI-native programming language designed around the LLM repair loop: when a model writes broken Lom, the language itself is built to diagnose it precisely and fix it mechanically.

LLM-coding-native first, workloads later. Built in Rust.

[![CI](https://github.com/lom-lang/lom/actions/workflows/ci.yml/badge.svg)](https://github.com/lom-lang/lom/actions/workflows/ci.yml)
![deps](https://img.shields.io/badge/dependencies-0-2ea44f)
![unsafe](https://img.shields.io/badge/unsafe-0-blue)
![license](https://img.shields.io/badge/license-Apache--2.0-9cf)

## Why Lom exists

Most languages treat "an AI wrote broken code" as someone else's problem. Lom inverts that: **repair is the language's reason for existing.** Every rejection carries a structured diagnostic code and, where a mechanical fix is safe, `lom fix` applies it for you — the same interface an LLM agent uses. The whole toolchain (diagnostics → `lom fix` plans → applied patches) is machine-readable by design.

## The evidence

| Claim | Receipt |
|---|---|
| **The repair loop works on real models** | On the then-full **128-task set** (10 categories; the suite has since grown to 129 via task 130), deepseek-v4-pro+thinking and glm-5.3 score **98.3% / 98.5% pass@1** (2 models × 10 samples @ t=1.0; pass@5 99.2% / 98.8%) — the only sub-100 tasks are a known prompt-ambiguity anchor and one deliberately hard cross-compiler repair — [report](eval/REPORT-2026-10-07-passk-128.md) |
| **The compiler compiles itself — provably** | `self_comp.lom` (12,671 lines of Lom) compiles its own source to WASM; the result is byte-identical from both sides (a 272,925-byte strong quine, re-verified on every CI run) |
| **A trustable core** | Zero third-party crates, zero `unsafe`, 635 unit + 58 integration tests, three-OS CI; the language surface (syntax / 20 keywords / diagnostic codes / 43 builtins) is [frozen](LANGUAGE_SPEC.md#14-v10-freeze-declaration-2026-09-02) — changes require an RFC |

## Feel the repair loop in 60 seconds

`hello.lom` — you (or your model) forgot the import:

```lom
fn main() -> Unit
    println(len("hello"))
end
```

Run it, and the diagnostic tells you exactly what to do — then let the language do it:

```
$ lom hello.lom
[runtime] error (2:13): [RUNTIME002] 符号 'len' 未导入。需在文件顶部声明：from string import {len}

$ lom fix hello.lom --apply
lom apply: hello.lom（迭代 2 轮）
  round 1: applied 1, skipped 0
    [1:1] insert (NAM005) — 在文件顶部插入 'from string import {len}'
  最终诊断（修复后源码）: 0 错误 / 0 警告

$ lom hello.lom
5
```

That insert is a `high`-confidence mechanical action; ambiguous cases surface as hints instead of silent rewrites. The same `lom fix` plan format (`lom-fix/v1`) is what LLM agents consume — see [SPEC_FOR_AI.md](SPEC_FOR_AI.md).

## Quick start

```powershell
cargo build --release            # builds target/release/lom (zero dependencies)
./target/release/lom examples/fib.lom    # run a program
```

A minimal Lom program (`fn` + tail-expression return; blocks close with `end`):

```lom
fn main() -> Unit
    println("hello, lom")
end
```

Save as `hello.lom`, run `lom hello.lom`. When a run fails at run time, `lom hello.lom --trace` prints a call-level JSONL event stream to stderr (`--trace-out <file>` to redirect, `--trace-max-events <n>` to cap, `--trace-builtins` to include builtin calls) — see [SPEC_FOR_AI.md §11g](SPEC_FOR_AI.md); AOT `lom build` artifacts never pass through the interpreter, so `--trace` does not apply to them. Since v1.17.0 a run-time error reports the line and column of the failing expression, and `lom fix hello.lom --plan --run` lets the fixer observe a run-time failure first-hand — it executes the program once under a time budget (stdout discarded, opt-in, side-effect-gated) and adds what it sees to the plan — see [SPEC_FOR_AI.md §11e](SPEC_FOR_AI.md#11e-ai-repair-plan-phase-27); the same AOT caveat applies: `lom build` artifacts fail without a position. New here? Read [SPEC_FOR_AI.md](SPEC_FOR_AI.md) (the LLM-facing language spec). AI maintainers start with the [copy-ready handoff prompt](docs/HANDOFF_PROMPT.md), the [maintenance protocol](docs/MAINTENANCE.md), then [docs/HANDOVER.md](docs/HANDOVER.md) — internal governance and working history, maintained in Chinese (see [docs/README.md](docs/README.md) for the map of English vs Chinese surfaces). 中文读者：[README.zh-CN.md](README.zh-CN.md)。

## What is Lom

Lom is a **progressively fused** AI-native programming language:

1. **Phase 0-3 — LLM-coding-native**: tolerant parser, structured JSON diagnostics, linear pipeline syntax, structural types, gradual typing, explicit effect system. Target: LLMs write Lom with low error rate and easy recovery. (Done — original 99/100 single-model baseline 2026-08-03 [REPORT.md](eval/REPORT.md); multi-model validation followed 2026-08-31: 99.1-100% across 4 models on the full 113-task set [multimodel report](eval/REPORT-2026-08-31-multimodel.md); single-run sampling closed by the 2026-09-07 pass@k retest — 2 models × 10 samples @ temperature=1.0, pass@1/5/10 = 99.1% each ([pass@k report](eval/REPORT-2026-09-07-passk.md)); the cross-language control group remains open, stated as such)
2. **Phase 4 — LLM-repair-native + toolchain** (direction adjusted 2026-08-07): expand `lom fix` in-place auto-repair (differentiation — MoonBit doesn't do in-place repair), add REPL / LSP / package manager. Original "workload-native" (tensor/autodiff/MLIR) dropped — Mojo (acquired by Qualcomm) saturates the AI-compute lane.
3. **Phase 5-7 — ecosystem & backends** (done 2026-08): bootstrap self-hosting feasibility (a 1450-line mini-interpreter written in Lom), production engineering (semver, 3-platform CI, `lom doc`/`fmt`), and a hand-written zero-dependency **WASM compiler backend** (`lom build --target wasm`) with byte-identical dual-backend output. Since 2026-08-25: fix-engine deepening (M1-M4 — spelling-repair `replace` actions, iterative `--apply` loop, targeted PARSE001 fixes, expanded error_repair eval).

## Why Lom

The "AI-native language" space (2026-08 retrospective):

| Direction | Leaders | Status |
|---|---|---|
| Workload-native (tensor / autodiff / GPU) | Mojo (acquired by Qualcomm $3.9B), Bend | Saturated. Mojo binds to Qualcomm hardware strategy. Not Lom's lane. |
| LLM-coding-native (write-it-right) | MoonBit (IEEE TSE 2026 paper: 1/7 corpus but 2x AI gen rate vs Gleam; ~400k users; 1.0 imminent Sep 2026) | MoonBit leads. Lom's 99% eval validates the same thesis. |
| **LLM-repair-native (fix-it-fast)** — Lom's lane | Lom (`lom fix --apply` iterative in-place repair, tolerant parse, gradual typing) | **Empty.** MoonBit regenerates on error; Lom patches in-place. Complementary, not competitive. |

Lom takes the **differentiated route**: occupy the empty LLM-repair-native lane — in-place auto-repair (`lom fix --apply`), tolerant parsing, confidence-graded fixes — complementary to MoonBit's write-it-right approach.

## Design Philosophy

1. **Locality > Globality** — declarations and usage must close within the same view.
2. **Tolerance > Strictness** — syntax errors non-fatal, type errors degradable.
3. **Linear > Nested** — pipeline, early return, guard clauses over deep nesting.
4. **Self-describing > Implicit convention** — intent in code (executable docstrings, schema-as-type).

## Status & boundaries (honest version)

- Current: **v1.17.0** (2026-11-23). Language surface frozen since v1.0 (2026-09-02) — zero surface changes since; every release since has been tooling/evidence work (runtime-error positions and `lom fix --run`, the agent execution trace `--trace`, online playground incl. the in-browser self-hosting demo and share links, ACRI native conformance with `lom explain`, the iterative-repair benchmark category, structured diagnostics, remediation). Full numbers: see [Benchmark and live numbers](#benchmark-and-live-numbers) below; the per-release log: [CHANGELOG.md](CHANGELOG.md); the Phase 1–8 feature checklist: [docs/milestone-history.md](docs/milestone-history.md).
- The **L2 self-hosted subset compiler** (`examples/selfhost/self_comp.lom`) is a **supported face** as of **v1.8.0** (2026-10-08; promoted from experimental by user adjudication after four consecutive A- review rounds, a zero-open ledger, and the v1.7.2 line-number batch completing its repair loop): it compiles a strict subset of Lom to WASM, rejects some programs the host accepts (each rejection carries an `[L2xxx]` code and a repair hint via `lom l2fix` / [tools/l2fix.py](tools/l2fix.py); rejections whose message carries a quoted name also carry a `ln:cl` source position — the registered 71/102 coverage boundary with 31 exempt type-string cases), and self-hosts provably.
- The 38 review rounds are **in-system adversarial reviews** (agent-run, evidence-based) — good hygiene, **not external peer review**; grades never extrapolate across baselines.
- **[Online playground](https://lom-lang.github.io/lom/playground/)** — run Lom, see structured diagnostics, and apply `lom fix --apply` with a line diff, entirely in your browser (the full host toolchain compiled to `wasm32-wasip1`, zero third-party JS). Pressing Run quietly folds your program into the URL hash (UTF-8-safe), so opening the shared link restores your exact editor content.
- Honest note on the exit criterion: "a third-party project deployed to production with Lom" cannot be self-certified — the engineering surface is done; adoption remains the long-term north star

## Benchmark and live numbers

**RepairLoopBench — the 138-task repair-suite benchmark** ([eval/README.md](eval/README.md) is its benchmark card): a contamination-safe, dual-backend benchmark for the repair loop — its error_repair tasks hand models real structured diagnostics (verbatim host `--check --json` plus the L2 cross-compiler's `[L2xxx]` channel) and score convergence through the advisory fix loop; a scope statement distinguishing it from SWE-bench-family suites is on the card. Headline: **full-set pass@1 = 98.3% / 98.5% across two frontier models** (10 samples each, unbiased estimator, 2026-10-07 — [full-set report](eval/REPORT-2026-10-07-passk-128.md)). For citations, see the **Citing** section of the benchmark card (cite the tag, not `main`).

**Verify the claims** (from a fresh clone; ~5 minutes — the full third-party reproduction guide, including the LLM pass-rate experiments and what a fresh clone can and cannot verify, is [docs/REPRODUCING.md](docs/REPRODUCING.md)):

```bash
cargo build --release && cargo test --release     # 635 unit + 58 integration tests
python tools/verify_selfcomp.py                   # L2 self-compilation: 368/368
python tools/verify_selfcomp.py --bootstrap       # three-layer bootstrap 14/14; the self-hosted
                                                   # compiler regenerates a byte-identical 272,925-byte quine
powershell -ExecutionPolicy Bypass -File eval/runner/run.ps1 -Verify -LomBin ./target/release/lom.exe   # eval 138/138
./target/release/lom.exe codes --json | python -c "import json,sys; print(json.load(sys.stdin)['total'])"  # 46
```

635/635 Rust unit tests pass (plus 58 process-level integration tests: CLI exit codes, LSP stdio e2e incl. the R63 transport-boundary pair and the R71/R72 transport-corner trio, the R94 in-package-alias pair, the R104 same-name-package determinism ×6 in `tests/r104_dedup_order.rs`, and the R108 variant-externals ×5 in `tests/r108_variant_externals.rs`, and the R107 alias-clash ×4 in `tests/r107_alias_clash.rs`, and the closure-externals ×3 in `tests/build_closure_externals.rs`, and the runtime-error-position ×6 + `fix --run` ×12 pair in `tests/d0028_runtime_position.rs` / `tests/d0028_fix_run.rs`). 38 `.lom` files (29 examples + 4 bootstrap self-hosting files + 2 in the `pkg_demo` package + 3 self-hosted sources in `examples/selfhost/` — the 5727-line reference interpreter `self_interp.lom`, the L2.1 encoder spike `l2_spike.lom`, and the 12671-line L2.3 subset compiler (v1.7.2; tagged v1.6.0 source was 12572 lines) `self_comp.lom`; `apply_test.lom` is an intentionally-broken `lom fix` demo). `eval/` 138/138 reference solutions pass on both backends (tree-walking interpreter and WASM; `./eval/runner/run.ps1 -Verify -LomBin target/release/lom.exe`, `-Backend wasm` for the WASM side). **LLM generation pass-rate: 99.1-100% across 4 models** (deepseek-v4-pro+thinking / v4-flash / glm-4.7 / glm-5.3, 2026-08-31, full 113-task set as it stood then — [multimodel report](eval/REPORT-2026-08-31-multimodel.md)); original 99/100 baseline: 2026-08-03, 100-task set ([REPORT.md](eval/REPORT.md)). **pass@k retest (2026-09-07)**: 2 models × 10 samples @ temperature=1.0 on the 116-task set — pass@1 = pass@5 = pass@10 = **99.1%** for both deepseek-v4-pro+thinking and glm-5.3 (unbiased estimator; only task 078, the prompt-ambiguity anchor, fails systematically — 0/10 both models) — [pass@k report](eval/REPORT-2026-09-07-passk.md). **full-set pass@k (2026-10-07, the current headline evidence)**: the same 2 models × 10 samples @ temperature=1.0 on the **full 128-task set** (first multi-sample measurement for the v1.5.0-era additions; tasks 126/127 tightened the same day per the ambiguity-fix lineage 078→118→126) — **pass@1 = 98.3% (deepseek-v4-pro+thinking) / 98.5% (glm-5.3)**, pass@5 = 99.2% / 98.8%, pass@10 = 100.0% / 99.2%; sub-100 tasks fully attributed: 078 (known prompt-ambiguity anchor, its explicit twin 118 passes 10/10 both models) and 127 (deliberately hard L2-cross-compiler repair where 18/20 candidates miss one Option-unwrapping step) — [full-set report](eval/REPORT-2026-10-07-passk-128.md). **error_repair hot-patch sampling (2026-09-16)**: the same 2 models × 10 samples @ temperature=1.0 on all 24 error_repair tasks (including the four warning-level repair tasks 119/120/122 and LEX005 task 121) — **24/24 tasks × both models = 24/24 each, zero failures** — [patch report](eval/REPORT-2026-09-16-err-repair-patch.md). **iterative-repair first sampling (2026-10-28, the new category's debut)**: both models × 10 samples, single-step feedback chain, R=5 — **C(2) = 100% for both** (glm-5.3 C(1) 83.3% → C(2) 100%; deepseek-v4-pro+thinking 91.1% → 100%; zero censored at R=5); the headline hypothesis confirmed on task 131 (the known-hard 127 copy): single-round 30% on both models, and **all 14 first-round failures converge in round 2 once fed the real diagnostics (14/14)** — against the historical no-feedback baseline where 18/20 candidates miss the same Option-unwrap step: the repair loop's closure value, quantified — [first iterative report](eval/REPORT-2026-10-28-iter-repair-first.md). The latest committed `main` milestone is v1.17.0 (the runtime-error positions + `lom fix --run` batch — run-time diagnostics point at the failing expression, and `lom fix --run` observes a run-time failure under a time budget; 35 new unit tests, 18 new integration tests), preceded by v1.16.0 (the iterative-repair task category — 9 new tasks measuring how many rounds a fix takes; task count 129 to 138), preceded by v1.15.0 (the ACRI native-conformance + `lom explain` batch — a nested `position` object in the check envelope and per-code long-form documentation; 7 new unit tests), preceded by v1.14.0 (the agent execution-trace batch — `--trace` call-level JSONL events for the interpreter run mode; 12 new unit tests), preceded by v1.13.0 (the playground share-links batch — Run folds the program into the URL hash and shared links restore the editor; suite 129 tasks), preceded by v1.12.1 (the wasm file-builtins fix — the playground engine's `file_exists`/`file_append` defects repaired at the WASI-harness layer per designs/0025; suite 129 tasks), preceded by v1.12.0 (the playground self-hosting demo — an in-browser four-step chain producing the 272925-byte quine byte-identically; tag `4a64471`), preceded by v1.11.0 (the RepairLoopBench runtime-diagnostic task 130 + the v1.10.3 claims precision-correction; tag `e83177b`). The twentieth in-system adversarial review (2026-09-30, covering the v1.4.2→v1.4.7 six-batch increment) rated the repository **A-** (zero claim distortion, 22 probes zero silent miscompiles); the twenty-first review (same day, covering the v1.4.8/v1.4.9 R94-closure batches) verified both batches' claims loss-free with the new behavior surfaces probe-clean, and opened R104/R105 (P1: interpreter cross-run nondeterminism on same-name package symbols, and opposite shadowing direction for alias-vs-user-fn — both pre-existing name-collision surfaces awaiting adjudication) plus R106 (P3 documentation staleness, fixed on the spot). The twenty-second review (same day, baseline 92502e2) confirmed the R104/R105 closure conditions loss-free and rated the repository **B+** (rebound per trajectory), opening R107 (P1, pre-existing: two packages importing the same alias with different real names resolve differently across backends, silently) / R108 (P2, pre-existing: package enum variants falsely rejected by `--check`) / R109 (P3, introduced by v1.4.10: merged-unit NAM006 noise on plain single-package imports) for adjudication.

The Phase 1–8 milestone checklist (the full feature history, kept as written at the time): [docs/milestone-history.md](docs/milestone-history.md).

## Release log

> **Status: v1.0 — language surface FROZEN (2026-09-02)**. Frozen: syntax / 20 reserved words / diagnostic codes / 43 builtins ([spec §14](LANGUAGE_SPEC.md)). Changes require a new RFC.
> **Current release: v1.17.0 (2026-11-23; runtime errors now say where they happened, and `lom fix` can watch a program fail — per [designs/0028](docs/designs/0028-runtime-repair-loop.md); versioned minor because it adds a CLI flag, like v1.14.0's `--trace`; zero syntax / keyword / diagnostic-code / builtin change)** — A program can pass every static check and still crash at run time. Until now that crash came back as a bare `[runtime] error (0:0)` — no line, no column — and `lom fix` had nothing to say about it. This release closes that gap in two steps. **Runtime errors carry a position**: the diagnostic names the innermost expression that failed and shows it with a caret (`[runtime] error (4:13): [RUNTIME000] 整数除以零`), so you — or your model — land on the offending line instead of re-reading the whole file. **`lom fix --run`** goes one step further: `lom fix hello.lom --plan --run` runs the program once (its output is discarded) and adds the observed failure, position included, to the repair plan as a hint. Running a program is a side effect, so this is opt-in and fenced three ways: a file with static errors is not run; a file that mentions `file_write` / `file_append` is refused unless you add `--allow-io`; and every run has a time budget (5000 ms by default, `--run-budget <ms>` to change) — a runaway loop reports `"outcome": "timeout"` instead of hanging. Advice learned from a run is never applied automatically: `--apply --run` reports it and leaves your file exactly as `--apply` alone would. Without `--run`, `lom fix` output is byte-for-byte unchanged. The limits, stated plainly: the position is the start of the failing expression, not the exact operator; errors raised outside expression evaluation (a bad import, a tuple-destructuring mismatch, a non-iterable `for`, a non-`Bool` `while` condition) still report `(0:0)`; and ahead-of-time `lom build` artifacts still fail without a position. Baseline unchanged: quine **272925**, verify_selfcomp 368/368; tests 600+40 → 635+58.
>
> Full release log — every version, as written at the time: [CHANGELOG.md](CHANGELOG.md).

## Roadmap (summary)

| Phase | Name | Duration | Exit criteria |
|---|---|---|---|
| 0 | Language Design & Spec | 1-2 w | LLM reads spec, generates valid code >80% |
| 1 | Minimal Interpreter | 4-6 w | 10+ test cases pass with LLM-generated code |
| 2 | LLM-coding-native Core | 6-10 w | LLM generation pass-rate eval set meets baseline |
| 3 | Usable MVP | 8-12 w | Write a complete CLI tool or simple web service | ✅ Done (3.5 todo CLI) |
| 4 | LLM-repair-native + toolchain (adjusted 2026-08) | — | `lom fix` covers main diagnostic auto-repair + confidence model + REPL + LSP + package manager |
| 5 | Ecosystem & Bootstrapping | done 2026-08-19 | Bootstrap feasibility proven (mini-interpreter in Lom; full self-hosting = RFC-0003 draft, parked) |
| 6 | Production engineering | done 2026-08-22 | Semver, 3-platform CI, governance, `lom doc`/`fmt`, SECURITY.md |
| 7 | WASM compiler backend | done 2026-08-23 (v0.15.0) | `lom build --target wasm`; dual-backend byte-identical golden + eval 113/113×2（时点值，2026-08-23；现值见 eval/README） |
| 8 | Full self-hosting | done 2026-09-02 (v0.23-v0.26) | Frontend + static checks + evaluator in Lom (self_interp.lom); interpreter-carrier three-layer self-certification; wasm-carrier golden blocked by an unresolved OOB bug (RFC-0003 rev 21, post-1.0 ledger) |
| **v1.0** | **Language-surface freeze** | **2026-09-02 (v1.0.0)** | **Frozen: syntax / 20 reserved words / diagnostic codes / 43 builtins** (spec §14). Post-1.0 ledger: wasm OOB deep-dive, L2 self-hosted compiler (blocker resolved), Pattern spans, package registry, debugger. Long-term north star: third-party production use — explicitly **not** a v1.0 gate |

See the [project guide](docs/lom-project-guide.html) for details.

## Target LLM Priority

| Priority | LLM | Phase | Adaptation focus |
|---|---|---|---|
| P0 | DeepSeek | Phase 0 | SPEC_FOR_AI readability, error JSON field response, error pattern sampling |
| P1 | Claude / GPT | Phase 2 | Cross-model consistency baseline |
| P2 | Kimi / GLM / Gemini | Phase 3+ | Coverage expansion |

## Tech Stack

- **Host language**: Rust
- **Phase 1+**: hand-written lexer / recursive-descent parser / tree-walking interpreter / WASM emitter — **zero third-party dependencies** (CI-enforced). The crate is a binary, not a library — a deliberate keep-it-readable decision.
- **Phase 4**: reuse existing interpreter/typechecker/diagnostics — no new low-level deps (no Cranelift/MLIR). Adds `lom fix` rule expansion, REPL, LSP server, package manager.

## License

Apache-2.0. See [LICENSE](LICENSE).

## Contact

- GitHub: [lom-lang/lom](https://github.com/lom-lang/lom)
- Issues: <https://github.com/lom-lang/lom/issues>
