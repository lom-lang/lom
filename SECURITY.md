# Security Policy & Audit Notes (Phase 6.6, 2026-08-22)

## Scope and threat model

Lom is a **tree-walking interpreter for trusted programs**. Running a `.lom` file grants it the same privileges as the invoking user (file I/O via the `file` module, process arguments via `env`). Lom is **not** a sandbox: do not run untrusted `.lom` code. This is the same threat model as `python script.py` or `node script.js`.

## Supply chain

- **Zero third-party dependencies** (permanent design decision, recorded in `Cargo.toml`). The entire toolchain — lexer, parser, interpreter, JSON, LSP — is hand-written in this repository. The lockfile is in-tree and contains no third-party packages; `cargo build` fetches nothing.
- CI (`.github/workflows/ci.yml`) pins only official actions (`actions/checkout@v4`, `dtolnay/rust-toolchain@stable`, `actions/cache@v4`).

## Interpreter hardening already in place

Every claim in this section ships with an executable verification command (rule 5 of the audit procedure below).

- **Stack exhaustion**: the interpreter runs on a dedicated 256 MB-stack thread (Phase 5.0). Measured safe depth is ~10⁴ Lom frames (Phase 5.18); beyond that, deep recursion aborts the process. This is a documented known limitation, not a memory-safety issue (Rust's stack overflow guard applies; there is no UB). Verify: `./target/release/lom examples/bench.lom -- recurse 10000` completes with exit 0; deeper recursion aborts (limitation 1 below).
- **Memory safety**: the interpreter is 100% safe Rust — no `unsafe` block anywhere in `src/`. Shared mutability uses `Rc<RefCell<...>>`; a `RefCell` double-borrow panics (safe abort), it cannot corrupt memory. Verify: `grep -rn "unsafe" src/` prints nothing.
- **Parser robustness**: the parser is total by design (holey AST, Phase 2.2/5.15): malformed, truncated, or adversarial input produces diagnostics, never panics. Unknown characters are reported as explicit lexer diagnostics (LEX005) and skipped, so parsing continues and collects the remaining errors — nothing is silently dropped. Verify: save `fn f( -> Int` as `bad.lom`; `./target/release/lom bad.lom` prints PARSE diagnostics and exits 1 — no panic.

## Known limitations (accepted risks)

1. **No recursion-depth guard for the evaluator** beyond the 256 MB stack — a hostile Lom program can abort the process (availability impact only).
2. **`file` module performs no path validation** — it reads/writes whatever the OS user can access (trusted-program threat model).
3. **`RefCell` reentrancy**: a builtin that borrows a Map while a user closure mutates the same Map would panic. No such reentrancy path is currently reachable (higher-order builtins operate on List, not Map), but it is a documented invariant to preserve when adding builtins.
4. **Int arithmetic is not checked** (corrected 2026-09-03; an earlier revision of this file wrongly claimed "checked arithmetic"). Lom `Int` is plain `i64`: on overflow, release builds **silently wrap** (two's complement) with no diagnostic. Checked arithmetic (`checked_add`/`checked_sub`/`checked_mul`) has never been implemented — verify the absence: `grep -rn "checked_add\|checked_sub\|checked_mul\|checked_div" src/` returns no matches. Only division/modulo by zero is a runtime diagnostic — verify: `println(9223372036854775807 + 1)` prints `-9223372036854775808` (wrap, no diagnostic); `println(1 / 0)` reports `RUNTIME000` and exits 1. Accepted risk: under the trusted-program threat model wraparound affects program correctness, not memory safety; moving to checked arithmetic is a semantic change that requires an RFC (spec §14 freeze).

## Reporting a vulnerability

Open a [GitHub issue](https://github.com/lom-lang/lom/issues) for non-sensitive reports. For sensitive issues, use GitHub's **private vulnerability reporting** (repository Security tab → Report a vulnerability) or contact the repository owner (@wyty) via GitHub directly. Do not file public issues for exploitable vulnerabilities before they are triaged.

## Audit procedure for future changes

For any PR touching `src/`:

1. No new dependencies (CI will fail the build if `Cargo.toml` gains one — review requires an RFC).
2. No `unsafe` (grep-enforced in review: `grep -rn "unsafe" src/` must stay empty).
3. New builtins must not hold a `RefCell` borrow across a user-closure callback.
4. New parsing paths must stay total (no bare indexing/unwrap on token streams).
5. **Every hardening claim in this file must ship with an executable verification command and its expected output.** A claim that cannot be verified by running a command does not belong here — write it as a limitation instead.
