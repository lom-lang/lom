# Contributing to Lom

Thanks for your interest in Lom — an AI-native programming language. This document explains how to contribute code, report bugs, and propose language changes.

## Ground rules

1. **Zero third-party dependencies.** The interpreter is deliberately dependency-free (hand-written lexer/parser/JSON). Do not add crates without an accepted RFC (see below).
2. **Never fabricate data.** Benchmarks and measurements must be real and reproducible. If a number is an estimate, label it as such.
3. **Every milestone ships with docs.** Code changes land together with updates to `README.md`, `docs/lom-project-guide.html`, and (for language changes) `LANGUAGE_SPEC.md` / `SPEC_FOR_AI.md`.
4. **Regression trilogy before every commit:**
   ```sh
   cargo test --release
   cargo build --release && ./target/release/lom examples/bootstrap/stmt_interp.lom
   powershell -ExecutionPolicy Bypass -File eval/runner/run.ps1 -Verify -LomBin ./target/release/lom.exe   # Windows
   ./eval/runner/run.sh ./target/release/lom                                                              # Unix
   ```
5. **Comments and diagnostics are written for both humans and LLMs.** Keep them precise; error messages follow the `<CATEGORY><NUMBER>` diagnostic-code scheme (`NAM003`, `TYPE001`, ...).

## Reporting bugs

Open a GitHub issue with:

- the `.lom` source that triggers the problem (minimal repro),
- the exact output of `lom <file> --check` and `lom <file> --json`,
- your platform and `lom --version`.

## Changing the language

Language changes (syntax, semantics, stdlib, diagnostics) require an **RFC**:

1. Copy `docs/rfc/0000-template.md` to `docs/rfc/NNNN-title.md`.
2. Fill in motivation, design, alternatives, and LLM-impact analysis.
3. Open a PR; the RFC is discussed there. Acceptance is recorded in the RFC file (`Status: accepted`).
4. Only then implement. The implementation PR references the RFC.

Small bug fixes, diagnostics improvements with existing codes, and stdlib function additions that follow existing conventions do **not** need an RFC — a PR is enough.

## Code conventions

- Rust 2024 edition, stable toolchain, no nightly features.
- Keep edits scoped: a bug fix does not include drive-by refactors.
- New behavior needs tests: unit tests in the same file (`#[cfg(test)]`), and an `eval/tasks/` entry if the change is user-visible in the language.
- Commit messages: `feat:` / `fix:` / `docs:` prefix, imperative, explain the *why*.

## The evaluation suite

`eval/` measures LLM generation pass-rate — the project's core metric. If your change affects the language surface, add or update a task and regenerate prompts with `eval/prompts/_generate.ps1`.

## License

Apache 2.0. By contributing you agree your contributions are licensed under the same terms.
