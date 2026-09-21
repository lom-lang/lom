# Security Policy & Audit Notes (Phase 6.6, 2026-08-22)

## Scope and threat model

Lom is a **tree-walking interpreter for trusted programs**. Running a `.lom` file grants it the same privileges as the invoking user (file I/O via the `file` module, process arguments via `env`). Lom is **not** a sandbox: do not run untrusted `.lom` code. This is the same threat model as `python script.py` or `node script.js`.

## Supply chain

- **Zero third-party Rust crate dependencies** (permanent design decision, recorded in `Cargo.toml`). The lexer, parser, interpreter, JSON, and LSP implementations are hand-written in this repository. The lockfile is in-tree and contains no third-party packages; with the Rust toolchain already installed, `cargo build` fetches no crates.
- CI is a separate supply-chain surface: it uses GitHub-maintained `actions/checkout@v4` / `actions/cache@v4` and the third-party `dtolnay/rust-toolchain@stable`. These are floating major/channel references, not immutable commit-SHA pins. The 2026-09-21 handoff audit also observed GitHub's Node.js 20 deprecation annotations for checkout/cache. Therefore “zero dependencies” must not be expanded into “zero CI supply-chain risk.”

## Interpreter hardening already in place

The hardening claims below include executable checks; remaining gaps are listed separately as limitations.

- **Recursion depth guard**: the interpreter runs on a dedicated 256 MB-stack thread (Phase 5.0) **and enforces a software depth limit of 80,000 Lom call frames** (N1, 2026-09-07; measured frame cost ~2.6 KB → theoretical limit ~100k on 256 MB, 80% headroom). Exceeding it produces a structured `[RUNTIME000] 递归深度超过 80000 层` diagnostic (exit 1) with the recursive function's signature location. Verify: save `fn down(n: Int) -> Int
    down(n + 1)
end
fn main() -> Unit
    println(down(0))
end` as `deep.lom`; `./target/release/lom deep.lom` prints the diagnostic and exits 1; `./target/release/lom examples/bench.lom -- recurse 10000` completes with exit 0. Parser expression nesting and JSON nesting have their own guards; recursive type/pattern parsing does not yet—see limitation 1.
- **Memory safety**: the interpreter is 100% safe Rust — no `unsafe` block anywhere in `src/`. Shared mutability uses `Rc<RefCell<...>>`; a `RefCell` double-borrow cannot corrupt memory. A panic in any residual bug path now maps to a nonzero exit (R56 fix, 2026-09-21: `main` maps `child.join()` errors to exit 1). Verify: `rg -n -F unsafe src` prints nothing (exit code 1).
- **Parser recovery**: malformed tokens are represented by diagnostics and hole nodes where recovery is implemented. Unknown characters are reported as LEX005 and skipped so later errors can be collected. Verify: `lom examples/apply_test.lom --json` returns `ok:false` with multiple diagnostics. This is not yet a proof that every malformed input is rejected or every recursive grammar path is bounded: EOF currently closes several block forms without a missing-`end` diagnostic, and type/pattern recursion lacks the expression-depth guard (limitations 1 and 6).
- **LSP transport is bounded and standards-shaped** (R63, 2026-09-21): the server caps `Content-Length` at 16 MiB — an oversized header is refused with a stderr line and exit 1 instead of the v1.2.2 behavior where a 20-byte `Content-Length: 999999999999` header caused an allocation abort that killed the server. Malformed JSON payloads get JSON-RPC error responses (`-32700` Parse error for invalid JSON, `-32600` Invalid Request for a non-request JSON value, `id:null`) instead of silent drops, and the server keeps serving afterwards. Verify: `cargo test --release --test r58_lsp_process` — `r63_oversized_content_length_rejected_not_aborted` asserts exit code 1 (not the abort code) and `r63_malformed_json_gets_error_response` asserts the `-32700`/`-32600` frames plus a subsequent successful `initialize`.

## Known limitations (accepted risks)

1. **Depth guards cover expression, JSON, type, and pattern parsing** (R60, 2026-09-21): expression nesting is bounded at 30,000 levels, runtime `json_parse` nesting at 100,000, and `parse_type`/`parse_pattern` recursion now shares the same software depth counter pattern (previously unbounded — a hand-crafted `List<List<…×300>>` or `A(A(…(x)))` input could overflow the stack; now a structured `PARSE001 类型嵌套超过…层` / `模式嵌套超过…层` error). Verify: the regression tests `r60_deep_type_nesting_reports_parse_error` / `r60_deep_pattern_nesting_reports_parse_error`.
2. **`file` module performs no path validation** — it reads/writes whatever the OS user can access (trusted-program threat model).
3. **`RefCell` reentrancy**: a builtin that borrows a Map while a user closure mutates the same Map would panic. No such reentrancy path is currently reachable (higher-order builtins operate on List, not Map), but it is a documented invariant to preserve when adding builtins.
4. **Int arithmetic is not generally checked**. Add/sub/mul overflow wraps (now explicit `wrapping_*` in both debug and release, 2026-09-21 R56 — the documented semantics). Division/modulo by zero and the `i64::MIN / -1` / `i64::MIN % -1` overflow edges are structured RUNTIME000 diagnostics with exit 1 (R56 fix; process-level regression in `tests/r56_process.rs`). Verify: write `fn main() -> Unit\n    let min = 9223372036854775807 + 1\n    println(min / -1)\nend` as `ovf.lom`; `./target/release/lom ovf.lom` prints `[RUNTIME000] 整数除法溢出…` to stderr and exits 1. This is a bug fix for two panic edges, not evidence that all arithmetic is checked.
5. **Worker-thread panics are mapped to exit 1 since R56 (2026-09-21)**: `main` maps `child.join()` errors to a stderr line plus exit 1, and the two known arithmetic panic edges are structured diagnostics (limitation 4). Any *future* panic path would still print a raw Rust panic message before the exit — ugly but no longer a false success code. Release remains frozen.
6. **Missing block terminators are rejected since R57 (2026-09-21)**: `parse_block` no longer treats EOF as a silent block terminator — a function/if/while/for/closure body missing its final `end` reports `PARSE001 期望 'end' (块闭合)，得到 文件结束` and exits 1 (strict mode errors; tolerant mode records the diagnostic with the hole-preserving AST). Verify: write `fn main() -> Unit\n    let x = 5\n    println(x)\n` as `noend.lom`; `./target/release/lom noend.lom --json` returns `ok:false` with one PARSE001. Remaining parser gaps are tracked by R60 (type/pattern recursion depth guards).

## Reporting a vulnerability

Open a [GitHub issue](https://github.com/lom-lang/lom/issues) for non-sensitive reports. For sensitive issues, use GitHub's **private vulnerability reporting** (repository Security tab → Report a vulnerability) or contact the repository owner (@wyty) via GitHub directly. Do not file public issues for exploitable vulnerabilities before they are triaged.

## Audit procedure for future changes

For any change touching `src/` (this repository pushes directly to `main`, without a PR gate):

1. No new Rust crate dependencies without an RFC. The CI gate (R60-hardened 2026-09-21) blocks all dependency-table TOML forms (`[dependencies]`/`[dev-]`/`[build-]`/`[dependencies.foo]`/`[target.'cfg(…)'.dependencies]`/`[workspace.dependencies]`) **and** asserts an empty resolved-dependency set from `cargo metadata` — the factual check no longer depends on the awk text scan alone.
2. No `unsafe` (grep-enforced in review: `grep -rn "unsafe" src/` must stay empty).
3. New builtins must not hold a `RefCell` borrow across a user-closure callback.
4. New parsing paths must stay bounded and reject missing required delimiters; cover expression, type, and pattern recursion separately.
5. **Every hardening claim in this file must ship with an executable verification command and its expected output.** A claim that cannot be verified by running a command does not belong here — write it as a limitation instead.
