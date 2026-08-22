# Security Policy & Audit Notes (Phase 6.6, 2026-08-22)

## Scope and threat model

Lom is a **tree-walking interpreter for trusted programs**. Running a `.lom` file grants it the same privileges as the invoking user (file I/O via the `file` module, process arguments via `env`). Lom is **not** a sandbox: do not run untrusted `.lom` code. This is the same threat model as `python script.py` or `node script.js`.

## Supply chain

- **Zero third-party dependencies** (permanent design decision, recorded in `Cargo.toml`). The entire toolchain — lexer, parser, interpreter, JSON, LSP — is hand-written in this repository. There is no `Cargo.lock` dependency surface to audit; `cargo build` fetches nothing.
- CI (`.github/workflows/ci.yml`) pins only official actions (`actions/checkout@v4`, `dtolnay/rust-toolchain@stable`, `actions/cache@v4`).

## Interpreter hardening already in place

- **Stack exhaustion**: the interpreter runs on a dedicated 256 MB-stack thread (Phase 5.0). Measured safe depth is ~10⁴ Lom frames (Phase 5.18); beyond that, deep recursion aborts the process. This is a documented known limitation, not a memory-safety issue (Rust's stack overflow guard applies; there is no UB).
- **Memory safety**: the interpreter is 100% safe Rust — no `unsafe` block anywhere in `src/`. Shared mutability uses `Rc<RefCell<...>>`; a `RefCell` double-borrow panics (safe abort), it cannot corrupt memory.
- **Parser robustness**: the parser is total by design (holey AST, Phase 2.2/5.15): malformed, truncated, or adversarial input produces diagnostics, never panics. Unknown characters become explicit `TUnknown` tokens rather than being silently dropped.
- **Integer handling**: all arithmetic on Lom `Int` is checked (`i64`); division by zero is a runtime error, not a trap.

## Known limitations (accepted risks)

1. **No recursion-depth guard for the evaluator** beyond the 256 MB stack — a hostile Lom program can abort the process (availability impact only).
2. **`file` module performs no path validation** — it reads/writes whatever the OS user can access (trusted-program threat model).
3. **`RefCell` reentrancy**: a builtin that borrows a Map while a user closure mutates the same Map would panic. No such reentrancy path is currently reachable (higher-order builtins operate on List, not Map), but it is a documented invariant to preserve when adding builtins.

## Reporting a vulnerability

Open a GitHub issue for non-sensitive reports. For sensitive issues, contact the repository owner directly (see `CODE_OF_CONDUCT.md` enforcement channel). Do not file public issues for exploitable vulnerabilities before they are triaged.

## Audit procedure for future changes

For any PR touching `src/`:

1. No new dependencies (CI will fail the build if `Cargo.toml` gains one — review requires an RFC).
2. No `unsafe` (grep-enforced in review: `grep -rn "unsafe" src/` must stay empty).
3. New builtins must not hold a `RefCell` borrow across a user-closure callback.
4. New parsing paths must stay total (no bare indexing/unwrap on token streams).
