# Lom Spec for AI (v0.1)

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
9. **One statement per line** — Lom is newline-sensitive (statements separated by newlines) but NOT indentation-sensitive (blocks delimited by `end`, not by indentation level). Like Go/Swift/Kotlin, not Python.
10. **`+` works on String** — `"a" + "b"` yields `"ab"`. No special concat operator.

---

## 2. Lexical

- **Comments**: `# line` and `#- block -#`
- **Keywords**: `fn let mut if elif else while for in return match end True False Int Float Bool String Unit Result Ok Err from import as struct enum trait impl type`
- **Operators** (low → high precedence): `|>`, `=`, `or and`, `== != < > <= >=`, `+ -`, `* / %`, `! -` (prefix), `?` (postfix), call/index/field
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

### for (iterate)
```
for c in "hello"
    println(c)
end
```
- Phase 1: iterates `String` (chars) or ranges (syntax TBD).

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

---

## 7. Pipeline `|>`

```
from io import { println }
from string import { trim, upper }

"  hello  "
    |> trim        # "hello"
    |> upper       # "HELLO"
    |> println     # prints HELLO
end               # wait — no, pipeline doesn't need end. See correction below.
```

**Correction**: pipeline is an expression, no `end` needed:
```
fn main() -> Unit
    "  hello  "
        |> trim
        |> upper
        |> println
end
```
The `end` closes `fn main`, not the pipeline.

---

## 8. Modules (Phase 3)

```
from math import { sin, cos, PI }
from io import { println } as log
```
- Explicit imports only. No wildcards.
- `pub` marks exportable items:
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
6. **Wildcard import** — `import math.*` is forbidden. Use `from math import {sin, cos}`.
7. **Nominal classes** — Lom uses structural records `{x: Int, y: Int}`, not `class Point`.
8. **Forgetting `?`** — `let x = read_file(path)` gives `Result<String, _>`, not `String`. Add `?` to unwrap.
9. **Non-exhaustive match** — `match` must cover all cases or have `_`.
10. **`return` as last statement** — discouraged. Use the last expression directly.

---

## 11. Full Example

```
from io import { println }
from string import { trim }

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
| Import | `from mod import {name1, name2}` |
| Comment | `# line` |

---

*End of Lom Spec for AI v0.1. Generate Lom code using only the constructs above. When unsure, prefer the explicit form (annotate types, handle all match cases, use `?` for error propagation).*
