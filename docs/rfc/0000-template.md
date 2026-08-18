# RFC 0000: Template

- **RFC**: NNNN
- **Title**: (short descriptive title)
- **Status**: draft | accepted | rejected | superseded by RFC-XXXX
- **Created**: YYYY-MM-DD

## Motivation

Why is this change needed? What problem does it solve? Include evidence (measurements, eval failures, LLM error patterns) where possible — Lom decisions are data-driven.

## Proposal

The change itself: syntax, semantics, stdlib signatures, diagnostics. Be precise enough that an implementer can build it without guessing.

## LLM-impact analysis

Lom is LLM-coding-native: every language change must be evaluated for how LLMs will interact with it. Will LLMs confuse this with an existing construct? Does it match or break Python/JS/Rust habits? How will it appear in `SPEC_FOR_AI.md`?

## Alternatives considered

What else was considered, and why was it rejected? (Include the "do nothing" option.)

## Drawbacks

Honest costs: implementation complexity, doc churn, migration of existing examples/eval tasks.

## Unresolved questions

What is explicitly left open?
