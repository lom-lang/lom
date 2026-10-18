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
| §1.2 `RepairSuggestion` unified enums | Host fix engine: 4 actions (insert/replace/delete/hint) × 3 confidences (High/Medium/Low) — matches after enum-name normalization; `l2-fix/v1`→`v2` uses `confidence`/`action` with the same vocabulary and (v2, β-2) a structured `position` field | **Conformant by mapping** (case normalization documented here; position delivered in v2) |
| §1.3 AXIOM 1 (auto-apply only high non-hint) | `fix --apply` applies only `confidence == High && action != Hint` (src/fix.rs:12, :3344) | **Conformant** |
| §1.3 AXIOM 2 (cognitive → hint, never auto) | fix_corpus: five pairs (04/07/08/12/13) assert `fixed == bad`; the two cognitive-warning pairs among them (12 = NAM006 alias notice, 13 = MUT002 closure capture) are never auto-rewritten — the other three are medium-confidence suggestions below the auto-apply threshold | **Conformant** |
| §1.3 AXIOM 3 (plan/apply separate) | `--plan` side-effect-free; `--apply` explicit | **Conformant** |

## L1 — CLI protocol

| ACRI clause | Lom current behavior | Status |
|---|---|---|
| §2.1 `check [--json]` | `lom <file> --check --json` → envelope `schema/file/ok/summary/diagnostics` | **Conformant** |
| §2.1 `fix --plan [--json]` | `lom fix <file> --plan --json` → `lom-fix/v1` plans | **Conformant** |
| §2.1 `fix --apply` | `lom fix <file> --apply`, per AXIOM 1 | **Conformant** |
| §2.1 `codes [--json]` | Delivered in v1.10.0 (`src/codes.rs`): `lom-codes/v1` envelope, 46 entries (38 host codes + 8 L2 families) with code/severity/category/summary; severity cross-checked against runtime classifiers by unit test | **Conformant (β-2, 2026-10-12)** — emission-time code assignment remains a registered internal-quality candidate, not a conformance clause |
| §2.1 `explain <code>` (OPTIONAL) | Missing | Not planned (optional capability) |
| §2.2 exit codes (check err=1 / warn=0 / fix=0 / tool-fail ≥2) | v1.10.0: 20 tool-self-failure paths moved to rc ≥2 (missing input, usage errors, internal errors, write failures); diagnostic errors stay 1; locked by `tests/beta2_exit_codes.rs` ×3 | **Conformant (β-2)** with two registered deviations: `pkg-expand`'s mixed failure path stays 1 (carries PKG diagnostic codes — treated as diagnostic semantics); the LSP protocol band stays 1 (r58-locked protocol behavior) |
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
- **L1-conformant: yes (β-2 delivered, v1.10.0)** — all REQUIRED commands
  present, exit-code matrix honored with the two registered deviations above.
- **L2-documented: yes** (RepairLoopBench is the reference instance).

β-2 delivered in v1.10.0 (2026-10-12): `codes --json` (46-entry registry);
`l2-fix/v1` → `l2-fix/v2` position field (append-only, rust+python in lockstep,
parity 30/30); tool-failure exit-code band. This file remains the authoritative
statement of deviations.
