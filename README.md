# Lom

**Lom (Language of Machine)** — an AI-native programming language.

LLM-coding-native first, workloads later. Built in Rust.

## Status

🚧 **Phase 0 — Language Design & Specification** (in progress)

See [`docs/lom-project-guide.html`](docs/lom-project-guide.html) for the full project guide (positioning, design philosophy, 7-phase roadmap, target LLM strategy, risk mitigation, repo governance).

## What is Lom

Lom is a **progressively fused** AI-native programming language:

1. **Phase 0-3 — LLM-coding-native**: tolerant parser, structured JSON diagnostics, linear pipeline syntax, structural types, gradual typing, explicit effect system. Target: LLMs write Lom with low error rate and easy recovery.
2. **Phase 4+ — Workload-native (adjustable milestone)**: tensor as first-class type, automatic differentiation, MLIR heterogeneous compute, Python interop. Target: AI/ML workloads.

## Why Lom

The "AI-native language" space is currently split in two directions, both with gaps:

| Direction | Leaders | Gap |
|---|---|---|
| Workload-native (tensor / autodiff / GPU) | Mojo, NEURON, Bend | Crowded. Mojo has 175k devs. Red ocean. |
| LLM-coding-native (tolerant parse / JSON diagnostics / linear syntax) | MoonBit (tooling side), Zero (JSON diagnostics) | Empty. Only MoonBit has intent, still in early stage. No public eval. |

Lom takes the **flanking route**: occupy the empty LLM-coding-native lane first with a differentiated position, then extend to workloads later.

## Design Philosophy

1. **Locality > Globality** — declarations and usage must close within the same view.
2. **Tolerance > Strictness** — syntax errors non-fatal, type errors degradable.
3. **Linear > Nested** — pipeline, early return, guard clauses over deep nesting.
4. **Self-describing > Implicit convention** — intent in code (executable docstrings, schema-as-type).

## Roadmap (summary)

| Phase | Name | Duration | Exit criteria |
|---|---|---|---|
| 0 | Language Design & Spec | 1-2 w | LLM reads spec, generates valid code >80% |
| 1 | Minimal Interpreter | 4-6 w | 10+ test cases pass with LLM-generated code |
| 2 | LLM-coding-native Core | 6-10 w | LLM generation pass-rate eval set meets baseline |
| 3 | Usable MVP | 8-12 w | Write a complete CLI tool or simple web service |
| 4 | Workload Extension (adjustable) | 12-20 w | Write a complete MNIST training script |
| 5 | Ecosystem & Bootstrapping | 16-24 w | Compiler self-hosts |
| 6 | Production | ongoing | Third-party projects deployed in production |

See the [project guide](docs/lom-project-guide.html) for details.

## Target LLM Priority

| Priority | LLM | Phase | Adaptation focus |
|---|---|---|---|
| P0 | DeepSeek | Phase 0 | SPEC_FOR_AI readability, error JSON field response, error pattern sampling |
| P1 | Claude / GPT | Phase 2 | Cross-model consistency baseline |
| P2 | Kimi / GLM / Gemini | Phase 3+ | Coverage expansion |

## Tech Stack

- **Host language**: Rust
- **Phase 1**: logos (lexer) + hand-written recursive descent (parser) + tree-walking (interp)
- **Phase 3**: + Cranelift JIT
- **Phase 4**: + MLIR (melior) + pyo3
- **Phase 5**: + LLVM AOT (inkwell) + WASM

## License

Apache-2.0. See [LICENSE](LICENSE).

## Contact

- GitHub: [lom-lang/lom](https://github.com/lom-lang/lom)
- Issues: <https://github.com/lom-lang/lom/issues>
