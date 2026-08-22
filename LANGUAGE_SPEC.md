# Lom Language Specification (v0.1 Draft)

> **Status**: Phase 0 Draft · 2026-08-02
> **Stability**: Unstable — syntax and semantics may change before Phase 1 freeze.
> **Scope**: This spec covers the Phase 1 minimal subset (interpreter-runnable) and drafts the Phase 2 LLM-coding-native features. ~~Workload-native features (tensor, autodiff, MLIR) are out of scope and will be specified in Phase 4.~~ **Update (2026-08-07 retrospective)**: Phase 4 direction changed from "workload-native" to "LLM-repair-native + toolchain" — `lom fix` auto-repair expansion, REPL, LSP, package manager. Workload-native (tensor/autodiff/MLIR) is dropped (Mojo acquired by Qualcomm saturates the AI-compute lane). See [§2.5 retrospective](docs/lom-project-guide.html).

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
match end
True False
Int Float Bool String Unit
Result Ok Err
from import as
struct enum trait impl
type
pipe  (reserved for |>)
```

*Reserved for later phases*: `async await mod use ref move where grad tensor`
*(note: `pub` was listed here historically but was never implemented or reserved — as of v0.5.x `pub` is an ordinary identifier; all top-level items are public, see §8.3)*

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

item          = fn_decl ;

fn_decl       = "fn" identifier "(" [ params ] ")" [ ":" type ] block ;
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

### 4.3 Phase 2 types (drafted, specified in §6)

| Feature | Syntax | Status |
|---|---|---|
| Structural records | `{x: Int, y: Int}` | §6.2 |
| `Result<T, E>` | `Ok(v)` / `Err(e)` | §6.1 |
| `Option<T>` | `Some(v)` / `None` | §6.1 |
| Tuples | `(Int, String)` | §6.3 |
| Type aliases | `type UserId = Int` | §6.5 |
| Traits | `trait Show { fn show(self) -> String }` | §6.6 |
| `List<T>` (immutable, Phase 3.3) | `List<Int>` via `Type::Generic("List", [T])` | §9.3 |

> **Phase 3.3 `List<T>`**: a runtime `Value::List { elems: Vec<Value> }` variant exposed through the `list` stdlib module (§9.3). No list literal syntax yet — construct via `list_cons` or `json_parse`. Type-checker signatures use `List<_Any>` to accept any element type; element type tracking is deferred.

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
| `lom <file>` (default run) | No | Dynamic execution; type errors ignored |
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

- Trait resolution (Phase 2.6)
- ~~Effect system (Phase 2.5)~~ — implemented in Phase 2.5 (see §6.7)
- Generic instantiation (only placeholder compatibility)
- Precise tuple index inference (`.0`/`.1`)
- Cross-function closure call-site type checking (closures as values return `Unknown`)

#### 4.5.6 Registered builtins

Prelude (`println`, `print`) and stdlib modules (`io`, `string`, `math`) function signatures are registered at startup so that calls to them are not flagged `NAM003`.

---

## 5. Semantic Rules (Phase 1)

### 5.1 Immutability

- `let x = 3` — `x` is immutable. Reassignment is a compile error.
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
- Inner scopes cannot redeclare outer bindings (shadowing disallowed in Phase 1, to reduce LLM confusion).

---

## 6. Phase 2 LLM-Coding-Native Features (drafted)

> These features define Lom's identity as an LLM-coding-native language. They are drafted here for spec completeness but implemented in Phase 2.

### 6.1 Result and Option (error-as-value)

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

Field access: `p.x`, mutation (if `mut`): `p.x = 5`.

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

**Exhaustiveness check**: `match` on `Result` and `Option` must cover all variants or have a `_` arm. Non-exhaustive match is a compile error. This forces LLMs to handle failure branches.

**Arm separation**: arms are separated by newlines. No semicolons. This is consistent with Lom's newline-sensitive statement separation (§2.4.1).

### 6.5 Type aliases

```
type UserId = Int
type Point = {x: Float, y: Float}
```

### 6.6 Traits (structural, Phase 2 draft)

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

Effects declare side effects in the function signature:

```
fn read_file(path: String) -> Result<String, IoError> ! [IO]
fn print(s: String) -> Unit ! [IO]
fn now() -> Int ! [Clock]
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
| `replace` | Replace `(line,col)..(end_line,end_col)` with `text` | Implemented in Phase 3.1 `apply.rs` (no current fix generator uses it, but the executor supports it) |
| `delete` | Delete `(line,col)..(end_line,end_col)` | LEX005 (delete unexpected char) |
| `hint` | Text guidance only; `line`/`col` may be `0` | Most type/name/runtime errors; MAT001 provides `text` snippet |

#### 6.9.3 Fix strategies by error code

| Code | Strategy | Confidence | Action |
|---|---|---|---|
| `LEX001`/`LEX002` | Insert `"` at end of error line | high | insert |
| `LEX003`/`LEX004` | Hint: check number format | low | hint |
| `LEX005` | Delete the unexpected char | high | delete |
| `PARSE001` | Hint: check syntax structure | low | hint |
| `PARSE002` | Hint: `Result<T, E>` needs 2 type params | medium | hint |
| `PARSE003` | Hint: `Option<T>` needs 1 type param | medium | hint |
| `PARSE099` | Hint: hole, complete syntax | low | hint |
| `TYPE001` | Hint: type mismatch | low | hint |
| `TYPE002` | Hint: condition must be Bool | medium | hint |
| `TYPE003` | Hint: arg count mismatch | low | hint |
| `TYPE010` | Hint: return type mismatch | low | hint |
| `TYPE020` | Hint: `?` misuse | medium | hint |
| `MAT001` | Provide missing branch text (e.g. `Green => ()`) | medium | hint (with text) |
| `NAM002` | Hint: duplicate definition | low | hint |
| `NAM003` | Hint: undefined variable | low | hint |
| `NAM004` | Hint: no such field/variant | low | hint |
| `EFF001` | Insert effect annotation: `! [E]` at line end (pure fn) or `, E` before `]` (partial effects) | high | insert |
| `RUNTIME001` | Hint: runtime type mismatch | low | hint |
| `RUNTIME002` | Hint: undefined at runtime | low | hint |
| `RUNTIME003` | Hint: hole execution | low | hint |
| `RUNTIME005` | Hint: module/symbol not found | medium | hint |

#### 6.9.4 Phase 3.1 `--apply` execution

`lom fix <file> --apply [--dry-run] [--json]` auto-applies high-confidence fixes to the source file.

- **Safety filter**: only `confidence=High` AND `action≠Hint` fixes are applied. Low-confidence fixes are left for the LLM to decide.
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
- **End-to-end verified**: `lom examples/effects_bad.lom --check` now reports `EFF001` at `(9:1)` and `(20:1)` (the `fn` keyword positions of `helper` and `bad_helper`), previously `(0:0)`. `lom fix --apply --dry-run` still produces correct inserts at `[9:25]` and `[20:35]`.

> **Scope**: Only `FnDecl`/`EnumDecl` carry spans. Statement/expression-level diagnostics (TYPE001, NAM003, etc.) still report `(0,0)`; runtime errors still report `(0,0)`. Full expression-level spans are deferred to Phase 3 LSP work.

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
- `span`: `{ "start": [line, col], "end": [line, col] }` — partially implemented (Phase 3.2: `FnDecl`/`EnumDecl` carry `Span`; expression-level spans deferred to Phase 3 LSP work)
- ~~`fix`: `{ "description", "suggestion", "start", "end" }` — machine-actionable repair (Phase 2.7)~~ Moved to `lom-fix/v1` (Phase 2.7 implemented; see §6.9). Kept out of `lom-diag/v1` so the diagnostic schema stays a pure error report.
- ~~`retry`: whether LLM should retry generation after applying `fix` (Phase 2.7)~~ Likewise in `lom-fix/v1` per-plan field (Phase 2.7 implemented; see §6.9).

### 7.3 Error code namespaces

**Phase 2.3 implemented namespaces**:

| Prefix | Stage | Range | Examples |
|---|---|---|---|
| `LEX` | lex | LEX001-099 | `LEX001` unclosed string, `LEX005` unexpected char |
| `PARSE` | parse | PARSE001-099 | `PARSE001` expected token, `PARSE099` hole (tolerant parser) |
| `RUNTIME` | runtime | RUNTIME001-099 | `RUNTIME001` type mismatch, `RUNTIME002` undefined, `RUNTIME003` hole execution |

**Reserved for future phases**:

| Prefix | Stage | Phase | Example |
|---|---|---|---|
| `TYPE` | type | 2.4 | `TYPE002` expected Int, got Float |
| `EFF` | type | 2.5 | `EFF001` pure function calls effectful |
| `MAT` | type | 2.4 | `MAT001` non-exhaustive match |
| `NAM` | type | 2.4 | `NAM003` undefined variable (compile-time) |

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
lom --help | -h               Show help
```

Exit codes: `0` = success / no diagnostics / info export OK / fix plan generation OK / apply OK (even if 0 fixes applied); `1` = read/lex/parse/runtime error / apply write failure (note: `lom fix` exits `0` as long as the plan was generated or apply completed, even if diagnostics exist — the plan/apply result is the product).

Output streams:
- `--json` / `--check`: diagnostics to **stdout** (the diagnostic report is the program's product).
- `lom info` (with or without `--json`): type info to **stdout** (the info report is the product).
- `lom fix` (with or without `--json`): repair plan to **stdout** (the plan is the product).
- Default run mode runtime errors: diagnostics to **stderr** (program failure).

### 7.6 Limitations in Phase 2.3

- **Runtime error positions**: AST nodes do not yet carry source positions at the expression level (planned for Phase 3 LSP work). Runtime diagnostics report `line=0, col=0`; the message itself carries enough context to identify the failure. Lex/parse diagnostics are fully positioned; typecheck diagnostics for EFF001/TYPE010/NAM002 are positioned via `FnDecl.span` (Phase 3.2). Statement/expression-level typecheck diagnostics (TYPE001, NAM003, etc.) still report `(0,0)`.
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

### 8.2 Standard library modules (Phase 2.1.5; Phase 3.3 adds `list`/`json`; Phase 3.4 adds `file` + `string` extensions; Phase 3.5 adds `env`; Phase 5.20 adds `map`)

| Module | Exports | Notes |
|---|---|---|
| `io` | `println`, `print` | Also in prelude (auto-available); explicit import only needed for aliasing |
| `string` | `len`, `int_to_string`, `string_to_int`, `trim`, `upper`, `lower`, `split`, `contains`, `replace`, `starts_with`, `ends_with` | Phase 3.4 adds `split`/`contains`/`replace`/`starts_with`/`ends_with` (§9.2) |
| `math` | `sqrt`, `abs`, `min`, `max` | Must be imported to use |
| `list` | `list_empty`, `list_length`, `list_get`, `list_is_empty`, `list_head`, `list_tail`, `list_cons`, `list_map`, `list_filter`, `list_fold` | Phase 3.3 — immutable list ops (§9.3); v0.4.3 adds higher-order ops |
| `json` | `json_parse`, `json_stringify` | Phase 3.3 — zero-dependency JSON parser + serializer (§9.4) |
| `file` | `file_read`, `file_write`, `file_append`, `file_exists` | Phase 3.4 — file system I/O, all declare `[IO]` effect (§9.5) |
| `env` | `args` | Phase 3.5 — command-line arguments (§9.6) |
| `map` | `map_empty`, `map_set`, `map_get`, `map_has`, `map_remove`, `map_keys`, `map_values`, `map_size` | Phase 5.20 (v0.5.1) — string-keyed dictionary, reference semantics (§9.7) |

**Prelude** (auto-imported, no `from` needed): `println`, `print`.

Calling an unimported non-prelude builtin produces a structured error:
```
符号 'len' 未导入。需在文件顶部声明：from string import {len}
```

### 8.3 Public/private (design sketch — **not implemented**)

> **Status (v0.5.x)**: `pub` is not a keyword and this syntax does not parse. The package manager (Phase 4.4) treats **all top-level `fn`/`enum` as public**; there is no privacy. This section is a design sketch for a future phase, kept for reference.

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
| `math` | `sqrt(x)` | `Float -> Float` (also accepts `Int`) | Square root |
| `math` | `abs(x)` | `Int -> Int \| Float -> Float` | Absolute value |
| `math` | `min(a, b)` | `(Int, Int) -> Int \| (Float, Float) -> Float` | Minimum |
| `math` | `max(a, b)` | `(Int, Int) -> Int \| (Float, Float) -> Float` | Maximum |
| `io` | `println`, `print` | same as prelude | Explicit import only needed for aliasing |

> All `string` functions are pure (no effect). `split` returns `List<_Any>`; element-type tracking is deferred.

### 9.3 `list` module (Phase 3.3 — implemented)

A pure, immutable list type exposed through the `list` standard library module. Internally backed by `Value::List { elems: Vec<Value> }`; all operations return new `List` values without mutating the input (immutable semantics, in the spirit of functional data structures).

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

The parser supports `\uXXXX` Unicode escapes, including surrogate pairs (e.g. `\uD83D\uDE00` → 😀). Both functions are pure (no `! [...]` effect). Examples: [examples/json_demo.lom](examples/json_demo.lom).

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

> `json_stringify` serializes a Map as a JSON object with sorted keys. Design note: copy-on-write was considered and rejected — the builtin argument slice always holds an `Rc`, so `Rc::get_mut` would never succeed and every write would clone.

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

## 11. Open Questions (to resolve before Phase 1 freeze)

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
6. **`self` / `self` keyword**: lowercase `self` (Rust) vs `this` (Java) vs explicit first param name?
7. **Trait dispatch**: static (monomorphized) only, or dynamic (vtable) too?
8. **`pub` granularity**: per-item `pub` keyword, or module-level `pub use`?

Questions 1-4 are resolved (recorded inline above). Questions 5-8 remain open and are deferred to v1.0 scoping decisions — each may legitimately be answered "not in v1.0" (e.g. `pub` is a deliberate non-feature so far: all top-level items are public).

---

## 12. Evaluation Suite (Phase 2.8 — implemented)

Lom ships a 100-task evaluation suite at `eval/` to measure LLM generation pass-rate — the hard metric for Lom's "AI-native" claim. It is not part of the language proper, but tests conformance to this spec.

### 12.1 Layout

```
eval/
  README.md              # design goals, format, runner usage
  manifest.json          # lom-eval/v1: 10 categories × task counts
  tasks/
    01_arithmetic.json        # 10 — §3 grammar, §4 types, §5 semantics
    02_control_flow.json      # 10 — §5 if/while/for/return, short-circuit
    03_types.json             # 10 — §4 Int/Float/Bool/String/Unit
    04_closures.json          # 10 — §6.3 first-class closures
    05_match_enum.json        # 15 — §6.4 match/enum, §6.5 Result/Option
    06_pipeline.json          # 10 — §6.6 `|>` linear pipeline
    07_records_tuples.json    # 10 — §6.7 structural records/tuples
    08_effects.json           #  5 — §6.8 explicit effects `! [IO, Clock]`
    09_modules.json           #  5 — §8 module system, §9 stdlib
    10_error_repair.json      # 15 — §7 diagnostics + §6.9 fix plan (AI-native core)
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

- `./run.ps1 -Verify` (Windows) / `./run.sh --verify` (Unix) — smoke-test reference solutions against `expected`. **100/100 pass.**
- `./run.ps1 -CandidatesDir <dir>` — evaluate LLM-generated code. Reads `<id>.lom` from `<dir>`, runs each, compares stdout to `expected`. Reports per-category and overall pass-rate. Exit code 1 on any failure (CI-friendly).
- The runner only runs `lom` + compares stdout; it does **not** call any LLM API. LLM candidates are produced out-of-band (e.g. DeepSeek API batch) into a `candidates/` directory.

### 12.4 AI-native focus

`10_error_repair.json` (15 tasks, 15% of the suite) is Lom's differentiator: instead of "can the LLM write code", it tests "can the LLM repair code given `lom-diag/v1` + `lom-fix/v1`" — directly validating the §7 / §6.9 toolchain that Phases 2.2–2.7 built.

### 12.5 Status

- Reference solutions: 100/100 pass (`./eval/runner/run.ps1 -Verify`).
- LLM pass-rate: **99/100 (99%)** — measured 2026-08-03 with expert model + thinking mode. 9/10 categories at 100%; sole failure (task 078) was output-format misunderstanding, not a language-feature error. See `eval/REPORT.md` for full analysis. **Phase 2 exit criterion met.**

---

## 13. Changelog

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
