# Lom conformance to ACRI v0.1

**Status: 2026-10-12 · reference-implementation mapping · maintained with the specification**

This file maps the Lom toolchain's current behavior to ACRI v0.1
(`acri-v0.1.md`). Conformance levels per §2.3. "Planned" items carry their
remediation batch from designs/0023 §5.

## L0 — Object model

| ACRI clause | Lom current behavior | Status |
|---|---|---|
| §1.1 `code` `<NS><NNN>`, stable | 9 namespaces (LEX/PARSE/TYPE/NAM/EFF/MAT/MUT/RUNTIME/PKG) + L2-side `[L2xxx]` 7 families; codes stable and machine-locked by 368 verify_selfcomp cases | **Conformant** (see §2.1 gap: no `codes` export yet) |
| §1.1 `severity` error/warning | `lom-diag/v1` severity field, same two values | **Conformant** |
| §1.1 `message` (any language) | Chinese messages; agents consume `code` | **Conformant** |
| §1.1 `position` object with `anchor` | Three channel conventions today: lex/parse carry `line`/`col` (top-level fields) — maps to `anchor: "exact"`; the L2 codegen channel carries an optional `ln:cl` name-first anchor, deliberately absent rather than fabricated when unresolvable — maps to `anchor: "name-first"` / `"none"`; the `check --json` envelope carries `line`/`col` per diagnostic | **Conformant by mapping** (physical unification of the envelope to the nested `position` object is a formatting change — deferred; consumers see a documented mapping) |
| §1.1 `stage` OPTIONAL | `stage` field (lex/parse/type/codegen…) | **Conformant** |
| §1.1 `hint` OPTIONAL | `hint` field | **Conformant** |
| §1.1 extension fields | `source_line`, `is_hole` — extensions under §1.1's open-field axiom | **Conformant** |
| §1.2 `RepairSuggestion` unified enums | Host fix engine: 4 actions (insert/replace/delete/hint) × 3 confidences (High/Medium/Low) — matches after enum-name normalization; `l2-fix/v1` uses `confidence`/`action` with the same vocabulary | **Conformant by mapping** (case normalization documented here; `l2-fix/v1` lacks a `position` field — see Planned) |
| §1.3 AXIOM 1 (auto-apply only high non-hint) | `fix --apply` applies only `confidence == High && action != Hint` (src/fix.rs:12, :3344) | **Conformant** |
| §1.3 AXIOM 2 (cognitive → hint, never auto) | fix_corpus pairs 04/12/13 assert `fixed == bad` for cognitive warnings (NAM006 alias notice, MUT002 closure capture) | **Conformant** |
| §1.3 AXIOM 3 (plan/apply separate) | `--plan` side-effect-free; `--apply` explicit | **Conformant** |

## L1 — CLI protocol

| ACRI clause | Lom current behavior | Status |
|---|---|---|
| §2.1 `check [--json]` | `lom <file> --check --json` → envelope `schema/file/ok/summary/diagnostics` | **Conformant** |
| §2.1 `fix --plan [--json]` | `lom fix <file> --plan --json` → `lom-fix/v1` plans | **Conformant** |
| §2.1 `fix --apply` | `lom fix <file> --apply`, per AXIOM 1 | **Conformant** |
| §2.1 `codes [--json]` | **Missing** — code conventions live in prose headers across four source files; no machine-readable export | **Planned (β-2)** — new CLI surface, minor version; includes registering the 9 + L2 namespaces as a table and (evaluated in batch) moving code assignment to emission time rather than post-hoc message classification |
| §2.1 `explain <code>` (OPTIONAL) | Missing | Not planned (optional capability) |
| §2.2 exit codes (check err=1 / warn=0 / fix=0 / tool-fail ≥2) | Measured: `--check --json` with an error → rc 1; warnings-only → 0; `fix --plan`/`--apply` → 0 (analysis-completed semantics); tool-internal failure path returns 1 today — a documented deviation while the ≥2 band is adopted in β-2 | **Conformant with one deviation** (tool-failure exit code) |
| §2.1 envelope unknown-field tolerance | Consumers ignore unknown fields — forward-compatible by construction | **Conformant** |
| LSP surface (informative) | The LSP adapter narrows positions to 0-based single-char ranges and drops `stage`/`source_line`/`is_hole`/`hint` — an adaptation loss, tracked as an improvement candidate alongside β-2, not a conformance clause | — |

## L2 — Benchmark methodology

| ACRI clause | Lom current behavior | Status |
|---|---|---|
| §3.1 dual real capture | eval prompts embed verbatim `--check --json` captures (machine-gated by `tools/eval_prompt_check.py`, 31/31) | **Conformant** |
| §3.2 synthesis declaration | RepairLoopBench card (`eval/README.md`) declares synthesis + non-comparability scope | **Conformant** |
| §3.3 multi-chain reference validation | Reference solutions verified on host interpreter / host WASM / L2 chain; expected outputs locked | **Conformant** |
| §3.4 black-box scoring | stdout + exit code only | **Conformant** |
| §3.5 machine gate on prompts | Code-multiset comparison, CI-resident | **Conformant** |
| §3.6 scope honesty | Card states what the suite is not and answers the SWE-bench failure modes | **Conformant** |

## Summary

- **L0-conformant: yes** (two "by mapping" normalizations documented above).
- **L1-conformant: pending β-2** (one REQUIRED command missing — `codes`; one
  exit-code deviation).
- **L2-documented: yes** (RepairLoopBench is the reference instance).

β-2 scope (from designs/0023 §5): `codes --json` export + registry; `l2-fix/v1`
position field (schema v2 with consumer sync); tool-failure exit-code band.
Until β-2 lands, this file is the authoritative statement of deviations.
