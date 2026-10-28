# docs/ — what's here, and in which language

This repository is deliberately bilingual, split by audience. Everything an
external reader needs is in **English**; the internal working documents are
maintained in **Chinese**.

## English (external-facing)

| Document | What it is |
|---|---|
| [LANGUAGE_SPEC.md](../LANGUAGE_SPEC.md) | The language spec. §14 is the v1.0 freeze declaration (syntax / 20 keywords / diagnostic codes / 43 builtins). |
| [SPEC_FOR_AI.md](../SPEC_FOR_AI.md) | The condensed spec written for LLM consumption — start here to write Lom. |
| [AGENTS.md](../AGENTS.md) | The first entry point for coding agents (build/verify commands, asset index, frozen surface, probe discipline). |
| [specs/acri/](../specs/acri/) | ACRI v0.1 — the open agent-compiler repair-interface spec; this repo is its reference implementation. |
| [eval/README.md](../eval/README.md) | The RepairLoopBench benchmark card (how to re-run, scope statement, SWE-bench-family differentiation). |
| [REPRODUCING.md](REPRODUCING.md) | The third-party reproduction guide — engineering baseline, LLM experiments, what a fresh clone can and cannot verify, and how to report results. |
| eval/REPORT*.md | The LLM evaluation reports — each opens with an English TL;DR block; bodies are Chinese. |
| [CONTRIBUTING.md](../CONTRIBUTING.md) / [SECURITY.md](../SECURITY.md) / [CODE_OF_CONDUCT.md](../CODE_OF_CONDUCT.md) / [DESIGN_RATIONALE.md](../DESIGN_RATIONALE.md) | Governance and design rationale. |
| [README.zh-CN.md](../README.zh-CN.md) | The Chinese README mirror (README.md itself is English). |

## Chinese (internal working documents)

| Document | What it is |
|---|---|
| [MAINTENANCE.md](MAINTENANCE.md) | The maintenance protocol: roles, iron rules, batch workflow, review cadence. |
| [HANDOVER.md](HANDOVER.md) | Baseline numbers, environment pitfalls, language pitfalls, full regression command list. |
| [TODO.md](TODO.md) | The living status block: ledger, adjudication menu, delivered-batch records. |
| [HANDOFF_PROMPT.md](HANDOFF_PROMPT.md) / [EXECUTOR_TEMPLATE.md](EXECUTOR_TEMPLATE.md) | Planner/executor collaboration documents. |
| [designs/](designs/) | Per-batch engineering design documents (all delivered). |
| [rfc/](rfc/) | The RFCs and their revision records — the per-batch behavioral history. |
| [reviews/](reviews/) | System-internal adversarial review reports (archived; originals are never rewritten). |
| [archive/](archive/) | The historical log. |
| [lom-project-guide.html](lom-project-guide.html) | The human tutorial (Chinese; English references: SPEC_FOR_AI / LANGUAGE_SPEC). |

## Why the split

The project is developed in LLM co-authorship with a Chinese-speaking
maintainer, so the working documents are Chinese. The machine-checked claim
anchors ([tools/doc_audit.py](../tools/doc_audit.py)) are written against the
Chinese wording — translating those documents would be a large, lossy change
with no reader to serve. Product output (diagnostics, fix suggestions) is
Chinese by design: the repair loop's contract is the structured code and
JSON, not the prose.
