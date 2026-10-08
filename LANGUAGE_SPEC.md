# Lom Language Specification (v1.0)

> **Status**: **FROZEN (v1.0, 2026-09-02)** — 语言面冻结生效。冻结范围与程序见 §14。
> **Stability**: Stable — 语法、语义、保留字、诊断码、内建集在 v1.x 内保持稳定；任何语言面变更须走新 RFC 解冻程序。
> **Scope**: 完整语言规范（Phase 1-8 实现对齐；解释器为参考实现，WASM 后端与自举实现为交叉验证载体）。Phase 4 方向为 "LLM-repair-native + toolchain"（`lom fix` 自动修复、REPL、LSP、包管理；workload-native 已放弃）。见 [§2.5 retrospective](docs/lom-project-guide.html)。

---

## 1. Design Goals

Lom is designed so that **LLMs can write it with low error rate and easy recovery**. Every syntax decision in this spec is justified against that goal:

| Decision | Chosen | Rejected | LLM-coding-native rationale |
|---|---|---|---|
| Block delimiter | `end` keyword | braces `{}` / indentation | LLMs close `end` more reliably than `}`; indentation errors are catastrophic |
| Type annotation | `let x: Int = 3` (postfix, infer-first) | `Int x = 3` (prefix) / mandatory | Postfix type follows the variable name visually; inference reduces LLM burden |
| Error handling | `Result<T,E>` + `match` + `?` | `try/catch` exceptions | Exhaustive `match` forces LLM to handle failure; `?` propagates concisely |
| Pipeline | `x \|> f \|> g` | method chain `x.f().g()` | Linear pipeline matches LLM's left-to-right generation flow |
| Import | `from math import {sin, cos}` (explicit) | `import math.*` (wildcard) | Explicit imports prevent LLM from fabricating symbols |
| Structural types | `{x: Int, y: Int}` (shape-based) | nominal classes | LLM doesn't need to remember class names, just the shape |

---

## 2. Lexical Structure

### 2.1 File extension
`.lom`

### 2.2 Keywords (Phase 1 subset in **bold**, Phase 2+ in *italic*)

```
fn let mut if else elif while for in return
match end and or
True False
from import as enum
```

*Type names (`Int Float Bool String Unit Result Option Ok Err Some None`) are **ordinary identifiers**, not keywords — the parser recognizes them in type position.*
*(verified against `src/lexer.rs` v0.6.1: the 20 keywords above are the complete reserved set. `struct` / `trait` / `impl` / `type` / `pub` were listed here historically but have **never been reserved** — they are ordinary identifiers today; `pub`/traits are rejected for v1.0, see RFC-0001. `pipe` is not a keyword either — `|>` is a punctuation token. The older "reserved for later phases: async await mod use ref move where grad tensor" note described intent, not lexer reality — none of those are reserved either.)*

### 2.3 Operators (by precedence, low → high)

| Level | Operators | Assoc | Notes |
|---|---|---|---|
| 1 | `or` `and` | left | Short-circuit boolean |
| 2 | `==` `!=` `<` `>` `<=` `>=` | none | Comparison |
| 3 | `\|>` | left | Pipeline (left value → first arg of right function) |
| 4 | `+` `-` | left | Additive |
| 5 | `*` `/` `%` | left | Multiplicative |
| 6 | `!` `-` (prefix) | right | Unary |
| 7 | `?` (postfix) | left | Error propagation |
| 8 | `(` `)` `[` `]` `.` `{` `}` | — | Call / index / field / struct |

> `|>` sits between comparison and arithmetic: `1 + 2 |> f == 3` parses as `((1 + 2) |> f) == 3`.
> `=` is a statement form (not an expression operator); assignment does not produce a value.

### 2.4 Comments

```
# line comment
#- block comment -#
```

> Rationale: `#` is unambiguous (not a hash of operators, not a shebang in a typed language) and LLMs handle it reliably. Block comments use `#- ... -#` to stay in the `#` family.

### 2.4.1 Statement separation (newline-sensitive, not indentation-sensitive)

Lom is **newline-sensitive but not indentation-sensitive**. This is a critical distinction:

- **Newline-sensitive**: statements are separated by newlines. Each statement begins on a new line. This is like Go / Swift / Kotlin.
- **Not indentation-sensitive**: the *amount* of leading whitespace does not affect parsing. A block's body is delimited by `end`, not by indentation level. This is unlike Python.

Concretely:
```
# Valid: indentation is irrelevant, only newlines matter
fn f() -> Int
    let x = 1
        let y = 2    # extra indentation is fine, just style
    x + y
end
```

```
# Invalid: two statements on one line without separator
let x = 1 let y = 2    # ERROR: unexpected 'let'
```

If a statement must span multiple lines, wrap it in parentheses or use a continuation context (e.g. inside `|>` pipeline, which allows each step on its own line):
```
from string import {trim, upper}

# Valid: pipeline steps can each be on their own line
fn main() -> Unit
    "hello"
        |> trim
        |> upper
        |> println
end
```

> Rationale: newline-sensitivity gives LLMs a clear structural signal (one statement per line) without the catastrophic failure mode of indentation-sensitivity (one mis-indented line changes semantics). This is the same trade-off chosen by Go, Swift, and Kotlin.

### 2.5 Identifiers

```
identifier = letter { letter | digit | "_" }
```

- Convention: `snake_case` for variables/functions, `PascalCase` for types/structs/enums/traits.
- Keywords are reserved and cannot be used as identifiers.

### 2.6 Literals

```
int_lit    = digit { digit }            # 42
float_lit  = digit { digit } "." digit { digit }  # 3.14
bool_lit   = "True" | "False"
string_lit = '"' { char } '"'           # "hello"
unit_lit   = "()"                       # the unit value
```

String literals support escape sequences: `\n \t \r \" \\`.

---

## 3. EBNF Grammar (Phase 1 subset)

> This EBNF covers the minimal interpreter subset. Phase 2 features (match, Result, ?, |>, structural types, effects) are specified in §6.

```
program       = { item } ;

item          = fn_decl | enum_decl | import_decl ;

fn_decl       = "fn" identifier "(" [ params ] ")" [ "->" type ] [ "!" "[" effects "]" ] block ;
params        = param { "," param } ;
param         = identifier ":" type ;
block         = { stmt } "end" ;

stmt          = let_stmt
              | if_stmt
              | while_stmt
              | for_stmt
              | return_stmt
              | expr_stmt ;

let_stmt      = "let" [ "mut" ] identifier [ ":" type ] "=" expr ;
if_stmt       = "if" expr block { "elif" expr block } [ "else" block ] "end" ;
while_stmt    = "while" expr block "end" ;
for_stmt      = "for" identifier "in" expr block "end" ;
return_stmt   = "return" [ expr ] ;
expr_stmt     = expr ;

expr          = or_expr ;
or_expr       = and_expr { "or" and_expr } ;
and_expr      = cmp_expr { "and" cmp_expr } ;
cmp_expr      = pipe_expr { ("==" | "!=" | "<" | ">" | "<=" | ">=") pipe_expr } ;
pipe_expr     = add_expr { "|>" add_expr } ;
add_expr      = mul_expr { ("+" | "-") mul_expr } ;
mul_expr      = unary_expr { ("*" | "/" | "%") unary_expr } ;
unary_expr    = ("!" | "-") unary_expr | postfix_expr ;
postfix_expr  = primary_expr { call_suffix | index_suffix | field_suffix | "?" } ;
call_suffix   = "(" [ args ] ")" ;
index_suffix  = "[" expr "]" ;
field_suffix  = "." identifier ;
primary_expr  = literal
              | identifier
              | "(" expr ")"
              | block_expr ;

block_expr    = "fn" "(" [ params ] ")" [ ":" type ] block ;   # closure literal

literal       = int_lit | float_lit | bool_lit | string_lit | unit_lit ;

type          = base_type ;
base_type     = "Int" | "Float" | "Bool" | "String" | "Unit"
              | identifier   ;   # user-defined types (Phase 1: forward ref only)
```

### 3.1 Notes on the grammar

- **`end` closes every block**: `fn`, `if`, `while`, `for`, closure. No braces in Phase 1.
- **`if` requires `end`**: `if cond ... end`, `if cond ... else ... end`, `if cond ... elif cond2 ... else ... end`.
- **`let` without `mut` = immutable**: default immutability reduces LLM state-tracking errors.
- **Closures use the same `fn ... end` form**: `let f = fn(x: Int) -> Int { x + 1 } end` — wait, this is inconsistent, see §3.2 fix.

### 3.2 Closure literal (corrected)

Closures reuse `fn` keyword but with arrow `->` for return type to distinguish from named function declarations:

```
closure       = "fn" "(" [ params ] ")" [ "->" type ] block ;
```

Example:
```
let add = fn(x: Int, y: Int) -> Int
    x + y
end
```

> The block's last expression is the return value (no explicit `return` needed for the last expr). Named functions use `fn name(...)` and closures use `fn (...)` (no name).

---

## 4. Type System (Phase 1: minimal, Phase 2: full)

### 4.1 Phase 1 types

| Type | Values | Literal examples |
|---|---|---|
| `Int` | 64-bit signed integer | `42`, `-7`, `0` |
| `Float` | 64-bit IEEE 754 | `3.14`, `-0.5` |
| `Bool` | `True` / `False` | `True`, `False` |
| `String` | UTF-8 immutable | `"hello"` |
| `Unit` | the unit value `()` | `()` |

### 4.2 Type inference rules (Phase 1)

- `let x = 42`        → `x : Int`
- `let x = 3.14`      → `x : Float`
- `let x = True`      → `x : Bool`
- `let x = "hi"`      → `x : String`
- `let x = ()`        → `x : Unit`
- `let x: Int = 42`   → explicit annotation, checked against inferred
- Function params **must** be annotated (no inference across function boundaries in Phase 1, for LLM-debuggability)
- Function return type **may** be omitted; inferred from body

### 4.3 Phase 2 types (implemented, specified in §6)

| Feature | Syntax | Status |
|---|---|---|
| Structural records | `{x: Int, y: Int}` | §6.2 |
| `Result<T, E>` | `Ok(v)` / `Err(e)` | §6.1 |
| `Option<T>` | `Some(v)` / `None` | §6.1 |
| Tuples | `(Int, String)` | §6.3 |
| ~~Type aliases~~ | ~~`type UserId = Int`~~ | §6.5 — **never implemented** (`type` is an ordinary identifier; parse error, verified v0.6.1) |
| ~~Traits~~ | ~~`trait Show { fn show(self) -> String }`~~ | §6.6 — **rejected for v1.0** (RFC-0001) |
| `List<T>` (immutable, Phase 3.3) | `List<Int>` via `Type::Generic("List", [T])` | §9.3 |

> **Phase 3.3 `List<T>`**: a runtime `Value::List(ListVal)` variant (Rc cons cells since v0.5.0/Phase 5.19) exposed through the `list` stdlib module (§9.3). No list literal syntax yet — construct via `list_cons` or `json_parse`. Type-checker signatures use `List<_Any>` to accept any element type; element type tracking is deferred.

### 4.4 Why structural types (not nominal)

Structural types are chosen for LLM-coding-native reasons:
- LLM doesn't need to remember "is this `Point` or `Vec2`?" — just the shape `{x: Float, y: Float}`.
- Records with the same shape are interchangeable. Reduces import-tracking burden.
- Trade-off: no dispatch-on-name, no nominal identity. Acceptable for Phase 0-3 scope.

### 4.5 Gradual Type Checker (Phase 2.4 — implemented)

Phase 2.4 adds a **gradual** type checker (`src/typechecker.rs`). "Gradual" means: type annotations are optional and type errors are **non-fatal warnings** — the program still runs dynamically. This follows the LLM-coding-native principle *Tolerance > Strictness*.

#### 4.5.1 When type checking runs

| Mode | Type check? | Behavior |
|---|---|---|
| `lom <file>` (default run) | Yes (since v0.6.0) | Diagnostics on stderr; **never blocks execution** |
| `lom <file> --check` | Yes | Reports type diagnostics (human-readable); exit 1 only on Error-level, exit 0 on warnings |
| `lom <file> --json` | Yes | Emits `lom-diag/v1` JSON including `stage: "type"` diagnostics |

#### 4.5.2 Two-pass analysis

1. **Signature collection** — registers all `fn` signatures, `enum` definitions, and `import` aliases (alias inherits the real function's signature).
2. **Body check** — for each function body, walks statements/expressions, infers types, and reports mismatches.

#### 4.5.3 Error codes

| Code | Severity | Meaning |
|---|---|---|
| `NAM002` | Error | Duplicate function/enum definition |
| `NAM003` | Error | Undefined variable / undefined function call |
| `NAM004` | Error | Record has no such field / enum has no such variant |
| `NAM005` | Warning | Known builtin used without import (since v1.2.0 — static early warning for the runtime `RUNTIME002` failure; fix: add `from <module> import {name}` at the top; prelude `println`/`print` exempt) |
| `NAM006` | Warning | Name shadowing by an import (since v1.4.10, user-adjudicated): an `import` alias (or a merged-unit package symbol) collides with a local definition — the **local definition wins** and the collision is surfaced instead of silently resolved. Non-blocking (`ok:true`). |
| `TYPE001` | Warning | Type mismatch (binary op, let annotation, assignment) |
| `TYPE002` | Warning | `if`/`while` condition is not `Bool` |
| `TYPE003` | Warning | Function/variant argument count or type mismatch |
| `TYPE010` | Warning | Function/closure return type mismatch |
| `TYPE020` | Warning | `?` operator misused (operand not Result/Option, or enclosing function returns incompatible type) |
| `MAT001` | Warning | `match` non-exhaustive (user enum / Result / Option missing a variant; `_` wildcard makes it exhaustive) |

#### 4.5.4 Type compatibility rules

- **Structural equivalence**: records match by field name+type regardless of field order.
- **`_Any`** (internal wildcard type): prelude/stdlib function signatures use `Named("_Any")` to accept any type; it is compatible with everything and silences `TYPE001` in arithmetic.
- **Generic placeholders `T`/`E`**: internal `Result`/`Option` variant fields use `Generic("T"/"E")`; treated as `Unknown` in inference to avoid false positives.
- **Closures**: closure bodies inherit the enclosing environment (capture outer variables).
- **match arms**: pattern binders are injected into the arm environment; nullary variants (e.g. `None`, `Red`) parsed as `Binder` are recognized as variant constructors for exhaustiveness.

#### 4.5.5 What Phase 2.4 does NOT check

- Trait resolution (traits rejected for v1.0 — RFC-0001)
- ~~Effect system (Phase 2.5)~~ — implemented in Phase 2.5 (see §6.7)
- Generic instantiation (only placeholder compatibility)
- Precise tuple index inference (`.0`/`.1`)
- Cross-function closure call-site type checking (closures as values return `Unknown`)

#### 4.5.6 Registered builtins

Prelude (`println`, `print`) and stdlib modules (`io`, `string`, `math`) function signatures are registered at startup so that calls to them are not flagged `NAM003`.

---

## 5. Semantic Rules (Phase 1)

### 5.1 Immutability

- `let x = 3` — `x` is immutable. Reassigning it makes the typechecker emit a **`MUT001` warning** (since v0.20.0; warning-level per the gradual-typing philosophy — diagnostics go to stderr and never block execution, and the interpreter stays permissive). The check also covers function parameters, `for` loop variables, `match` pattern bindings, and compound assignments (`+=` etc., which desugar to `=`), since all of those are immutable bindings.
- `let mut x = 3` — `x` is mutable. `x = 4` is allowed.
- Compound assignment (v0.4.1, Phase 5.5): `x += e` / `x -= e` / `x *= e` / `x /= e` desugar to `x = x + e` etc. Target must be mutable. `+=` composes with string concat promotion.
- Function parameters are always immutable (no `mut` param in Phase 1).

### 5.2 Block return value

- The **last expression** in a block is the block's value.
- `return` is for **early exit** only. Using `return` as the last statement is allowed but discouraged.
- A block ending in a statement (e.g. `let` or assignment) has value `()`.

Example:
```
fn double(x: Int) -> Int
    x * 2
end

fn early_exit(x: Int) -> Int
    if x < 0
        return 0
    end
    x * 2
end
```

### 5.3 Control flow

- `if`/`elif`/`else` are expressions (each branch is a block with a value).
- `while` and `for` evaluate to `Unit`.
- `for x in collection` iterates. In Phase 1, `collection` must be a `String` (char iteration) or a range `a..b` (TODO: range syntax). Since Phase 5.3 (v0.4.1), `collection` may also be a `List<T>`, binding each element in order. Since Phase 5.6 (v0.4.2), the range expression `a..b` evaluates to a `List<Int>` over `[a, b)` and can be iterated or used anywhere a list is accepted.

### 5.4 Scope

- Blocks create scopes.
- Inner scopes can read outer bindings.
- Inner scopes may shadow outer bindings (the interpreter/codegen implement block scoping; collisions follow the §8.1 rules and the typechecker restores shadowed mutability flags at block exit since v1.4.13/v1.4.14/v1.4.15, covering `let`, destructuring `let (a, b)`, and for-loop variables). The Phase 1 draft disallowed shadowing to reduce LLM confusion; the implemented language allows it — import/package name collisions surface via NAM006/PKG007 per §8.1, while block-level shadowing itself is silent (mutability flags are restored at block exit).

---

## 6. Phase 2 LLM-Coding-Native Features (implemented)

> These features define Lom's identity as an LLM-coding-native language. Drafted in Phase 0; implemented in Phase 2.

### 6.1 Result and Option (error-as-value)

<!-- spec-check: skip: conceptual definition of builtin types — redefining Result/Option in real code is NAM002 -->

```
enum Result<T, E> =
    Ok(T)
  | Err(E)

enum Option<T> =
    Some(T)
  | None
```

Usage with `?` operator (early-return on `Err`/`None`):
```
fn read_config(path: String) -> Result<Config, String>
    let content = read_file(path)?      # propagates Err if read_file fails
    let parsed = parse_json(content)?   # propagates Err if parse fails
    Ok(parsed)
end
```

The `?` postfix operator:
- On `Result<T, E>`: if `Ok(v)`, yields `v`; if `Err(e)`, returns `Err(e)` from the enclosing function.
- On `Option<T>`: if `Some(v)`, yields `v`; if `None`, returns `None` from the enclosing function.
- Requires the enclosing function to return a compatible `Result` or `Option`.

### 6.2 Structural records

```
let p = { x: 3, y: 4 }          # inferred type {x: Int, y: Int}
let q = { x: 3, y: 4, z: 5 }    # inferred type {x: Int, y: Int, z: Int}
# p and q have different types; p cannot be used where {x, y, z} is expected
let r = { x: 3, y: 4 }
# p and r have the same structural type; interchangeable
```

Field access: `p.x`. **Field assignment is not implemented** — `p.x = 5` is a parse error (PARSE000, verified; the assignment target must be a plain variable, and `let mut` does not enable field assignment). Records are immutable (§11: "for immutable structured data use records"); to change a field, construct a new record (`let p2 = {x: 5, y: p.y}`), or use the `map` module for mutable shared structures.

### 6.3 Tuples

```
let pair: (Int, String) = (1, "hello")
let (n, s) = pair       # destructuring
```

### 6.4 Pattern matching (`match`)

```
match expr
    pattern1 => arm1
    pattern2 => arm2
    _ => default         # wildcard, catches all
end
```

**Match arm syntax** — two forms, both valid:

**Form A: single-expression arm** (compact, each arm on one line)
```
match n
    0 => "zero"
    1 => "one"
    _ => "many"
end
```
- The arm expression is everything after `=>` on the same line.
- Arms are separated by newlines (no semicolons needed).
- Use this form when each arm's body is a single expression.

**Form B: block arm** (multi-statement, closed with `end`)
```
match result
    Ok(name) =>
        let message = greet(name)
        println(message)
    end
    Err(e) =>
        println("Failed: " + e)
    end
end
```
- `=>` is followed by a block of statements, closed with `end`.
- The block's last expression is the arm's value.
- Use this form when an arm needs multiple statements (e.g. `let` bindings before the result).

**Mixing forms**: allowed. Some arms can be single-expression, others can be blocks.
```
match result
    Ok(name) => println(name)          # Form A
    Err(e) =>                           # Form B
        let msg = "Error: " + e
        println(msg)
    end
end
```

**Guards** (v0.4.2, Phase 5.7): `pattern if cond => body`. The arm wins only when the pattern matches AND `cond` evaluates to `True`; otherwise matching continues with the next arm. Guards may reference variables bound by the pattern. A guarded arm does NOT count toward exhaustiveness (MAT001 still requires an unguarded covering arm or `_`), mirroring Rust semantics — the guard's truth is only known at runtime.
```
match n
    m if m < 0 => "negative"
    0 => "zero"
    m if m > 100 => "big"
    _ => "normal"
end
```

Patterns (Phase 2 subset):
- literals: `0`, `"hi"`, `True`
- binders: `x` (binds any value to `x`)
- wildcards: `_`
- enum variants: `Ok(v)`, `Err(e)`, `Some(v)`, `None`
- tuple destructure: `(a, b)`
- record destructure: `{x, y}` or `{x: px, y: py}`
- `or` patterns: `1 or 2 or 3`

**Exhaustiveness check**: `match` on `Result` and `Option` should cover all variants or have a `_` arm. Non-exhaustive match is a compile-time **warning** (`MAT001`, non-fatal — the program still runs). This nudges LLMs to handle failure branches.

**Arm separation**: arms are separated by newlines. No semicolons. This is consistent with Lom's newline-sensitive statement separation (§2.4.1).

### 6.5 Type aliases (design sketch — **never implemented**)

> **Status (v0.6.1)**: `type` is an ordinary identifier; `type UserId = Int` is a parse error (PARSE001, verified). Kept as a design sketch only — no type aliases are planned for v1.0.

❌ Do not write — never implemented (PARSE001, verified):

```
type UserId = Int
type Point = {x: Float, y: Float}
```

### 6.6 Traits (structural, Phase 2 draft — **rejected for v1.0**, RFC-0001)

> **Status (v0.6.1)**: `trait` / `impl` / `self` have never been keywords or implemented; there are no methods in the language (structural records carry data only). The v1.0 scoping decision (RFC-0001) is to **not** add traits — shared behavior needs will be re-evaluated by a future RFC if real demand appears. Kept as a rejected design sketch for reference.

❌ Do not write — never implemented (PARSE001, verified):

```
trait Show
    fn show(self) -> String
end

impl Show for Int
    fn show(self) -> String
        int_to_string(self)
    end
end
```

Traits are **structural** in Phase 2 (duck-typed): if a type has all methods of a trait, it implements the trait. No explicit `impl` required for structural conformance — but explicit `impl` is allowed for documentation and disambiguation.

> Rationale: structural traits mean LLMs don't need to track impl blocks to know if a method is available. The shape is enough.

### 6.7 Effect system (Phase 2.5 — implemented)

Effects declare side effects in the signature (bodies shown minimal so the examples are runnable — `print` is renamed `print_msg` because `print` is a builtin name):

```
fn read_file(path: String) -> Result<String, IoError> ! [IO]
    Ok("contents")
end

fn print_msg(s: String) -> Unit ! [IO]
    println(s)
end

fn now() ! [Clock]
    0
end
```

- `! [Effect1, Effect2]` after return type declares effects.
- Pure functions (no `!`) cannot call effectful functions — `EFF001` warning.
- In Phase 2.5, effects are a **compile-time annotation only** (no runtime effect tracking). They help LLMs (and humans) see at a glance which functions have side effects.
- Phase 5+ may introduce effect handlers (deferred).

#### 6.7.1 Effect checking rules

| Caller declares | Callee declares | Result |
|---|---|---|
| (nothing, pure) | (nothing, pure) | OK |
| (nothing, pure) | `! [IO]` | `EFF001` warning |
| `! [IO]` | `! [IO]` | OK |
| `! [IO]` | `! [Clock]` | `EFF001` warning (Clock not declared) |
| `! [IO, Clock]` | `! [IO]` / `! [Clock]` | OK |
| `! []` (explicit empty) | `! [IO]` | `EFF001` warning (equivalent to pure) |

#### 6.7.2 `main` is special

The `main` function **implicitly has all effects** — `EFF001` is never reported inside `main`. Rationale: `main` is the entry point; calling `println` and other side-effectful functions is the norm. Forcing `main` to declare `! [IO]` would add LLM burden without value (LLM-coding-native principle: *Tolerance > Strictness*).

#### 6.7.3 Closures

Closures do **not** carry their own effect annotations. Effect checking inside a closure body uses the enclosing function's effect set (i.e. closures inherit the outer function's effects). This avoids forcing LLMs to annotate closure literals.

#### 6.7.4 Standard library effect signatures

| Function | Effects |
|---|---|
| `println`, `print` | `[IO]` |
| `len`, `int_to_string`, `string_to_int`, `trim`, `upper`, `lower` | (pure) |
| `sqrt`, `abs`, `min`, `max` | (pure) |

#### 6.7.5 What Phase 2.5 does NOT do

- **No runtime effect tracking** — effects are compile-time annotations only.
- **No effect polymorphism** — a function either has a fixed effect set or is pure.
- **No effect handlers** — deferred to Phase 5+.
- **No effect inference** — effects must be explicitly declared (except `main`).
- **Cross-function effect propagation is not transitive** — calling a pure function `b` that internally calls an effectful `c` does not flag `b`'s caller (only `b` itself is flagged at its `c` call site).

### 6.8 Type info export (Phase 2.6 — implemented)

`lom info <file> [--json]` exports **declarations** (not type-check results) so an LLM can quickly learn "what does this file define?" before writing code that calls into it.

- **No type checking is performed.** `info` describes *what was declared*, not *what is wrong*. Type errors are reported by `--check` / `--json` (Phase 2.4).
- **Parse failures are still diagnostics.** If the source does not parse, `lom info` emits the standard `lom-diag/v1` schema (Phase 2.3) with parse errors and exits with code 1.
- **Schema: `lom-info/v1`** — a separate schema from `lom-diag/v1`, so LLMs can distinguish "context query" from "error report".

#### 6.8.1 lom-info/v1 schema

```json
{
  "schema": "lom-info/v1",
  "file": "main.lom",
  "ok": true,
  "functions": [
    {
      "name": "double",
      "params": [ { "name": "x", "type": "Int" } ],
      "ret_type": "Int",
      "effects": [],
      "is_main": false
    },
    {
      "name": "print_double",
      "params": [ { "name": "x", "type": "Int" } ],
      "ret_type": "Unit",
      "effects": ["IO"],
      "is_main": false
    }
  ],
  "enums": [
    {
      "name": "Result",
      "type_params": ["T", "E"],
      "variants": [
        { "name": "Ok",    "fields": ["T"] },
        { "name": "Err",   "fields": ["E"] }
      ]
    }
  ],
  "imports": [
    {
      "module": "string",
      "items": [
        { "name": "len",           "alias": "len" },
        { "name": "int_to_string", "alias": "int_to_string" }
      ]
    }
  ]
}
```

Top-level fields:
- `schema`: always `"lom-info/v1"` (stability contract for LLM consumers)
- `file`: source file path
- `ok`: `true` iff the file parsed successfully (declarations were collected)
- `functions[]`: every `fn` declaration in source order
- `enums[]`: every user `enum` declaration (built-in `Result`/`Option` are not repeated here)
- `imports[]`: every `from <module> import {...}` declaration

Per-function fields:
- `name`: function name
- `params[]`: `{ name, type }` — types are stringified (e.g. `"Int"`, `"Result<Int, String>"`, `"{ x: Int, y: Int }"`, `"(Int, String)"`)
- `ret_type`: declared return type as a string, or `null` if the function omits the return annotation
- `effects[]`: declared effect names (empty array = pure / no `! [...]` annotation)
- `is_main`: `true` for the `main` function (convenience flag; LLMs can locate the entry point without scanning names)

Per-enum fields:
- `name`: enum name
- `type_params[]`: type parameter names (e.g. `["T", "E"]` for `Result<T, E>`)
- `variants[]`: `{ name, fields[] }` — `fields` is a list of stringified types (empty list = nullary variant like `None`)

Per-import fields:
- `module`: module path (e.g. `"string"`, `"io"`, `"math"`)
- `items[]`: `{ name, alias }` — `alias` equals `name` when no `as` clause is used

#### 6.8.2 Type stringification rules

The `type` strings in `lom-info/v1` are produced by these rules (mirrors the `Type::to_string` representation):

| Type form | String |
|---|---|
| `Int` / `Float` / `Bool` / `String` / `Unit` | `Int` / `Float` / ... |
| Named (e.g. `MyType`) | `MyType` |
| `Option<T>` | `Option<T>` |
| `Result<T, E>` | `Result<T, E>` |
| Generic app `Name<A, B>` | `Name<A, B>` |
| Record `{ x: Int, y: Int }` | `{ x: Int, y: Int }` (with a single space after `{` and before `}`) |
| Tuple `(Int, String)` | `(Int, String)` |

#### 6.8.3 Human-readable format

Without `--json`, `lom info <file>` prints a terminal-friendly summary:

<!-- spec-check: skip: terminal output sample, not Lom source -->

```
=== examples/effects.lom ===

[functions] (5):
  fn double(x: Int) -> Int
  fn print_double(x: Int) -> Unit ! [IO]
  fn now() -> Int ! [Clock]
  fn log_with_timestamp(msg: String) -> Unit ! [IO, Clock]
  fn main() -> Unit (main)

[imports] (1):
  from string import {len, int_to_string}
```

The `[enums]` section is omitted when the file declares no enums; same for `[imports]`.

#### 6.8.4 What Phase 2.6 does NOT do

- **No type-check results.** Use `lom --check` or `lom --json` for diagnostics.
- **No cross-file info.** `info` reads a single file; transitive imports are not expanded (Phase 3 module system).
- **No expression-level types.** Only top-level declarations are exported; local `let` bindings and inferred expression types are not reported.
- **No positions.** Declaration line/col is not yet reported in `lom info` output. (Phase 3.2 adds `Span` to `FnDecl`/`EnumDecl` for diagnostic positioning, but `lom info` does not yet surface them.)

### 6.9 AI repair plan (Phase 2.7 — implemented; Phase 3.1 — `--apply` execution)

`lom fix <file> [--plan] [--json]` generates a **repair plan** for every diagnostic in the file. Each plan contains one or more `fixes` — machine-readable actions an LLM can apply (or use as guidance) to repair the code.

- **`--plan` (default)** generates the repair plan (`lom-fix/v1` schema). `--apply` (Phase 3.1) auto-applies `confidence=High` + `action≠Hint` fixes to the source file (`lom-apply/v1` schema); `--dry-run` previews without writing.
- **Fixes are generated for all diagnostic codes**: lex, parse, type, name, match, effect, runtime. Even when a precise edit can't be produced, a `hint` action with guidance text is emitted.
- **Schema: `lom-fix/v1`** — separate from `lom-diag/v1` (errors) and `lom-info/v1` (context), so an LLM can distinguish "repair plan" from "error report" and "context query".

#### 6.9.1 lom-fix/v1 schema

```json
{
  "schema": "lom-fix/v1",
  "file": "main.lom",
  "ok": false,
  "summary": {
    "total": 2,
    "applicable": 2,
    "skipped": 0
  },
  "plans": [
    {
      "diagnostic": {
        "code": "LEX001",
        "severity": "error",
        "stage": "lex",
        "line": 2,
        "col": 13,
        "message": "未闭合的字符串"
      },
      "fixes": [
        {
          "description": "在字符串末尾添加 \" 闭合",
          "action": "insert",
          "line": 2,
          "col": 19,
          "end_line": null,
          "end_col": null,
          "text": "\"",
          "confidence": "high"
        }
      ],
      "retry": true
    }
  ]
}
```

Top-level fields:
- `schema`: always `"lom-fix/v1"`
- `file`: source file path
- `ok`: `true` iff no diagnostics were found (plans is empty)
- `summary`: `{ total, applicable, skipped }` — `applicable` counts plans with at least one non-hint fix or hint-with-text; `skipped` = `total - applicable`
- `plans[]`: one plan per diagnostic

Per-plan fields:
- `diagnostic`: embedded diagnostic reference (`code`, `severity`, `stage`, `line`, `col`, `message`) — redundant with `lom-diag/v1` but self-contained so the LLM doesn't need to cross-reference
- `fixes[]`: 0..N fix actions (currently always ≥1; every code has at least a hint)
- `retry`: `true` if at least one fix provides an applicable repair (non-hint action, or hint with concrete `text`); `false` if only pure-text hints

Per-fix fields:
- `description`: human-readable explanation of the fix
- `action`: `insert` / `replace` / `delete` / `hint`
- `line`, `col`: start position (1-based; `0` means "no specific location" — used by `hint`)
- `end_line`, `end_col`: end position (only `replace`/`delete`; `null` for `insert`/`hint`)
- `text`: text to insert/replace with (string for `insert`/`replace`; `null` for `delete`; string or `null` for `hint` — when present, it's a suggested snippet the LLM can use directly)
- `confidence`: `high` / `medium` / `low` — how certain the fix generator is

#### 6.9.2 Action types

| Action | Semantics | When used |
|---|---|---|
| `insert` | Insert `text` at `(line, col)` | LEX001/LEX002 (insert `"` at line end); EFF001 (insert `! [E]` or `, E`) |
| `replace` | Replace `(line,col)..(end_line,end_col)` with `text` | NAM003/NAM004 spelling suggestions (medium); MUT001 `let` → `let mut` rescan (high, with R55 caveat) |
| `delete` | Delete `(line,col)..(end_line,end_col)` | LEX005 (delete unexpected char) |
| `hint` | Text guidance only; `line`/`col` may be `0` | Most type/runtime errors; user-enum MAT001 and ambiguous name/mutability cases |

#### 6.9.3 Fix strategies by error code

| Code | Strategy | Confidence | Action |
|---|---|---|---|
| `LEX001`/`LEX002` | Insert `"` at end of error line | high | insert |
| `LEX003`/`LEX004` | Hint: check number format | low | hint |
| `LEX005` | Delete the unexpected char | high | delete |
| `PARSE001` | Missing `)` at a line boundary → precise insert; ambiguous/missing-`end` cases remain advisory | high/medium/low | insert or hint |
| `PARSE002` | Hint: `Result<T, E>` needs 2 type params | medium | hint |
| `PARSE003` | Hint: `Option<T>` needs 1 type param | medium | hint |
| `PARSE099` | Hint: hole, complete syntax | low | hint |
| `TYPE001` | Hint: type mismatch | low | hint |
| `TYPE002` | Hint: condition must be Bool | medium | hint |
| `TYPE003` | Hint: arg count mismatch | low | hint |
| `TYPE010` | Hint: return type mismatch | low | hint |
| `TYPE020` | Hint: `?` misuse | medium | hint |
| `MAT001` | Missing Result/Option branch → insert before match `end`; user enum → text hint | high/medium | insert or hint |
| `NAM002` | Hint: duplicate definition | low | hint |
| `NAM003` | Did-you-mean spelling replacement when available; otherwise hint | medium/low | replace or hint |
| `NAM004` | Field/variant spelling replacement when available; otherwise hint | medium/low | replace or hint |
| `NAM005` | Insert exact missing builtin import at file start | high | insert |
| `MUT001` | Unique-name declaration rescan, replace `let` with `let mut`; otherwise hint | high/medium | replace or hint |
| `EFF001` | Insert effect annotation: `! [E]` at line end (pure fn) or `, E` before `]` (partial effects) | high | insert |
| `RUNTIME000` | Hint: generic runtime error (div/mod by zero; recursion depth > 80,000 since 2026-09-08 — structured message with the recursive fn's signature position) | low | hint |
| `RUNTIME001` | Hint: runtime type mismatch | low | hint |
| `RUNTIME002` | Hint: undefined at runtime | low | hint |
| `RUNTIME003` | Hint: hole execution | low | hint |
| `RUNTIME005` | Hint: module/symbol not found | medium | hint |

#### 6.9.4 Phase 3.1 `--apply` execution

`lom fix <file> --apply [--dry-run] [--json]` auto-applies high-confidence fixes to the source file.

- **Safety filter**: only `confidence=High` AND `action≠Hint` fixes are applied. Low-confidence fixes are left for the LLM to decide. This is an intent, not a current proof of safety: audit R55 (2026-09-21) reproduced incorrect High edits for cross-scope MUT001, multiline-signature EFF001, and multiple effects inserted at the same source position. Until R55 closes, use `--dry-run`, inspect, and re-run diagnostics.
- **Text patching**: fixes are applied via `(line, col)` → byte-offset translation; `insert`/`delete`/`replace` all supported.
- **Reverse-order application**: multiple fixes are sorted by `(line, col)` descending and applied back-to-front to avoid offset drift.
- **`--dry-run`**: outputs the apply result (`lom-apply/v1` schema or human-readable) without writing the file.
- **`--json`**: outputs `lom-apply/v1` schema with `applied`/`skipped`/`changes`/`ok` fields.

`lom-apply/v1` schema:

```json
{
  "schema": "lom-apply/v1",
  "file": "main.lom",
  "applied": 2,
  "skipped": 1,
  "changes": [
    { "line": 3, "col": 22, "action": "insert", "description": "..." }
  ],
  "ok": true
}
```

Current implementation note (R55): `lom-apply/v1.ok` is computed as `applied > 0`; it does **not** certify that the final source parses or has zero diagnostics. This field must not be used as a repair-success oracle until the open remediation changes its semantics or adds an explicit final-diagnostics field.

#### 6.9.5 Current limitations

- **No fix prioritization.** When multiple fixes exist for one diagnostic, they are listed in order but not ranked; the LLM chooses.
- **No cross-file fixes.** A fix in file A referencing a missing import in file B is out of scope (Phase 3 module system).
- **LEX001 position precision.** When an unclosed string spans multiple lines, the lexer reports the position of the last unclosed `"` rather than the first; `--apply` follows the reported position. This is a lexer diagnostic-precision issue, not an `--apply` issue.
- **EFF001 multi-effect merge.** When a function already declares `! [IO]` and is missing `Clock`, `--apply` inserts `, Clock` before `]` to produce `! [IO, Clock]`. This handles the common case; deeply nested effect expressions are not parsed.

#### 6.9.6 Phase 3.2 AST span-based diagnostic positioning

Phase 3.1 used `find_fn_line` (a source-line scanner in the typechecker) to locate the function signature line for EFF001. Phase 3.2 replaces this hack with proper AST `Span` metadata:

- **`Span` type** (`src/ast.rs`): `{ line, col, end_line, end_col }` (1-based, matching `SpannedToken`). Added to `FnDecl` and `EnumDecl`.
- **Parser fills spans**: `parse_fn_decl`/`parse_enum_decl` record the `fn`/`enum` keyword position as the start and use `prev_token_pos()` (the token before the body) as the signature end.
- **Typechecker consumes spans**: `FnSig` now stores `span: Span` (replacing `sig_line: usize`). `check_fn_body` sets `current_fn_span = f.span`; EFF001/TYPE010 diagnostics use `current_fn_span.line/col` instead of `(0,0)`. `collect_fn_sig` uses `f.span` for NAM002 (duplicate function). The `find_fn_line` source-scanning hack is removed.
- **End-to-end verified**: `lom examples/effects_bad.lom --check` reports `EFF001` at `(10:1)` and `(21:1)` (the `fn` keyword positions of `helper` and `bad_helper`), previously `(0:0)`; `lom fix --plan` produces inserts at `[10:25]` and `[21:35]` (re-verified 2026-08-31).

> **Scope**: Only `FnDecl`/`EnumDecl` carry spans in Phase 3.2. Superseded by §6.9.7 (Phase 3.2b, v0.21.0) — expressions and `let`/assignment statements now carry spans too.

#### 6.9.7 Phase 3.2b expression-level spans (v0.21.0)

Expression-level spans — the pending item from Phase 3.2 — are now implemented:

- **AST shape**: `Expr` changed from an enum to `struct Expr { kind: ExprKind, span: Span }` (the former enum is now `ExprKind`); `Stmt::Let` / `Stmt::Assign` each gained a `span` field. `Pattern` still carries no span (variant-name diagnostics in `match` patterns remain `(0,0)`).
- **Convention**: `start` = the node's first token position; `end` = the **start** of the node's last token (the lexer only records token starts — same convention as Phase 3.2 signature spans). All positions are 1-based **byte columns** (the lexer advances per byte); `lom fix` converts to char columns internally when building `replace` actions.
- **Parser fills**: `span_since(line, col)` / `span_from(parent_span)` helpers; left-associative combinators (binary/logical/pipe/range/call chains) take the left operand's start. Desugared synthetic nodes (e.g. `x += 1` → `x = x + 1`) reuse the target identifier's span.
- **Consumers**: NAM003 (undefined variable/call target), NAM004 (record field — positioned at the **field-name token** via the `Field` span's `end`), MUT001 (assignment target), TYPE001/TYPE002/TYPE003/TYPE020 now report precise positions instead of `(0,0)`. `lom fix` NAM003/NAM004 spelling repairs use the diagnostic position for a single-point `replace` and fall back to whole-token scanning only when the position is missing or fails a content sanity check. The interpreter and WASM codegen are behavior-neutral (they match on `expr.kind`); runtime errors still report `(0,0)`.

---

## 7. Error Model (Phase 2.3: structured JSON diagnostics — implemented)

### 7.1 Design goal

Errors are **machine-readable first, human-readable second**. Every diagnostic is emit-able as JSON for LLM consumption.

> **Phase 2.3 status**: implemented in `src/diagnostics.rs`. CLI flags `--json` / `--check` control output format.
> Future fields (`fix`, `retry`) and finer-grained code namespaces (NAM/TYP/EFF/MAT) are reserved for Phase 2.4-2.7.

### 7.2 JSON diagnostic format (lom-diag/v1, implemented)

```json
{
  "schema": "lom-diag/v1",
  "file": "main.lom",
  "ok": false,
  "summary": { "total": 1, "errors": 1, "warnings": 0, "holes": 0 },
  "diagnostics": [
    {
      "severity": "error",
      "stage": "runtime",
      "code": "RUNTIME002",
      "message": "未定义变量: 'fooo'",
      "file": "main.lom",
      "line": 12,
      "col": 4,
      "source_line": "    println(fooo)",
      "is_hole": false,
      "hint": "确认变量已声明/导入，拼写无误"
    }
  ]
}
```

Top-level fields:
- `schema`: always `"lom-diag/v1"` (stability contract for LLM consumers)
- `file`: source file path
- `ok`: `true` iff `diagnostics` is empty
- `summary`: counts (total/errors/warnings/holes)
- `diagnostics[]`: array of single diagnostics

Per-diagnostic fields (Phase 2.3):
- `severity`: `error` / `warning` / `info` (warning/info reserved for Phase 2.4 type checker)
- `stage`: `lex` / `parse` / `type` (Phase 2.4) / `runtime`
- `code`: stable string code (see §7.3); LLMs can learn these
- `message`: human-readable
- `file`, `line`, `col`: location
- `source_line`: the source line containing the error (or `null` if unavailable)
- `is_hole`: `true` if this diagnostic corresponds to a `Stmt::Hole` inserted by the tolerant parser
- `hint`: optional fix suggestion

Reserved for future phases (not in v1):
- `span`: `{ "start": [line, col], "end": [line, col] }` — the AST carries spans (Phase 3.2 declarations, Phase 3.2b/v0.21.0 expressions + `let`/assignment statements), but the `lom-diag/v1` JSON still surfaces only the flat `line`/`col` pair (the node's start; NAM004 on record fields uses the field-name token instead); the span's `end` is not serialized.
- ~~`fix`: `{ "description", "suggestion", "start", "end" }` — machine-actionable repair (Phase 2.7)~~ Moved to `lom-fix/v1` (Phase 2.7 implemented; see §6.9). Kept out of `lom-diag/v1` so the diagnostic schema stays a pure error report.
- ~~`retry`: whether LLM should retry generation after applying `fix` (Phase 2.7)~~ Likewise in `lom-fix/v1` per-plan field (Phase 2.7 implemented; see §6.9).

### 7.3 Error code namespaces

**Phase 2.3 implemented namespaces**:

| Prefix | Stage | Range | Examples |
|---|---|---|---|
| `LEX` | lex | LEX001-099 | `LEX001` unclosed string, `LEX005` unexpected char |
| `PARSE` | parse | PARSE001-099 | `PARSE001` expected token, `PARSE099` hole (tolerant parser) |
| `RUNTIME` | runtime | RUNTIME001-099 | `RUNTIME001` type mismatch, `RUNTIME002` undefined, `RUNTIME003` hole execution |
| `TYPE` | type | TYPE001-099 (since Phase 2.4) | `TYPE001` mismatch, `TYPE003` arg count, `TYPE020` `?` misuse |
| `EFF` | type | EFF001-099 (since Phase 2.5) | `EFF001` pure function calls effectful |
| `MAT` | type | MAT001-099 (since Phase 2.4) | `MAT001` non-exhaustive match |
| `NAM` | type | NAM001-099 (since Phase 2.4) | `NAM003` undefined variable (compile-time) |
| `MUT` | type | MUT001-099 (since v0.20.0) | `MUT001` reassigning an immutable binding; `MUT002` (v1.1.0, warning) closure body references a captured outer `mut` binding — interpreter/WASM capture semantics diverge (reads: interpreter sees the latest value, WASM the creation-time copy; assignments: since v1.5.1 the interpreter executes them per shared scope, host-WASM/L2 compile-reject) |

Note: in Phase 2.3 (no static type checker), name resolution errors were caught at runtime and classified as `RUNTIME002`. Phase 2.4 introduces `NAM` codes at compile time (via the gradual type checker); runtime `RUNTIME002` is still emitted when a dynamically-run program hits an undefined name that the type checker flagged as `NAM003`.

### 7.4 Tolerant parsing (Phase 2.2 — implemented)

The parser produces a **"holey AST"** on error:
- Unparseable statements become `Stmt::Hole { line, col }` placeholders, not parse failures.
- All errors are collected (not thrown) into `ParseResult { program, errors }`.
- Synchronization-point recovery: item-level (`fn`/`enum`/`from`/EOF), statement-level (newline + statement-start keyword), match-arm-level (discard bad arm, continue).
- The holey AST is **not directly executable** — the interpreter raises `RUNTIME003` on `Hole` — but it can be consumed by `lom --json` / `lom info` (Phase 2.6, implemented) / `lom fix` (Phase 2.7, implemented) to give LLMs full-context feedback.

This lets LLMs get partial feedback on partially-correct code, enabling iterative repair.

### 7.5 CLI (Phase 2.3 — implemented; Phase 2.6 adds `info`; Phase 2.7 adds `fix`; Phase 3.1 adds `--apply`)

```
lom <file.lom>                Run the program (default)
lom <file.lom> --json         Diagnose only, output JSON (lom-diag/v1), do not run
lom <file.lom> --check        Diagnose only, output human-readable with source pointer
lom info <file.lom>           Export type info (human-readable) — Phase 2.6
lom info <file.lom> --json    Export type info (lom-info/v1) — Phase 2.6
lom fix <file.lom>            Generate repair plan (human-readable) — Phase 2.7
lom fix <file.lom> --json     Generate repair plan (lom-fix/v1) — Phase 2.7
lom fix <file.lom> --plan     Explicit --plan flag (default)
lom fix <file.lom> --apply    Apply high-confidence fixes to source (lom-apply/v1) — Phase 3.1
lom fix <file.lom> --apply --dry-run   Preview apply without writing file — Phase 3.1
lom fix <file.lom> --apply --json      Apply with JSON output (lom-apply/v1) — Phase 3.1
lom <file.lom> --dump-ast    Print the AST as a deterministic indentation tree (spans excluded) — Phase 8 prerequisite (RFC-0003 §8.1 acceptance baseline)
lom --help | -h               Show help
```

Exit codes: `0` = success / no diagnostics / info export OK / fix plan generation OK / apply OK (even if 0 fixes applied); `1` = read/lex/parse/runtime error / apply write failure (note: `lom fix` exits `0` as long as the plan was generated or apply completed, even if diagnostics exist — the plan/apply result is the product).

Output streams:
- `--json` / `--check`: diagnostics to **stdout** (the diagnostic report is the program's product).
- `lom info` (with or without `--json`): type info to **stdout** (the info report is the product).
- `lom fix` (with or without `--json`): repair plan to **stdout** (the plan is the product).
- Default run mode runtime errors: diagnostics to **stderr** (program failure).

### 7.6 Limitations in Phase 2.3

- **Runtime error positions**: runtime diagnostics still report `line=0, col=0`; the message itself carries enough context to identify the failure (the interpreter does not thread spans through evaluation). Lex/parse diagnostics are fully positioned; typecheck diagnostics are positioned via `FnDecl.span` (Phase 3.2: EFF001/TYPE010/NAM002) and expression-level spans (Phase 3.2b / v0.21.0: NAM003/NAM004-field/MUT001/TYPE001/TYPE002/TYPE003/TYPE020).
- ~~**No `fix` / `retry` fields**: reserved for Phase 2.7 `lom fix --plan --json`.~~ **Implemented in Phase 2.7** — see §6.9. `fix` / `retry` live in the `lom-fix/v1` schema (separate from `lom-diag/v1`), not as fields of individual diagnostics.
- **Single-file only**: cross-file diagnostics arrive with the module system (Phase 3).

---

## 8. Module System

> **Phase 2.1.5 implements explicit imports for standard library modules** (io/string/math).
> User multi-file modules (`from utils.helpers import {...}`) arrive in Phase 3.

### 8.1 Syntax

```
from math import { sqrt, abs, min, max }
from string import { len, upper, lower, trim, int_to_string }
from io import { println as log }            # per-item alias
```

- **Explicit imports only**. Wildcard `import *` is forbidden.
- **Per-item alias**: `name as alias` (Python/Rust-style). The alias becomes the local name.
- **No re-export**. Re-export via explicit `pub` items (Phase 3).
- **Dotted module path**: `from utils.helpers import { format_date }` parses, but user modules are Phase 3; Phase 2.1.5 only resolves standard library module names (io/string/math).
- **Name resolution on collision (since v1.4.10, user-adjudicated; repeated aliases since v1.4.12)**: when an import alias, a package symbol, or another package's symbol collides with a **local definition, the local definition wins** (all three implementations); two packages exporting the same symbol resolve **by package-root path order — the later package wins** (deterministic across runs, backends, and rebuilds); and two imports binding the **same alias** (different real names) resolve to the **later import declaration** (all three implementations). Collisions are not errors: the type checker emits `NAM006` (warning) for alias/local and merged-unit shadowing, and the package layer emits `PKG007` (warning) naming both packages. Package `fn`s are auto-public — a package's real symbol name is callable without importing it (registered design since the package batch; `import` gates builtins, not package-symbol visibility).

### 8.2 Standard library modules (Phase 2.1.5; Phase 3.3 adds `list`/`json`; Phase 3.4 adds `file` + `string` extensions; Phase 3.5 adds `env`; Phase 5.20 adds `map`)

| Module | Exports | Notes |
|---|---|---|
| `io` | `println`, `print` | Also in prelude (auto-available); explicit import only needed for aliasing |
| `string` | `len`, `int_to_string`, `string_to_int`, `trim`, `upper`, `lower`, `split`, `contains`, `replace`, `starts_with`, `ends_with`, `char_from_code` | Phase 3.4 adds `split`/`contains`/`replace`/`starts_with`/`ends_with` (§9.2); v0.27.0 adds `char_from_code` |
| `math` | `sqrt`, `abs`, `min`, `max` | Must be imported to use |
| `list` | `list_empty`, `list_length`, `list_get`, `list_is_empty`, `list_head`, `list_tail`, `list_cons`, `list_map`, `list_filter`, `list_fold` | Phase 3.3 — immutable list ops (§9.3); v0.4.3 adds higher-order ops |
| `json` | `json_parse`, `json_stringify` | Phase 3.3 — zero-dependency JSON parser + serializer (§9.4) |
| `file` | `file_read`, `file_write`, `file_append`, `file_exists` | Phase 3.4 — file system I/O, all declare `[IO]` effect (§9.5) |
| `env` | `args` | Phase 3.5 — command-line arguments (§9.6) |
| `map` | `map_empty`, `map_set`, `map_get`, `map_has`, `map_remove`, `map_keys`, `map_values`, `map_size` | Phase 5.20 (v0.5.1) — string-keyed dictionary, reference semantics (§9.7) |

**Prelude** (auto-imported, no `from` needed): `println`, `print`.

Calling an unimported non-prelude builtin produces a structured error:
<!-- spec-check: skip: diagnostic message sample, not Lom source -->
```
符号 'len' 未导入。需在文件顶部声明：from string import {len}
```

### 8.3 Public/private (**rejected for v1.0** — RFC-0001)

> **Status (v0.6.1)**: `pub` is not a keyword and this syntax does not parse. The package manager (Phase 4.4) treats **all top-level `fn`/`enum` as public**; there is no privacy. RFC-0001 closed this question: **no `pub` keyword is planned** — per-item privacy adds a modifier LLMs must track without a demonstrated need at current package scale. The sketch below is kept for reference only.

❌ Do not write — never implemented (PARSE001, verified):

```
pub fn greet(name: String) -> String
    "Hello, " + name
end

fn helper() -> Unit       # private, not importable
    ...
end
```

### 8.4 Rationale (LLM-coding-native)

- Explicit imports prevent LLMs from fabricating symbols (the #1 source of LLM code errors in docs File 1's analysis).
- No wildcard means the LLM must know exactly what it's importing — it can't rely on "maybe this exists in the namespace".
- Per-item alias (not whole-import alias) matches Python/Rust convention, reducing LLM confusion.
- Prelude keeps the common case (`println`) zero-ceremony for examples and tests.

---

## 9. Standard Library

### 9.1 Prelude (auto-imported, Phase 1)

Available without `from` declaration:

| Function | Type | Notes |
|---|---|---|
| `println(x)` | `Any -> Unit ! [IO]` | Print with newline |
| `print(x)` | `Any -> Unit ! [IO]` | Print without newline |

### 9.2 Standard library modules (Phase 2.1.5; Phase 3.3 adds `list`/`json`; Phase 3.4 adds `file` + `string` extensions)

Require explicit `from <module> import { ... }`:

| Module | Function | Type | Notes |
|---|---|---|---|
| `string` | `len(s)` | `String -> Int` | String length (UTF-8 char count) |
| `string` | `int_to_string(n)` | `Int -> String` | |
| `string` | `string_to_int(s)` | `String -> Int \| Unit` | Phase 1 simplification: returns Unit on parse failure; the Phase 2.4 type checker does not enforce Result here (gradual typing — this signature is accepted as-is) |
| `string` | `trim(s)` | `String -> String` | Strip leading/trailing whitespace |
| `string` | `upper(s)` | `String -> String` | Uppercase |
| `string` | `lower(s)` | `String -> String` | Lowercase |
| `string` | `split(s, sep)` | `(String, String) -> List<_Any>` | Phase 3.4 — split by separator; empty `sep` splits into UTF-8 characters |
| `string` | `contains(s, sub)` | `(String, String) -> Bool` | Phase 3.4 — substring test |
| `string` | `replace(s, from, to)` | `(String, String, String) -> String` | Phase 3.4 — replace all occurrences |
| `string` | `starts_with(s, prefix)` | `(String, String) -> Bool` | Phase 3.4 — prefix test |
| `string` | `ends_with(s, suffix)` | `(String, String) -> Bool` | Phase 3.4 — suffix test |
| `string` | `char_from_code(cp)` | `Int -> String` | v0.27.0 — Unicode code point → single-char String (full planes incl. surrogate-pair range); invalid code points (negative, > 0x10FFFF, surrogate D800-DFFF) are runtime errors |
| `math` | `sqrt(x)` | `Float -> Float` (also accepts `Int`) | Square root |
| `math` | `abs(x)` | `Int -> Int \| Float -> Float` | Absolute value |
| `math` | `min(a, b)` | `(Int, Int) -> Int \| (Float, Float) -> Float` | Minimum |
| `math` | `max(a, b)` | `(Int, Int) -> Int \| (Float, Float) -> Float` | Maximum |
| `io` | `println`, `print` | same as prelude | Explicit import only needed for aliasing |

> All `string` functions are pure (no effect). `split` returns `List<_Any>`; element-type tracking is deferred.

### 9.3 `list` module (Phase 3.3 — implemented)

A pure, immutable list type exposed through the `list` standard library module. Internally backed by `Value::List(ListVal)` — Rc cons cells since v0.5.0 (O(1) cons/head/tail; random-access `list_get` is an O(n) walk); all operations return new `List` values without mutating the input (immutable semantics, in the spirit of functional data structures).

**Type representation**: `List<T>` is encoded as `Type::Generic("List", [T])`. The type checker signatures use `List<_Any>` to accept any element type; element-type tracking is deferred to a later phase.

**Construction**: there is no list literal syntax `[1, 2, 3]` yet. Build a list by chaining `list_cons` on `list_empty()`, from the range expression `1..4` (v0.4.2, `List<Int>`), or obtain one from `json_parse("[1,2,3]")` (which maps JSON arrays to `List`).

| Function | Type | Notes |
|---|---|---|
| `list_empty()` | `() -> List<_Any>` | Return the empty list |
| `list_cons(head, list)` | `(_Any, List<_Any>) -> List<_Any>` | Return a new list with `head` prepended; original list unchanged |
| `list_length(list)` | `List<_Any> -> Int` | Number of elements |
| `list_get(list, idx)` | `(List<_Any>, Int) -> _Any` | Element at 0-based index; runtime error on out-of-bounds (`idx < 0` or `idx >= length`) |
| `list_is_empty(list)` | `List<_Any> -> Bool` | True iff length is 0 |
| `list_head(list)` | `List<_Any> -> _Any` | First element; runtime error on empty list |
| `list_tail(list)` | `List<_Any> -> List<_Any>` | All elements but the first; runtime error on empty list |
| `list_map(f, list)` (v0.4.3) | `(Fn, List<_Any>) -> List<_Any>` | Apply `f` to each element, return the new list. `f` may be a closure literal or a named function (v0.4.2+) |
| `list_filter(f, list)` (v0.4.3) | `(Fn, List<_Any>) -> List<_Any>` | Keep elements where `f(x)` is `True` (non-Bool result is a runtime error, same truthiness rule as `if`) |
| `list_fold(f, init, list)` (v0.4.3) | `(Fn, _Any, List<_Any>) -> _Any` | Left fold: `acc = f(acc, x)` starting from `init` |

All functions are pure (no `! [...]` effect). Examples: [examples/list_demo.lom](examples/list_demo.lom).

### 9.4 `json` module (Phase 3.3 — implemented)

A hand-written, zero-dependency JSON parser and serializer (`src/json.rs`). Maps JSON values to Lom `Value` and back:

| JSON | Lom `Value` |
|---|---|
| object `{"k": v}` | `Record { fields: [("k", v'), ...] }` (key order preserved) |
| array `[a, b]` | `List { elems: [a', b'] }` |
| string `"..."` | `Str(...)` |
| number `42` | `Int(42)` when the fractional part is zero, otherwise `Float` |
| number `3.14` | `Float(3.14)` |
| `true` / `false` | `Bool(true)` / `Bool(false)` |
| `null` | `Unit` |

| Function | Type | Notes |
|---|---|---|
| `json_parse(s)` | `String -> _Any` | Parse a JSON string into a Lom value; runtime error on malformed JSON (carries the parser's position) |
| `json_stringify(v)` | `_Any -> String` | Serialize a Lom value to JSON; `Record` → object, `List`/`Tuple` → array, `Str` → string (with `"` escaping), `Int`/`Float` → number, `Bool` → `true`/`false`, `Unit` → `null`, closures/enums fall back to a best-effort string form |

The parser supports `\uXXXX` Unicode escapes, including surrogate pairs (e.g. `\uD83D\uDE00` → 😀). Strings follow RFC 8259: raw (unescaped) control characters U+0000..U+001F inside string literals are rejected (audit R59, 2026-09-21; structural whitespace between values remains legal); the same boundary is enforced on all three implementations (host, WASM host binding, self-hosted interpreter). Both functions are pure (no `! [...]` effect). Examples: [examples/json_demo.lom](examples/json_demo.lom).

> **Known limitation**: `json_stringify` of nested `Record`/`List` produces compact output (no pretty-printing); enum and closure values are not round-trippable through JSON. These are acceptable for the Phase 3 MVP scope.

### 9.5 `file` module (Phase 3.4 — implemented)

File system I/O exposed through the `file` standard library module. All four functions declare the `[IO]` effect — calling them from a user function requires that function to declare `! [IO]` (EFF001 enforces this); `main` implicitly has all effects.

| Function | Type | Notes |
|---|---|---|
| `file_read(path)` | `String -> String ! [IO]` | Read entire file as UTF-8 string; runtime error if the file cannot be read (missing, permission, non-UTF-8) |
| `file_write(path, content)` | `(String, String) -> Unit ! [IO]` | Overwrite (create or truncate) the file with `content` |
| `file_append(path, content)` | `(String, String) -> Unit ! [IO]` | Append `content` to the file (creates if missing) |
| `file_exists(path)` | `String -> Bool ! [IO]` | True iff a file/directory exists at `path` |

> **Effect discipline**: because every `file_*` function carries `[IO]`, a helper that reads a config file must be declared `fn read_config(p: String) -> String ! [IO]`. The type checker emits `EFF001` if the `! [IO]` annotation is missing. This keeps file I/O explicit and LLM-debuggable — a core LLM-coding-native goal.

Examples: [examples/file_demo.lom](examples/file_demo.lom).

### 9.6 `env` module (Phase 3.5 — implemented)

Command-line argument access. Pure function (reads interpreter-internal state, no side effect).

| Function | Type | Notes |
|---|---|---|
| `args()` | `() -> List<_Any>` | Return the program argument list. `argv[0]` is the `.lom` file path; `argv[1..]` are user arguments passed via the CLI `--` separator |

**CLI usage**: `lom <file.lom> -- <arg1> <arg2> ...` — everything after `--` is passed to the Lom program via `env::args()`.

> **Convention**: like C/Rust/Python, `argv[0]` is the program path. User arguments start at index 1. See [examples/todo.lom](examples/todo.lom) for a complete CLI tool that dispatches on `args()`.
>
> **Backend note (known divergence #8, since 2026-09-15)**: on the WASM backend `argv[0]` is the compiled `.wasm` binary path rather than the `.lom` source path (structural, not a bug); user arguments after `--` are byte-identical on both backends. Consume `args()[1..]` only — never print, hash, or branch on `args[0]` (see SPEC_FOR_AI §11f item 8).

Examples: [examples/todo.lom](examples/todo.lom) — a complete todo list CLI (add/list/done/remove/help) with JSON persistence.

### 9.7 `map` module (Phase 5.20, v0.5.1 — implemented)

A string-keyed dictionary backed by `Value::Map(Rc<RefCell<HashMap<String, Value>>>)`. **Reference semantics with interior mutability**: `map_set`/`map_remove` mutate the map in place (O(1)); `let` aliases share the same underlying map. This is deliberately unlike `List`'s immutable persistence (§9.3) — Map is the mutable shared-structure type; for immutable structured data use records.

| Function | Type | Notes |
|---|---|---|
| `map_empty()` | `() -> Map<_Any>` | Empty map |
| `map_set(m, k, v)` | `(Map<_Any>, String, _Any) -> Unit` | Insert or overwrite in place |
| `map_get(m, k)` | `(Map<_Any>, String) -> Option<_Any>` | `Some(v)` if present, else `None` |
| `map_has(m, k)` | `(Map<_Any>, String) -> Bool` | Key existence test |
| `map_remove(m, k)` | `(Map<_Any>, String) -> Bool` | Remove; `True` iff the key existed |
| `map_keys(m)` | `Map<_Any> -> List<_Any>` | All keys, **sorted** for deterministic output |
| `map_values(m)` | `Map<_Any> -> List<_Any>` | Values in the same sorted-key order as `map_keys` |
| `map_size(m)` | `Map<_Any> -> Int` | Entry count |

> `json_stringify` serializes a Map as a JSON object with sorted keys (byte order — UTF-8, matching `map_keys`). Design note: copy-on-write was considered and rejected — the builtin argument slice always holds an `Rc`, so `Rc::get_mut` would never succeed and every write would clone.

---

## 10. Examples

### 10.1 Fibonacci (Phase 1)

```
fn fib(n: Int) -> Int
    if n < 2
        n
    else
        fib(n - 1) + fib(n - 2)
    end
end

fn main() -> Unit
    let i = 0
    while i < 10
        println(fib(i))
        i = i + 1
    end
end
```

> Note: `i = i + 1` requires `let mut i = 0`. Fix:
```
fn main() -> Unit
    let mut i = 0
    while i < 10
        println(fib(i))
        i = i + 1
    end
end
```

### 10.2 Result + ? + match (Phase 2)

```
fn parse_and_double(s: String) -> Result<Int, String>
    let n = string_to_int(s)?
    Ok(n * 2)
end

fn handle(s: String) -> Unit
    match parse_and_double(s)
        Ok(n) => println(n)
        Err(e) => println("Error: " + e)
    end
end
```

### 10.3 Pipeline (Phase 2)

`|>` passes the left value as the **first argument** of the right function.
Left-associative; precedence is higher than comparison, lower than arithmetic.

```
fn double(x: Int) -> Int
    x * 2
end

fn add(x: Int, y: Int) -> Int
    x + y
end

fn main() -> Unit
    5 |> double |> println        # 10  (= double(5))
    10 |> add(3) |> println       # 13  (= add(10, 3))
    1 + 2 |> double               # 6   (= double(1+2), `+` binds tighter)
    5 |> double == 10             # True (`|>` binds tighter than `==`)
end
```

### 10.4 Structural record (Phase 2)

```
from math import {sqrt}

fn distance(p1: {x: Float, y: Float}, p2: {x: Float, y: Float}) -> Float
    let dx = p1.x - p2.x
    let dy = p1.y - p2.y
    sqrt(dx * dx + dy * dy)
end

fn main() -> Unit
    let a = {x: 0.0, y: 0.0}
    let b = {x: 3.0, y: 4.0}
    println(distance(a, b))
end
```

### 10.5 Explicit imports (Phase 2.1.5)

```
from string import { len, upper, trim }
from math import { sqrt, abs as absolute }

fn main() -> Unit
    println(len("hello"))              # 5
    println(upper(trim("  hi  ")))     # HI
    println(sqrt(16.0))                # 4
    println(absolute(-7))              # 7 (called via alias)
end
```

---

## 11. Open Questions (all closed — RFC-0001, 2026-08-23; see per-item adjudication)

1. **Range syntax**: `a..b` (Rust) vs `range(a, b)` (function) vs `a..=b` (inclusive)? Affects `for` loops.
   - **Resolved (v0.4.2, Phase 5.6)**: `a..b` (Rust-style, left-inclusive right-exclusive). It evaluates to `List<Int>`, so it reuses the for-in-List semantics (Phase 5.3) and the whole list module with zero new runtime machinery. `a..=b` rejected: two range operators are a known LLM confusion source; `1..(n+1)` is the explicit inclusive idiom.
2. **String concatenation**: `+` (overloaded) vs `++` (dedicated) vs `concat(a, b)` (function)?
   - **Resolved (v0.1.1)**: `+` is overloaded for `String + String`. Rationale: LLMs already expect `+` for string concat (Python/JS behavior), and Lom has no custom operator overloading for user types in Phase 0-3, so `+` on String is a built-in special case. `++` would be unfamiliar; `concat(a, b)` is verbose for a common operation.
   - **Extended (v0.4.1, Phase 5.4)**: if either operand of `+` is a `String`, the other operand is promoted via `to_display()` — `"n = " + 42` works without `int_to_string`. Rationale: `"x = " + n` was the most common LLM-natural pattern rejected by the language; promotion matches Python `f-string`/JS template-literal habits. Typechecker result type is `String`.
3. **Char type**: separate `Char` type, or treat single-char strings as Char (Phase 1 simplicity)?
   - **Resolved (v0.5.1, Phase 5.24): no separate `Char` type — single-char Strings are Char** (Python/JS model). Rationale: (a) LLM-native: Python — the language LLMs know best — has no char type, and Rust's `'a'` vs `"a"` distinction is a documented LLM confusion source (same class as `a..b` vs `a..=b`, see question 1); (b) the bootstrap lexer has used `split(s, "")` char scanning since Phase 5.0 and it works fine — the real performance bottlenecks were List's representation (5.19) and linear lookups (5.20/5.21), never the absence of Char; (c) one fewer primitive type keeps the type system small for both LLMs and the checker. Character-level work uses `split(s, "")` → `List<String>`; byte-level work is out of scope for a tree-walking interpreter.
4. **Match arm separator**: `=>` (chosen) vs `->` (conflicts with closure return type)?
   - **Resolved (v0.1.1)**: `=>` confirmed. `->` is used for closure return type (`fn(x: Int) -> Int`), so `=>` for match arms avoids ambiguity.
5. **Multiple return values**: tuples only, or out-params, or destructuring assignment?
   - **Resolved (v0.6.1, RFC-0001)**: **tuples + destructuring** — both already implemented (`return (a, b)`; `let (a, b) = ...` since v0.4.0/Phase 5.1). Out-params rejected: they add a second, mutable output channel that LLMs routinely confuse (C# `out` is a known error source); tuple return keeps outputs in the value domain where match/destructure apply uniformly.
6. **`self` / `self` keyword**: lowercase `self` (Rust) vs `this` (Java) vs explicit first param name?
   - **Resolved (v0.6.1, RFC-0001)**: **no `self`** — Lom has no methods at all (structural records carry data; behavior lives in plain functions). The question is moot by design, not deferred: with traits rejected (question 7), no construct needs a receiver keyword.
7. **Trait dispatch**: static (monomorphized) only, or dynamic (vtable) too?
   - **Resolved (v0.6.1, RFC-0001)**: **no traits in v1.0** — the §6.6 structural-trait sketch is rejected. Trait dispatch (static or dynamic) presupposes a method system Lom deliberately lacks. If real shared-behavior demand appears post-v1.0, it enters via a new RFC.
8. **`pub` granularity**: per-item `pub` keyword, or module-level `pub use`?
   - **Resolved (v0.6.1, RFC-0001)**: **no `pub`** — all top-level items are public (the Phase 4.4 package manager already works this way; `pub` was never a keyword). Per-item privacy is a modifier LLMs must track with no demonstrated need at current package scale. See §8.3.

All eight questions are now resolved (1-4 inline above; 5-8 by RFC-0001, 2026-08-23). The earlier note that these "will be resolved in Phase 0 spec iterations" was stale.

---

## 12. Evaluation Suite (Phase 2.8 — implemented)

Lom ships a 121-task evaluation suite at `eval/` to measure LLM generation pass-rate — the hard metric for Lom's "AI-native" claim. It is not part of the language proper, but tests conformance to this spec.

### 12.1 Layout

<!-- spec-check: skip: directory tree, not Lom source -->

```
eval/
  README.md              # design goals, format, runner usage
  manifest.json          # lom-eval/v1: 10 categories × task counts
  tasks/
    01_arithmetic.json        # 10 — §3 grammar, §4 types, §5 semantics
    02_control_flow.json      # 13 — §5 if/while/for/return, short-circuit
    03_types.json             # 13 — §4 Int/Float/Bool/String/Unit
    04_closures.json          # 13 — §6.3 first-class closures
    05_match_enum.json        # 16 — §6.4 match/enum, §6.5 Result/Option
    06_pipeline.json          # 10 — §6.6 `|>` linear pipeline
    07_records_tuples.json    # 10 — §6.7 structural records/tuples
    08_effects.json           #  5 — §6.8 explicit effects `! [IO, Clock]`
    09_modules.json           #  6 — §8 module system, §9 stdlib
    10_error_repair.json      # 22 — §7 diagnostics + §6.9 fix plan (AI-native core)
  runner/
    run.ps1                   # PowerShell (Windows, zero-dep)
    run.sh                    # Bash (Unix, needs jq)
    README.md
```

### 12.2 Task format

Each task is a JSON object:

```json
{
  "id": "001",
  "category": "arithmetic",
  "difficulty": "easy",
  "prompt": "Natural-language requirement for the LLM.",
  "solution": "Reference .lom source (must pass `lom <file>` and produce `expected`).",
  "expected": "Expected stdout (case-sensitive, LF-normalized).",
  "notes": "What the task exercises (spec section / pitfall)."
}
```

### 12.3 Runner

- `./run.ps1 -Verify` (Windows) / `./run.sh --verify` (Unix) — smoke-test reference solutions against `expected`. **128/128 pass on both backends (interpreter and WASM).**
- `./run.ps1 -CandidatesDir <dir>` — evaluate LLM-generated code. Reads `<id>.lom` from `<dir>`, runs each, compares stdout to `expected`. Reports per-category and overall pass-rate. Exit code 1 on any failure (CI-friendly).
- The runner only runs `lom` + compares stdout; it does **not** call any LLM API. LLM candidates are produced out-of-band (e.g. DeepSeek API batch) into a `candidates/` directory.

### 12.4 AI-native focus

`10_error_repair.json` (22 tasks, ~18% of the suite) is Lom's differentiator: instead of "can the LLM write code", it tests "can the LLM repair code given `lom-diag/v1` + `lom-fix/v1`" — directly validating the §7 / §6.9 toolchain that Phases 2.2–2.7 built.

### 12.5 Status

- Reference solutions: 128/128 pass on both backends (`./eval/runner/run.ps1 -Verify`, `-Backend wasm`).
- LLM pass-rate: **99/100 (99%)** — measured 2026-08-03 with expert model + thinking mode. 9/10 categories at 100%; sole failure (task 078) was output-format misunderstanding, not a language-feature error. See `eval/REPORT.md` for full analysis. **Phase 2 exit criterion met.**

---

## 13. Changelog

- **v1.9.1 (2026-10-09, repository version)**: build-gaps remediation (designs/0021; three registered boundaries closed, zero language-surface change). **① Multi-file package intra-package cross-reference false positive** — `lom build` (manifest-driven) checked each package file with only `externals_map[P]` (the package's transitive dependency closure), so file B referencing file A's symbol in the same multi-file package hit a false NAM003 (B's functions table has only B's own top-level items; A's symbols were in neither the table nor the externals). The fix passes `externals_map[P] ∪ (P.public_symbols − the file's own top-level symbols)` — the subtraction is mandatory to prevent same-file duplicate-fn NAM002→NAM006 downgrading (the external_symbols branch in the typechecker); single-file packages: subtraction = empty set, zero behavior change. **② PKG007 on manifest-driven builds** — `warn_public_symbol_clashes(&graph)` now called on the human-readable path (stderr; the JSON path's schema is untouched). **③ Main-file diagnostics** — the root directory's `.lom` files are now checked after the package loop, with full-graph externals (the root's dependency closure = all packages), symmetric with package-file behavior; a main-file fn colliding with a package symbol emits NAM006 (R105 semantics, already on the file-path route, now on the manifest route too — no new diagnostic code). A misplaced CLI doc block (claiming "rc 1 for package errors") corrected to the actual rc=0 semantics. 4 new integration tests; Rust 573 unit + **31→35** integration; verify_selfcomp 368/368 and the quine unchanged (self_comp untouched).

- **v1.9.0 (2026-10-08, repository version)**: l2fix CLI subcommand (designs/0018 adjudication-1-甲 follow-through). The L2 rejection-text code-line parser and seven-family repair-suggestion mapper (L2C/L2P/L2S/L2T/L2U/L2V/LEX + L2G fallback) move into the host binary as `lom l2fix <file|-> [--json] [--text ...]` — a new user-visible CLI subcommand. `src/l2fix.rs` (828 lines, zero regex): the python tool's two regex patterns are hand-translated search loops; the 43-name/8-module lookup wires directly to `interpreter::module_of` (single source of truth); JSON output uses an independent `l2-fix/v1` schema (byte-identical to the python envelope). Parity: 18 synthetic (code,message) pairs × 5 fields + 12 real negatives through the full self_comp → l2fix chain — zero field differences. 12 new unit tests (parse/suggest/all-families/name-table/JSON-roundtrip/pipeline) → Rust **561→573**; integration 31 unchanged. The frozen language surface remains unchanged (L2 tool codes are not host diagnostics — v1.6.0 precedent).

- **v1.8.0 (2026-10-08, repository version)**: L2 promotion — claims-face only. The L2 subset compiler's status wording across the README boundary section is updated from "experimental" to supported; the promotion basis (four consecutive A- review rounds, zero-open ledger, repair-loop completeness) is on record in the review reports. No behavior, no language-surface change.
- **v1.7.2 (2026-10-08, repository version; no external release; tag/CI backfilled in TODO)**: **L2 diagnostic line numbers** (designs/0020; user-adjudicated on all four points — route 甲 name-anchor output-layer lookup / `ln:cl` before the code / summary assertion / graduation-criteria inventory included; **zero `src/` changes, zero language-surface changes**). The L2 subset compiler's `codegen error` channel prefixes `ln:cl` (unified with the lex/parse channels' format) when the message's first quoted name resolves in a new **name-anchor table** built by `build_name_pos` — a linear prescan over the token stream (`{t: Tok, ln, cl}` triples already carry positions) recording each identifier's **first occurrence**; the table is built only inside the error branch of the single output-assembly point, so the success path is untouched: all 204 positive-case hex outputs are old-vs-new byte-identical (a direct comparison with the pre-change compiler via `git show`, beyond the behavior-level harness), and the strong quine re-derives at **272925 bytes** both sides. Registered precision boundary: the position is the name's first occurrence, not a precise span (same-name multi-sites point to the first; assignment-type errors may point at the definition site); messages without a quoted name, or whose first quoted name never appears as an identifier token (internal type strings 'i64'/'mp{i64}', dotted paths 'json.json_dump'), keep the bare format — measured on the corpus: 71 of 102 quoted-name negative lines carry a position, 31 miss via the registered exemption list. Chain updates: l2fix's `CODEGEN_LINE` gains an optional position group (group indices re-based; message stays pure text so the seven-family mapping is unchanged; `--self-check` 6/6); verify_selfcomp adds a machine-checked summary assertion "quoted-name negative ⇒ non-empty `ln:cl`" over the 164 negatives with the 31-entry exemption list (the 148 existing code+message dual locks untouched — substring matching is prefix/suffix-addition-safe by construction); eval task 129's embedded L2 text re-captured verbatim from the new output (line:col matching the host NAM005 diagnostic), 127/128 byte-identical. self_comp 12591→**12671 lines** (+80, four new functions); negative-lock count unchanged (148/164); eval 128/128 per backend unchanged. The L2 graduation criteria are inventoried in designs/0020 §4 (graduation itself a separate user adjudication). The frozen language surface remains unchanged.
- **v1.7.1 (2026-10-07, repository version; no external release; tag/CI backfilled in TODO)**: closes the twenty-seventh review's two openings R117/R118 (playground edge hardening; **zero `src/` changes, zero language-surface changes**). **R117**: the registered playground boundaries now quantify **source-code nesting depth** as a second, distinct path (parser/compile-side recursion — the interpreter call-depth path is separately guarded at 300): it shares the same engine-stack ceiling with **no structured guard**; node-measured ≈500 nested expressions trap (`RuntimeError: memory access out of bounds`) while desktop's 256 MB stack runs 3000+ fine; the trap surfaces through the existing three-layer fallback (harness rethrow → worker `{trap:true}` translation → app watchdog + neutral wording) rather than a structured diagnostic. The trap wording in `app.js`/`worker.js` is neutralized from "栈上限" (stack ceiling) to "引擎执行限制" (engine execution limit), because measured trap forms include `memory access out of bounds` and `unreachable`, not only `call stack exhausted`. **R118**: `playground/harness.js run()` falls back to the default argv `["lom", "/playground.lom"]` when `args` is absent **or an empty array** (an empty array is truthy and previously passed through, trapping the host on argv[1]; A/B probe: pre-fix `RuntimeError: unreachable`, post-fix normal run exit 0); the args contract is documented in the harness header. Negative locks honored (no parser depth guard, no new diagnostics, traps never swallowed). `node playground/smoke.mjs` ALL PASS (11 items); desktop baseline unchanged by construction (verify_selfcomp 368/368, bootstrap 14/14 quine 272293, Rust 561+31, eval 128/128 per backend, doc_audit 71/71). The frozen language surface remains unchanged.
- **v1.7.0 (2026-10-06, repository version; no external release yet; tag/CI backfilled in TODO)**: the online playground (designs/0019, user-adjudicated "review the plan, revise, then execute"; frozen language surface untouched; **host behavior on desktop unchanged — two `cfg(target_family = "wasm")`-gated spots only**): the full host toolchain is compiled to `wasm32-wasip1` (a new `[profile.wasm-release]`, `debug = false`, ~1.7 MB) and runs in the browser behind a hand-written zero-dependency WASI binding (`playground/harness.js`): args/environ, `clock_time_get` (fix-history timestamps), `random_get` (HashMap seeding), stdout/stderr capture, a minimal preopen + `path_open` over a single read-write virtual file `/playground.lom` (so `lom fix --apply`'s file rewrite works — the page shows a line diff), and ENOSYS stubs elsewhere; `proc_exit` maps to the result's exit code. The wasm depth guard is calibrated to **300** (node-measured engine stack ≈ 500 Lom frames; the structured diagnostic fires before the trap, deeper traps are caught and translated by the harness — dual defense). The desktop gate skips the 256MB-stack worker thread on wasm (wasi has no threads); the desktop call-depth constant stays 80,000. A node smoke suite (`playground/smoke.mjs`) runs the harness end-to-end against the desktop binary (arithmetic/fib byte-parity, the four-step repair loop, depth guard, `--check`) and joins CI as a seventh job together with the wasm build. Registered boundaries: browser recursion depth ≪ desktop (guard 300), the `file` module is a virtual single file in the playground, `env::args()` sees only the virtual script path, parser nesting has no software guard (engine stack applies). The frozen language surface remains unchanged.
- **v1.6.1 (2026-10-03, repository version; no external release; tag/CI backfilled in TODO)**: closes the twenty-sixth review's three P3 openings in one batch (RFC-0004 revision 54; user-adjudicated "execute then stand by"; host `src/` untouched). **R115**: the L2 subset compiler now rejects same-file duplicate user `fn` definitions in single-file mode — `codegen error: [L2P001] L2.2 子集不支持: 函数 'go' 重复定义（同文件唯一命名，请重命名其一）` — mirroring the host's NAM002 error face and the existing enum/variant-duplication precedent; the check lives in `collect_sigs` and is gated to single-file mode (`map_size(pkgs) == 0`) because file boundaries are unrecoverable in package-expanded units (cross-package same-name fns are the NAM006 deterministic-order legal surface, locked by pkg cases 106-108), and it only counts prior user fns (idx ≥ 0) so fn-over-import-alias "local wins" semantics are preserved; `classify_l2` gains the "重复定义" keyword (L2P family); new negative `neg_l2_dup_fn.lom`. **R114**: the v1.6.0 classify claims are scope-annotated at every claim site (corpus scope; the ~29 out-of-corpus keywords are deliberately not added — denser keywords raise misclassification risk on unseen combinations); the l2fix fallback comment and help text are corrected. **R116**: SECURITY.md's CI supply-chain version references refreshed to `@v5`/Node 24 (remaining annotations: Ubuntu 26 migration notices only, per the 2026-10-03 pre-unfreeze audit). Baseline: verify_selfcomp 367→**368** (196 + 8 + **164**), the quine moves to **272293 bytes** (byte-identical both sides), self_comp 12572→**12591 lines**, stock positive hex identical by construction, Rust 561+31 and eval 128/128 per backend unchanged. The frozen language surface and external publication freeze remain unchanged.
- **v1.6.0 (2026-10-01, repository version; no external release; tag at `6227a89` after six-job CI run `36878845005` #249 passed)**: L2 diagnostic structuring prerequisite (RFC-0004 revision 52; designs/0017; user-adjudicated on all three points; host `src/` untouched — pure L2/tooling/assets scope). **① Code system**: the L2 subset compiler's `codegen error` output now prefixes a structured code on every rejection — `codegen error: [L2V001] L2.2 子集不支持: …` — produced by a new output-layer keyword classifier in self_comp (`classify_l2`, 7 families L2E/L2P/L2S/L2V/L2C/L2U/L2T + L2G fallback, modeled on the host's `classify_runtime_error` and L1's `classify_lex_msg` precedents). All 263 `subset_err` call sites and all message bodies stay byte-identical; of the 141 distinct negative messages **100% classify into a family with a 0% fallback rate** (corpus scope — per R114, annotated in v1.6.1: roughly 29 further combination messages outside the negative corpus constructively reach the L2G fallback; the code stays nonempty, the message intact, and l2fix handles L2G as a low-confidence defensive hint). These are L2-tool codes, deliberately **not** host diagnostic codes (the §14 freeze governs the host surface; "the host accepts but L2 rejects" keeps its own code space to avoid same-code-different-meaning confusion). **② lex-channel latent bug fixed** (found by the design-phase probe): the L2 `lex error` records lacked the `code` field the reporter reads — any lex error crashed the L2 run with RUNTIME000; the four construction sites now fill it via the previously-dead `classify_lex_msg` (e.g. `lex error 4:15 [LEX005] 意外字符 '@'`), with 2 new lex negatives (verify_selfcomp 365→**367** = 196 + 8 + 163). **③ assertion upgrade**: the negative-lock table wholesale becomes code+message dual locks (147 entries) plus a code-nonempty assertion over all negatives. **④ eval re-capture**: tasks 127-129's embedded L2 text re-captured verbatim from the new output (L2V001/L2S001/L2U001; solutions/expected unchanged). Baseline: the quine moves to **271989 bytes** (byte-identical both sides, +2434 from the classifier code), self_comp 12544→**12572 lines**, stock positive hex identical by construction, selfhost six modes unaffected, eval 128/128 per backend. Line numbers deliberately deferred (full coverage needs AST spans, conflicting with the registered no-span front-end design; a ~118-site main-cluster pass is a future batch). The frozen language surface and external publication freeze remain unchanged.
- **v1.5.1 (2026-10-01, repository version; no external release; tag at `142a118` after six-job CI run `36859203394` #246 passed)**: closes **R112 (P2)** (RFC-0004 revision 51; user-delegated to the maintainer's judgment) — an interpreter-only bug fix, +9/−1 in `src/interpreter.rs`. Assigning to a captured outer `mut` binding from inside a closure body used to panic the interpreter with `RefCell already borrowed` (rc=1, guarded, non-silent; found during v1.5.0 and opened as R112). Root cause: in the named-closure call path the `env.borrow()` `Ref` lived as the if-let scrutinee temporary **across the whole branch including the closure-body execution**, so the body's `Stmt::Assign → set_existing` re-entered `borrow_mut` on the same Scope. The fix takes the closure value out first (dropping the Ref before the call); call sites otherwise unchanged. **Method note (honest deviation)**: the planner had initially suggested rejecting the form with an explicit runtime error (aligning with the other two backends' outright rejections), but reading the code showed MUT002's registered semantics already state "interpreter = shared scope" — the panic was an implementation bug blocking that registered semantics, so the fix releases the borrow instead and the assignment now **executes per the shared-scope semantics** (the R112 probe: `6`, outer variable visible after the call). Three-sided shape after the fix: interpreter executes (shared, rc=0, MUT002 warning still emitted) / host-WASM still **compile-rejects** ("赋值给未定义变量") per designs/0001 §2.3 / the L2 negative stays in-repo — the assignment form joins the read form (eval task 124: interpreter 100 vs WASM 10) in the same MUT002-flagged divergence family. The other four `call_closure` call sites were audited (direct-callee and HOF paths hold no Scope Ref across the body — probe-verified). An A/B old-vs-new binary comparison over 247 repo files (eval candidates + fix_corpus + examples) shows zero diffs — the change only affects the previously-panicking assignment form. 3 new integration tests (`tests/r112_closure_assign.rs`: shared-scope output, outer visibility, WASM-side still-rejects); Rust 561 unit + **28→31 integration** (592 total); verify_selfcomp 365/365 and the quine (269555) unchanged; eval 128/128 per backend unchanged. **R112 closed — the ledger is empty again.** The frozen language surface and external publication freeze remain unchanged.
- **v1.5.0 (2026-10-01, repository version; no external release; tag at `b61182f` after six-job CI run `36841096349` #242 passed)**: repair-loop deepening batch 1 (RFC-0004 revision 50; designs/0016; user-adjudicated on all three design points) — the repair-native thesis gains new assets on four fronts, with **zero language-surface change** (existing codes only gained handling; no new codes): **① fix dispatch table completed** — `NAM006` (import-alias/package-symbol collision notice) and `MUT002` (closure captured a `mut` binding) previously fell through to the misleading "unknown error code" fallback; both now emit proper Medium hints (NAM006: behavior is already deterministic, rename only to remove ambiguity; MUT002: don't rely on captured `mut` state — take a local copy or pass the value as a parameter). Both are hint-level and never auto-applied (2 new unit tests + apply-negatives). **② SPEC_FOR_AI updated** — §11c's code table and §11e's fix-strategy table gain the NAM006/MUT002 rows they were missing (context budget 39,959→41,029 chars). **③ fix_corpus 11→13 pairs** — `12_nam006_alias_clash` / `13_mut002_closure_capture` with fixed==bad (informative warnings are deliberately not auto-rewritten, per pair 04's precedent); selfhost six modes stay green with the new files in the bad-file sets. **④ error_repair 24→31 tasks (repo-wide 121→128)** — four host-face tasks (123 NAM006 disambiguation, 124 MUT002 rewrite whose broken form's genuine interpreter/WASM divergence (100 vs 10) is documented in notes, 125 TYPE001 annotation mismatch, 126 NAM002 genuine-duplicate — the error-level gap), plus **three L2-face tasks (127-129), a new task genre**: code the host accepts (warning or clean) but the experimental L2 subset compiler rejects — the prompt embeds the *real* host `--check --json` **and** the *real* L2 `codegen error` text (dual real-capture per the R60 discipline, the L2 text labeled as its own source, not passed off as lom-diag/v1); reference solutions are verified on three chains (host interp, host WASM, and the full L2 chain self_comp→hex→wasm); the runner still verifies host stdout only, with the L2-face generation discipline (real-capture + three-chain reference verification) documented in eval/README. Rust **559→561 unit** (+2); eval **128/128 per backend**; eval_prompt_check 31/31 (auto-traverses the task set); verify_selfcomp 365/365 and the quine (269555) unchanged (self_comp untouched); selfhost dump 161 (154 + 7 new task solutions). **New finding R112 (P2, pre-existing, opened not fixed)**: assigning to a captured outer `mut` binding from inside a closure body panics the interpreter (`RefCell already borrowed`, interpreter.rs:336 — rc=1 with the internal-error guard, not silent; the other two backends reject outright: host-WASM "assignment to undefined variable" per designs/0001 §2.3, L2 negative in-repo). Read-only capture does not panic. Probe retained under target/probes/. Also registered: the release-unfreeze checklist (7 informational items — the freeze stands until the user lifts it). The frozen language surface and external publication freeze remain unchanged.
- **v1.4.15 (2026-10-01, repository version; no external release; tag at `cf73de2` after six-job CI run `36829534116` #238 passed)**: closes the R110 **adjacent surface** — block-level `LetDestruct` name shadowing (RFC-0004 revision 48; user-adjudicated per the maintainer menu; designs/0015) — pure typecheck-diagnostics scope (+18/−6 in `src/typechecker/mod.rs`; self_comp/codegen/interpreter/fix engine untouched). The v1.4.14 `check_block` snapshot/restore matched only `Stmt::Let`, so a block-level `let (x, y) = …` destructuring (whose bindings are always immutable — both type branches `define(..., false)`) overwrote a same-named outer `let mut x`'s mutability flag without restoring it, drawing a spurious MUT001 on post-block `x = 5` (the twenty-fourth review's p13 probe: 1 warning at (8:5), runtime correct at 10/20/5). The snapshot condition now covers both statement forms name-by-name (the `contains_key` first-seen gate and the restore loop are unchanged), completing the three-step generalization: for-variable (v1.4.13) → block-level `let` (v1.4.14) → block-level destructuring (v1.4.15). The missed-report direction cannot exist for destructuring (bindings are always immutable); the negative — outer `let x` + block-level destructuring + post-block assignment — keeps its genuine 1 MUT001 with hint positioning, and the R75 fix engine keeps its conservative medium-hint demotion (applied=0, no rewrite). A whole-repo old-vs-new `--check` sweep over all 4,300 `.lom` files shows **zero MUT001-surface diffs** (10 lines before / 10 after, each verified a genuine immutable reassignment; the twenty-fourth review's own "8 files, 9 diagnostics" under-counted by one — the twenty-fifth review's R113 correction measures **10 lines across 8 files**, now unified to the by-line count). 5 new unit tests lock the three block kinds both ways, partial-name restore with the no-shadow leak divergence preserved, the unknown-type branch, and nesting/chaining; the MUT001/R65/for-quirk/r110 families stay green. Rust **554→559 unit** (+5; integration 28 unchanged); verify_selfcomp 365/365 and the quine (269555) unchanged (self_comp untouched). §5.4's shadowing sentence is reworded to attribute NAM006/PKG007 surfacing to the §8.1 import/package collision surface only (block-level shadowing is silent; mutability flags are restored at block exit — twenty-fourth-review precision note 2). The registered no-shadow leak divergence and the read-after-loop divergence remain untouched. The frozen language surface and external publication freeze remain unchanged.
- **v1.4.14 (2026-10-01, repository version; no external release; tag at `e8b099e` after six-job CI run `36770326768` #232 passed)**: twenty-third-review remediation closing **R110 (P2)** (RFC-0004 revision 47; user-adjudicated block-level snapshot/restore, the v1.4.13 for-quirk fix generalized) — pure typecheck-diagnostics scope. The typechecker's `check_block` had no block scoping: a `let` inside an `if`/`while`/`for` body overwrote the same-named outer binding's mutability flag in the function-level env and never restored it, failing **both ways** — a spurious MUT001 after `let mut x` + block-level `let x` + post-block `x = 5` (byte-identical to the genuine immutable case), and a missed MUT001 after `let x` + block-level `let mut x` + post-block assignment — while all three runtimes agree on block scoping (pure static-diagnostic distortion; the pre-existing form was even sitting in the checked-in R79 case with a false warning). `check_block` now snapshots the current-layer entry of each first-seen block-level `let` name (reusing `local_entry`) and restores it after the block's statements and tail expression: shadowed entries are restored (both failure directions fixed), no-previous-entry bindings are deliberately kept (the registered no-shadow leak divergence — post-block reads still pass statically, fail at runtime — stays untouched, per the v1.4.13 precedent); same-block repeated names restore to the pre-block entry; nested blocks restore inner-first; the for-variable snapshot/restore stacks independently (body restore before var restore); match arms run on child envs and never hit this surface. A whole-repository old-vs-new `--check` comparison over all 4,300 `.lom` files shows exactly one diff — the R79 case's false MUT001 cleared, runtime byte-identical. 6 new unit tests lock the three block kinds both ways plus the leak/nesting/closure preserved surfaces; the MUT001/R65/for-quirk families stay green. Rust **548→554 unit** (+6; integration 28 unchanged); verify_selfcomp 365/365 and the quine (269555) unchanged (self_comp untouched); eval 121/121 per backend, selfhost six modes, doc_audit 71/71, fmt gates re-verified. Registered adjacent surface (not in this adjudication): block-level `LetDestruct` name shadowing. **R110 closed — the ledger is empty again.** The frozen language surface and external publication freeze remain unchanged.
- **v1.4.13 (2026-10-01, repository version; no external release; tag at `7a48810` after six-job CI run `36753460493` #228 passed)**: closes the two remaining registered ledger items (user-adjudicated "resolve the in-ledger items, then run the twenty-third review"; RFC-0004 revision 46) — both pure diagnostics/tooling scope, interpreter and codegen untouched. **For-variable mutability quirk (registered since v1.2.3)**: the typechecker's `Stmt::For` branch defined the loop variable in the function-level env, and a same-named outer `let mut x` stayed shadowed-as-immutable after the loop — `let mut x = 1; for x in 1..3 …; x = 5` produced a spurious MUT001 warning (byte-identical to the genuine immutable case, so indistinguishable to a repair LLM) while the interpreter ran correctly. The branch now snapshots the current-layer entry before `define` and restores it after checking the body (a new `TypeEnv::local_entry()` helper; same-name shadowing is restored, no-name entries are deliberately kept — popping them would have flipped the separate registered divergence "reading the loop variable after the loop passes statically / fails at runtime" into a new NAM003 error, out of scope). The genuine immutable-shadowing negative, the loop-body-let leak surface, and the MUT001/R65 test families all stay green (5 new unit tests lock the three forms plus the two preserved surfaces). **Manifest-less `lom build` per-file NAM003 (registered since v1.4.9)**: the dependency-package per-file typecheck passed no externals, so every cross-package reference in a package source (aliases and plain real names alike — the 105 case's three `t3` errors and the 102 chain's four) was a false NAM003 error. The flow now precomputes each package's dependency-closure public symbols (the `manifest` field, previously dead code, supplies per-package dependency lists; cycle-guarded DFS) and typechecks each source with them as externals — 105/102 both go clean; a negative integration test locks that symbols from packages outside a file's own dependency closure still fail NAM003 (no over-release). Registered remaining boundaries: multi-file packages' cross-file references can still false-positive in this per-file view; the file-less flow does not emit PKG007 (it bypasses `collect_package_symbols`); the read-after-loop divergence is unchanged. Rust **543→548 unit + 25→28 integration** (5 + 3 new); verify_selfcomp 365/365 and the quine (269555, self_comp untouched) unchanged; selfhost six modes, eval 121/121 per backend, doc_audit 71/71, fmt gates re-verified. **Both registered items closed; the ledger is empty.** The frozen language surface and external publication freeze remain unchanged.
- **v1.4.12 (2026-09-30, repository version; no external release; tag at `11cdc68` after six-job CI run `36723102998` #225 passed)**: R107 remediation (user-adjudicated: later-import-wins + an NAM006 variant), closing the last open ledger item. When two packages import **different real names under the same alias** (`from libr import {origin as g}` + `from libs import {base2 as g}`), the interpreter and host-WASM already resolved to the **later import declaration** while the L2's alias registry kept the first — the same program silently printed different values across backends (103 vs 4, no diagnostics; the twenty-second review's sole P1). The L2's registration now overwrites (an `alias_regs` set in collect_sigs pass 2; designs/0008's "keep the existing summary" note is superseded), making all three sides byte-consistent (103; the reversed declaration order yields the other later import, order-insensitive), and the collision surfaces as a new NAM006 variant — "import 别名 'g' 重复——取后写声明（遮蔽 libr::origin）" — on run/`--check`/build without blocking (no-alias imports stay exempt per v1.4.11; disjoint aliases emit nothing; the "alias vs local fn" shape reports independently). §8.1's collision paragraph now covers repeated aliases explicitly. New `tests/r107_alias_clash.rs` ×4 (integration 21→**25**) and pkg_cases 108 (verify_selfcomp **364→365 = 196 + 8 + 161**); the stock 203-pair comparison is byte-identical with the one new case as the single intended diff (1 byte — the fn-index swap; old hex printed 4, new prints 103 matching the host); self_comp 12529→**12544 lines**, quine **269555 bytes**; host `src/` untouched. eval 121/121 per backend, selfhost six modes, doc_audit 71/71, fmt gates re-verified. **R107 closed — R94 through R109 all closed, the ledger is empty.** The frozen language surface and external publication freeze remain unchanged.
- **v1.4.11 (2026-09-30, repository version; no external release; tag at `13a4cf4` after six-job CI run `36714193624` #223 passed)**: twenty-second-review remediation closing **R108 (P2) and R109 (P3)** (RFC-0004 revision 44) — pure typecheck-diagnostics scope, two changes in `src/typechecker/mod.rs`. **R108**: the `ExprKind::Ident` check treated imported package enum variants as undefined (error-level NAM003) because `is_variant_constructor` only consulted locally-declared and builtin enums — a legal program using a package variant failed `--check` with rc=1 (the checked-in 103 case itself: run output all-correct but three stderr errors). The Ident branch now releases names in `external_symbols` to `Unknown` exactly like the function-call path's externals release (deliberately not inside `is_variant_constructor` itself: that function also feeds `check_call`/`check_pattern`, where externals-blindness would mint TYPE003 arity false-positives and change Binder semantics); misspelled or genuinely undefined names still fail NAM003 (negative-locked). The auto-public direct-use form stays consistent with the registered "package real names callable without import" design. **R109** (introduced by v1.4.10): the NAM006 post-pass flagged *same-origin* merged-unit collisions — a plain `from pkg import {fn}` (no alias) always collides with the package's own merged-in `fn` item, so every ordinary single-package import drew a misleading "local definition shadows pkg::fn" warning (101 case: three). The post-pass now fires only for true aliases (`alias != real name`): the three shadowing shapes split cleanly — genuine alias-vs-local-fn shadowing keeps its NAM006 (the r104 integration tests stay green), same-origin no-alias imports stay silent, and a main-file fn colliding with a package fn still surfaces through the duplicate-definition branch's externals-aware NAM006. New `tests/r108_variant_externals.rs` (5 integration cases: --check rc=0 clean on the 103 shape, negative misspelling still rejected, dependency-outside-graph still rejected, build zero-NAM006, alias-form NAM006 preserved); Rust 543 unit + 16→**21** integration; verify_selfcomp stays 364/364 (the negative lock lives in the CLI-level tests — the selfcomp corpus locks the L2 compiler, not host typecheck); self_comp/codegen untouched (quine 269521, 14/14); eval 121/121 per backend, selfhost six modes, doc_audit 71/71, fmt gates re-verified. **R108/R109 closed; R107 (the alias-vs-alias cross-backend divergence) remains open for adjudication.** The frozen language surface and external publication freeze remain unchanged.
- **v1.4.10 (2026-09-30, repository version; no external release; tag at `ce8c731` after six-job CI run `36694182024` #219 passed)**: twenty-first-review remediation closing **R104/R105** (designs/0014; RFC-0004 revision 43; user-adjudicated: local-definition-wins + two new warning codes, both within the §14 freeze's warning-safe zone). **R104 (P1 — interpreter cross-run nondeterminism)**: `load_packages` iterated `graph.packages` (a `HashMap`) so two packages exporting the same symbol resolved to *whichever registered last in this process's randomized iteration order* — the same program printed different values across runs of the same backend (probe-verified 11/22 mixed across 10 runs) while WASM/L2 deterministically take the later package in root-path order (probes with values swapped still pick the path-later package). The registration loop now sorts by package-root path (the same key `merge_packages_for_wasm`/`expand_package_unit` use), making the interpreter deterministic and three-side-identical. **R105 (P1 — opposite shadowing direction)**: the interpreter's `eval_call` resolved the callee name through the import-alias map *before* consulting the user-function table, so an `import {origin as dup}` colliding with a local `fn dup` silently ran the package function (bypassing the user's code) while WASM/L2 run the local one — the call path now checks the local name first (alias lookup demoted to fallback), aligning all three sides to local-definition-wins. **Two warning codes** (per adjudication): `NAM006` — an import alias (or a merged-unit package symbol) collides with a local definition; emitted by an order-insensitive post-pass over user-fn and imported-alias sets (fixing the old order-sensitive NAM002 behavior where import-then-fn reported a false "duplicate definition" error and fn-then-import reported nothing); genuine same-file duplicates keep `NAM002` (error), while externals-collisions downgrade from the old misleading non-blocking NAM002 to NAM006; and `PKG007` — two packages export the same symbol, naming both packages and the path-order winner (package-layer stderr, single wiring point covering run/`--check`/build). Both are non-blocking. Also registered (unchanged, now documented in §8.1): package `fn`s are auto-public (real names callable without import — a three-side-consistent registered design). New integration tests `tests/r104_dedup_order.rs` (6 cases: deterministic 22/11 across value/declaration-order variants with hard equality assertions — nondeterminism is regression-locked out, local-wins on both item orders, PKG007/NAM006 presence, NAM002 negative unflipped) and pkg_cases 106/107 (three-side behavior pairs); verify_selfcomp **362→364 = 196 single-file + 7 package projects + 161 negatives**; Rust 543 unit + 10→**16** integration; self_comp and codegen untouched (the quine stays 269521 bytes, 14/14; stock hex identical by construction); eval 121/121 per backend, selfhost six modes, doc_audit 67/67, fmt gates re-verified. The D5 generator's name-collision coverage (constructively avoided since designs/0008) is deferred to a tooling batch. **R104/R105 closed; open ledger is empty pending the deferred tooling items.** The frozen language surface and external publication freeze remain unchanged.
- **v1.4.9 (2026-09-30, repository version; no external release; tag at `e71749e` after six-job CI run `36677072901` #216 passed)**: user-approved R94 host-ledger batch 2 (designs/0013 §1.2/§1.3; RFC-0004 revision 42) — two host-side behavior fixes aligning implementations with already-frozen claims; the v1.0 language surface is untouched. **map_remove unified to Bool**: SPEC §9.7 has always declared `map_remove(m, k) -> Bool` ("True iff the key existed") and the interpreter/typechecker/self_interp already returned Bool; the host-WASM `build_map_remove` ended with `V_UNIT` (discarding the hit information — v1.2.13's adjudication A had aligned the then-experimental L2 to that WASM side) and the L2 compiler carried two matching "void" verdicts plus the R89 message naming map_remove. The host helper now tags the probe result like `build_map_has` (tombstone + size-- semantics unchanged; an e2e Rust test locks `true/false/true` on the println and bound forms), and the L2 side flips its verdicts to `i32`/Bool — a deliberate supersession of v1.2.13 adjudication A in favor of the frozen spec text. The three implementations now agree (`true/false/true`, rc=0, all modules valid). verify_selfcomp stays **362/362** but recomposes to **196 single-file pairs + 5 package projects + 161 negatives** (new positive `201_r94_map_remove_bool`; `neg_map_remove_unit_bind` deleted as its rejection surface vanishes; `neg_println_void` rewritten over `map_set` to keep locking the R89 message); the stock 195+5 comparison against the v1.4.8-exported old compiler shows **198 byte-identical and 2 intentional diffs** (cases 81/84, helper-signature byte growth only, both modules valid with identical behavior — no checked-in case prints map_remove's return). self_comp grows **12525→12529 lines**; the quine moves to **269521 bytes** (byte-identical on both sides, 14/14). Rust grows **542→543** unit + 8→**10** integration. **In-package `as` aliases fixed on the host**: the interpreter's `load_packages` used to skip package-source imports entirely (only the main file's `process_import` registered alias→real-name mappings), so `from libinner import {triple as t3}` inside a package died with RUNTIME002 while WASM/L2 ran fine (R90's registered three-voice divergence); it now registers both tables (also unblocking the latent `from io import {println as log}` stdlib-alias form, covered by a test), and `lom build <file> --target wasm` typechecks the merged unit **with package externals** (aligning the default-run/`--check` precedent), eliminating the 3 false NAM003 errors (the eighteenth review's "2 errors" registration was a miscount — corrected in the ledger) while codegen was already correct. Two integration tests lock the interpreter run (`18/18/7`) and the zero-NAM003 build; the 105 case now agrees on all three sides. **Registered unfixed boundary**: the manifest-driven `lom build` flow (per-file, no externals) still reports NAM003 for the alias inside the package source's single-file view — needs dependency-graph-aware externals, out of this batch's minimal scope. eval 121/121 per backend, selfhost six modes, doc_audit 67/67, fmt gates, and bootstrap 14/14 re-verified. **R94 is fully closed** (json key order in v1.4.8, map_remove + in-package `as` here); the frozen language surface and external publication freeze remain unchanged.
- **v1.4.8 (2026-09-30, repository version; no external release; tag at `8c86a60` after six-job CI run `36668306358` #214 passed)**: user-approved ledger batch 1 of the R94 host-ledger family (designs/0013 §2; RFC-0004 revision 41) — harness/diagnostic-corpus/documentation scope with **zero `src/` and zero `self_comp.lom` changes**. **R94 (json astral key order)**: the eval harness `run_wasm.mjs`'s only Map-key sort — the json_stringify mediation path in `readVal` — used JS string relational comparison (UTF-16 code-unit order), so a Map whose keys mix U+E000–U+FFFF with astral-plane characters serialized those keys in a different order than every other surface (interpreter Rust `str` Ord, the wasm-inlined `rt_str_cmp` behind `map_keys`/`map_values` and Map display, and the L2 stringify are all UTF-8 byte order; `map_keys`/`map_values` never went through the harness and were verified unforked). The comparator is now `Buffer.compare(Buffer.from(...,'utf8'), ...)`; the astral-key probe (U+E000 + U+1F600) agrees **byte-for-byte on both backends** after the fix (`ee 80 80` first, i.e. byte order; before it the host-WASM side put the surrogate pair first). Existing-corpus impact is zero — no checked-in case mixes those key ranges. The §9.7 note now states "sorted keys (byte order — UTF-8, matching `map_keys`)" explicitly. **Twentieth-review §6-①② negatives promoted**: `neg_user_fn_named_println` (a user fn named `println` → `用户函数不得命名 'println'`) and `neg_match_scalar_on_list` (a scalar literal pattern matched against a list value → `字面量模式类型不符（被测 ls{i64} 模式 i64）`) join the verifier, both COMPILE-ERROR with no hex; verify_selfcomp grows **360→362 = 195 single-file pairs + 5 package projects + 162 negative rejections** (self_comp untouched, so all existing pairs are unchanged by construction; `--bootstrap` stays 14/14 with the quine at 269510 bytes). §6-③'s trust-boundary line ("unannotated Bool / nested-container iteration display stays conservatively rejected") is added to the HANDOFF_PROMPT checklist. **R103 (planner-found documentation staleness, R93/R98 family, opened-and-fixed)**: designs/0012's header still said "E甲 pending / R100-R101 open" though its §12/§13 record their delivery; HANDOVER §9-3 carried v1.4.5-era baseline numbers; HANDOFF_PROMPT's anchor section still called R101 an open host-WASM divergence though v1.4.7 closed it — all refreshed. Rust 542+8, eval 121/121 per backend, selfhost six modes, doc_audit 67/67, fmt gates (examples 37 + selfcomp corpus incl. the two new negatives), and bootstrap are re-verified green. **R94's json key-order face is closed**; the family's remaining items (map_remove Bool/Unit, in-package `as` alias) are batch 2. The frozen v1.0 language surface and external publication freeze remain unchanged.
- **v1.4.7 (2026-09-30, repository version; no external release; tag at `22a37a6` after six-job CI run `36612026578` #210 passed)**: user-approved batch 2 closing **R101** (designs/0012 §13; RFC-0004 revision 39) — a host-side fix in `src/wasm_codegen.rs`: the `ExprKind::Logical` short-circuit emitted its `if` without pushing `Label::If`, so a `return` actually executed on the `and`/`or` right side computed its branch depth one level short and the host-WASM backend trapped with `unreachable` after the first output (the interpreter and the L2 WASM were both correct). The fix pushes/pops `Label::If` around the short-circuit `if` exactly as `compile_if` has done since the W-2 precedent. The R101 four-branch probe now agrees on **all three implementations** (interpreter, host WASM, L2: `0/9/1/8`, rc=0 each — the host WASM previously printed only `0` then trapped), and a durable Rust e2e test (`e2e_return_inside_logical_rhs`, W-2 style) locks the fix. Rust grows **541→542** unit tests; the L2 side is untouched (its short-circuit was already correct and only the right-side-skipped paths were locked by B甲 tests). eval 121/121 per backend, verify_selfcomp 360/360, and the six selfhost modes are unchanged by this host fix (re-verified in the full gate). **R101 is closed**; with E甲/R100 closed in v1.4.6 this completes all three user-adjudicated items (1/2/3) except the twentieth review, which follows next; R94 stays a host ledger item. The frozen v1.0 language surface (this is a bug fix in codegen, not a semantic change) and external publication freeze remain unchanged.
- **v1.4.6 (2026-09-30, repository version; no external release; tag at `96c26b6` after six-job CI run `36608219947` #207 passed)**: user-approved batch closing the E甲 expansion and R100 (designs/0012 §11→§12; RFC-0004 revision 38) — the Bool display prescan gains the three structural blind spots' fixes and the empty-List read builtins no longer compile to modules with memory instructions but no memory section. All five §11 adjacent gaps now compile with host-identical output: match-arm Bool binders (direct `Some(True)` construction, parameter-annotation flow via a new **bbind** knowledge table symmetric to D甲's cbind, and helper-return flow through `ret_bool_fns`), annotated `List<Bool>` iteration, and `for`-iteration variables in both families (container side adds an `@ec:`-prefixed element-container knowledge so only "the element is a container" seeds display knowledge — an Int element in `for cell in row` no longer mis-seeds). A first-draft over-recursion was caught by the stock hex diff itself (two cases: a generic Either<Int,Bool> parameter and an Int for-element mis-seeding) and tightened to strict seeding forms before delivery; the knowledge/release layering keeps the R85 pb5 boundary (a Bool parameter directly `println`'d without a binder stays rejected — `neg_bool_param_flow` does not flip). R100: the heap prescan's open-heap whitelist adds `list_head/tail/length/get/fold` (they emit loads), while `is_empty`/`empty` stay non-triggering — the previously uninstantiable 138-byte module now compiles to a valid 217-byte module printing `0` identically on both sides; empty-head/tail/get runtime traps are avoided in tests via guarded forms (host behavior preserved). Positives 194–200 (seven) plus one negative (unannotated-helper Bool binder stays rejected); sixteen pre-existing negatives re-verified unflipped one by one. Local verifier: **360/360 = 195 single-file behavior pairs + 5 package projects + 160 negative rejections** (prior 352 = 188 + 5 + 159); **all 193 v1.4.5 behavior pairs produce byte-identical hex** (188 single-file + 5 packages, IDENTICAL=193/DIFF=0/MISSING=0 in the same host); `self_comp.lom` grows **12199→12525 lines** (+326); `--bootstrap` stays 14/14 with the quine at **269510 bytes** byte-identical on both sides. Rust 541 unit + 8 integration tests and eval 121/121 per host backend are unchanged, as is host `src/`. **R100 is closed and the E甲 evidence gaps are remediated**; R101 (host WASM logical short-circuit label depth) is next in the user-approved order; R94 stays a host ledger item. The frozen v1.0 language surface and external publication freeze remain unchanged.
- **v1.4.5 (2026-09-29, repository version; no external release; tag at `df7f83b` after six-job CI run `36594643927` #204 passed)**: user-approved R97 D甲 (designs/0012 §3.4 route 甲; RFC-0004 revision 37) — the display prescan now tracks container payloads bound by match-arm binders, so no-String-literal programs like `let v = Some(list_cons(1, list_empty())); match v { Some(xs) => println(xs) }` compile with correct host/L2 output instead of hitting the ibase fallback rejection. The fix keeps the prescan strictly AST-only (no fns/env — the has_str→ibase→collect_sigs order is untouched): three AST-collected tables (`enum_defs` user-enum container payloads, `fn_ret_cont` annotated container-returning fns, per-function `cbind` container-binding knowledge seeded from parameter annotations), an arm-local copy of the binding tables per arm (`copy_bool_map` + an unspellable `@arm_local` sentinel — in-arm `let` shadowing overwrites True with False precisely; the outer shared table keeps its append-only True behavior), and a planner-refined payload semantics: an unannotated `let` feeding a variant constructor marks the binding container only when a constructor **argument** is a container (`load_cont_expr`) — the display-semantics "any enum construction counts" stays on the release side, which keeps the one initially-diverged stock case (22_enum_closure, an Int-payload enum matched with `v + n`) byte-identical. All three closure criteria pass as positives (direct chain, parameter-annotation flow, helper-return flow) plus nested match, guards, sibling-arm isolation, in-arm scalar shadowing, closures, user enums, and mixed-payload Result (coarse over-approximation is deliberate and behavior-correct). The cross-function **direct** container `println` (no match binder) stays rejected — the existing deep-flow negatives do not flip; two new negatives lock the no-return-annotation helper boundary (rejected earlier as a void-typed call) and the param-direct boundary. Local verifier: **352/352 = 188 single-file behavior pairs + 5 package projects + 159 negative rejections** (prior 340 = 178 + 5 + 153, plus 10 positives and 2 negatives); **all 183 v1.4.4 behavior pairs produce byte-identical hex** (IDENTICAL=183/DIFF=0/MISSING=0 in the same host; the single stock-case divergence found during implementation was root-caused and closed by the `load_cont_expr` refinement before delivery); `self_comp.lom` grows **11938→12199 lines** (+261); `--bootstrap` stays 14/14 with the quine at **266073 bytes** byte-identical on both sides. Rust 541 unit + 8 integration tests and eval 121/121 per host backend are unchanged, as is host `src/`. **R97 is closed**; E甲 (neighboring Bool-binder/`for` evidence) remains per the adjudicated order; R94 stays a host ledger item; R100/R101 remain open awaiting separate adjudication. The frozen v1.0 language surface and external publication freeze remain unchanged.
- **v1.4.4 (2026-09-29, repository version; no external release; tag at `1851964` after six-job CI run `36578254262` #202 passed)**: user-approved R96 C甲 narrow fix (designs/0012 §3.3; RFC-0004 revision 36) — closures may now call the prelude names `println`/`print` without an `io` import, matching the host. The closure free-variable finalization keeps parent-environment lookup first (a same-named parent binding is still genuinely captured; direct calls keep the existing prelude dispatch priority), and only then exempts the two prelude names from the "closure captured an undefined variable" rejection — no other builtin is auto-exempted. A `@closure:prelude` sentinel key (contains `@`, unspellable as a source identifier) marks closure-body environments so return-type synthesis treats a direct `print` as void, and the display prescan threads an `in_closure` flag so a direct in-closure `print` reliably opens the String facilities; a top-level `print` without import is still rejected. Eight positives (cases 176–183: plain/nested/recursive closures, same-name shadowing by value capture and by direct call, explicit `io`-import control, String-literal control) and four new negatives lock the boundary: naked `println`/`print` as a value inside a closure still rejects (`未定义变量 'println'/'print'`), a zero-argument in-closure `print` still meets the one-argument check, and an unimported builtin (`len`) inside a closure keeps the capture rejection — two further candidates were deduped against the existing `neg_closure_undef_capture`. The local verifier passes **340/340 = 178 single-file behavior pairs + 5 package projects + 157 negative rejections** (prior 328 = 170 + 5 + 153, plus 8 positives and 4 negatives); **all 175 v1.4.3 behavior pairs produce byte-identical hex** in the same host (IDENTICAL=175, DIFF=0, MISSING=0; the exemption only touches programs the old compiler rejected — spot checks confirm 176/177 fail on the v1.4.3 compiler); `self_comp.lom` grows **11926→11938 lines** (+12); `--bootstrap` stays 14/14 with the quine at **262552 bytes** byte-identical on both sides (+415 from the source growth). Rust 541 unit + 8 integration tests and eval 121/121 per host backend are unchanged, as is host `src/`. **R96 is closed**; R97/D甲 and the E甲 neighboring-evidence pass remain next per the user's adjudicated order; R94 stays a host ledger item; R100/R101 remain open awaiting separate adjudication. The frozen v1.0 language surface and external publication freeze remain unchanged.
- **v1.4.3 (2026-09-29, repository version; no external release; tag at `a5dac74` after six-job CI run `36563915297` passed)**: user-approved R95 B甲 full-bottom stage (designs/0012; RFC-0004 revision 35). The L2 compiler now separates internal `never` (no normal value) from `?` (unknown generic element), merges `never` with reachable value types, and emits valid WASM for all-return value-position `if`/`match` forms, including the formerly rejected d1/d3 arithmetic cases and the formerly unsafe d2/direct-tail forms. Strict operands stop emission after the first terminal expression while preserving earlier side effects; `and`/`or` keep their short-circuit paths, with no unconditional promotion of a terminal right side. Match guards, terminal conditions/iterables, closure return hints, and explicit `unreachable` after terminal structured blocks preserve the validator's stack effect. An incidental existing gap was fixed: an unannotated closure's return inference now binds its parameters in a copy of the parent environment without leaking bindings. A durable negative now locks that an inferred closure parameter named `secret` cannot leak into the enclosing scope (`未定义变量 'secret'`). The local verifier passes **328/328 = 170 single-file behavior pairs + 5 package projects + 153 negative rejections**: the prior 298 = 137 + 5 + 156, plus 33 new behavior pairs (143–175), nine R99 negatives flipped to positives, and six new strict-rejection negatives. `self_comp.lom` grows **11469→11926 lines** (+457); `--bootstrap` remains 14/14 with a new **262137-byte** strong quine, byte-identical on both sides. In the same host, **140/142** v1.4.2 behavior-pair hex outputs remain identical; two intentional differences are case 108 (**299→300 bytes**, terminal `unreachable`) and case 142 (**118→117 bytes**, dead-tail removal), with both old and new modules validating and behaving identically. Performance evidence is narrow: an N120 single run was 3.030s old vs 3.008s new, while the first self-compilation stage was 315.5s A甲 vs 328.5s B甲; no general speedup is claimed. Rust remains 541 unit + 8 integration tests and eval remains 121/121 per host backend. **R95's original d1/d3 shape is closed after planner full regression and green CI**; R99 was completed and tagged in v1.4.2. R96/C甲 is not part of this tag and remains open (local WIP began afterward); R97/D甲 and E甲 remain unimplemented; R100 remains open. Newly registered R101 (P2, awaiting separate user adjudication) is a host-WASM divergence when the right side of `and`/`or` executes `return`: the host interpreter and L2 WASM complete, while host WASM traps after its first output because the logical `if` is absent from return-branch label depth. B甲 does not modify the host or imitate that trap; tests lock the path where the right side is skipped. The frozen v1.0 language surface and external publication freeze remain unchanged.
- **v1.4.2 (2026-09-29, repository version; no external release; tag at 8012c38 after six-job CI run 36549057031 passed)**: user-approved R99 A甲 safety stage (designs/0012; RFC-0004 revision 34). The L2 compiler now detects value-position `if`/`match` expressions whose normal paths all terminate, using an AST control-flow predicate distinct from the existing `?` unknown-generic placeholder, and rejects them before emitting hex with `全终止值表达式无正常产值（R99 安全拒绝）`. This closes the known uninstantiable-WASM route: the nineteenth review's d2 was correctly reported as `COMPILED 121 bytes` but its claimed L2 output was wrong; fresh Node instantiation fails at `local.set` with an empty stack. Direct-tail forms similarly lack a fallthrough result. The original review and B+ rating remain historical, with a visible post-review correction. Local verification is **298/298 = 137 single-file behavior pairs + 5 package projects + 156 negative rejections** (from 286 = 134 + 5 + 147: three new positive controls and nine new negative locks); `--bootstrap` is 14/14, with the new self-apply quine **249786 bytes byte-identical** on both sides; self_comp grows **11321→11469 lines** (+148). Rust 541 unit + 8 integration tests and eval 121/121 on each host backend remain at their prior counts. **R95 remains open**: host-legal all-return expressions used in arithmetic still meet an L2 compile-time rejection, to be addressed in user-approved B甲. R96/C甲, R97/D甲 and the neighboring E甲 investigation are not implemented in this stage. A separate R100 P2 remains open: no-String/no-heap `list_empty` plus list read builtins can compile to a module with memory instructions but no memory section (Node instantiation fails); no R100 code change is included. The frozen v1.0 language surface is unchanged; external publication remains frozen.
- **v1.4.1 (2026-09-28, repository version; no external release)**: the container-display batch (designs/0011, user-approved all-recommended adjudications; RFC-0004 revision 32) — the L2 compiler's last long-deferred display surface closes out in one batch. `println`/`print` and concat-promotion now display all six container kinds with byte-identical host/L2 output: Lists as `[1, a, true]` (Nil=0 sentinel walk), Maps as `{a: 1, 中: 2}` in byte-order key sequence (mp_keys reuse), records as `{a: 1, b: x}` in declaration order with interned field names, tuples as `(1, a)` **with the single-element `(a,)` comma form**, enums as `Name` / `Node(Leaf, 1, Leaf)` (per-instance variant-name select chains over interned names with payloads recursing through display; recursive enums and enum↔list mutual recursion self-reference through a placeholder-patched disp section — an implementation refinement over the design's inline-fidx sketch), and closures as `<闭包>` (static string, flipping the previously registered divergence). Concat promotion accepts containers on either side (`x=[1, 2]`, `[1, 2]=x`); json values stay rejected (registered `_Any` trust boundary); bare `ls{?}`/`mp{?}` display is a friendly compile-time rejection. A `scan_has_display` prescan plus an ibase fallback guards deep container flows (the Bool-batch dual-line precedent; cross-function flows stay rejected — negative-locked), and a mid-implementation design gap (the global enum defensive arm needed an Add-with-String-side exemption for concat) was found and closed in delivery. Nine deferred negatives flip to positives (deleted); four new deep-flow negatives lock the fallback message. `verify_selfcomp` passes **286/286 = 134 single-file cases + 5 package projects + 147 negative rejections**; the **127 pre-existing pairs produce byte-identical hex** (disp helpers emit only on demand; zero-append passthrough when unused); self_comp grows 10548→**11321 lines**; `--bootstrap` stays 14/14 (the self-apply quine is now 247009 bytes, byte-identical on both sides); host `src/` untouched (Rust 541+8). The frozen v1.0 language surface is unchanged.
- **v1.4.0 (2026-09-27, repository version; no external release)**: **RFC-0004 is complete — the L2.4 bootstrap closure lands with a strong quine.** The self-hosted compiler compiles its own 10548-line source to a 234734-byte wasm module; that wasm (run under `node --stack-size=60000`, where the V8 JIT makes it far faster than budgeted) compiles a 12-case representative subset (one per L2.3 batch plus a package project with third-argument pass-through) with every output hex **byte-identical** to the host-produced ones; and in the self-application layer the wasm compiler recompiles the compiler's own source in 1.1s, producing a module **byte-identical to the host build (same sha256) — a strong quine**. Getting there meant clearing nine self-compilation gates: eight equivalent rewrites of the compiler's own source (a hand-written digit parser replacing `string_to_int`'s indistinguishable Int|Unit union at 12 call sites with i64-overflow parity; Result<Unit,_> signatures re-carried over String; Unit-binding helpers re-typed to Int after a tail-position pitfall; `Option<?>` placeholders pinned by annotations; a signature-reuse ambiguity split; two cross-block map-type annotations) and one genuine compiler gap closed — **bottom-typed return-terminated arms in value-position if/match** (a block whose last statement is `return` now synthesizes a `?` bottom placeholder that merges with any type, flipping the R91-registered boundary for Form-B/if block shapes into dual-side agreement; a return inside a Form-A single-line match arm is rejected identically by both sides at the syntax layer (PARSE001, matching messages), single-return-arm shapes agree on both sides per the eighteenth-review R92 correction; the nineteenth review opened R95 for the both-arms-return shape where the host warns-and-runs and L2 rejects — adjudication pending). `--bootstrap` passes 14/14 with a `--ci-smoke` tier wired into the CI selfhost job. The **127 pre-existing pairs remain byte-identical** (compiler-logic equivalence across the rewrite); Rust tests unchanged **541 + 8**; the frozen v1.0 language surface and host `src/` are untouched.
- **v1.3.2 (2026-09-27, repository version; no external release)**: the L2 compiler gains record/tuple compilation plus the file/env builtins (designs/0010 stage one, RFC-0004 revision 28), and closes the seventeenth review's R90/R91. **Value representation** — `vt rc{name:vt;name:vt}` (field names enter the vt: records are structural types) and `tp{vt;vt}`; heap layout `[n:i32][8B slots]` (no padding, host-identical), slot widths per field vt (the third application of the cons-slot rule); field access is a **compile-time offset** (untagged static typing — name→position→width load; no runtime name comparison). Tuple indexing arrives at the L2 front end as `ExField` with numeric-string names; field order is annotation-order-first with literal-order fallback, and set/name/order mismatches are compile-time rejections via vt merge. **file/env builtins** run through five harness imports under a `has_fileenv` prescan (json-batch precedent; the st layout is byte-identical on both sides; `args` materializes directly in the List-cons shape; harness argv pass-through); math's four names compile inline (per the src/wasm.rs constant table); io's println import is a no-op pass-through. Seventeenth-review **R90**: `pkg-expand` now keeps package-source imports containing `as` aliases in place instead of stripping them — the L2 pipeline matches the host WASM on the alias-chain form (the host interpreter's own RUNTIME002 on that shape remains a registered host-side dual-backend divergence); +2 Rust tests. **R91**: the value-context return-arm boundary is now registered for both `if` and `match`. Seven rejection forms are negative-locked (field-set mismatch, unknown field, tuple index out of bounds, container-display/arithmetic/comparison family, arity, destructuring stays rejected, same-name-different-order). `verify_selfcomp` passes **279/279 = 127 behavior pairs (122 single-file + 5 package projects) + 152 negative rejections**; the **115 pre-existing pairs produce byte-identical hex** (the R90 rewrite only affects package sources containing `as` — none in the corpus); self_comp grows 9491→10402 lines; Rust tests grow 539→**541** + 8; host changes are cli.rs +125 lines (R90 fix + tests). The frozen v1.0 language surface is unchanged; eval 121/121 per host backend.
- **v1.3.1 (2026-09-27, repository version; no external release)**: the L2.3 statement-surface finale — the return batch (designs/0009, user-approved wasm `return`-instruction emission + full B1+B2+B3 scope + strict `?` checking; RFC-0004 revision 26). **Emission** uses the wasm `return` instruction (0x0f) rather than the host's br-to-$ret-block: zero depth tracking, no function-body wrapping (programs without return/`?` compile byte-identical), and the host's three historical label-depth bug families (7.2/7.5/W-2) are structurally impossible. `return` compiles as value + `0f` (void form bare `0f`), checked against the current function's return vt threaded via a compiler-state key that is saved/restored around closure bodies (a closure's return belongs to the closure itself). `?` compiles as variant-index test (Ok|Some via i32.or), payload read width-specialized per vt (f64 reinterpret/i32 extend as needed), else branch re-emitting the original enum value + `0f`; inference yields the Ok/Some payload vt; context checking is stricter than the host's warning-level family check — the function's return type must be family- and Err-payload-compatible (compile-time rejection, no bad wasm). Node's validator accepts all 0x0f shapes including the i64-then-return else arm under `if (result f64/i32)` (unreachable-exempt arm typing, locked by a regression case). 11 new behavior pairs (105-115: first_even, return inside for/while/match-Form-B/closures/recursion, try basic/chain/in-for/in-tail-if with f64 payload, Err payloads over String/List) + 8 negatives; 2 deferred negatives flipped to positives. `verify_selfcomp` passes **255/255 = 115 behavior pairs + 140 negative rejections**; the **104 pre-existing pairs produce byte-identical hex**; self_comp grows 9361→9491 lines. Registered boundary: a `return` arm inside a value-position `if` remains a pre-existing host-L2 divergence (host permissive, L2 compile-time rejection). The frozen v1.0 language surface and host `src/` are unchanged; Rust tests remain **539 unit + 8 integration**, eval 121/121 per host backend.
- **v1.3.0 (2026-09-27, repository version; no external release)**: the L2.3 package batch (designs/0008, user-approved hybrid route C+D + strict-enum-rejection + argv package list; RFC-0004 revision 24). **First host-side tooling change of the L2.3 line**: a new `lom pkg-expand <main.lom>` CLI subcommand (pure tooling — the frozen language surface is untouched) reuses the host's dependency resolution and file collection as the single source of truth and emits a source-level expanded compilation unit (packages ordered exactly as the host merge — root-path sort plus per-package filename sort; top-level imports inside dependency packages are stripped, mirroring the interpreter's "package-internal imports are not propagated"); `--list` prints the package-name comma string; projects without a manifest expand idempotently. On the L2 side, an optional third argv parameter carries the package-name list: imports dispatch in the host's order — builtin modules first, then the known-package list (each imported name is checked against the merged fn/enum/variant tables in a deferred post-pass, with `as` aliases copying the real-name summary so R74 checks apply unchanged) — and any other module name is now a compile-time "unknown module" error (closing the silent-fallthrough gap; imports of math/file/env/io likewise now fail at the import line instead of at the call site — verified zero impact on the existing corpus). Enum/variant name clashes across packages stay explicit rejections (registered trust boundary: the host silently keeps the first definition). 4 multi-file package projects (single package, a three-package chain C→B→A with cross-package imports, alias + in-package enum + match, enum payloads + two packages) and 4 negatives join the verifier. `verify_selfcomp` passes **238/238 = 104 behavior pairs + 134 negative rejections**; the **100 pre-existing pairs produce byte-identical hex** (argv backward compatibility); self_comp grows 9295→9361 lines; Rust tests grow 535→**539** (+4 pkg-expand tests) + 8 integration; eval 121/121 per host backend. The frozen v1.0 language surface, the 43 builtins, and diagnostic codes are unchanged.
- **v1.2.15 (2026-09-26, repository version; no external release)**: The experimental L2 compiler adds the L2.3 json batch (designs/0007, user-approved hybrid-mediation route + B1+B2 consumption tiers + JS-value number splitting; RFC-0004 revision 22). **Value representation** — `vt js` is an opaque JSON-node i64 pointer (`_Any` annotations normalize to js); node layout `[kind:i32][payload 8B]` with kinds 0-6 (null/bool/int/float/str/array/object); arrays materialize as `ls{js}` cons chains (cons-slot 8B rules reused) and objects as **insertion-ordered kv sequences** (deliberately not Map, whose key reordering would diverge from the host Record's insertion order). **Route A (hybrid mediation)** — parse/stringify run through two new harness imports (`lom_json_parse` JS-parses and materializes the node tree via the product's newly exported alloc; `lom_json_stringify` serializes the node tree with escStr/key-order/`String(v)` logic ported from the host run_wasm.mjs stringifyVal — number splitting by JS value judgment aligns with the L2 comparison baseline, the host WASM backend); modules gain the two imports plus the alloc export only under a `has_json` prescan — **the 90 pre-existing pairs produce byte-identical hex**. **B1 consumption** — `data.field` compiles through a js_field helper (object keys st_eq-matched; miss or non-object traps, aligned with the host's missing-field trap); a js value in list-argument position is consumed as `ls{js}` (vt normalization plus a kind!=5 guard helper); `println(js)` dispatches by kind for the four scalar channels plus null→`()` while array/object kinds trap after prior stdout is flushed. **B2 stringify wide parameters** — scalars, `List`, and `Map` arguments convert to js nodes in-product (js_of helper family; list/map conversion specialized per element type with the "|" key rule; map key order = byte order = the host's stringify-Map semantics); enum/closure values are explicit compile-time rejections. `from json import` with unknown names now errors instead of being silently ignored. 14 rejection forms are negative-locked (including as-alias, unary neg, js comparisons, and Unit stringify). `verify_selfcomp` passes **230/230 = 100 behavior pairs + 130 negative rejections** (10 new pairs 91-100; inputs avoid the registered dual-backend divergence surfaces); self_comp grows 8633→9295 lines. The frozen v1.0 language surface and host `src/` are unchanged; Rust tests remain **535 unit + 8 integration**, eval 121/121 per host backend.
- **v1.2.14 (2026-09-26, repository version; no external release)**: sixteenth-review remediation release (4×P3, RFC-0004 revision 20; frozen language surface and host behavior unchanged — experimental L2 compiler diagnostics/documentation precision only). **R86-R88 (registry closures)** — the println(Bool) residual rejection surface is now registered as "cross-function Bool parameter flow **and same-function HOF-produced Bool relays**" (list_fold Bool-accumulator bindings and list_map Bool closures consumed via list_get hit the same ibase guard); `mp{?}` refinement semantics are registered generically — **map_set inside a branch or loop never writes its refinement back to the outer binding** (if/while alike; the refinement happens on the block-environment copy, and writing back would reopen R79); list_fold's **result vt derives from the init argument** (`list_empty()` → `ls{?}`) rather than the closure's accumulator annotation — element-level access on the result needs an explicit annotation, a documented asymmetry vs list_map/list_filter. **R89 (diagnostic fix)** — `println` whose argument evaluates to void now gets its own message (`println 实参求值为 void/Unit（map_remove/map_set 等内建返回 void，void 返回函数同理）——void 值不可打印；去掉 println 包裹或改用语句形式调用`) instead of the container-display fallback, whose direction was misleading for repair LLMs; the container-display message itself is unchanged for List/Map/enum/closure arguments. `verify_selfcomp` passes **206/206 = 90 behavior pairs + 116 negative rejections** (new negative `neg_println_void` locking the message); the **90 pairs produce byte-identical hex** before/after the change (implementer full comparison 90/90 identical; planner re-verified). self_comp grows 8627→8633 lines. Rust tests remain **535 unit + 8 integration**, eval 121/121 per host backend. The sixteenth-review A- grade applies only to its baseline; post-remediation status awaits the next independent review.
- **v1.2.13 (2026-09-26, repository version; no external release)**: The experimental L2 compiler adds the L2.3 Map batch (designs/0006, user-approved B1+B2 in one batch, map_remove adjudication option A, compile-time strictness option A; RFC-0004 revision 19). **Value representation** — `vt mp{V}` is single-parameter (keys are always String and do not enter the vt, matching the host typechecker's `Generic("Map",[_Any])`); `map_empty()` yields `mp{?}` until a `map_set` or annotation context resolves the value type (the c2 `?` mechanism reused); the object layout ports the host's (`[buckets:i32][cap:i32][size:i32]` head, 16-byte `[state][key_off:i32][val:8B]` buckets — state 0 empty/1 used/2 tombstone), with val slots loaded/stored per value vt (f64 direct, i32 extend/wrap, everything else i64 — same shape as cons slots). **B1** — all eight map builtins: map_get returns `en:Option{V}` constructing Some(idx=2)/None(idx=3) heap objects aligned value-for-value with the L2 variant table (consumed by existing c2 match patterns); map_keys/values return key-sorted lists (byte order per Rust `str` Ord, reusing st_cmp; multi-byte UTF-8 keys covered); map_remove returns Unit (aligned with the host WASM backend per adjudication A — the host interpreter/typechecker returning Bool is a documented host-side dual-backend divergence awaiting separate adjudication); map_size inlines; rehash doubles capacity at load >0.5 moving only live buckets; reference semantics (let-alias writes are shared) verified by a paired case. **B2** — structural `==`/`!=` via specialized recursive `mp_eq<V>` (i64/i32/f64/st elements and nested ls/mp; enum/closure value elements explicitly rejected). **Incidental fix**: `from map import` used to be silently ignored (misleading "unknown function" error) — unknown builtin names now report `未知内建 'map.<name>'`. Compile-time strictness: non-Map receivers, non-String keys, value-type-incompatible set/assignment/annotation, and bare `mp{?}` reads are explicit rejections without hex. `verify_selfcomp` passes **205/205 = 90 behavior pairs + 115 negative rejections** (11 new pairs 80-90 incl. reference semantics/tombstone reuse/rehash; 13 new negatives with locked reasons); the **79 pre-existing pairs produce byte-identical hex** (full before/after compiler comparison by the implementer; planner independently re-sampled 5). The frozen v1.0 language surface and host interpreter/WASM behavior are unchanged; Rust tests remain **535 unit + 8 integration**, eval 121/121 per host backend.
- **v1.2.12 (2026-09-26, repository version; no external release)**: fifteenth-review remediation release (user-approved R84+R85 in one package, R85 option A; frozen language surface and host behavior unchanged — experimental L2 compiler quality only; RFC-0004 revision 17). **R84 (P2)** — `list_fold` helpers are now specialized per accumulator vt: the accumulator parameter and return slot follow `acc_vt` (f64 via f64 slots, i32 via extend/wrap, i64-pointer carriers direct) instead of a hardcoded `(i64,i64,i64)->i64` signature; Float/Bool accumulators previously compiled to uninstantiable modules (`call[1] expected i64, found f64/i32` at instantiation) while the host evaluated correctly; accumulator types outside the carrier whitelist (e.g. Unit) are explicit compile-time rejections without hex. **R85 (P3, option A)** — the `println(Bool)` prescan (`scan_has_bool_display`) now recognizes value-position `if` (per-branch tail recursion), `match`-produced Bools (arm tails), and local-closure calls returning Bool (a `cl_bools` table built from let-bound closure signatures; direct closure-literal calls likewise); these host-legal forms were previously compile-rejected by the defensive ibase guard. Cross-function Bool parameter flow remains an explicit L2 subset boundary (negative-locked, message unchanged). `verify_selfcomp` passes **181/181 = 79 behavior pairs + 102 negative rejections** (5 new pairs as cases 75-79 incl. both review probe families; 2 new negatives); the **74 pre-existing pairs produce byte-identical hex** (full before/after compiler comparison by the implementer; planner independently re-sampled 5 cases via a `git show` export of the previous compiler). Rust tests remain **535 unit + 8 integration**, eval 121/121 per host backend. The fifteenth-review B+ rating remains specific to its baseline; post-remediation status awaits the next independent review.
- **v1.2.11 (2026-09-24, repository version; no external release)**: The experimental L2 compiler adds the L2.3 List batch (designs/0005, user-approved B1+B2+B3+B4 in one batch plus the compile-time-strictness package; RFC-0004 revision 16). **Value representation** — `vt ls{T}` (nested braces, same family as `en:Name{args}`); `list_empty()` yields `ls{?}` until cons/annotation context resolves the element type (the c2 `?` mechanism reused); **Nil = 0** (the address-0 sentinel reserved by the closure batch's heap pointer); cons cells are `[head: 8B][tail: 8B]` with element access by vt (f64 via f64.load/store slot writes, i32 via extend/wrap, everything else i64-direct — same shape as the closure env slots). **B1 value channel** — the seven non-HOF list builtins (empty/is_empty/head/tail inline like the host; length/get via helpers, **get specialized per element vt**), the range expression `a..b` (half-open), **`split` unlocked** (returns `ls{st}`), for-in-List (loop variable typed as the element), and generic payloads (`List<T>` signatures, `en:Box{ls{i64}}`, `Result<Int, List<String>>`). **B2 HOF** — map/filter/fold as helpers specialized per closure signature and element vt with dedup ("|" keys); named-function values flow through the existing shims. **B3 structural equality** — `==`/`!=` on lists via specialized recursive eq (i64/i32/f64/st elements and nested lists; enum/closure elements explicitly rejected). **B4 for-in-String** — char_at materialization + UTF-8 stepping. **Compile-time strictness package** — filter predicates must return Bool, map/fold signatures checked, element-type-incompatible assignment/annotation rejected, range bounds must be Int, bare `ls{?}` rejected where a width is needed; `println(List)` and concat-promotion of lists rejected (container-display batch); list ordering/cross-type comparisons rejected; **pipe syntax is registered as never-implemented in L2** (a new negative locks the boundary). **Incidental host-side gap fixed (found during implementation)**: `println(Bool)` in programs with no String literal previously emitted a call to an unimported `print_str` (uninstantiable wasm; reproducible with `fn main() { println(1 == 2) }`) — fixed by a three-layer `scan_has_bool_display` prescan plus an ibase guard in `comp_println`; zero impact on existing case hex. `verify_selfcomp` passes **174/174 = 74 behavior pairs + 100 negative rejections** (2 negatives promoted — split/for-String; 12 new pairs; 16 new negatives with locked reasons); the **62 pre-existing cases produce byte-identical hex** (verified against the previous compiler via stash, re-verified after fmt normalization). The frozen v1.0 language surface and host interpreter/WASM behavior are unchanged; Rust tests remain **535 unit + 8 integration**, eval 121/121 per host backend. The fourteenth-review B rating remains specific to its v1.2.6 baseline.
- **v1.2.10 (2026-09-24, repository version; no external release)**: The experimental L2 compiler adds the L2.3 String batch (designs/0004, user-approved B1+B2+B3; RFC-0004 revision 14). **B1 value channel** — string literals compile to interned data-segment objects (`vt "st"` = i64 raw pointer; static and heap strings share the `[len: i32][utf8 bytes]` layout), String flows through let/parameter/return/assignment/if/match/closure-capture and generic payloads (`Result<Int, String>`, `Option<String>`, `en:Name{st}`), `println(String)`, the six String comparisons (byte order per Rust `str` Ord), Str/Str concatenation, and match on String literal patterns (byte equality). **B2 concat promotion** (v0.4.1 host semantics) — `"n = " + 42` promotes the non-String side through `to_display`: Int/Float via helpers (Float through a ported `env.ftoa` import, same JS `String(v)` source as the host harness), Bool/Unit via compile-time static strings; `println(Bool)` is unlocked in passing. **B3 builtins** — ten string builtins are ported tag-free from the host helpers (len/int_to_string/starts_with/ends_with/contains/trim/upper/lower/replace/char_from_code) with compile-time arity/type checks; imports (`from string import {...}`) register builtin summaries. Module assembly gains a data(11) section (after code, per section-id order), on-demand imports (`print_str`/`ftoa`, import base 2→4 with all func/type indices parameterized), a second global (64-byte itoa/ftoa scratch at the data tail), and memory export. `string_to_int` (its Int|Unit union is runtime-indistinguishable untagged) and `split` (returns List) are explicit compile-time rejections; `for`-over-String is deferred. `verify_selfcomp` passes **148/148 = 62 behavior pairs + 86 negative rejections** (4 negatives promoted to positives, 1 re-locked to a type-mismatch reason, 14 new negatives); the **52 pre-String cases produce byte-identical hex** against the previous compiler. The frozen v1.0 language surface (syntax/keywords/diagnostic codes/43 builtins) and host interpreter/WASM behavior are unchanged; Rust tests remain **535 unit + 8 integration**, eval 121/121 per host backend. The fourteenth-review B rating remains specific to its v1.2.6 baseline.
- **v1.2.9 (2026-09-23, repository version; no external release)**: The experimental L2 compiler adds L2.3-c2: builtin `Result`/`Option` variants and user generic enum instances can be constructed, passed through typed parameters/returns, captured by closures, and matched with instantiated payload types. The compiler tracks nested type arguments and rejects inconsistent instantiations before emitting hex. `verify_selfcomp` passes **128/128 = 52 paired WASM behavior cases + 76 negative rejections** (including recursive/nested generics, Bool/Float payloads, zero-argument `None` at a memory-page edge, and explicit rejection reasons). String/List/Map payloads, Unit/Fn type arguments, unresolved payload patterns, enum display/equality, and `?`/return remain L2 subset boundaries. The frozen v1.0 language surface and host interpreter/WASM behavior are unchanged; Rust tests remain **535 unit + 8 integration**, eval 121/121 per host backend. The fourteenth-review B rating remains specific to its v1.2.6 baseline (RFC-0004 revision 12).
- **v1.2.8 (2026-09-23, repository version; no external release)**: R81/R82 repair in the experimental L2 compiler. Bool/Bool equality and ordering now use i32 WASM comparisons; Bool arithmetic and unary negation fail at compile time without emitting hex. Bool/number mixed comparisons remain an explicit compile-time-strict L2 subset boundary. Value-position `if` branches now scan local `let` bindings in declaration order in a temporary lexical scope before inferring the tail type or closure signature; nested `if`, `match` Form B arm tails, and closure bodies share the rule. `verify_selfcomp` passes **92/92 = 35 behavior pairs + 57 negative rejections**. The frozen v1.0 language surface and host interpreter/WASM behavior are unchanged; Rust tests remain **535 unit + 8 integration**, eval 121/121 per host backend. The fourteenth-review B rating remains specific to its v1.2.6 baseline (RFC-0004 revision 11).
- **v1.2.7 (2026-09-23, repository version; no external release)**: R79/R80 repair in the experimental L2 self-hosted compiler. Control-flow child blocks now get independent lexical binding maps while sharing function-local slot allocation, so branch and loop `let` bindings cannot leak into sibling blocks or outside; `for` iteration variables also stay in their own environment. Closure capture resolves a parent local before a same-name global function or enum variant. The host compiler/interpreter and v1.0 language surface are unchanged. `verify_selfcomp` now passes **81/81 = 30 behavior-paired cases + 51 negative rejections**, including four formerly silent wrong-code inputs and four formerly accepted out-of-scope names; R81/R82 remain open. Rust tests remain **535 unit + 8 integration**; eval remains 121/121 on each host backend (RFC-0004 revision 10).
- **v1.2.6 (2026-09-22, repository version; no external release)**: frozen language surface unchanged. `lom fmt` no longer counts the `if` in an existing `match` guard (`pattern if cond =>`) as a block opener; Form A/B indentation stays stable (+2 Rust regression tests, **535 unit + 8 integration**). The experimental L2 self-hosted compiler adds L2.3-c1: non-generic user `enum` construction and scalar/user-enum `match` (literal/binder/wildcard/nested-variant patterns, guards, Form A/B arm bodies, runtime trap on no matching arm), reusing the L2.3-b allocator with memory/global emitted even when no closure table is needed. Exact type/arity checks reject unsupported constructs before emitting hex; the L2 WASM harness now preserves stdout produced before a trap. `verify_selfcomp` covers **72/72 = 25 behavior-paired cases + 47 negative rejections** (new negative cases also check the rejection reason); host interpreter and host WASM behavior are unchanged. Generic user enums, builtin `Result`/`Option`, enum equality/display, String patterns, and closure-valued match results remain explicit L2 subset boundaries (RFC-0004 revision 9). The R68/R74 compile-time-strict trust-boundary precedent also covers enum constructor arguments, match arm/guard types, assignment value types, and closure-signature reassignment. eval remains 121/121 on each host backend.
- **v1.2.5 (2026-09-22)**: twelfth-review remediation release (user-approved R73–R76 full package; frozen language surface unchanged — fix-engine apply layer, experimental L2.2 compiler call checking, and audit tooling only). **R73 (P1)** — `apply_plan` now deduplicates exactly-equivalent actions (same kind + start/end position + text) before applying: two or more MUT001 diagnostics on the same declaration within one round used to double-apply the same `let` → `let mut` Replace, corrupting the source into `let mut mut x` (PARSE001) in a single `--apply` run; now `x = 2; x = 3` applies once, yields a valid `let mut x`, and a clean `final` with `ok:true`. Non-identical same-position actions (e.g. the LEX001/PARSE001 ordered-insert interplay) are not merged. **R74 (P2)** — the L2.2 self-hosted compiler's call sites now validate arity and per-argument types against the callee signature (the `fns` summary carries a parameter-type string; `comp_call` rejects mismatches with a subset error): `f(1)` against `fn f(x: Float)` or `f(1,2,3)` against a two-parameter `add` used to print COMPILED and emit an unvalidatable module failing only at instantiation; two negative cases join the verifier (20 items total). This is the same compile-time-strict trust boundary as R68 (the host's TYPE003-warning + runtime-promotion semantics stay host-side; RFC-0004 revision 5). **R75 (P3)** — MUT001 same-level destructuring shadowing: the flat-scope binding list now carries declaration order (line numbers); when the nearest same-name binding before the assignment is a `let (x, y)` destructure (immutable, no `mut` syntax), the fix degrades to a Medium hint instead of mis-targeting an outer same-name `let` (which left the diagnostic untreated with `ok:true`). **R76 (P3)** — `doc_audit` gains a self_comp line-count anchor (65 → 67 items incl. the Z self-reference lock), pinning the README/HANDOFF_PROMPT "N-line" claims against drift. Tests 529 → **533 + 8 integration** (+2 R73, +2 R75). eval stays 121/121 on both backends.
- **v1.2.4 (2026-09-22)**: eleventh-review remediation release (user-approved R65–R72 full package; frozen language surface unchanged — fix-engine, LSP-transport, and experimental L2.2 compiler behavior only). **R65 (P1)** — MUT001 now resolves each assignment's scope-binding owner (`FnInfo::assign_binds`: a lexical scope-path walk mirroring the typechecker's environment — closure bodies and match arms are child scopes, `for` variables shadow within the loop body, if/while/for blocks are flat): a re-binding that resolves to a nested scope degrades to a Medium hint **even when an outer same-name `let` exists** (v1.2.3 mis-selected the outer declaration, and iterative `--apply` then re-replaced the already-`mut` line into `let mut mut x` — a PARSE001 corruption of the user's source; all three instantiations locked by negative tests: closure body / match arm / for-variable shadowing). Already-`mut` declarations are excluded from the hit set, making re-application idempotent; an outer-binding re-assign inside a closure still gets the correct High Replace (locked positive). **R66/R67 (P1/P2)** — the L2.2 self-hosted compiler's statement dispatch no longer discards rejection values: the per-statement match is value-threaded (`frag?`), so if/while/for/`return` statements are real compile errors (COMPILE-ERROR, no wasm emitted) instead of silently vanishing into behaviorally wrong modules; a 13-case negative set (`tools/selfcomp/negative/`) is now part of `verify_selfcomp` (18 items total). **R68 (P2)** — `let` type annotations are checked against the inferred value type at compile time (mismatch = subset rejection; a documented trust-boundary difference from the host's TYPE001-warning-then-runtime-value semantics — RFC-0004 revision 4). **R69/R70 (P2/P3)** — stale case-1 byte anchors replaced with magnitude-level wording; stale §2.2 sentence closed at handoff refresh. **R71/R72 (P3)** — non-UTF-8 LSP payloads get a `-32700` response (server survives); duplicate `Content-Length` headers are refused with exit 1; `exit` before `shutdown` exits 1 per LSP 3.17 (after `shutdown` stays 0). Tests 523 → **529 + 8 integration** (+6 R65 unit, +3 R71/R72 process-level). eval stays 121/121 on both backends.
- **v1.2.3 (2026-09-21)**: tenth-review remediation release (user-approved R62/R63 + L2 go-ahead; frozen language surface unchanged — fix-engine and LSP-transport behavior only). **R62** — MUT001's declaration location is now fully structural: the fix pipeline carries the enclosing function's `let` declarations extracted from the AST (`FnInfo::let_decls`, collected over the function's flat scope — if/while/for statement blocks share the function environment), replacing the v1.2.2 in-function text rescan that mis-fired on same-function closure shadowing (`let x` inside a closure body got rewritten to `let mut` while the original diagnostic went untreated with `ok:true`), inline comments (`let z = 1 # let x = 0` — the comment text was treated as a declaration), and string-literal text. Diagnostics whose declaration lives inside a closure body or match arm (separate scopes, not collected) now degrade to a Medium hint — never auto-rewritten. **R63** — the LSP server enforces a 16 MiB `Content-Length` cap (an oversized header is refused with a stderr line and exit 1 instead of the v1.2.2 `vec![0u8; len]` allocation abort — a 20-byte header could kill the server), and malformed JSON payloads get JSON-RPC error responses (`-32700` Parse error for invalid JSON, `-32600` Invalid Request for a non-request JSON value, `id: null` per the spec) instead of silent dropping; the server stays alive after either. Tests 513 → **523 + 5 integration** (+6 R62 fix-engine unit, +4 R63 LSP unit, +2 R63 process-level). eval stays 121/121 on both backends.
- **v1.2.2 (2026-09-21)**: ninth-review remediation release (user-approved full R55–R61 order; frozen language surface unchanged — this is bug fixing toward the frozen grammar and documented contracts, no RFC needed). Four P1 correctness blockers closed: **R55** High auto-fix safety — MUT001's `let` rescan is now scoped to the enclosing function (cross-scope rewrites impossible; param/loop/match rebinding gets a Medium hint with local-copy advice), EFF001 inserts at the signature **end** line (multiline signatures stay parseable) and merges same-function missing effects into one annotation, `lom-apply/v1` reports a `final` diagnostics block and `ok` = applied > 0 **and** final errors == 0; fix column basis unified to character columns (multibyte lines no longer mis-locate). **R56** — a worker-thread panic now exits 1 (join error mapped), and `i64::MIN / -1` / `i64::MIN % -1` produce structured RUNTIME000 instead of a Rust panic with exit 0; add/sub/mul are explicit `wrapping_*` (debug/release unified). **R57** — EOF no longer silently closes a block: a missing final `end` is a `PARSE001` error (strict errors, tolerant mode records with the hole-preserving AST; aligned across host and self-hosted parsers). **R58** — LSP JSON-RPC transport goes through the standard in-crate JSON parser (whitespace-tolerant, escape-decoding, nested-object safe; the v1.2.1 scanner ignored valid spaced `initialize` with a 0-byte response and mis-lexed multiline `didOpen.text` into 3× LEX005), all outbound strings fully escaped; real stdio e2e tests. P2: **R59** `json_parse` rejects unescaped control characters in strings per RFC 8259 (host/self-hosted/WASM aligned); **R60** CI zero-dependency gate hardened (all dependency-table TOML forms + `cargo metadata` factual assertion), `parse_type`/`parse_pattern` depth guards, eval prompt diagnostic check tooling (24/24, four drifted prompts fixed). P3: **R61** repo-wide rustfmt mechanical pass and CI actions upgraded to Node 24. Tests 487 → **513 + 3 integration**; two process-level test suites added (`tests/r56_process.rs`, `tests/r58_lsp_process.rs`); eval stays 121/121 on both backends.
- **v0.1 (2026-08-02)**: Initial draft. Phase 1 EBNF, Phase 2 features drafted, open questions listed.
- **v0.1.1 (2026-08-02)**: Patch after DeepSeek readability test (5/5 = 100% pass). Resolved 4 spec gaps:
  - Added §2.4.1 statement separation (newline-sensitive, not indentation-sensitive).
  - Expanded §6.4 match arm syntax (Form A single-expression + Form B block, mixing allowed).
  - Resolved open question #2: `+` overloaded for String concatenation.
  - Resolved open question #4: `=>` confirmed for match arms (avoids `->` ambiguity with closure return type).
  - Clarified arm separation: newlines, no semicolons.
- **v0.1.2 (2026-08-02)**: Phase 2.3 — structured JSON diagnostics implemented.
  - Rewrote §7 to match `lom-diag/v1` implementation (schema/file/ok/summary/diagnostics).
  - Added §7.3 implemented vs reserved error code namespaces (LEX/PARSE/RUNTIME implemented; TYPE/EFF/MAT/NAM reserved).
  - Added §7.4 tolerant parsing implementation notes (Phase 2.2 `Stmt::Hole` + sync-point recovery).
  - Added §7.5 CLI (`--json` / `--check` / `--help`) and output stream semantics.
  - Added §7.6 Phase 2.3 limitations (runtime positions, no `fix`/`retry`, single-file).
- **v0.1.3 (2026-08-03)**: Phase 2.8 — evaluation suite added.
  - Added §12 Evaluation Suite documenting the `eval/` 100-task benchmark (layout, task format, runner, AI-native focus, status). Renumbered Changelog to §13.
  - LLM pass-rate measured: 99/100 (99%), expert model + thinking mode. Phase 2 exit criterion met.
- **v0.2.1 (2026-08-03)**: Phase 3.2 — AST span-based diagnostic positioning.
  - Added §6.9.6 documenting `Span` on `FnDecl`/`EnumDecl`, parser `prev_token_pos()`, and typechecker `current_fn_span`. Removed Phase 3.1 `find_fn_line` hack. EFF001/TYPE010/NAM002 now report signature positions instead of `(0,0)`.
- **v0.2.2 (2026-08-03)**: Phase 3.3 — `list` + `json` standard library modules.
- **v0.19 (2026-08-31)**: Consolidated catch-up (this spec's prose was drifting; full detail lives in README milestone entries + docs/HANDOVER.md):
  - §4.3/§6 "(drafted)" labels removed — all Phase 2 features implemented long ago.
  - §9.3 List internal representation corrected: `Vec` → Rc cons cells (v0.5.0).
  - §12 eval counts updated to the 113-task set (was 100-task era).
  - §3 EBNF: `item` now includes `enum_decl`/`import_decl`; `fn_decl` return type is `->` (not `:`) with optional effect annotation.
  - Phase 4-7 feature surface (REPL/LSP/packages, `map`/`file`/`env` modules, WASM backend via `lom build --target wasm`) is documented in README + docs/lom-project-guide.html; this spec's §1-§9 language core remains accurate as of v0.19.0.
  - Added §4.3 `List<T>` type entry; §8.2/§9.2 stdlib module tables extended with `list` and `json`.
  - Added §9.3 `list` module (immutable list API: `list_empty`/`list_cons`/`list_length`/`list_get`/`list_is_empty`/`list_head`/`list_tail`) and §9.4 `json` module (`json_parse`/`json_stringify` with JSON↔Lom Value mapping table, surrogate pair support, compact serialization).
  - Added `Value::List { elems: Vec<Value> }` runtime variant (immutable semantics). Type-checker signatures use `List<_Any>`.
- **v0.2.3 (2026-08-03)**: Phase 3.4 — `file` module + `string` extensions.
  - §8.2/§9.2 stdlib tables extended: `string` adds `split`/`contains`/`replace`/`starts_with`/`ends_with`; new `file` module with `file_read`/`file_write`/`file_append`/`file_exists`.
  - Added §9.5 `file` module (all functions declare `[IO]` effect; EFF001 enforces that callers declare `! [IO]`; `main` is exempt).
  - `split` returns `List<_Any>` (empty separator splits by UTF-8 char); all `string` extensions are pure.
- **v0.2.4 (2026-08-03)**: Phase 3.5 — `env` module + todo list CLI demo. **Phase 3 exit criterion met.**
  - §8.2 stdlib table extended with `env` module (`args`); added §9.6 `env` module (pure function, returns `List<_Any>`, `argv[0]` = .lom path).
  - CLI `--` separator: `lom <file.lom> -- <args...>` passes trailing args to the Lom program via `env::args()`.
  - Added [examples/todo.lom](examples/todo.lom) — complete todo list CLI (add/list/done/remove/help) with JSON persistence, recursive list traversal, effect-correct `! [IO]` annotations. End-to-end verified all commands.
- **v0.20.0 (2026-08-31)**: MUT001 immutable-reassignment warning.
  - §5.1 rewritten: `let` without `mut` / function parameters / `for` loop variables / `match` bindings are immutable; reassignment (including `+=` desugars) now yields a `MUT001` warning — stderr, non-blocking, interpreter unchanged (gradual-typing philosophy).
  - §7.3: `MUT` namespace added (and the long-stale "reserved" table folded into implemented).
- **v0.21.0 (2026-08-31)**: Phase 3.2b — expression-level spans.
  - Added §6.9.7: `Expr` is now `struct { kind: ExprKind, span: Span }`; `Stmt::Let`/`Stmt::Assign` carry spans; NAM003/NAM004-field/MUT001/TYPE001/TYPE002/TYPE003/TYPE020 diagnostics report precise positions; `lom fix` spelling repairs become single-point `replace` (scan fallback retained). §6.9.6 scope note and §7.6 limitations updated.
- **v0.22.0 (2026-08-31)**: Phase 8 readiness — RFC-0003 (full self-hosting) accepted; `lom <file> --dump-ast` added (deterministic AST indentation tree, spans excluded — §7.5). Language surface unchanged.
- **v0.27.0 (2026-09-02)**: `string.char_from_code(cp)` added (§9.2) — Unicode code point → single-char String. The only language-surface change of the pre-v1.0-freeze json adjudication (RFC-0003 revision 24): it unblocks the self-hosted interpreter's `json_parse`/`json_stringify` (Unicode escapes) and removes the L2 self-hosted compiler's byte-construction blocker. Invalid code points (negative / > 0x10FFFF / surrogate D800-DFFF) are runtime errors. Builtin count 42 → 43.
- **v1.0.0 (2026-09-02)**: **Language-surface freeze** (§14). Shipped in the freeze cut: the lexer's non-ASCII string-literal repair — string literals (and generalized `\` escapes) now decode complete UTF-8 characters instead of per-byte Latin-1 expansion, which had corrupted non-ASCII literals on the reference implementation since Phase 3 (todo.lom's Chinese literals printed mojibake; the self-hosted lexer was already correct). ASCII-only programs are byte-identical. The tolerant lexer now reports one error per character (full character in the message) for non-ASCII identifier positions instead of one per byte. Regression: 454 tests; eval 114/114 dual backend; self-host six-mode acceptance with the Latin-1 fold logic now a zero-count defensive net.
- **v1.1.0 (2026-09-03)**: post-1.0 review-rectification release (docs/TODO.md T1-T7). Language surface unchanged (freeze §14 intact; the one new diagnostic code is warning-only, as §14-③ permits):
  - **MUT002** (warning): a closure body referencing a captured outer `mut` binding — the interpreter (shared scope) and WASM (value-copy at creation) disagree on this pattern, so the check flags it. Non-blocking, stderr-only, same shape as MUT001. Since v1.5.1 the interpreter executes closure-body assignments to captured `mut` bindings per the shared-scope semantics (previously a RefCell panic, R112); host-WASM and L2 compile-reject the form outright. The self-hosted checker (§8.2 subset) does not produce MUT-family codes.
  - **NAM003 false-positive fix**: `let f = fn ... f(...) ... end` (self-referencing let-bound closure — a designed feature; the interpreter captures the scope by reference and WASM pre-binds + patches the env slot) no longer reports NAM003. Both host and self-hosted checkers fixed symmetrically; non-closure self-references (`let x = x + 1`) still report.
  - **Float non-finite display unified**: `inf`/`-inf`/`NaN` print verbatim (no `.0` appended) on both backends; finite-float display unchanged. The interpreter's `1000000000000000200000000000000.0` vs WASM's `1.0000000000000002e+30` large-float divergence is recorded (not unified) in SPEC_FOR_AI §11f.
  - Regression: 456 tests; eval 116/116 dual backend (tasks 116 recursive-closure-let, 117 float inf/NaN added); self-host five-mode acceptance green.
- **v1.1.1 (2026-09-07)**: WASM backend bug-fix release (W package, docs/TODO.md; full archive in RFC-0003 revisions 25-29). Language surface unchanged, interpreter unchanged — both fixes are in the WASM code generator:
  - **Variant-arm eager payload load OOB (the Phase 8.4 blocker, root-caused)**: match lowering placed the payload extraction `i64.load(ptr+8)` in a flat AND with the arm test — WASM's eager evaluation ran it before the tag/variant guard, so a nullary enum (8-byte header-only object) sitting at the end of linear memory faulted with `memory access out of bounds`. The old "non-deterministic ~6.7KB threshold" was actually heap-layout sensitivity (the argv[0] string length alone flips it). Fix: payload loads deferred until the variant test passes; the discriminant read is tag-guarded.
  - **`return`/`?` inside `for` silently swallowed (latent since v0.12.0)**: the three iteration-dispatch `if`s in `for` lowering did not push `Label::If`, so `br $ret` inside the loop body under-counted depth and branched to the dispatch exit — the early return vanished (e.g. `for x in xs ... return x` kept falling through). Fix: the dispatch ifs push/pop the label; +3 e2e regression tests.
  - With these, the Phase 8.4 wasm-carrier self-proof completes: layer-2 full set (examples + bootstrap + 116 eval reference solutions, 5 stable rounds), layer-3 golden (wasm self-hosted interpreter running `stmt_interp.lom`, 39 lines verbatim, needs `--stack-size`), and self-application (the wasm self-hosted interpreter parsing its own 5703-line source, dump byte-identical to the native host). The wasm acceptance layer re-enters CI. Regression: 459 tests; eval 116/116 dual backend; self-host six-mode acceptance green.

- **v1.1.2 (2026-09-08)**: robustness patch — deep-recursion and deep-nesting failures are now structured diagnostics instead of process aborts (M/N/Q workpackages, docs/TODO.md). Language surface unchanged (no new syntax/keywords/diagnostic codes/builtins — the new messages reuse `RUNTIME000`/`PARSE000`):
  - **Evaluator depth guard (N1)**: call/closure recursion beyond 80,000 frames (measured ~100k theoretical on the 256 MB stack) reports `[RUNTIME000] 递归深度超过 80000 层...` positioned at the recursive `fn` signature, exit 1.
  - **Parser depth guard (Q3)**: expression nesting beyond 30,000 levels (40k passes / 50k overflows measured) reports `[PARSE000] 表达式嵌套超过 30000 层...` and terminates parsing; `json_parse` nesting beyond 100,000 reports a structured error. All recursion entry points now fail with diagnostics (SECURITY limitation 1 closed).
  - Also in this window (behavior-neutral): example-audit gates extended to LANGUAGE_SPEC + tutorial (M1), `Pattern::Variant` carries `name_span` so NAM004 variant diagnostics pinpoint the name token (M2), fix_corpus 4→8 pairs (M3), main.rs/typechecker.rs split into readable modules (Q2), eval tasks 118 (078 unambiguous twin) + 119 (MUT001 warning repair) added — 118/118 both backends, test count 473/473, line coverage 84.4%.

- **v1.1.3 (2026-09-14)**: checker fix + differential-testing tooling (D workpackage, docs/TODO.md). Language surface unchanged (no new syntax/keywords/codes/builtins):
  - **`and`/`or` operands skipped by the type checker (latent since Phase 2.4)**: `ExprKind::Logical` returned `Bool` without recursing into its operands, so undefined functions/variables inside `and`/`or` operands escaped `NAM003` entirely — `--check` passed, the interpreter only failed at runtime if short-circuit evaluation reached the call, while the WASM build failed at compile time (a backend asymmetry on erroneous programs). Found by the D-workpackage differential tester: a generator slip (`not (x)` — `not` is not a Lom operator) surfaced the checker hole because the interpreter short-circuited past the undefined call while the wasm build could not. Fix: the Logical branch recurses into both operands (purely static checks such as NAM003 are not exempted by evaluation order; operand types stay lenient per gradual typing). The self-hosted checker (Phase 8.2) already recursed — this realigns the host implementation with it. +2 regression tests (475/475).
  - **Differential testing tooling**: `tools/diff_gen.py` (typed template-driven random program generator — deterministic output, avoids the §11f known divergences by construction, weighted coverage of early-return-inside-control-flow shapes that hid the W-workpackage `for`-return bug for 7 versions) + `tools/diff_test.py` (dual-backend differential runner with a `--check` gate separating generator bugs from backend diffs: 1000 programs across two seed ranges byte-identical on both backends; `--probe` mode verifies the executable §11f divergences still behave as documented — guarding the whitelist itself against rot). Fixed-seed smoke rounds wired into CI doc-gates.

- **v1.1.4 (2026-09-14)**: package & alias patch + differential-coverage expansion (D workpackage phase 2, docs/TODO.md). Language surface unchanged:
  - **`lom build` package discovery followed the CWD instead of the main file's directory** (latent since 7.8): the interpreter path discovered `lom.toml` next to `main.lom`, but the wasm build path looked in the current working directory — the same file compiled differently depending on where you stood (and `cd` was effectively required, contradicting the interpreter). Fix: `merge_packages_for_wasm` takes the file's directory as `base_dir` (found by D5 package-mode differential testing before the generator even ran — pkg_demo failed from the repo root). +1 unit test.
  - **Package symbols imported with an `as` alias were broken in three places** (latent since 4.4): the type checker's `collect_import` only resolved aliases against the builtin table (false `NAM003` on `--check`); the interpreter's `eval_call` only resolved aliases on the builtin path (runtime `RUNTIME002`); the wasm backend's arity check looked up signatures under the alias name (compile-time "expects 0 arguments"). All three fixed to resolve alias→original before consulting the user-function tables. +1 unit test.
  - **Int value range divergence documented as §11f-7** (found by package-mode differential testing; latent since Phase 7.6a / v0.11.0 — the 4-bit tag era; R24 correction: first recorded as the non-existent "v0.7.2"): the wasm backend's tagged-i64 (4-bit tag) only carries 60 payload bits — at ≥2⁵⁹ values wrap into the sign bit, at ≥2⁶⁰ they silently truncate, while the interpreter is full i64. Structural fix would require re-boxing the value representation (out of freeze scope); documented with an `int-range` probe, generator avoids the range.
  - **Differential-coverage expansion** (all byte-identical on both backends): D5 package mode — multi-file projects with flat and chained (`libb`→`liba`) dependency graphs, cross-package imports with aliases, 400 projects over two seed ranges; D6 JSON — divergence-2 boundary measured (safe: integer literals and true decimals; divergent: `30.0`/`1e2`/`-0.0`), 3 JSON templates (parse-consume / construct-stringify / nested round-trip), 1000 programs; D7 recursion gradient — depth tiers up to 8000 (within the interpreter's 80k guard and V8's default stack), mutual recursion, early-return-inside-recursion, 1000 programs. Probes now cover 6 of the 7 §11f divergences (`json-number` and `int-range` added).
- **v1.2.1 (2026-09-16)**: repair-loop asset release (③ workpackage of the user-approved 2026-09-16 four-pack; frozen language surface unchanged — `lom fix` behavior only). Two more diagnostics get machine-applicable fix actions: **NAM005** now inserts the exact `from <module> import {<name>}` line carried by its own diagnostic message at the top of the file (High — insert-and-done); **MUT001** rescans the declaration by variable name and rewrites the unique `let x` declaration to `let mut x` (High when exactly one hit; multi-hit shadowing or non-`let` targets fall back to a Medium hint — comment lines and word boundaries respected). fix_corpus grows **8 → 11 pairs** (09 MUT001 / 10 NAM005 / 11 LEX005 unexpected ASCII char), all driven end-to-end through the iterative `--apply` loop. eval grows **119 → 121 tasks**, error_repair **22 → 24**: task 121 (LEX005 fullwidth punctuation — CJK input-method contamination, real `--check` JSON) and task 122 (TYPE002 truthiness warning forecasting a RUNTIME001 failure — the 120-style "warning as prophecy" shape). 121/121 reference solutions pass; +5 fix tests.
- **v1.2.0 (2026-09-15)**: checker capability release (B workpackage, docs/TODO.md — the unimported-builtin blind spot found by D-phase-4 differential testing, user-approved). Language surface unchanged except one new **warning**-severity diagnostic code (permitted by freeze §14-③, MUT001/MUT002 precedent):
  - **`NAM005` (warning): known builtin used without import.** Previously a call to a real builtin that was never imported passed `--check` silently (the typechecker registers all 43 builtin signatures up front and the old division of labor left import-availability to the runtime) and only failed at run time with `RUNTIME002`. Since "forgot the import line" is a top LLM mistake, the checker now flags it statically: `[NAM005] 内建 '<name>' 未导入——需在文件顶部声明：from <module> import {<name>}` (module name from the same `module_of` table the runtime uses — single source of truth). Non-blocking (warning, `ok:true`), same gradual-typing promise as MUT001/MUT002. Prelude (`println`/`print`) is exempt; `f as g` imports unlock only the alias `g` (using the original name `f` still warns — matching the runtime's `available_builtins`); a user `fn` colliding with a builtin name stays a single `NAM002` error (no warning stacking). The self-hosted checker (§8.2 subset) does not produce NAM-family warnings — `verify_selfhost --static` compares the four-code subset only (T3/MUT002 precedent). +5 unit tests (482 total); eval task 120 (warning-repair form: static warning previews the runtime failure).

---

## 14. v1.0 Freeze Declaration (2026-09-02)

The language surface below is **frozen for the v1.x line**. Any change requires a new numbered RFC (with LLM-impact analysis, per the governance docs) that explicitly supersedes this section — no silent additions.

1. **Syntax**: the grammar of §3 in its entirety — statement separation semantics (§2.4.1), match Form A/B with guards and binders, closures, pipeline, `?` propagation, effect annotations, enums, structural records, tuples, ranges, compound assignment, `for` over Int/String/List, explicit imports with per-item aliasing. No new syntax forms in v1.x.
2. **Reserved words**: exactly 20 — `fn let mut if else elif while for in return match end and or True False from import as enum` (§2, verified against the lexer). No new keywords.
3. **Diagnostic codes**: the implemented namespaces of §7.3 (`LEX` / `PARSE` / `TYPE` / `NAM` / `EFF` / `MAT` / `MUT` / `RUNTIME` / `PKG`). New warning-only codes for new checks may be added (they cannot alter the semantics of existing programs); existing codes are not removed, renumbered, or reclassified from warning to error.
4. **Builtins**: exactly 43 — the §9 tables (`io` 2, `string` 12, `math` 4, `list` 10, `json` 2, `map` 8, `file` 4, `env` 1). `char_from_code` (v0.27.0) is the last builtin added before the freeze; `type_of` was adjudicated **not** added (RFC-0003 revision 24).

Adjudications leading to this freeze (user, 2026-09-02):

- **①** Phase 6's north star "third-party production use" is **not** a v1.0 gate — retained as a long-term north star. v1.0 certifies engineering completeness, not market validation; public claims must keep that qualification.
- **②** Freeze scope = the language surface above. Engineering debts remain in the post-1.0 ledger and do not block the freeze: the wasm OOB bug (RFC-0003 revision 21 — affects only the self-hosted-interpreter-compiled-to-wasm scenario; the wasm backend passes 114/114 on regular programs), Pattern spans, structured stack-overflow diagnostics, package registry, debugger, and the L2 self-hosted compiler (its `char_from_code` blocker is resolved; starting it is a workload decision). *(Status annotation, 2026-10-03 — wording refresh only, adjudications unchanged: the wasm OOB bug was root-fixed on 2026-09-07 in the W work package, RFC-0003 revisions 25-29; the L2 self-hosted compiler has since been delivered per RFC-0004 — strong-quine self-hosting since v1.4.0, structured `[L2xxx]` diagnostic codes since v1.6.0. Pattern spans, structured stack-overflow diagnostics, package registry, and debugger remain open post-1.0 ledger items.)*
- **③** The json dual-builtin gap was resolved pre-freeze by adding `char_from_code` only (42→43; §9.2), closing RFC-0003 revision 17's ledger item.

Non-goals reaffirmed at freeze (RFC-0001): no `self`/methods, no traits, no `pub`, no out-params, no type aliases, no independent `Char` type, no wildcard imports, no `break`/`continue`, no first-class mutable closure captures — captures are value-copies; the interpreter's shared-scope write form is a registered cross-backend divergence flagged by MUT002, not a mutability feature (reworded 2026-10-03 for precision; the adjudicated intent is unchanged).
