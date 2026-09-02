# Lom Spec for AI (v1.0)

> This is a condensed spec for LLMs. After reading this, you should be able to write valid Lom code.
> Language: **Lom** (Language of Machine). Extension: `.lom`. Host: Rust.

---

## 1. Core Rules (memorize these)

1. **Blocks end with `end`** — never braces, never indentation. `fn`/`if`/`while`/`for`/`match` all close with `end`.
2. **Default immutable** — `let x = 3` is immutable. Use `let mut x = 3` to allow reassignment.
3. **Postfix types** — `let x: Int = 3`. Types come after `:`. Inference is default; annotation is optional.
4. **Errors are values** — no `try/catch`. Use `Result<T, E>` + `match` + `?` operator.
5. **Pipeline with `|>`** — `x |> f |> g` means `g(f(x))`. Prefer linear flow over nesting.
6. **Explicit imports only** — `from math import {sin, cos}`. Never `import *`.
7. **Structural types** — records are shapes: `{x: Int, y: Int}`. No class names needed.
8. **Last expression = return value** — blocks evaluate to their last expression. Use `return` only for early exit.
9. **One statement per line** — Lom is newline-sensitive (statements separated by newlines) but NOT indentation-sensitive (blocks delimited by `end`, not by indentation level). Like Go/Swift/Kotlin, not Python. A `-` or `(` at the start of a new line is NOT a binary operator or function call — it starts a new expression.
10. **`+` works on String** — `"a" + "b"` yields `"ab"`. No special concat operator. Since v0.4.1, if either side is a String, the other side is auto-promoted (`"n = " + 42` yields `"n = 42"`) — no `int_to_string` needed for concatenation.

---

## 2. Lexical

- **Comments**: `# line` and `#- block -#`
- **Keywords**: `fn let mut if elif else while for in return match end and or True False from import as enum` — the complete reserved set (verified against the v0.6.1 lexer). Type names (`Int Float Bool String Unit Result Ok Err Some None`) are ordinary identifiers recognized in type position; `struct trait impl type pub` are NOT keywords.
- **Operators** (low → high precedence): `or`, `and`, `== != < > <= >=`, `|>`, `+ -`, `* / %`, `! -` (prefix), `?` (postfix), call/index/field
- **Literals**: `42` (Int), `3.14` (Float), `True`/`False` (Bool), `"hi"` (String), `()` (Unit)

---

## 3. Functions

### Named function
```
fn add(x: Int, y: Int) -> Int
    x + y
end
```
- Params **must** have type annotations.
- Return type optional (inferred), but **recommended** for public functions.
- Last expression is the return value. No `return` needed unless early-exiting.

### Early return
```
fn safe_divide(x: Int, y: Int) -> Int
    if y == 0
        return 0
    end
    x / y
end
```

### Closure
```
let double = fn(x: Int) -> Int
    x * 2
end
```
- Closures use `fn (...)` (no name). Named functions use `fn name(...)`.
- Closures are first-class values: can be passed as arguments, returned, stored in variables.
- **Named functions are also first-class (v0.4.2)**: `let f = double` wraps a named function as a value — pass it directly to higher-order functions like `apply_twice(double, 3)`. Recursion inside the wrapped function is unaffected.
- **Phase 1**: Use `Fn` as the type annotation for closure parameters (types are not checked at runtime):
```
let apply = fn(f: Fn, x: Int) -> Int
    f(x)
end
```

---

## 4. Control Flow

### if / elif / else (expression)
```
let grade = if score >= 90
    "A"
elif score >= 80
    "B"
else
    "F"
end
```
- Every `if` closes with `end`.
- `if` is an expression — each branch yields a value.

### while
```
let mut i = 0
while i < 5
    println(i)
    i = i + 1
end
```

### Compound assignment (v0.4.1)
`x += e` / `x -= e` / `x *= e` / `x /= e` — shorthand for `x = x + e` etc. Target must be a `let mut` variable. `+=` also works with string promotion: `s += 1` appends `"1"`.
```
let mut n = 10
n += 5       # n = 15
n *= 2       # n = 30
```

### for (iterate)
```
for c in "hello"
    println(c)
end
```
- Phase 1: iterates `String` (chars) or `Int` (range `0..n`):
```
for i in 10
    println(i)      # prints 0 through 9
end
```
- Phase 5.3 (v0.4.1): also iterates `List<T>` (element binding, in order):
```
for x in xs
    println(x)      # x is each element of xs
end
```
- Phase 5.6 (v0.4.2): range expression `a..b` — evaluates to `List<Int>`, left-inclusive right-exclusive `[a, b)`:
```
for i in 1..10
    println(i)      # prints 1 through 9
end
let xs = 1..4        # xs is List<Int> [1, 2, 3]
```

---

## 5. Types

### Basic types
`Int` `Float` `Bool` `String` `Unit`

### Inference
```
let x = 42           # Int
let y = 3.14         # Float
let z = True         # Bool
let s = "hi"         # String
let n = ()           # Unit
let annotated: Int = 42
```

### Structural records (Phase 2)
```
let p = {x: 3, y: 4}        # type: {x: Int, y: Int}
let q = {x: 3.0, y: 4.0}    # type: {x: Float, y: Float}
println(p.x)                # field access: 3
```
- Two records with the same fields have the **same type** (structural).
- Field mutation requires `let mut`:
```
let mut p = {x: 3, y: 4}
p.x = 5
```

### Tuples (Phase 2)
```
let pair: (Int, String) = (1, "hello")
let (n, s) = pair           # destructuring
```

### Type alias (Phase 2)
```
type UserId = Int
type Point = {x: Float, y: Float}
```

### Gradual type checking (Phase 2.4 — implemented)

Type annotations are **optional**. Type errors are **non-fatal warnings** — the program still runs.

- `lom <file>` — runs the program. Type checking also runs (since v0.6.0): diagnostics go to stderr and **never block execution**.
- `lom <file> --check` — type check only (no execution), prints human-readable diagnostics. Exit 1 only on Error-level; warnings exit 0.
- `lom <file> --json` — emits `lom-diag/v1` JSON including `stage: "type"` diagnostics.
- `lom <file> --dump-ast` — prints the AST as a deterministic indentation tree (no execution, no type check; spans excluded). Debug/verification tool — Phase 8.1's verbatim-diff baseline.

Type-error codes (all `Warning` unless noted): `TYPE001` (mismatch), `TYPE002` (cond not Bool), `TYPE003` (arg count/type), `TYPE010` (return mismatch), `TYPE020` (`?` misuse), `MAT001` (match non-exhaustive), `MUT001` (reassigning an immutable binding — `let` without `mut`, a function parameter, a `for` loop variable, or a `match` binding; fix: declare with `let mut`, or introduce a local `let mut` copy for params/loop vars). Name-resolution: `NAM002` (Error, duplicate), `NAM003` (Error, undefined), `NAM004` (Error, no such field/variant).

When you write Lom: annotate function params and return types — the checker will flag mismatches in `--check`/`--json`, helping you fix errors before running. Missing annotations are fine (inferred as `Unknown`, no error).

### Explicit effect system (Phase 2.5 — implemented)

Declare side effects in the function signature with `! [Effect1, Effect2]` after the return type:

```
fn read_file(path: String) -> Result<String, String> ! [IO]
fn now() -> Int ! [Clock]
fn log(msg: String) -> Unit ! [IO, Clock]
```

Rules (LLMs must follow these):
1. **Pure functions (no `!`)** cannot call functions declared with effects — `EFF001` warning in `--check`/`--json`.
2. **`main` is exempt** — it implicitly has all effects. Never write `! [...]` on `main`.
3. **`! []` (empty list)** is the same as no annotation (pure).
4. **Closures** don't carry effects — they inherit the enclosing function's effect set.
5. **Effects are compile-time only** — the program still runs even with `EFF001` (gradual: warnings don't block).
6. **Standard library effects**: `println`/`print` are `[IO]`; `len`/`trim`/`upper`/`sqrt`/etc. are pure.

When you write a function that calls `println`/`print`, declare `! [IO]`:
```
fn greet(name: String) -> Unit ! [IO]
    println("Hello, " + name)
end
```

If you forget, `--check` reports: `[EFF001] 纯函数或未声明效应 [] 的函数调用了带效应 [IO] 的函数 'println'`. Fix by adding `! [IO]` to the function signature.

---

## 6. Errors: Result, Option, ?, match

### Result type
```
enum Result<T, E> = Ok(T) | Err(E)
enum Option<T> = Some(T) | None
```

### `?` operator (propagate error)
```
fn read_config(path: String) -> Result<Config, String>
    let content = read_file(path)?      # if Err, return Err immediately
    let parsed = parse(content)?        # if Err, return Err immediately
    Ok(parsed)
end
```
- `?` on `Ok(v)` yields `v`. On `Err(e)`, returns `Err(e)` from enclosing function.
- `?` on `Some(v)` yields `v`. On `None`, returns `None`.
- Enclosing function must return compatible `Result` or `Option`.

### `match` (exhaustive)
```
match parse_and_double(s)
    Ok(n) => println(n)
    Err(e) => println("Error: " + e)
end
```
- `match` **must** be exhaustive. Missing cases = compile error.
- Patterns: literals (`0`, `"hi"`), binders (`x`), wildcard (`_`), variants (`Ok(v)`, `None`), tuples `(a, b)`, records `{x, y}`.

**Two arm forms** — both valid:

Form A (single-expression, one line per arm):
```
match n
    0 => "zero"
    1 => "one"
    _ => "many"
end
```

Form B (block, multi-statement, each arm closed with `end`):
```
match result
    Ok(name) =>
        let msg = greet(name)
        println(msg)
    end
    Err(e) =>
        println("Failed: " + e)
    end
end
```
- Arms are separated by newlines. No semicolons.
- You can mix Form A and Form B in the same match.

**Guards (v0.4.2)** — `pattern if cond => body`: the arm wins only when the pattern matches AND `cond` (Bool) is true; otherwise the next arm is tried. Guards can use variables bound by the pattern. A guarded arm does NOT count toward exhaustiveness (its condition is only known at runtime), so add a `_` fallback:
```
match n
    m if m < 0 => "negative"
    0 => "zero"
    m if m > 100 => "big"
    _ => "normal"
end
```

---

## 7. Pipeline `|>`

`|>` passes the left value as the **first argument** of the right function. Left-associative.

```
x |> f          # means f(x)
x |> f(y)       # means f(x, y)   — x prepended to the arg list
x |> f |> g     # means g(f(x))   — chains
```

Example:
```
fn double(x: Int) -> Int
    x * 2
end

fn add(x: Int, y: Int) -> Int
    x + y
end

fn main() -> Unit
    5 |> double |> println        # prints 10
    10 |> add(3) |> println       # prints 13  (add(10, 3))
    5 |> double == 10             # True  — `|>` binds tighter than `==`
    1 + 2 |> double               # 6     — `+` binds tighter than `|>` => double(1+2)
end
```

- Pipeline is an expression — no `end` needed. The `end` above closes `fn main`.
- Precedence: higher than comparison (`==` `<` etc.), lower than arithmetic (`+` `-`). So `a + b |> f == c` parses as `((a + b) |> f) == c`.
- The right side can be a named function, a closure value, or a call with extra args.

---

## 8. Modules (stdlib imports; user packages since Phase 4.4)

```
from math import { sqrt, abs, min, max }
from string import { len, upper, trim, int_to_string }
from io import { println as log }    # per-item alias: name as alias
```
- **Explicit imports only**. No wildcards (`import *` forbidden).
- **Per-item alias**: `name as alias` (Python/Rust-style).
- **Prelude** (auto-imported, no `from` needed): `println`, `print`.
- **Non-prelude builtins must be imported** or you get an error:
  - `len`, `int_to_string`, `string_to_int`, `trim`, `upper`, `lower`, `split`, `contains`, `replace`, `starts_with`, `ends_with`, `char_from_code` → `from string import {...}`
  - `sqrt`, `abs`, `min`, `max` → `from math import {...}`
- Standard library modules: `io`, `string`, `math`, `list`, `json`, `map` (v0.5.1), `file`, `env`.
- **User packages** (Phase 4.4): declare local path dependencies in `lom.toml` (`[dependencies]` + `mathlib = { path = "mathlib" }`), then `from mathlib import { square }`; `lom build` resolves the dependency graph (with cycle detection) and type-checks package sources. Local paths only — no registry yet. Example: `examples/pkg_demo/`.
- `pub` marks exportable items (**rejected for v1.0** — RFC-0001; `pub` is an ordinary identifier, all top-level items are public, no privacy is planned):
```
pub fn greet(name: String) -> String
    "Hello, " + name
end

fn helper() -> Unit    # private
    ...
end
```

---

## 9. Effects (Phase 2 annotation)

```
fn read_file(path: String) -> Result<String, IoError> ! [IO]
fn print(s: String) -> Unit ! [IO]
fn now() -> Int ! [Clock]
```
- `! [Effect1, Effect2]` declares side effects.
- Pure functions (no `!`) cannot call effectful functions.

---

## 10. Common Mistakes to Avoid

1. **Using braces** — Lom uses `end`, not `{}`.
   - ❌ `fn f() { ... }`
   - ✅ `fn f() ... end`
2. **Indentation blocks** — Lom is not indentation-sensitive.
3. **Missing `end`** — every `fn`/`if`/`while`/`for`/`match` must close with `end`.
4. **Reassigning immutable** — `let x = 3; x = 4` is an error. Use `let mut`.
5. **try/catch** — Lom uses `Result` + `?`. No exceptions.
6. **Wildcard import** — `import math.*` is forbidden. Use `from math import {sqrt, abs}`.
7. **Nominal classes** — Lom uses structural records `{x: Int, y: Int}`, not `class Point`.
8. **Forgetting `?`** — `let x = read_file(path)` gives `Result<String, _>`, not `String`. Add `?` to unwrap.
9. **Non-exhaustive match** — `match` must cover all cases or have `_`.
10. **`return` as last statement** — discouraged. Use the last expression directly.
11. **Using non-prelude builtin without import** — `len`, `int_to_string`, `sqrt`, etc. must be imported. `println`/`print` are prelude (auto-available).
    - ❌ `fn main() -> Unit \n    println(len("hi")) \nend` — error: `len` not imported
    - ✅ `from string import { len }\nfn main() -> Unit \n    println(len("hi")) \nend`

---

## 11. Full Example

```
from string import { trim, len }

# A named function with type annotations
fn greet(name: String) -> String
    "Hello, " + name
end

# Result-returning function with ? propagation
fn parse_name(s: String) -> Result<String, String>
    let cleaned = s |> trim
    if len(cleaned) == 0
        Err("empty name")
    else
        Ok(cleaned)
    end
end

# Pattern matching on Result
fn main() -> Unit
    match parse_name("  Alice  ")
        Ok(name) =>
            let message = greet(name)
            println(message)
        end
        Err(e) =>
            println("Failed: " + e)
        end
    end
end
```

---

## 11b. Built-in Functions (Phase 2.1.5)

**Prelude** (auto-imported, no `from` needed):

| Function | Signature | Description |
|---|---|---|
| `println(x)` | `Any -> Unit` | Print value with newline |
| `print(x)` | `Any -> Unit` | Print value without newline |

**`string` module** (requires `from string import {...}`):

| Function | Signature | Description |
|---|---|---|
| `len(s)` | `String -> Int` | String length (UTF-8 char count) |
| `int_to_string(n)` | `Int -> String` | Convert integer to string |
| `string_to_int(s)` | `String -> Int \| Unit` | Parse string to integer (returns `Unit` on failure) |
| `trim(s)` | `String -> String` | Strip leading/trailing whitespace |
| `upper(s)` | `String -> String` | Uppercase |
| `lower(s)` | `String -> String` | Lowercase |
| `split(s, sep)` | `(String, String) -> List<String>` | Split; empty separator splits into characters |
| `contains(s, sub)` | `(String, String) -> Bool` | Substring test |
| `replace(s, from, to)` | `(String, String, String) -> String` | Replace all occurrences |
| `starts_with(s, prefix)` | `(String, String) -> Bool` | Prefix test |
| `ends_with(s, suffix)` | `(String, String) -> Bool` | Suffix test |
| `char_from_code(cp)` | `Int -> String` | Unicode code point → single-char String (v0.27.0; invalid code points are runtime errors) |

**`math` module** (requires `from math import {...}`):

| Function | Signature | Description |
|---|---|---|
| `sqrt(x)` | `Float -> Float` (also `Int`) | Square root |
| `abs(x)` | `Int -> Int` \| `Float -> Float` | Absolute value |
| `min(a, b)` | `(Int,Int)->Int` \| `(Float,Float)->Float` | Minimum |
| `max(a, b)` | `(Int,Int)->Int` \| `(Float,Float)->Float` | Maximum |

**`io` module** (same as prelude; explicit import only needed for aliasing): `println`, `print`.

**`list` module** (requires `from list import {...}`; Lists are immutable — every function returns a new List):

| Function | Signature | Description |
|---|---|---|
| `list_empty()` | `() -> List<T>` | Empty list |
| `list_length(xs)` | `List<T> -> Int` | Element count |
| `list_get(xs, i)` | `(List<T>, Int) -> T` | Element at index (0-based) |
| `list_is_empty(xs)` | `List<T> -> Bool` | Empty check |
| `list_head(xs)` | `List<T> -> T` | First element |
| `list_tail(xs)` | `List<T> -> List<T>` | All but first |
| `list_cons(x, xs)` | `(T, List<T>) -> List<T>` | Prepend: `[x, ...xs]` |
| `list_map(f, xs)` (v0.4.3) | `(Fn, List<T>) -> List<U>` | Apply `f` to each element |
| `list_filter(f, xs)` (v0.4.3) | `(Fn, List<T>) -> List<T>` | Keep elements where `f(x)` is `True` |
| `list_fold(f, init, xs)` (v0.4.3) | `(Fn, U, List<T>) -> U` | Left fold: `acc = f(acc, x)` |

`f` can be a closure literal or a named function (v0.4.2+): `list_map(double, 1..6)` → `[2, 4, 6, 8, 10]`.

**`map` module** (requires `from map import {...}`; v0.5.1; string-keyed dictionary with **reference semantics** — `map_set` mutates in place, `let` aliases share the same Map):

| Function | Signature | Description |
|---|---|---|
| `map_empty()` | `() -> Map<T>` | Empty map |
| `map_set(m, k, v)` | `(Map<T>, String, T) -> Unit` | Insert/overwrite key in place |
| `map_get(m, k)` | `(Map<T>, String) -> Option<T>` | `Some(v)` or `None` |
| `map_has(m, k)` | `(Map<T>, String) -> Bool` | Key existence |
| `map_remove(m, k)` | `(Map<T>, String) -> Bool` | Remove key; `True` if it existed |
| `map_keys(m)` | `Map<T> -> List<String>` | All keys, **sorted** (deterministic) |
| `map_values(m)` | `Map<T> -> List<T>` | Values in the same sorted-key order |
| `map_size(m)` | `Map<T> -> Int` | Entry count |

---

## 11c. Error Diagnostics (Phase 2.3)

Lom emits **all** errors at once (tolerant parser — does not stop at first error). Two output modes:

**Human-readable** (`lom file.lom --check` or default run mode on error):
```
[lex] error (3:13): [LEX001] 未闭合的字符串
    |     let s = "hello
    |             ^
    hint: 在字符串末尾添加 " 闭合
[parse] error (7:5): [PARSE001] 期望 '('，得到 Let
    |     let x = 1
    |     ^
    hint: 检查语法结构是否完整，关键字/分隔符是否匹配
共 2 个诊断（2 错误，0 警告，0 代码洞）。
```

**JSON** (`lom file.lom --json`) — `lom-diag/v1` schema, for LLM consumption:
```json
{
  "schema": "lom-diag/v1",
  "file": "file.lom",
  "ok": false,
  "summary": { "total": 2, "errors": 2, "warnings": 0, "holes": 0 },
  "diagnostics": [
    {
      "severity": "error", "stage": "lex", "code": "LEX001",
      "message": "未闭合的字符串",
      "file": "file.lom", "line": 3, "col": 13,
      "source_line": "    let s = \"hello",
      "is_hole": false,
      "hint": "在字符串末尾添加 \" 闭合"
    }
  ]
}
```

**Error codes** (LLMs can learn these):
- `LEX001`-`LEX099`: lexical (unclosed string, unexpected char, etc.)
- `PARSE001`-`PARSE099`: syntax (`PARSE001` expected token, `PARSE099` = hole)
- `RUNTIME001`-`RUNTIME099`: runtime (`RUNTIME001` type mismatch, `RUNTIME002` undefined, `RUNTIME003` hole execution)
- `TYPE001`-`TYPE099`: type errors (Phase 2.4 — `TYPE001` mismatch, `TYPE002` cond not Bool, `TYPE003` arg count/type, `TYPE010` return mismatch, `TYPE020` `?` misuse)
- `MAT001`-`MAT099`: match exhaustiveness (Phase 2.4 — `MAT001` non-exhaustive)
- `NAM001`-`NAM099`: name resolution (Phase 2.4 — `NAM002` duplicate, `NAM003` undefined, `NAM004` no such field/variant)
- `EFF001`-`EFF099`: effect errors (Phase 2.5 — `EFF001` pure function calls effectful)
- `MUT001`-`MUT099`: mutability (v0.20.0 — `MUT001` reassigning an immutable binding: `let` without `mut`, function parameter, `for` loop variable, or `match` binding)

**Tolerant parsing & holes**: when the parser cannot parse a statement, it inserts a `Stmt::Hole` placeholder and continues. The hole is reported as `PARSE099` / `RUNTIME003` (if executed). This means LLMs get **all** errors in one round, not just the first — repair them all at once.

---

## 11d. Type Info Export (Phase 2.6)

`lom info <file> [--json]` exports **declarations** (function signatures, enums, imports) so you can quickly learn what a file defines — **without** running it, **without** type-checking it. Use this before writing code that calls into an existing file.

- `lom info <file>` — human-readable summary to stdout.
- `lom info <file> --json` — `lom-info/v1` JSON to stdout (for LLM consumption).
- If the file does not parse, `lom info` emits `lom-diag/v1` (see §11c) with parse errors and exits 1. **It does not emit `lom-info/v1` on parse failure.**

**`lom-info/v1` schema**:
```json
{
  "schema": "lom-info/v1",
  "file": "main.lom",
  "ok": true,
  "functions": [
    {
      "name": "double", "params": [{"name":"x","type":"Int"}],
      "ret_type": "Int", "effects": [], "is_main": false
    },
    {
      "name": "print_double", "params": [{"name":"x","type":"Int"}],
      "ret_type": "Unit", "effects": ["IO"], "is_main": false
    },
    {
      "name": "main", "params": [],
      "ret_type": "Unit", "effects": [], "is_main": true
    }
  ],
  "enums": [
    { "name": "Color", "type_params": [],
      "variants": [
        {"name":"Red","fields":[]},
        {"name":"Green","fields":[]}
      ]
    }
  ],
  "imports": [
    { "module": "string",
      "items": [{"name":"len","alias":"len"}] }
  ]
}
```

Key points:
- **`ok`** is `true` iff the file parsed successfully. On parse failure, `lom info` does not produce `lom-info/v1` — it produces `lom-diag/v1` instead.
- **`ret_type`** is `null` when the function omits the return annotation.
- **`effects`** is an empty array `[]` for pure functions (no `! [...]`).
- **`is_main`** lets you locate the entry point without scanning names.
- **Type strings**: `Int`, `Float`, `Bool`, `String`, `Unit`, `Result<T, E>`, `Option<T>`, `Name<A, B>`, `{ x: Int, y: Int }` (record, with spaces inside braces), `(Int, String)` (tuple).
- **`info` does not type-check.** For type errors, use `lom --check` or `lom --json` (§5 gradual type checking).

---

## 11e. AI Repair Plan (Phase 2.7)

`lom fix <file> [--plan] [--json]` generates a **repair plan** for every diagnostic in the file. Use this **after** `lom --json`/`--check` reports errors — feed the plan to yourself (the LLM) and apply fixes.

- `lom fix <file>` — human-readable plan to stdout.
- `lom fix <file> --json` — `lom-fix/v1` JSON to stdout (for LLM consumption).
- `lom fix <file> --plan` — explicit flag; `--plan` is the default.
- `lom fix <file> --apply [--dry-run] [--json]` (Phase 3.1) — applies `confidence=high` + non-`hint` fixes to the source file in place; `--dry-run` previews without writing; output follows the `lom-apply/v1` schema. Since v0.17.0 (M2), `--apply` **iterates**: re-diagnose → apply → repeat until no high-confidence fixes remain or the source stops changing (max 5 rounds, anti-oscillation brake); JSON output gains `rounds` and per-change `round` fields, and each applied round is recorded as a separate entry (with `round`) in fix history.
- `lom fix --history [--json]` (Phase 4.1.3) — shows past applied fixes from `.lom/fix-history.jsonl` (NDJSON, `lom-fix-history/v1` schema).
- Exit code is `0` whenever the plan was generated successfully — **even if the file has errors.** This lets you consume the JSON without parsing exit codes.

**What `fix` (plan mode) does NOT do**: it does not edit the source file, does not run the program, does not re-check after applying fixes. Only `fix --apply` edits the source — and only for high-confidence, non-hint fixes; everything else remains advisory.

**`lom-fix/v1` schema**:
```json
{
  "schema": "lom-fix/v1",
  "file": "main.lom",
  "ok": false,
  "summary": { "total": 2, "applicable": 2, "skipped": 0 },
  "plans": [
    {
      "diagnostic": {
        "code": "LEX001", "severity": "error", "stage": "lex",
        "line": 2, "col": 13, "message": "未闭合的字符串"
      },
      "fixes": [
        {
          "description": "在字符串末尾添加 \" 闭合",
          "action": "insert",
          "line": 2, "col": 19,
          "end_line": null, "end_col": null,
          "text": "\"",
          "confidence": "high"
        }
      ],
      "retry": true
    }
  ]
}
```

**Field semantics**:
- **`ok`**: `true` iff no diagnostics were found (plans is empty).
- **`summary.applicable`**: count of plans with at least one non-hint fix, or a hint that carries concrete `text` (e.g. EFF001's `! [IO]` snippet).
- **`plans[].diagnostic`**: embedded copy of the diagnostic — you don't need to cross-reference `lom-diag/v1`.
- **`plans[].fixes[].action`**:
  - `insert` — insert `text` at `(line, col)`.
  - `replace` — replace range `(line,col)..(end_line,end_col)` with `text`. Emitted by NAM003/NAM004 spelling fixes (medium confidence — never auto-applied).
  - `delete` — delete the range. Used by LEX005 (1-char delete).
  - `hint` — guidance text only; `line`/`col` may be `0`. May still carry `text` (a snippet to use, e.g. `! [IO]`, `Green => ()`).
- **`plans[].fixes[].confidence`**: `high` / `medium` / `low`. When multiple fixes exist for one diagnostic, they are listed in order but not ranked — you choose.
- **`plans[].retry`**: `true` if at least one fix provides an applicable repair (non-hint action, or hint with concrete `text`). When `false`, the hints are advisory and may not directly resolve the error.

**Fix strategies by error code** (most useful ones):

| Code | What `fix` suggests | Action | Confidence |
|---|---|---|---|
| `LEX001`/`LEX002` | Insert `"` at end of error line | `insert` | high |
| `LEX005` | Delete the unexpected char | `delete` | high |
| `PARSE002` | `Result<T, E>` needs 2 type params | `hint` | medium |
| `PARSE003` | `Option<T>` needs 1 type param | `hint` | medium |
| `MAT001` | Missing branch text (e.g. `Green => ()`) | `hint` with `text` | medium |
| `EFF001` | Effect annotation snippet (e.g. `! [IO]`) | `hint` with `text` | high |
| `TYPE002` | Condition must be `Bool` | `hint` | medium |
| `TYPE020` | `?` misuse (operand not Result/Option, or return incompatible) | `hint` | medium |
| `NAM003` | Undefined variable — check spelling / import | `hint` | low |
| Other | Code-specific guidance | `hint` | low/medium |

**Recommended workflow** when you (the LLM) are writing Lom code:
1. Write the file.
2. Run `lom <file> --json` to get diagnostics.
3. If `ok == false`, run `lom fix <file> --json` to get the repair plan.
4. Apply the fixes — either yourself (`fix` plan mode does not auto-edit) or via `lom fix <file> --apply` (applies high-confidence non-hint fixes in place). For `hint`-with-`text` fixes like EFF001/MAT001, the `text` is a ready-to-paste snippet.
5. Re-run `lom <file> --json` to verify. Repeat until `ok == true`.
6. Run `lom <file>` to execute.

**Limitations**:
- NAM003/NAM004 spelling fixes emit `replace` actions at **medium** confidence (`--apply` never touches them — guessed repairs require human/LLM confirmation). Since Phase 3.2b (v0.21.0) positions come from **expression-level spans** on the diagnostic (single-point replace); the whole-token source scan remains only as a fallback for span-less diagnostics (e.g. variant names in `match` patterns — `Pattern` carries no span).
- No cross-file fixes: a missing import in file B is not auto-added to file A.
- Runtime errors (`RUNTIME001`-`RUNTIME005`) only get `hint`-level guidance. Positions: declaration-level spans since Phase 3.2 (EFF001/TYPE010/NAM002 point at the `fn` signature); **expression-level spans since Phase 3.2b / v0.21.0** (NAM003/MUT001/TYPE001-3/TYPE020/NAM004-field point at the exact expression/assign-target; note the Field diagnostic points at the field-name token via the span's `end`); runtime positions remain coarse. Caveat: `line`/`col` follow the lexer convention — **1-based byte columns** (pure-ASCII lines coincide with char columns; `lom fix` converts internally).

---

## 11f. Compiling to WebAssembly (Phase 7, v0.15.0)

`lom build <file> --target wasm [-o out.wasm]` compiles a Lom program to a `.wasm` binary (hand-written zero-dependency emitter). The tree-walking interpreter remains the reference implementation and the default run path; WASM is a second backend compiling the same dynamic semantics — stdout is byte-identical across the full example suite, the bootstrap self-hosted interpreter, and all 108 eval tasks.

- Type checking runs before compilation (diagnostics on stderr, never blocking — the same gradual-typing promise as the interpreter).
- Running the `.wasm` requires a host providing the `env.lom_*` imports (print / file / env / json); the repo ships a Node.js harness at `eval/runner/run_wasm.mjs`.
- Known divergences (documented; do not rely on either side's behavior): closure capture is value-copy at creation in WASM vs shared-scope in the interpreter; JSON numbers are split Int/Float by the JS host value, not by source syntax.
- Recursion depth is bounded by the host stack (~10k–30k frames under Node's default stack; `node --stack-size=60000` reaches 10⁵).

---

## 12. Quick Reference

| Want to... | Write this |
|---|---|
| Declare immutable | `let x = 3` |
| Declare mutable | `let mut x = 3` |
| Annotate type | `let x: Int = 3` |
| Define function | `fn f(x: Int) -> Int ... end` |
| Define closure | `let f = fn(x: Int) -> Int ... end` |
| Early return | `return value` |
| If/else | `if cond ... elif cond ... else ... end` |
| While loop | `while cond ... end` |
| For loop | `for x in collection ... end` |
| Propagate error | `expr?` |
| Match | `match expr ... end` |
| Pipeline | `x \|> f \|> g` |
| Record | `{x: 3, y: 4}` |
| Tuple | `(1, "hi")` |
| Ok value | `Ok(v)` |
| Err value | `Err(e)` |
| Import | `from mod import {name1, name2 as alias}` |
| Comment | `# line` |

---

*End of Lom Spec for AI v1.0. Phase 2.1 implements: everything in Phase 1 plus `match` (Form A single-expr + Form B block arms), `enum` declarations (single-line `enum Name = V1 | V2` and multi-line `enum Name\n V1\n V2\n end`), built-in variants `Ok(v)`/`Err(e)`/`Some(v)`/`None`, `Result<T, E>` and `Option<T>` type annotations, pattern matching (literals, binders, `_` wildcard, variant patterns `Ok(n)`/`None`), `|>` pipeline (left value as first arg of right function), `?` error propagation (Result/Option), structural records `{x: Int, y: Int}`, tuples `(Int, String)` with `.0`/`.1` indexing, explicit imports `from mod import {name as alias}` (stdlib io/string/math modules; prelude `println`/`print` auto-available). Phase 2.2 adds: tolerant parser with holey AST (`Stmt::Hole` on parse error, all errors collected, sync-point recovery). Phase 2.3 adds: structured JSON diagnostics (`lom-diag/v1` schema), `--json` / `--check` / `--help` CLI flags, error code namespaces (LEX/PARSE/RUNTIME implemented; TYPE/EFF/MAT/NAM reserved for 2.4-2.5). Phase 2.4 adds: gradual type checker (`--check` / `--json` emits TYPE/MAT/NAM diagnostics; warnings are non-fatal — the program still runs). Phase 2.5 adds: explicit effect system (`! [IO, Clock]` annotation, `EFF001` warning when a pure function calls an effectful one; `main` is exempt). Phase 2.6 adds: `lom info <file> [--json]` type info export (`lom-info/v1` schema — functions/enums/imports; no type-check, no run; parse failure falls back to `lom-diag/v1`). Since then: MAT001 non-exhaustive-match compile warnings, the `fix`/`retry` diagnostic fields with `lom fix --plan` / `--apply` / `--history` (Phases 2.7 / 3.1 / 4.1.3), the 114-task eval suite (`eval/`), the `list` / `json` / `map` / `file` / `env` stdlib modules, user packages via `lom.toml` path dependencies (Phase 4.4), `lom doc` / `lom fmt` / `lom repl` / `lom lsp`, and type checking visible on the default run path (diagnostics on stderr, never blocking execution). Phase 7 (v0.7.0–v0.15.0) adds the WASM compiler backend (`lom build --target wasm`, §11f) — the interpreter remains the reference implementation and default run path. Phase 8 (v0.23.0–v0.26.0) delivers full self-hosting: a complete Lom frontend, checker subset, and tree-walking interpreter written in Lom (`examples/selfhost/self_interp.lom`, ~5700 lines — the largest LLM-readable Lom codebase in existence and the best reference for real-world Lom style). v0.27.0 adds `string.char_from_code` (§4). **v1.0 (2026-09-02): the language surface is frozen** — syntax / 20 reserved words / diagnostic codes / 43 builtins; nothing in this spec will change within v1.x without a new RFC. When unsure, prefer the explicit form (annotate types, handle all match cases with `_`).*
