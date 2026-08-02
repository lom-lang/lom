# Lom

**Lom (Language of Machine)** — an AI-native programming language.

LLM-coding-native first, workloads later. Built in Rust.

## Status

✅ **Phase 1 — Minimal Interpreter** (completed)

- 13/13 `.lom` examples pass (`examples/*.lom`)
- 20/20 Rust unit tests pass (`cargo test`)
- Implements: `fn` / `let` / `let mut` / `if`/`elif`/`else` / `while` / `for` / `return`, Int/Float/Bool/String/Unit, closures (first-class), builtins (`println`/`print`/`int_to_string`/`string_to_int`/`len`)

✅ **Phase 2 — LLM-coding-native Core** (completed)

- ✅ 2.1.1 `match` + `enum` + Result/Option + pattern matching
- ✅ 2.1.2 `?` error propagation (Result/Option)
- ✅ 2.1.3 `|>` pipeline operator
- ✅ 2.1.4 structural records `{x: Int, y: Int}` + tuples
- ✅ 2.1.5 explicit imports `from mod import {name as alias}` (stdlib: io/string/math)
- ✅ 2.2 tolerant parser with holey AST (`Stmt::Hole`, sync-point recovery, all errors collected)
- ✅ 2.3 structured JSON diagnostics (`lom-diag/v1` schema, `--json` / `--check` / `--help` CLI, LEX/PARSE/RUNTIME error codes)
- ✅ 2.4 gradual type checker (two-pass: signature collection + body check; TYPE/MAT/NAM error codes; `--check` runs type check, `--json` emits structured type diagnostics; progressive: type errors are warnings, dynamic run still works)
- ✅ 2.5 explicit effect system (`! [IO, Clock]` annotation; `EFF001` warning when pure functions call effectful functions; `main` implicitly has all effects; closures inherit enclosing effects; stdlib `println`/`print` declare `[IO]`)
- ✅ 2.6 `lom info --json` type info export (`lom-info/v1` schema — functions/enums/imports; `info` subcommand with `--json`; no type-check, no run; parse failure falls back to `lom-diag/v1`)
- ✅ 2.7 `lom fix --plan --json` AI repair plan (`lom-fix/v1` schema — per-diagnostic plans with `insert`/`delete`/`hint` actions, `text` snippets for EFF001/MAT001, `retry` flag, `confidence` levels; covers 20+ error codes across LEX/PARSE/TYPE/MAT/NAM/EFF/RUNTIME; `fix` subcommand with `--json`)
- ✅ 2.8 `eval/` 100-task benchmark suite (10 categories × {arithmetic, control_flow, types, closures, match_enum, pipeline, records_tuples, effects, modules, error_repair}; per-task `{id, prompt, solution, expected, notes}`; PowerShell + Bash runners with `--verify` reference-solution smoke test and `--candidates-dir` LLM evaluation; 100/100 reference solutions pass; **LLM 实测 99/100 (99%)** — error_repair 15/15, 0 syntax/import errors; see [eval/REPORT.md](eval/REPORT.md))
- ✅ 3.1 `lom fix --apply` repair execution (`lom-apply/v1` schema — applies `confidence=High` + `action≠Hint` fixes to source; `--dry-run` preview, `--json` structured output; text patching via line/col offsets, reverse-order application to avoid drift; EFF001 upgraded from Hint to precise Insert — pure functions get `! [E]` appended, partial-effect functions get `, E` inserted before `]`)

159/159 Rust unit tests pass. 22 `.lom` examples pass (both run and `--check`). `eval/` 100/100 reference solutions pass (`./eval/runner/run.ps1 -Verify`). **LLM generation pass-rate: 99%** (expert model + thinking mode, 2026-08-03).

Next: **Phase 3 — Usable MVP** (Cranelift JIT, standard library expansion, complete CLI tool / simple web service). `--apply` repair execution done in 3.1.

See [`docs/lom-project-guide.html`](docs/lom-project-guide.html) for the full project guide (positioning, design philosophy, 7-phase roadmap, target LLM strategy, risk mitigation, repo governance).

## What is Lom

Lom is a **progressively fused** AI-native programming language:

1. **Phase 0-3 — LLM-coding-native**: tolerant parser, structured JSON diagnostics, linear pipeline syntax, structural types, gradual typing, explicit effect system. Target: LLMs write Lom with low error rate and easy recovery.
2. **Phase 4+ — Workload-native (adjustable milestone)**: tensor as first-class type, automatic differentiation, MLIR heterogeneous compute, Python interop. Target: AI/ML workloads.

## Why Lom

The "AI-native language" space is currently split in two directions, both with gaps:

| Direction | Leaders | Gap |
|---|---|---|
| Workload-native (tensor / autodiff / GPU) | Mojo, NEURON, Bend | Crowded. Mojo has 175k devs. Red ocean. |
| LLM-coding-native (tolerant parse / JSON diagnostics / linear syntax) | MoonBit (tooling side), Zero (JSON diagnostics) | Empty. Only MoonBit has intent, still in early stage. No public eval. |

Lom takes the **flanking route**: occupy the empty LLM-coding-native lane first with a differentiated position, then extend to workloads later.

## Design Philosophy

1. **Locality > Globality** — declarations and usage must close within the same view.
2. **Tolerance > Strictness** — syntax errors non-fatal, type errors degradable.
3. **Linear > Nested** — pipeline, early return, guard clauses over deep nesting.
4. **Self-describing > Implicit convention** — intent in code (executable docstrings, schema-as-type).

## Roadmap (summary)

| Phase | Name | Duration | Exit criteria |
|---|---|---|---|
| 0 | Language Design & Spec | 1-2 w | LLM reads spec, generates valid code >80% |
| 1 | Minimal Interpreter | 4-6 w | 10+ test cases pass with LLM-generated code |
| 2 | LLM-coding-native Core | 6-10 w | LLM generation pass-rate eval set meets baseline |
| 3 | Usable MVP | 8-12 w | Write a complete CLI tool or simple web service |
| 4 | Workload Extension (adjustable) | 12-20 w | Write a complete MNIST training script |
| 5 | Ecosystem & Bootstrapping | 16-24 w | Compiler self-hosts |
| 6 | Production | ongoing | Third-party projects deployed in production |

See the [project guide](docs/lom-project-guide.html) for details.

## Target LLM Priority

| Priority | LLM | Phase | Adaptation focus |
|---|---|---|---|
| P0 | DeepSeek | Phase 0 | SPEC_FOR_AI readability, error JSON field response, error pattern sampling |
| P1 | Claude / GPT | Phase 2 | Cross-model consistency baseline |
| P2 | Kimi / GLM / Gemini | Phase 3+ | Coverage expansion |

## Tech Stack

- **Host language**: Rust
- **Phase 1**: logos (lexer) + hand-written recursive descent (parser) + tree-walking (interp)
- **Phase 3**: + Cranelift JIT
- **Phase 4**: + MLIR (melior) + pyo3
- **Phase 5**: + LLVM AOT (inkwell) + WASM

## License

Apache-2.0. See [LICENSE](LICENSE).

## Contact

- GitHub: [lom-lang/lom](https://github.com/lom-lang/lom)
- Issues: <https://github.com/lom-lang/lom/issues>
