# Lom Language Specification (v0.1 Draft)

> **Status**: Phase 0 Draft · 2026-08-02
> **Stability**: Unstable — syntax and semantics may change before Phase 1 freeze.
> **Scope**: This spec covers the Phase 1 minimal subset (interpreter-runnable) and drafts the Phase 2 LLM-coding-native features. Workload-native features (tensor, autodiff, MLIR) are out of scope and will be specified in Phase 4.

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

*Reserved for later phases*: `async await mod pub use ref move where grad tensor`

### 2.3 Operators (by precedence, low → high)

| Level | Operators | Assoc | Notes |
|---|---|---|---|
| 1 | `\|>` | left | Pipeline |
| 2 | `=` | none | Assignment (statement, not expression) |
| 3 | `or` `and` | left | Short-circuit boolean |
| 4 | `==` `!=` `<` `>` `<=` `>=` | none | Comparison |
| 5 | `+` `-` | left | Additive |
| 6 | `*` `/` `%` | left | Multiplicative |
| 7 | `!` `-` (prefix) | right | Unary |
| 8 | `?` (postfix) | left | Error propagation |
| 9 | `(` `)` `[` `]` `.` `{` `}` | — | Call / index / field / struct |

> `|>` is the lowest-precedence binary operator so `a |> f(b) |> g(c)` parses as `(a |> f(b)) |> g(c)` without extra parens.

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

expr          = pipe_expr ;
pipe_expr     = or_expr { "|>" or_expr } ;
or_expr       = and_expr { "or" and_expr } ;
and_expr      = cmp_expr { "and" cmp_expr } ;
cmp_expr      = add_expr { ("==" | "!=" | "<" | ">" | "<=" | ">=") add_expr } ;
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

### 4.4 Why structural types (not nominal)

Structural types are chosen for LLM-coding-native reasons:
- LLM doesn't need to remember "is this `Point` or `Vec2`?" — just the shape `{x: Float, y: Float}`.
- Records with the same shape are interchangeable. Reduces import-tracking burden.
- Trade-off: no dispatch-on-name, no nominal identity. Acceptable for Phase 0-3 scope.

---

## 5. Semantic Rules (Phase 1)

### 5.1 Immutability

- `let x = 3` — `x` is immutable. Reassignment is a compile error.
- `let mut x = 3` — `x` is mutable. `x = 4` is allowed.
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
- `for x in collection` iterates. In Phase 1, `collection` must be a `String` (char iteration) or a range `a..b` (TODO: range syntax).

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

### 6.7 Effect system (drafted, Phase 2)

Effects declare side effects in the function signature:

```
fn read_file(path: String) -> Result<String, IoError> ! [IO]
fn print(s: String) -> Unit ! [IO]
fn now() -> Int ! [Clock]
```

- `! [Effect1, Effect2]` after return type declares effects.
- Pure functions (no `!`) cannot call effectful functions.
- In Phase 2, effects are a **compile-time annotation only** (no runtime effect tracking). They help LLMs (and humans) see at a glance which functions have side effects.
- Phase 5+ may introduce effect handlers (deferred).

---

## 7. Error Model (Phase 2: structured JSON diagnostics)

### 7.1 Design goal

Errors are **machine-readable first, human-readable second**. Every diagnostic is emit-able as JSON for LLM consumption.

### 7.2 JSON diagnostic format

```json
{
  "error_code": "NAM003",
  "severity": "error",
  "message": "Undefined variable: 'fooo'",
  "file": "main.lom",
  "span": { "start": [12, 4], "end": [12, 8] },
  "fix": {
    "description": "Did you mean 'foo'?",
    "suggestion": "foo",
    "start": [12, 4],
    "end": [12, 8]
  },
  "retry": true,
  "hint": "Check the spelling. The nearest defined variable is 'foo' at line 10."
}
```

Fields:
- `error_code`: stable string code (NAM= naming, TYP= type, SYN= syntax, etc.). LLMs can learn these.
- `severity`: `error` / `warning` / `info`
- `message`: human-readable
- `file`, `span`: location
- `fix`: optional, machine-actionable repair suggestion
- `retry`: whether LLM should retry generation after applying `fix`
- `hint`: optional extra context for the LLM

### 7.3 Error code namespaces (Phase 2 draft)

| Prefix | Category | Example |
|---|---|---|
| `SYN` | Syntax | `SYN001` unexpected token |
| `NAM` | Name resolution | `NAM003` undefined variable |
| `TYP` | Type mismatch | `TYP002` expected Int, got Float |
| `EFF` | Effect violation | `EFF001` pure function calls effectful |
| `MAT` | Match exhaustiveness | `MAT001` non-exhaustive match |

### 7.4 Tolerant parsing (Phase 2 core)

The parser produces a **"holey AST"** on error:
- Unparseable regions become `ErrorNode` placeholders, not parse failures.
- Type checking proceeds on the valid portions of the AST.
- Diagnostics are collected, not thrown.

This lets LLMs get partial feedback on partially-correct code, enabling iterative repair.

---

## 8. Module System (Phase 3 draft)

> Phase 1 and 2 are single-file. Modules arrive in Phase 3. Drafted here for forward-compatibility.

### 8.1 Syntax

```
from math import { sin, cos, PI }
from io import { println } as log
from utils.helpers import { format_date }
```

- **Explicit imports only**. Wildcard `import *` is forbidden.
- **No re-export**. Re-export via explicit `pub` items.
- Each `.lom` file is a module. The module name is the file path relative to the project root.

### 8.2 Public/private

```
pub fn greet(name: String) -> String
    "Hello, " + name
end

fn helper() -> Unit       # private, not importable
    ...
end
```

### 8.3 Rationale (LLM-coding-native)

- Explicit imports prevent LLMs from fabricating symbols (the #1 source of LLM code errors in docs File 1's analysis).
- No wildcard means the LLM must know exactly what it's importing — it can't rely on "maybe this exists in the namespace".

---

## 9. Standard Library (Phase 1 minimal)

Phase 1 ships a minimal prelude (auto-imported):

| Function | Type | Notes |
|---|---|---|
| `println(x)` | `Any -> Unit ! [IO]` | Print with newline |
| `print(x)` | `Any -> Unit ! [IO]` | Print without newline |
| `int_to_string(n)` | `Int -> String` | |
| `string_to_int(s)` | `String -> Result<Int, String>` | |
| `len(s)` | `String -> Int` | String length |
| `push(s, c)` | `String, Char -> String` | (Phase 1: Char is a single-char String) |

> Phase 2+ adds `Result`, `Option`, `match` to the prelude. Phase 3 adds collections, IO, HTTP.

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

```
from io import { println }
from string import { trim, split }

fn main() -> Unit
    "  hello,world  "
        |> trim
        |> split(",")
        |> println
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

---

## 11. Open Questions (to resolve before Phase 1 freeze)

1. **Range syntax**: `a..b` (Rust) vs `range(a, b)` (function) vs `a..=b` (inclusive)? Affects `for` loops.
2. **String concatenation**: `+` (overloaded) vs `++` (dedicated) vs `concat(a, b)` (function)?
   - **Resolved (v0.1.1)**: `+` is overloaded for `String + String`. Rationale: LLMs already expect `+` for string concat (Python/JS behavior), and Lom has no custom operator overloading for user types in Phase 0-3, so `+` on String is a built-in special case. `++` would be unfamiliar; `concat(a, b)` is verbose for a common operation.
3. **Char type**: separate `Char` type, or treat single-char strings as Char (Phase 1 simplicity)?
4. **Match arm separator**: `=>` (chosen) vs `->` (conflicts with closure return type)?
   - **Resolved (v0.1.1)**: `=>` confirmed. `->` is used for closure return type (`fn(x: Int) -> Int`), so `=>` for match arms avoids ambiguity.
5. **Multiple return values**: tuples only, or out-params, or destructuring assignment?
6. **`self` / `self` keyword**: lowercase `self` (Rust) vs `this` (Java) vs explicit first param name?
7. **Trait dispatch**: static (monomorphized) only, or dynamic (vtable) too?
8. **`pub` granularity**: per-item `pub` keyword, or module-level `pub use`?

These are tracked and will be resolved in Phase 0 spec iterations based on DeepSeek readability tests.

---

## 12. Changelog

- **v0.1 (2026-08-02)**: Initial draft. Phase 1 EBNF, Phase 2 features drafted, open questions listed.
- **v0.1.1 (2026-08-02)**: Patch after DeepSeek readability test (5/5 = 100% pass). Resolved 4 spec gaps:
  - Added §2.4.1 statement separation (newline-sensitive, not indentation-sensitive).
  - Expanded §6.4 match arm syntax (Form A single-expression + Form B block, mixing allowed).
  - Resolved open question #2: `+` overloaded for String concatenation.
  - Resolved open question #4: `=>` confirmed for match arms (avoids `->` ambiguity with closure return type).
  - Clarified arm separation: newlines, no semicolons.
