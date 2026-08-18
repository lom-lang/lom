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

320/320 Rust unit tests pass (316 prior + 4 new in Phase 5.9). 30 `.lom` examples (29 run + 1 `--check` diagnostic sample). `eval/` 107/107 reference solutions pass (`./eval/runner/run.ps1 -Verify -LomBin target/release/lom.exe`). **LLM generation pass-rate: 99%** (expert model + thinking mode, 2026-08-03, on the 100-task v2.8 set).

Phase 4 complete. All four sub-phases implemented: `lom fix` rule expansion + fix history (differentiation core) + REPL (interactive toolchain) + LSP (IDE integration) + package manager prototype (project structure + dependency resolution). Direction adjusted per 2026-08-07 retrospective — original "workload-native" (tensor/autodiff/MLIR) dropped: Mojo acquired by Qualcomm makes the AI-compute lane crowded; Lom focuses on `lom fix` differentiation (in-place repair, which MoonBit doesn't do). See [§2.5 retrospective](docs/lom-project-guide.html).

**Phase 5 — Ecosystem & Bootstrapping** (in progress):

- ✅ 5.0 Self-host feasibility verified ([examples/bootstrap/](examples/bootstrap/)) — mini interpreter written in Lom itself: `String → split("") → lex → List<Token> → recursive descent parse → AST → eval`. Correct operator precedence & left-associativity (`"3+4*2"→11`, `"1+2*3+4"→11`). Recursive enum (AST nodes) + match + `split(s,"")` char scan sufficient for compiler core 3 stages. Language gaps found: no tuple destructure (`let (a,b)=...`), no char type, no HashMap — see [Phase 5 notes](docs/lom-project-guide.html#2.7). Fixed tree-walking stack limit by running interpreter in 256MB-stack thread.
- ✅ 5.1 `let` tuple destructuring — `let (a, b, ...) = expr` binds tuple elements to names ([src/ast.rs](src/ast.rs) `Stmt::LetDestruct`; [src/parser.rs](src/parser.rs) `parse_let_destruct` with trailing-comma tolerance & `mut` rejection; [src/interpreter.rs](src/interpreter.rs) runtime tuple/count checks; [src/typechecker.rs](src/typechecker.rs) per-element type binding or `Unknown` fallback). Motivation: the #1 "LLM writes it naturally but language rejects it" pattern discovered during self-hosting (PARSExxx "expected identifier, got LParen"); now [examples/bootstrap/mini_interp.lom](examples/bootstrap/mini_interp.lom) parses with natural `(Expr, List<Token>)` tuple returns — no record workaround. 5 unit tests + 287/287 total.
- ✅ 5.2 statement-layer self-hosting — [examples/bootstrap/stmt_interp.lom](examples/bootstrap/stmt_interp.lom) adds Stmt layer (`SLet`/`SExpr`/`SIf`) over the expression interpreter: let bindings, if/else, blocks, comparisons; functional env via `List<(String, Int)>` + cons shadowing; built on 5.1 tuple destructuring
- ✅ 5.3 `for` over `List<T>` (v0.4.1 P0 gap fix) — `for x in xs` now iterates Lists (previously String/Int only, `RUNTIME000`); interpreter gains a `Value::List` iteration branch (element binding, `return` propagates), typechecker derives element type (`List<T>` → x: T); 4 new tests (291/291 total), eval task 101 added (101/101 pass)
- ✅ 5.4 string concat promotion (v0.4.1 P0 gap fix) — if either operand of `+` is a String, the other is promoted via `to_display()`: `"n = " + 42` works (previously `RUNTIME` error, required `int_to_string`); typechecker result type is String, non-promotable combos (Int+Bool) still warn TYPE001; 3 new tests (294/294 total), eval task 102 added (102/102 pass)
- ✅ 5.5 compound assignment (v0.4.1 P0 gap fix) — `+=` `-=` `*=` `/=`; lexer gains 4 tokens (maximal munch, distinct from `->`), parser desugars to `x = x op e` with newline guard, so interpreter/typechecker reuse the full Assign pipeline unchanged (NAM003/TYPE001 checks included); `+=` composes with 5.4 concat promotion; 5 new tests (299/299 total), eval task 103 added (103/103 pass). **v0.4.1 P0 gap trilogy complete.**
- ✅ 5.6 range expression (v0.4.2 P1 gap fix) — `a..b` (left-inclusive right-exclusive) **evaluates to `List<Int>`**, reusing 5.3's for-in-List and the whole list module with zero new runtime machinery (`list_length(1..4)` works); lexer gains `DotDot` (distinct from Float `3.14` and tuple index `t.0`), parser adds lowest-precedence `parse_range` (below `or`, non-associative, newline-guarded), typechecker records `List<Int>` and warns TYPE001 on known non-Int ends; resolves LANGUAGE_SPEC open question #1 (Rust-style `a..b` chosen, `a..=b` rejected — two range operators are a known LLM confusion source); 8 new tests (307/307 total), eval task 104 added (104/104 pass)
- ✅ 5.7 match guard (v0.4.2 P1 gap fix) — `pattern if cond => body`: arm wins only when pattern matches AND guard (Bool) is true, otherwise falls through; guard may reference pattern-bound variables; typechecker checks guard is Bool (TYPE002) and **guarded arms don't count toward exhaustiveness** (Rust semantics — guard truth is runtime-only); 6 new tests (313/313 total), eval task 105 added (105/105 pass)
- ✅ 5.8 named functions as first-class values (v0.4.2 P1 gap fix) — `let f = double` wraps a named function as a closure value (env = globals, identical to `call_function`'s parent env; recursion unaffected since in-body name calls still go through the functions table); typechecker already allowed function references (gradual Unknown) so zero changes there; 3 new tests (316/316 total), eval task 106 added (106/106 pass). **Language prerequisite for `list_map`/`list_filter`-style higher-order stdlib.**
- ✅ 5.9 higher-order list stdlib (v0.4.3) — `list_map(f, xs)` / `list_filter(f, xs)` / `list_fold(f, init, xs)`; `call_builtin` widened from `&self` to `&mut self` so builtins can call back into closures; `f` accepts closure literals or named functions (5.8 wrapped values); composes naturally with range (`list_map(double, 1..6)`) and concat promotion (`list_fold` string building); typechecker signatures registered (`f: Fn`); 4 new tests (320/320 total), eval task 107 added (107/107 pass). Also backfilled SPEC_FOR_AI's missing `list` module table (doc debt since Phase 3.3)
- ✅ 5.10 bootstrap deepening — [examples/bootstrap/stmt_interp.lom](examples/bootstrap/stmt_interp.lom) gains `SWhile` (source-level while loops; `let mut` + Lom while threads the functional env across iterations, cons shadowing makes rebinding work); `exec_stmts` rewritten from hand recursion to `list_fold` + closure — real dogfooding of 5.8/5.9 inside self-host code; new test program 3 (while countdown sum = 6), outputs `11 / 10 / 6` all correct; effect annotation chain completed (`! [IO]`) and TYPE010 eliminated — `--check` now reports **0 diagnostics** (was 2 historical warnings)
- ✅ 5.11 bootstrap functions — source language gains `fn name(p1, p2) ... end` definitions and calls with **lexical scoping** (fresh param env per call); `Decl` layer (`DFn`/`DStmt`) with collect-fns-first execution so forward references and mutual recursion work (program 5: `collatz_len` calls later-defined `is_odd`, 8 recursion steps correct). Key refactor: **statement-value semantics** — naive "last SExpr is the return value" broke at `if` (values can't escape branches; in-function SExpr mis-prints), rebuilt as `exec_stmt → (env, value)` where `SIf`'s value is the taken block's value (if-as-expression), block value = tail statement value, `in_fn` flag separates top-level printing from in-function evaluation. Debug finding recorded: mini-language has no `==` (lexer silently drops `=`, causing infinite recursion + stack overflow — equality now expressed arithmetically). Outputs `11/10/6/49/8` all correct, `--check` 0 diagnostics
- ✅ 5.12 bootstrap value system — values grow from Int-only to `Val = VInt | VStr | VBool` (recursive enum, feasibility proven in 5.0); source language gains string literals, `==` comparison, lowercase `true`/`false` Bool printing; `Add` mixing mirrors Lom v0.4.1 (either-side-String promotes via `show()`); conditions unified under `truthy()`. **The 5.11 trap is fixed at the root**: `=`/`==` are now explicit tokens (`TAssign`/`TEq`) instead of being silently dropped. 9 test programs all correct (incl. `hello, lom`, `true`, `ababab`), `--check` 0 diagnostics. Value-system prerequisite for a self-host diagnostics layer
- ✅ 5.13 bootstrap diagnostics layer — the whole evaluator moves from "silently yield 0 on error" to **`Result<_, String>` + `?` propagation**: Lom's errors-as-values philosophy implemented *in* Lom. Undefined variables/functions, arity mismatches, type errors, division by zero all report explicitly (program 8's four diagnostics verified verbatim); `?` threads through while conditions and loop bodies; exec_stmts folds a Result (Err short-circuits); error messages concat Ints via v0.4.1 promotion. The typechecker's TYPE020 `?`-compatibility rules got a real stress test. 13 programs correct, `--check` 0 diagnostics
- ✅ 5.14 bootstrap `VList` recursive values — `Val` gains `VList(List<Val>)` (first recursive value type in the self-hosted runtime); source language gains list literals `[1, 2]`, chainable indexing `xs[i]`, and a `len()` builtin (works on List and String — first builtin mechanism in the mini-language); literal element evaluation reuses `eval_args`; show/values_eq/truthy extend recursively (`[[1, 2], [3]]` prints correctly); out-of-bounds/negative-index/type errors go through the 5.13 diagnostics channel (original index preserved in messages — first version reported the decremented counter, fixed). 9 programs / 18 outputs verified verbatim, `--check` 0 diagnostics
- ✅ 5.15 bootstrap fault-tolerant parsing — mirrors the host's holey-AST philosophy: parse errors become AST nodes (`EError`/`SError`) reported via the 5.13 diagnostics channel at execution; lexer unknown chars yield `TUnknown` tokens instead of silent drops; parser is now total (`peek_tok` Option guards replace all bare `list_head` — empty/truncated/unknown input never crashes). Program 10: statements before the error still run (`7` printed), then `parse error: unexpected unknown character '@'`; truncated `1 +` and empty input are safe. Debug log: missing `end` in a Form B arm containing an inner match (parse_call_args — the §4.1 trap again), an over-correction adding `end` to a Form A arm (Form A needs none — the A/B asymmetry was the root confusion), and two latent `list_cons` arg-order bugs caught by the typechecker's TYPE003. 10 programs / 21 outputs verified, `--check` 0 diagnostics

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
