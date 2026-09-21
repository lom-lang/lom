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
- **Memory safety**: the interpreter is 100% safe Rust — no `unsafe` block anywhere in `src/`. Shared mutability uses `Rc<RefCell<...>>`; a `RefCell` double-borrow cannot corrupt memory. It can still panic, and the current top-level thread wrapper mishandles panics as exit 0 (limitation 5). Verify: `rg -n -F unsafe src` prints nothing (exit code 1).
- **Parser recovery**: malformed tokens are represented by diagnostics and hole nodes where recovery is implemented. Unknown characters are reported as LEX005 and skipped so later errors can be collected. Verify: `lom examples/apply_test.lom --json` returns `ok:false` with multiple diagnostics. This is not yet a proof that every malformed input is rejected or every recursive grammar path is bounded: EOF currently closes several block forms without a missing-`end` diagnostic, and type/pattern recursion lacks the expression-depth guard (limitations 1 and 6).

## Known limitations (accepted risks)

1. **Depth guards are partial**: Q3 guards expression nesting at 30,000 levels and runtime `json_parse` nesting at 100,000. `parse_type` and `parse_pattern` are independently recursive and currently have no depth counter. The old statement “all recursion entry points fail structurally” was too broad; R60 tracks the correction and missing tests.
2. **`file` module performs no path validation** — it reads/writes whatever the OS user can access (trusted-program threat model).
3. **`RefCell` reentrancy**: a builtin that borrows a Map while a user closure mutates the same Map would panic. No such reentrancy path is currently reachable (higher-order builtins operate on List, not Map), but it is a documented invariant to preserve when adding builtins.
4. **Int arithmetic is not generally checked**. Add/sub/mul overflow wraps in release builds. Division/modulo by zero is a RUNTIME000 diagnostic, but the distinct `i64::MIN / -1` and `i64::MIN % -1` overflow cases panic in Rust. The 2026-09-21 audit reached the division case via `9223372036854775807 + 1` and observed a panic; see limitation 5 for the incorrect exit status. Handling these two cases as an existing RUNTIME000 failure is a bug fix, not evidence that all arithmetic is checked.
5. **A panic in the 256 MB worker thread currently exits the Lom process with code 0**: `main` discards `child.join()`. This violates the CLI success contract and can fool runners that trust exit status. R56 is open and release remains frozen.
6. **Missing block terminators can be silently accepted at EOF**: `parse_block` treats EOF as a normal terminator and consumes `end` only when present. A function missing its final `end` currently reports zero diagnostics and executes. This contradicts the frozen grammar; R57 is open.

## Reporting a vulnerability

Open a [GitHub issue](https://github.com/lom-lang/lom/issues) for non-sensitive reports. For sensitive issues, use GitHub's **private vulnerability reporting** (repository Security tab → Report a vulnerability) or contact the repository owner (@wyty) via GitHub directly. Do not file public issues for exploitable vulnerabilities before they are triaged.

## Audit procedure for future changes

For any change touching `src/` (this repository pushes directly to `main`, without a PR gate):

1. No new Rust crate dependencies without an RFC. Current CI catches ordinary dependency tables but does not cover every valid target-specific/workspace TOML form; review `cargo metadata` as the factual check until R60 strengthens the gate.
2. No `unsafe` (grep-enforced in review: `grep -rn "unsafe" src/` must stay empty).
3. New builtins must not hold a `RefCell` borrow across a user-closure callback.
4. New parsing paths must stay bounded and reject missing required delimiters; cover expression, type, and pattern recursion separately.
5. **Every hardening claim in this file must ship with an executable verification command and its expected output.** A claim that cannot be verified by running a command does not belong here — write it as a limitation instead.
