# Lom Design Rationale (v0.1)

> **Status**: Phase 0 Draft · 2026-08-02
> **Purpose**: Explain *why* Lom is AI-native, decision by decision. This document is the defense against the critique that "AI-native language" is marketing hype without language-level substance.

---

## 1. The Core Claim

Lom is AI-coding-native, not AI-workload-native. These are different:

| Direction | Question it answers | Lom's stance |
|---|---|---|
| AI-workload-native | "How do I write tensor ops / autodiff / GPU kernels?" | Phase 4+ (adjustable, not identity) |
| AI-coding-native | "How do I make LLMs write correct code with low error rate?" | **Phase 0-3 identity** |

The critique that "AI-native is marketing hype" (see `docs/ai-native-programming-languages.html` §6) is valid *when a language claims AI-nativeness without language-level design decisions that back the claim*. Lom's response is to make every syntax decision answerable to the question: **"does this reduce LLM error rate or aid recovery?"**

This document enumerates those decisions.

---

## 2. Decision: `end` keyword (not braces, not indentation)

**Chosen**: `fn f() ... end`, `if c ... end`, `match x ... end`
**Rejected**: `{}` (Rust/C), indentation (Python)

### LLM-coding-native rationale

LLM code generation has three failure modes that block delimiters affect:

1. **Unclosed braces**: LLMs writing 100+ lines frequently produce unbalanced `}`. The error is detected only at EOF, far from the cause.
2. **Misaligned indentation**: Python's whitespace sensitivity means a single mis-indented line changes semantics silently. LLMs mis-indent routinely.
3. **Deep nesting**: braces enable arbitrary nesting depth. LLMs produce "回字型" code that's hard to verify.

`end` addresses these:
- **Explicit closure**: `end` is a word, not a punctuation. LLMs generate it more reliably than matching `}` counts.
- **Indentation-free**: Lom ignores indentation. A mis-indented line is not a syntax error.
- **Nesting visibility**: `end` after each block makes nesting depth visible in the source, encouraging early returns.

### Trade-off

- More verbose than braces (extra `end` lines).
- No tooling ecosystem yet for `end`-based formatters (most formatters assume braces or indentation).

**Accepted**: verbosity is a feature for LLM generation (more tokens = more signal), and formatters can be built.

### Evidence

This decision aligns with docs File 2 (`ai-native-lang-construction-guide.html`) recommendation and Ruby/Elixir/Ada precedent. Docs File 4 (`ai-native-programming-languages.html`) does not record any AI-native language choosing indentation sensitivity.

---

## 3. Decision: Postfix types with infer-first

**Chosen**: `let x: Int = 3` (postfix, optional, infer-first)
**Rejected**: `Int x = 3` (prefix, mandatory)

### LLM-coding-native rationale

1. **Postfix reads naturally**: "let x be an Int, equal to 3" matches how LLMs describe variables.
2. **Infer-first reduces burden**: `let x = 42` is valid. LLMs don't need to annotate every variable.
3. **Annotations at boundaries**: function params and return types *should* be annotated (for LLM-debuggability of signatures), but locals can be inferred.
4. **Prefix types (C/Java style) are a known LLM error source**: LLMs frequently forget the type entirely or mismatch it. Postfix with optional inference sidesteps this.

### Trade-off

- Slightly more verbose than full inference (`let x = 3` vs `let x: Int = 3`).
- Two styles (annotated / inferred) means LLMs may inconsistently annotate.

**Mitigation**: `SPEC_FOR_AI.md` §10 "common mistakes" tells LLMs to prefer explicit annotation at function boundaries.

---

## 4. Decision: Result + match + `?` (not try/catch)

**Chosen**: `Result<T, E>` + exhaustive `match` + `?` operator
**Rejected**: `try { } catch (e) { }` exceptions

### LLM-coding-native rationale

This is the single most important LLM-coding-native decision. The rationale:

1. **Exhaustive match forces failure handling**: if a function returns `Result<T, E>`, the LLM *cannot* ignore the error case — the type system rejects non-exhaustive matches. Exceptions allow silent swallowing (LLMs routinely forget `catch`).
2. **`?` is concise**: `let x = read_file(path)?` is one token of overhead. Exceptions require 4-5 lines of `try/catch` boilerplate that LLMs get wrong.
3. **Errors are values, not control flow**: LLMs reason better about data than about non-local jumps. A `Result` is data; an exception is a goto.
4. **Type-directed repair**: when an LLM-generated program has a type error like "expected `Int`, got `Result<Int, String>`", the fix is obvious (add `?` or `match`). With exceptions, the equivalent bug (missing `try`) produces no type error.

### Evidence

Docs File 4 §3 "twelve design principles" lists "strong type system" and "exhaustive pattern matching" as core. Rust's `Result` is the proven model. Zero (docs File 4) uses a similar approach.

### Trade-off

- More verbose for functions that propagate many errors (mitigated by `?`).
- LLMs must learn `?` semantics (mitigated by explicit `SPEC_FOR_AI.md` documentation).

---

## 5. Decision: Pipeline `|>`

**Chosen**: `x |> f |> g` (left-to-right)
**Rejected**: `g(f(x))` (nested calls), `x.f().g()` (method chain)

### LLM-coding-native rationale

1. **Matches generation order**: LLMs generate left-to-right. `x |> f |> g` lets the LLM write the data source first, then transformations in order. `g(f(x))` forces it to write the outermost function first, before the inner ones exist.
2. **No method name tracking**: `x.f().g()` requires LLM to know that `f` returns a type with method `g`. With `|>`, `f` and `g` are just functions taking the previous value.
3. **Linear is verifiable**: each step is a one-function transformation. LLMs can self-check "does `f` accept what `x` produces?" at each step.

### Trade-off

- `|>` requires functions to be written in curried / first-argument form. Multi-arg functions need partial application or lambda:
  ```
  "hello" |> fn(s) { s + " world" } end
  ```
  This is verbose. **Phase 2 may add partial application syntax** (open question in LANGUAGE_SPEC §11).

---

## 6. Decision: Explicit imports (no wildcards)

**Chosen**: `from math import {sin, cos}`
**Rejected**: `import math.*` / `import math`

### LLM-coding-native rationale

1. **Prevents symbol fabrication**: docs File 1 (`ai-lang-project-setup-guide.html`) identifies "LLM 编造符号" as a top error source. Wildcard imports make this worse — the LLM has no list of available symbols, so it guesses.
2. **Explicit is debuggable**: when an import is missing, the error is `NAM003: undefined 'sin'` with a `fix: "add 'sin' to import from 'math'"`. With wildcards, the LLM has no signal that `sin` was never exported.
3. **Forces LLM to know what it uses**: this is a feature, not a bug. LLMs that must declare imports produce more predictable code.

### Trade-off

- More verbose (must list every imported name).
- Refactoring requires updating imports (mitigated by tooling: `lom fix --plan --json` can suggest additions).

---

## 7. Decision: Structural types (not nominal)

**Chosen**: `{x: Int, y: Int}` (shape-based records)
**Rejected**: `class Point { x: Int; y: Int }` (nominal classes)

### LLM-coding-native rationale

1. **No name tracking**: LLMs don't need to remember "is this `Point` or `Vec2` or `Coordinate`?" — only the shape `{x: Int, y: Int}`.
2. **Shape compatibility is obvious**: `{x: 3, y: 4}` and `{x: 3.0, y: 4.0}` have different types (Int vs Float fields), and the LLM can see this from the literals. With nominal types, the LLM must track class hierarchies.
3. **Reduces import burden**: no need to import a `Point` type. The shape is self-describing.

### Trade-off

- No dispatch on type name (no virtual methods based on nominal identity).
- No way to distinguish two records with the same shape but different intent (`UserId = Int` vs `OrderId = Int` are the same type structurally).
- **Mitigation**: type aliases (`type UserId = Int`) provide documentation but not type safety. Nominal newtypes may be added in Phase 3 if needed.

### Evidence

TypeScript's structural typing is the proven model for LLM-friendly code generation. Docs File 4 §3 lists "structural types" as a generation-friendly design.

---

## 8. Decision: Default immutability

**Chosen**: `let x = 3` is immutable; `let mut x = 3` for mutable
**Rejected**: everything mutable by default (Python/JS)

### LLM-coding-native rationale

1. **Reduces state-tracking burden**: LLMs struggle to track mutation across long functions. Immutable-by-default means the LLM can rely on "x is always 3" once declared.
2. **Forces explicit mutation**: `let mut` is a signal to the LLM (and human reader) that this variable changes. The LLM can't accidentally mutate.
3. **Parallel to Rust's lesson**: Rust proved that `let`/`let mut` distinction is teachable to LLMs (Rust code generation is well-supported).

---

## 9. Decision: Last expression = return value

**Chosen**: blocks evaluate to their last expression; `return` is for early exit only
**Rejected**: explicit `return` required (C/Java)

### LLM-coding-native rationale

1. **Reduces `return` keyword noise**: LLMs frequently forget `return` or add it in wrong places. Making it optional removes a failure mode.
2. **Matches functional style**: `fn double(x) -> x * 2 end` is concise and matches how LLMs describe transformations.
3. **Early return still available**: `if cond return 0 end` for guard clauses — the LLM can still short-circuit.

---

## 10. Decision: Structured JSON diagnostics with `fix` field

**Chosen**: every error emits JSON with `error_code`, `fix`, `retry`, `hint`
**Rejected**: human-readable-only error messages

### LLM-coding-native rationale

This is Lom's **identity feature** (Phase 2 core). The rationale:

1. **LLMs consume JSON natively**: an error message like "TypeError: expected Int, got Float" is English prose. A JSON diagnostic with `fix: {suggestion: "add `: Float` annotation", ...}` is machine-actionable.
2. **Stable error codes enable learning**: `NAM003` always means "undefined variable". LLMs can be trained (or prompted) to recognize codes and apply fixes. Prose messages vary by wording.
3. **`retry` field guides iteration**: telling the LLM "retry: true" after a fixable error vs "retry: false" after a fundamental type mismatch saves wasted regeneration.
4. **Tolerant parsing enables partial feedback**: holey AST means the type checker can report errors on the *valid* portions of partial code, letting LLMs iterate without full rewrites.

### Evidence

Docs File 4 identifies Zero's JSON diagnostics as "可能成为最具影响力的单点创新" (potentially the most influential single innovation). Lom adopts and extends this.

---

## 11. What Lom does NOT do (and why)

### 11.1 No indentation sensitivity

Indentation-sensitive languages (Python) are the worst case for LLM generation: one mis-indented line silently changes semantics. Lom rejects this entirely.

### 11.2 No exceptions

Exceptions are non-local control flow. LLMs reason poorly about where an exception will be caught. `Result` makes error handling local and explicit.

### 11.3 No wildcard imports

See §6. Wildcards let LLMs fabricate symbols.

### 11.4 No nominal classes (Phase 0-3)

Nominal classes require tracking type names, hierarchies, and method tables. Structural records are simpler for LLMs. (Phase 3 may add traits for shared behavior, but types remain structural.)

### 11.5 No tensor / autodiff / GPU (Phase 0-3)

These are workload-native features. Lom defers them to Phase 4 (adjustable milestone). Adding them in Phase 0-3 would dilute the LLM-coding-native focus and put Lom in direct competition with Mojo (a losing battle).

### 11.6 No complex metaprogramming

Macros, compile-time code generation, and reflection are excluded in Phase 0-3. They make LLM code harder to verify (the generated code is not visible). Lom prioritizes *what you see is what runs*.

### 11.7 No separate `Char` type (Phase 5.24 decision)

Single-char Strings are Char (the Python/JS model). Three reasons: (a) **LLM-native** — Python, the language LLMs know best, has no char type; Rust's `'a'` vs `"a"` distinction is a documented LLM confusion source (same class as `a..b` vs `a..=b`). (b) **The need never materialized** — the bootstrap lexer has used `split(s, "")` char scanning since Phase 5.0 without issue; the measured bottlenecks were List's representation (5.19) and linear lookups (5.20/5.21), never the absence of Char. (c) **Simplicity** — one fewer primitive type for both LLMs and the checker. If a future compiler backend needs byte-level control, that decision belongs to the compiler phase, not the interpreter.

---

## 12. How to evaluate Lom's AI-nativeness

The test is not "can LLMs write Lom?" (they can write any language). The test is:

> **Given a fixed set of tasks, do LLMs produce correct Lom code at a higher rate than they produce correct code in a baseline language (e.g. Python, Rust), with fewer repair iterations?**

This is measured by the Phase 2 evaluation set (100 tasks, in `eval/`). The evaluation methodology:

1. Prompt LLM with `SPEC_FOR_AI.md` + task description.
2. Generate N samples per task.
3. Run each sample through the Lom parser + type checker + interpreter.
4. Measure: syntax validity rate, type-check pass rate, runtime correctness rate, repair iterations to fully correct.
5. Compare against baseline: same tasks in Python/Rust, run through their toolchains.

**Only if Lom shows measurable improvement is the "AI-native" claim substantiated.** This is the hard evidence that distinguishes Lom from marketing-driven "AI-native" claims.

---

## 13. Relationship to existing projects

| Project | What Lom borrows | What Lom does differently |
|---|---|---|
| MoonBit | Tooling-first philosophy, LLM-friendly syntax choices | Lom makes LLM-coding-native the *primary* identity, not a side effect; open evaluation set |
| Zero | JSON diagnostics, `fix` field, tolerant parsing | Lom is a full language (not just a diagnostic system); includes structural types, Result, pipeline |
| Rust | `Result`, `?`, `match`, `let`/`let mut`, postfix types | Lom uses `end` not `{}`; structural not nominal; no borrow checker (Phase 0-3) |
| TypeScript | Structural typing, infer-first | Lom uses `end` blocks; no JS ecosystem baggage; explicit effects |
| Elixir | `|>` pipeline | Lom is statically typed; not BEAM-bound |
| Unison | Content-addressing (considered, not adopted) | Lom uses file-based modules; content-addressing is optional future optimization |

---

## 14. Open philosophical questions

These are not resolved by this document and may require Phase 1-2 experience to answer:

1. **Is "LLM-coding-native" a stable concept?** LLMs evolve. Syntax decisions optimal for GPT-4 may be suboptimal for GPT-6. Lom's mitigation: stable error codes and JSON diagnostics are model-agnostic; syntax is conservative (familiar Rust-like shell).
2. **Should Lom optimize for humans or LLMs when they conflict?** Lom's answer: optimize for LLMs first (Phase 0-3), reassess at Phase 3 exit. If LLM capabilities plateau, human readability matters more. If LLMs improve, more LLM-friendly features can be added.
3. **Is the evaluation set a sufficient proof?** 100 tasks is small. The set will grow over time and accept community contributions. The claim "AI-native" is always *relative to the current eval set*, not absolute.

---

## 15. Changelog

- **v0.1 (2026-08-02)**: Initial draft. 10 design decisions documented, trade-offs and evidence cited, evaluation methodology defined.
