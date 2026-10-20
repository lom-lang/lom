# AGENTS.md

Guidance for coding agents (and humans in a hurry) working in this repository.

Lom is an **LLM-repair-native** dynamic language: the repair loop — stable
diagnostic codes, graded fix suggestions, and a safety axiom governing what may
be auto-applied — is the language's reason to exist. Rust implementation,
**zero third-party crates, zero unsafe code**. The language surface (syntax /
20 keywords / diagnostic codes / 43 builtins) is **frozen**; changes there
require a new RFC (see `LANGUAGE_SPEC.md` §14).

## Build & verify (Windows-friendly; Git Bash or PowerShell)

```bash
cargo build --release && cargo test --release   # zero warnings; expected counts: docs/HANDOVER.md §2.2 (single source, R160 pointerized)
python tools/doc_audit.py                       # document-number interlock (item count: HANDOVER §2.2)
python tools/verify_selfcomp.py                 # L2 self-compilation gate (count: HANDOVER §2.2)
python tools/verify_selfcomp.py --bootstrap     # three-layer bootstrap gate (counts & quine bytes: HANDOVER §2.2; ~20-45 s)
```

The full regression suite is `docs/HANDOVER.md` §2.2. `target/release/lom.exe`
is the toolchain entry point (`lom <file>` runs; `lom <file> --check --json`
emits diagnostics; `lom fix <file> --plan|--apply`; `lom l2fix`; `lom codes
--json`).

## Where things are

| Asset | Path |
|---|---|
| Language spec (frozen surface §14) | `LANGUAGE_SPEC.md` |
| **Spec written for LLM consumption** | `SPEC_FOR_AI.md` |
| Maintenance rules & workflow | `docs/MAINTENANCE.md` |
| Executor (subagent) instructions | `docs/EXECUTOR_TEMPLATE.md` |
| Map of English vs Chinese document surfaces | `docs/README.md` |
| ACRI — agent-compiler repair interface spec (this repo is the reference implementation) | `specs/acri/` |
| RepairLoopBench benchmark card | `eval/README.md` |
| L2 self-hosted compiler source | `examples/selfhost/self_comp.lom` |
| Performance benchmark assets | `examples/bench.lom` + `tools/bench.py` |

## Rules agents must respect

1. **Never change the frozen language surface** without a new RFC.
2. **Never run git write operations** (commit/push/tag are the maintainer's);
   probes and scratch output go under `target/probes/` (gitignored) only.
3. **Every claim with a number must be reproducible** — this repo machine-locks
   its own claims (quine, eval, doc interlock); match that standard in anything
   you write here.
4. Behavior changes require the full regression suite green and behavior/doc
   pairs in the same batch (see `docs/MAINTENANCE.md` §1-2).
