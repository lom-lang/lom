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
- ✅ 3.2 AST span-based diagnostic positioning (`Span` type on `FnDecl`/`EnumDecl`; parser records `fn`/`enum` keyword position + signature end via `prev_token_pos`; typechecker uses `current_fn_span` for EFF001/TYPE010/NAM002 instead of `(0,0)`; removes Phase 3.1 `find_fn_line` source-scanning hack; end-to-end verified — `effects_bad.lom` EFF001 now points to `9:1`/`20:1`)
- ✅ 3.3 Standard library expansion — `list` + `json` modules (`Value::List` variant with immutable semantics; `list` module: `list_empty`/`list_length`/`list_get`/`list_is_empty`/`list_head`/`list_tail`/`list_cons`; `json` module: hand-written zero-dependency `json_parse`/`json_stringify` with JSON↔Lom Value mapping — object→Record, array→List, string→Str, number→Int/Float, true/false→Bool, null→Unit; supports surrogate pair escapes; examples: [examples/list_demo.lom](examples/list_demo.lom) + [examples/json_demo.lom](examples/json_demo.lom))
- ✅ 3.4 Standard library expansion — `file` module + `string` extensions (`file` module: `file_read`/`file_write`/`file_append`/`file_exists` with `[IO]` effect annotation — file system read/write/append/existence; `string` extensions: `split`/`contains`/`replace`/`starts_with`/`ends_with` — split returns `List<String>` (empty sep splits by char); all string extensions are pure functions; examples: [examples/file_demo.lom](examples/file_demo.lom) + [examples/string_demo.lom](examples/string_demo.lom))
- ✅ 3.5 CLI demo + `env` module — **Phase 3 exit criterion met**: `env` module (`args()` returns `List<String>`); CLI `--` separator passes args to Lom program; **todo list CLI** ([examples/todo.lom](examples/todo.lom)) — add/list/done/remove/help with JSON persistence, recursive list traversal, effect-correct (`! [IO]` annotations); end-to-end verified all commands

**Phase 4 — LLM-repair-native + toolchain** (direction adjusted 2026-08-07):
- ✅ 4.1.1 NAM003 spelling suggestion via Levenshtein (`levenshtein()` ≤2 edit distance; candidates = scope vars + top-level fns + stdlib + enum variants; tie-break prefers longer name; `fix_nam_undefined` emits Hint with "Did you mean X?"; e2e tests: `printl`→`println`, `lengt`→`length`, no-similar-name path)
- ✅ 4.1.2 MAT001 Result/Option auto-variant-insert (`MatchExpr::end_line` records `end` keyword line; typechecker fills `end_line` into MAT001 diagnostic; `fix_mat_non_exhaustive` upgraded to graded fix — builtin variants `Ok(_)`/`Err(_)`/`Some(_)`/`None` get High-confidence precise Insert before `end`; user enum variants stay Medium Hint (params unknown, safe boundary); e2e tests: Result-missing-Err, Option-missing-None, user-enum-not-auto-applied)
- ✅ 4.1.3 Fix history record ([src/fix_history.rs](src/fix_history.rs)) — `.lom/fix-history.jsonl` NDJSON format (one JSON object per line); `lom fix --apply` auto-appends entry after successful write (timestamp/file/applied/skipped/changes with diagnostic_code); `lom fix --history [--json]` reads and displays past fixes (`lom-fix-history/v1` schema); zero-dependency ISO 8601 UTC timestamp via epoch calc; `AppliedChange` now carries `diagnostic_code` for history tracking; history write failure does not block apply success; 15 unit tests + 2 e2e tests

**Phase 4.2 — REPL** (interactive trial-and-error, LLM-friendly):
- ✅ 4.2 REPL interactive session ([src/repl.rs](src/repl.rs)) — `lom repl` subcommand; `is_input_complete()` multiline completeness check (paren/brace/bracket depth + string state + block keyword/end pairing; `enum` is single-line decl, `fn`/`if`/`while`/`for`/`match` need `end`); `ReplSession` with context-preserving incremental execution (`exec_item()` registers fn/enum/import, `exec_repl_block()` runs stmt/expr in globals scope so `let` binds persist); special commands `:q`/`:help`/`:reset`/`:show`; errors converted to output text (REPL never crashes on bad input); `lom>` / `..>` prompts for multiline; EOF (Ctrl+D) exits cleanly; 26 tests (8 completeness + 18 e2e)

**Phase 4.3 — Simple LSP** (IDE integration):
- ✅ 4.3 LSP server ([src/lsp.rs](src/lsp.rs)) — `lom lsp` subcommand; stdio JSON-RPC 2.0 protocol (`Content-Length` header + JSON payload); `handle_hover()` returns markdown type signature for fn/enum names (position-based via `FnDecl.span`/`EnumDecl.span`); `handle_completion()` returns functions + enum variants + imports + builtin variants (Ok/Err/Some/None) + keywords; `compute_diagnostics()` reuses `Diagnostics::from_parse_result` + `typechecker::check_program`; `diagnostic_to_lsp_json()` converts to LSP format (0-based line/character); handles `initialize`/`initialized`/`shutdown`/`exit`/`didOpen`/`didChange`/`hover`/`completion`; unknown methods return `-32601`; 19 tests (hover 4, completion 5, diagnostics 3, JSON-RPC 7)
- ✅ 4.4 Package manager prototype ([src/package.rs](src/package.rs)) — `lom build` subcommand + `lom.toml` manifest (`name`/`version`/`[dependencies]`); local path dependencies (`Dependency::Path`); `resolve_dependencies()` DFS recursive resolution with cycle detection (`CircularDep`); `collect_public_symbols()` auto-exposes top-level `fn`/`enum`; `process_import()` resolves external packages (PKG005 unknown package / PKG006 symbol not exported); runtime `load_packages()` registers package `fn`/`enum` variants in interpreter; 16 unit tests (manifest parse, cycle detection, symbol collection, path resolution); example: [examples/pkg_demo/](examples/pkg_demo/) — `mathlib` local dep with `square`/`cube`/`factorial`

299/299 Rust unit tests pass (294 prior + 5 new in Phase 5.5). 30 `.lom` examples (29 run + 1 `--check` diagnostic sample). `eval/` 103/103 reference solutions pass (`./eval/runner/run.ps1 -Verify -LomBin target/release/lom.exe`). **LLM generation pass-rate: 99%** (expert model + thinking mode, 2026-08-03, on the 100-task v2.8 set).

Phase 4 complete. All four sub-phases implemented: `lom fix` rule expansion + fix history (differentiation core) + REPL (interactive toolchain) + LSP (IDE integration) + package manager prototype (project structure + dependency resolution). Direction adjusted per 2026-08-07 retrospective — original "workload-native" (tensor/autodiff/MLIR) dropped: Mojo acquired by Qualcomm makes the AI-compute lane crowded; Lom focuses on `lom fix` differentiation (in-place repair, which MoonBit doesn't do). See [§2.5 retrospective](docs/lom-project-guide.html).

**Phase 5 — Ecosystem & Bootstrapping** (in progress):

- ✅ 5.0 Self-host feasibility verified ([examples/bootstrap/](examples/bootstrap/)) — mini interpreter written in Lom itself: `String → split("") → lex → List<Token> → recursive descent parse → AST → eval`. Correct operator precedence & left-associativity (`"3+4*2"→11`, `"1+2*3+4"→11`). Recursive enum (AST nodes) + match + `split(s,"")` char scan sufficient for compiler core 3 stages. Language gaps found: no tuple destructure (`let (a,b)=...`), no char type, no HashMap — see [Phase 5 notes](docs/lom-project-guide.html#2.7). Fixed tree-walking stack limit by running interpreter in 256MB-stack thread.
- ✅ 5.1 `let` tuple destructuring — `let (a, b, ...) = expr` binds tuple elements to names ([src/ast.rs](src/ast.rs) `Stmt::LetDestruct`; [src/parser.rs](src/parser.rs) `parse_let_destruct` with trailing-comma tolerance & `mut` rejection; [src/interpreter.rs](src/interpreter.rs) runtime tuple/count checks; [src/typechecker.rs](src/typechecker.rs) per-element type binding or `Unknown` fallback). Motivation: the #1 "LLM writes it naturally but language rejects it" pattern discovered during self-hosting (PARSExxx "expected identifier, got LParen"); now [examples/bootstrap/mini_interp.lom](examples/bootstrap/mini_interp.lom) parses with natural `(Expr, List<Token>)` tuple returns — no record workaround. 5 unit tests + 287/287 total.
- ✅ 5.2 statement-layer self-hosting — [examples/bootstrap/stmt_interp.lom](examples/bootstrap/stmt_interp.lom) adds Stmt layer (`SLet`/`SExpr`/`SIf`) over the expression interpreter: let bindings, if/else, blocks, comparisons; functional env via `List<(String, Int)>` + cons shadowing; built on 5.1 tuple destructuring
- ✅ 5.3 `for` over `List<T>` (v0.4.1 P0 gap fix) — `for x in xs` now iterates Lists (previously String/Int only, `RUNTIME000`); interpreter gains a `Value::List` iteration branch (element binding, `return` propagates), typechecker derives element type (`List<T>` → x: T); 4 new tests (291/291 total), eval task 101 added (101/101 pass)
- ✅ 5.4 string concat promotion (v0.4.1 P0 gap fix) — if either operand of `+` is a String, the other is promoted via `to_display()`: `"n = " + 42` works (previously `RUNTIME` error, required `int_to_string`); typechecker result type is String, non-promotable combos (Int+Bool) still warn TYPE001; 3 new tests (294/294 total), eval task 102 added (102/102 pass)
- ✅ 5.5 compound assignment (v0.4.1 P0 gap fix) — `+=` `-=` `*=` `/=`; lexer gains 4 tokens (maximal munch, distinct from `->`), parser desugars to `x = x op e` with newline guard, so interpreter/typechecker reuse the full Assign pipeline unchanged (NAM003/TYPE001 checks included); `+=` composes with 5.4 concat promotion; 5 new tests (299/299 total), eval task 103 added (103/103 pass). **v0.4.1 P0 gap trilogy complete.**

See [`docs/lom-project-guide.html`](docs/lom-project-guide.html) for the full project guide (positioning, design philosophy, 7-phase roadmap, target LLM strategy, risk mitigation, repo governance).

## What is Lom

Lom is a **progressively fused** AI-native programming language:

1. **Phase 0-3 — LLM-coding-native**: tolerant parser, structured JSON diagnostics, linear pipeline syntax, structural types, gradual typing, explicit effect system. Target: LLMs write Lom with low error rate and easy recovery. (Done — 99% LLM pass-rate)
2. **Phase 4 — LLM-repair-native + toolchain** (direction adjusted 2026-08-07): expand `lom fix` in-place auto-repair (differentiation — MoonBit doesn't do in-place repair), add REPL / LSP / package manager. Original "workload-native" (tensor/autodiff/MLIR) dropped — Mojo (acquired by Qualcomm) saturates the AI-compute lane.

## Why Lom

The "AI-native language" space (2026-08 retrospective):

| Direction | Leaders | Status |
|---|---|---|
| Workload-native (tensor / autodiff / GPU) | Mojo (acquired by Qualcomm $3.9B), Bend | Saturated. Mojo binds to Qualcomm hardware strategy. Not Lom's lane. |
| LLM-coding-native (write-it-right) | MoonBit (IEEE TSE 2026 paper: 1/7 corpus but 2x AI gen rate vs Gleam; ~400k users; 1.0 imminent Sep 2026) | MoonBit leads. Lom's 99% eval validates the same thesis. |
| **LLM-repair-native (fix-it-fast)** — Lom's lane | Lom (`lom fix --apply` in-place repair, tolerant parse, gradual typing) | **Empty.** MoonBit regenerates on error; Lom patches in-place. Complementary, not competitive. |

Lom takes the **differentiated route**: occupy the empty LLM-repair-native lane — in-place auto-repair (`lom fix --apply`), tolerant parsing, confidence-graded fixes — complementary to MoonBit's write-it-right approach.

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
| 3 | Usable MVP | 8-12 w | Write a complete CLI tool or simple web service | ✅ Done (3.5 todo CLI) |
| 4 | LLM-repair-native + toolchain (adjusted 2026-08) | — | `lom fix` covers main diagnostic auto-repair + confidence model + REPL + LSP + package manager |
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
- **Phase 4**: reuse existing interpreter/typechecker/diagnostics — no new low-level deps (no Cranelift/MLIR). Adds `lom fix` rule expansion, REPL, LSP server, package manager.

## License

Apache-2.0. See [LICENSE](LICENSE).

## Contact

- GitHub: [lom-lang/lom](https://github.com/lom-lang/lom)
- Issues: <https://github.com/lom-lang/lom/issues>
