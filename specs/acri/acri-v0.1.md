# ACRI — Agent-Compiler Repair Interface

**Version 0.1 (Draft) · 2026-10-12 · Maintained by the Lom project (implementation-neutral)**

> **Authoritative source moved (2026-10-28)**: the normative home of this
> specification is now [lom-lang/acri](https://github.com/lom-lang/acri)
> (`spec/acri-v0.1.md`, byte-identical, tagged v0.1). This copy is a
> **reference-repo snapshot** kept for local linking; on any divergence the
> acri repository wins (a numbered version is immutable there — changes ride
> version bumps and its CHANGELOG).

ACRI defines the interface between a compiler/toolchain and the AI agents that
consume its diagnostics and act on repair suggestions. It is a thin, deliberate
contract: structured diagnostics, confidence-graded repair suggestions with a
safety axiom, a minimal CLI protocol, and an evaluation methodology for
repair-loop benchmarks.

> **Status**: Draft v0.1. Field additions are minor-version changes; breaking
> changes require a major version. Consumers MUST ignore unknown fields
> (forward-compatibility axiom). See §6 for governance.

## 0. Scope and neutrality

- ACRI standardizes the **interface**, not any language, runtime, or vendor.
  An implementation MAY be a compiler, linter, transpiler, or language server
  backend for any language. The reference implementation is Lom
  (https://github.com/lom-lang/lom); conformance requires no adoption of the
  Lom language.
- Motivating evidence (external, verifiable): on repo-level compilation-error
  repair, GPT-5 reaches 73% syntactic correctness but only 41% semantically
  valid patches ([ComBench, arXiv:2603.27333](https://arxiv.org/abs/2603.27333));
  OpenAI stopped reporting SWE-bench Verified citing contamination and test
  defects ([why we no longer evaluate SWE-bench Verified](https://openai.com/index/why-we-no-longer-evaluate-swe-bench-verified/));
  and no cross-vendor structured-diagnostics schema for agents exists as of
  2026-10 (MCP-LSP bridges reuse LSP types without redefining them).

## 1. L0 — Object model (normative)

### 1.1 Diagnostic object

| Field | Type | Req | Semantics |
|---|---|---|---|
| `code` | string | REQUIRED | Stable diagnostic code, format `<NS><NNN>` (namespace + digits). The namespace registry lives in the implementation's machine-readable code table (§2.1 `codes`). Codes MUST be stable across releases of the implementation. |
| `severity` | `"error"` \| `"warning"` | REQUIRED | `error` blocks compilation/equivalent; `warning` does not. |
| `message` | string | REQUIRED | Human-readable text. Any natural language; agents consume `code`, not `message` wording. |
| `file` | string | REQUIRED | File path as the tool addresses it. |
| `position` | object | REQUIRED | `{ "line": ≥1, "col": ≥1, "anchor": "exact" \| "name-first" \| "none" }`. `exact` = precise point. `name-first` = the first occurrence of the nearest associated name — an honest degradation for tools whose diagnostics cannot afford precise spans; MUST NOT be fabricated as `1:1`. `none` = the channel has no position (e.g. whole-file checks). |
| `stage` | string | OPTIONAL | Pipeline stage that produced it (e.g. `lex`, `parse`, `type`, `codegen`). Implementation-defined vocabulary. |
| `hint` | string | OPTIONAL | One-line corrective hint aimed at the reader (human or agent). |

Implementations MAY add extension fields (e.g. `source_line`, `is_hole`);
consumers MUST ignore unknown fields.

**Example** (captured verbatim from the reference implementation; extension
fields `source_line`/`is_hole` shown):

```json
{
  "severity": "error",
  "stage": "type",
  "code": "NAM003",
  "message": "未定义变量 'y'",
  "file": "sample.lom",
  "line": 3,
  "col": 13,
  "source_line": "    println(y)",
  "is_hole": false,
  "hint": "是否想用 'x'？"
}
```

(Note: the reference implementation's current `lom-diag/v1` carries `line`/`col`
at top level and three position conventions across channels; its mapping to the
ACRI `position` object — `anchor` reflecting each channel's semantics — is in
`lom-conformance.md`.)

### 1.2 RepairSuggestion object

| Field | Type | Req | Semantics |
|---|---|---|---|
| `code` | string | REQUIRED | The diagnostic code this suggestion addresses. |
| `confidence` | `"high"` \| `"medium"` \| `"low"` | REQUIRED | Unified grading across all suggestion sources. |
| `action` | `"insert"` \| `"replace"` \| `"delete"` \| `"hint"` | REQUIRED | `hint` = advisory text only, never a rewrite. |
| `suggestion` | string | REQUIRED | The repair instruction, written for the agent that will act on it. |
| `payload` | object | OPTIONAL | Action parameters (e.g. replacement text, insertion point). Implementation-defined shape per action. |
| `position` | object | OPTIONAL | Same shape as §1.1; when absent, the associated diagnostic's position applies. |

### 1.3 Safety axioms (normative)

- **AXIOM 1** — An auto-apply mode MUST only apply suggestions with
  `confidence = "high"` AND `action ≠ "hint"`.
- **AXIOM 2** — Cognitive suggestions (behavior is already determined; the fix
  merely removes ambiguity, e.g. a deterministic name-collision notice) MUST be
  emitted with `action = "hint"` and are never auto-applied.
- **AXIOM 3** — Plan and apply are separate gates: producing suggestions MUST
  be side-effect-free; applying is always an explicit, distinct invocation.

Provenance: these axioms generalize the reference implementation's empirically
locked corpus (13 end-to-end fix pairs, five of which assert `fixed == bad`:
the two cognitive-warning pairs — a deterministic name-collision notice and a
closure-capture notice, whose auto-rewriting is a correctness regression
rather than a fix — and three medium-confidence suggestion pairs below the
auto-apply threshold).

## 2. L1 — CLI interaction protocol (normative)

### 2.1 Commands

| Command | Req | Semantics |
|---|---|---|
| `check [--json]` | REQUIRED | Analyze without executing. `--json` emits the envelope below. |
| `fix --plan [--json]` | REQUIRED | Emit `RepairSuggestion[]` for the current diagnostics. Side-effect-free (AXIOM 3). |
| `fix --apply` | REQUIRED | Apply per AXIOM 1. |
| `codes [--json]` | REQUIRED | Machine-readable code table: every emitted code with severity, one-line meaning, and category. (Agents and spec consumers bootstrap from this; it is also the namespace registry.) |
| `explain <code>` | OPTIONAL | Long-form documentation for one code. |

**Envelope** (`check --json`):

```json
{
  "schema": "<impl-schema-id>",
  "file": "…",
  "ok": false,
  "summary": { "total": 1, "errors": 1, "warnings": 0 },
  "diagnostics": [ /* §1.1 objects */ ]
}
```

### 2.2 Exit-code matrix (normative)

| Situation | Exit code |
|---|---|
| `check`: one or more error-severity diagnostics | 1 |
| `check`: warnings only, or clean | 0 |
| `fix --plan` / `fix --apply`: analysis completed | 0 |
| The tool itself failed (internal error, unreadable input) | ≥ 2 |

### 2.3 Conformance levels

- **L0-conformant**: emits §1.1/§1.2 objects and honors §1.3.
- **L1-conformant**: L0 + the §2.1 command set with the §2.2 exit-code matrix.
- **L2-documented**: publishes a repair benchmark following §3.

## 3. L2 — Repair-benchmark methodology (informative appendix)

A contamination-safe repair-loop benchmark SHOULD:

1. **Dual real capture** — diagnostics embedded in task prompts are captured
   verbatim from the real tool (`check --json`), never hand-written.
2. **Synthesis declaration** — tasks are authored, not scraped from public
   issue trackers; state this in the benchmark card.
3. **Multi-chain reference validation** — each reference solution is verified
   on every backend the toolchain ships (e.g. interpreter + compiled target),
   and expected outputs are locked.
4. **Black-box scoring** — score on `stdout` + exit code only; no
   implementation-detail leakage.
5. **Machine gate on prompts** — a checker re-runs the tool and compares the
   embedded diagnostics' code multiset against live output.
6. **Scope honesty** — state what the benchmark is not (e.g. not repo-scale,
   not directly comparable to SWE-bench-family leaderboards) and how its
   contamination defenses answer the published failure modes of those
   leaderboards.

Reference instance: **RepairLoopBench** (`eval/README.md` in this repository)
— 129 tasks, 32 repair tasks with verbatim host and cross-compiler diagnostics;
full-set pass@1 = 98.3% / 98.5% across two frontier models (measured on the
then-full 128-task set, 2026-10-07; the suite has since grown to 129).

## 4. Reference consumer (informative)

A minimal ACRI consumer (Python, ~40 lines) — bootstrap the code table, filter
actionable suggestions, drive the plan→apply loop:

```python
import json, subprocess
BIN, SRC = "lom", "sample.lom"

codes = {c["code"]: c for c in
         json.loads(subprocess.run([BIN, "codes", "--json"],
                     capture_output=True, text=True).stdout)}
check = json.loads(subprocess.run([BIN, SRC, "--check", "--json"],
                   capture_output=True, text=True).stdout)
for d in check["diagnostics"]:
    print(d["code"], "→", codes.get(d["code"], {}).get("summary", "?"))
plan = json.loads(subprocess.run([BIN, "fix", SRC, "--plan", "--json"],
                  capture_output=True, text=True).stdout)
# AXIOM 1 client-side: only high-confidence, non-hint actions are auto-applied
applicable = [f for f in plan["plans"]
              if f["fixes"] and f["fixes"][0]["confidence"] == "high"
              and f["fixes"][0]["action"] != "hint"]
print(f"{len(applicable)} applicable of {len(plan['plans'])}")
```

## 5. Change protocol

- Additive field/enum changes: minor version.
- Removal or semantic change of a REQUIRED field, or an axiom change: major.
- Draft (0.x) may revise freely between drafts; 1.0 is the first stable cut.

## 6. Governance

- Current steward: the Lom project. The specification is
  **implementation-neutral** (§0); stewardship of the language and of the
  specification are separable, and a future transfer to a neutral foundation
  is explicitly not precluded (precedents: MCP and AGENTS.md → Linux
  Foundation-hosted Agentic AI Foundation, 2025-12).
- Reference implementation: Lom (`specs/acri/lom-conformance.md` maps current
  behavior to this specification, including known gaps and their remediation
  batches).
