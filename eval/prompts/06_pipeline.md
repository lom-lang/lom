# 任务：生成 Lom 代码

你是一名 Lom 语言程序员。请根据下方任务描述，为每个任务生成一份完整的 `.lom` 源文件。

## Lom 语言速览

Lom (Language of Machine) 是一门 AI 原生编程语言。核心规则：

1. **块以 `end` 闭合** — 不用花括号，不靠缩进。`fn`/`if`/`elif`/`else`/`while`/`for`/`match` 都用 `end` 闭合。
2. **默认不可变** — `let x = 3` 不可变；`let mut x = 3` 可重新赋值。
3. **后置类型** — `let x: Int = 3`。类型注解可选（有推断），但函数参数必须标注类型。
4. **错误是值** — 无 try/catch。用 `Result<T, E>` + `match` + `?` 运算符。
5. **管道 `|>`** — `x |> f` 等价 `f(x)`；`x |> f(y)` 等价 `f(x, y)`；`x |> f |> g` 等价 `g(f(x))`。
6. **显式导入** — `from math import { sqrt, abs }`。禁止通配符 `import *`。
7. **结构类型** — 记录是形状 `{x: Int, y: Int}`，无类名。字段相同即同类型。
8. **尾表达式即返回值** — 块的最后一个表达式是返回值。`return` 仅用于提前退出。
9. **每行一条语句** — Lom 换行敏感（语句用换行分隔），但非缩进敏感（块用 `end` 分隔，非缩进）。类似 Go/Swift，不是 Python。
10. **`+` 可拼接字符串** — `"a" + "b"` 得 `"ab"`。

### 函数

```
fn add(x: Int, y: Int) -> Int
    x + y
end
```
- 参数必须标注类型。返回类型可选（推荐标注）。
- 尾表达式即返回值，无需 `return`。

闭包：
```
let double = fn(x: Int) -> Int
    x * 2
end
```
闭包是一等公民，可传参、返回、存变量。闭包参数类型用 `Fn`：
```
let apply = fn(f: Fn, x: Int) -> Int
    f(x)
end
```

### 控制流

```
let grade = if score >= 90
    "A"
elif score >= 80
    "B"
else
    "F"
end
```
每个 `if` 用 `end` 闭合。`if` 是表达式，每个分支产生值。

```
let mut i = 0
while i < 5
    println(i)
    i = i + 1
end
```

```
for c in "hello"
    println(c)
end

for i in 10
    println(i)      # 打印 0 到 9
end
```

### 类型

基本类型：`Int` `Float` `Bool` `String` `Unit`

```
let x = 42           # Int
let y = 3.14         # Float
let z = True         # Bool
let s = "hi"         # String
let n = ()           # Unit
```

结构记录：
```
let p = {x: 3, y: 4}
println(p.x)                # 字段访问: 3
```

元组：
```
let pair: (Int, String) = (1, "hello")
println(pair.0)             # 1
println(pair.1)             # hello
```

渐进式类型检查：类型注解可选，类型错误是警告（不阻止运行）。

### 显式效应系统

在函数返回类型后用 `! [Effect1, Effect2]` 声明副作用：
```
fn greet(name: String) -> Unit ! [IO]
    println("Hello, " + name)
end

fn now() -> Int ! [Clock]
    1234567890
end
```
规则：
- 纯函数（无 `!`）不能调用带效应的函数 — 会报 `EFF001` 警告。
- `main` 隐式拥有所有效应，**不要**在 `main` 上写 `! [...]`。
- `println`/`print` 是 `[IO]`；`len`/`trim`/`upper`/`sqrt` 等是纯函数。
- 效应仅编译期注解，不影响运行时（渐进式：警告不阻止运行）。

### 错误处理：Result, Option, ?, match

```
enum Result<T, E> = Ok(T) | Err(E)
enum Option<T> = Some(T) | None
```

`?` 运算符传播错误：
```
fn read_config(path: String) -> Result<String, String>
    let content = read_file(path)?      # 若 Err，立即返回 Err
    Ok(content)
end
```
- `?` 作用于 `Ok(v)` 得 `v`；作用于 `Err(e)` 则从外层函数返回 `Err(e)`。
- `?` 作用于 `Some(v)` 得 `v`；作用于 `None` 则返回 `None`。

`match` 必须穷尽：
```
match result
    Ok(n) => println(n)
    Err(e) => println("Error: " + e)
end
```
两种臂形式（可混用）：
```
# Form A: 单表达式，每臂一行
match n
    0 => "zero"
    _ => "many"
end

# Form B: 块，每臂用 end 闭合
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
模式：字面量（`0`/`"hi"`）、绑定（`x`）、通配符（`_`）、变体（`Ok(v)`/`None`）、元组 `(a, b)`。

### 模块导入

```
from math import { sqrt, abs, min, max }
from string import { len, upper, trim, int_to_string }
from io import { println as log }    # 别名: name as alias
```
- **Prelude（自动可用，无需导入）**：`println`, `print`。
- **非 prelude 内置函数必须导入**，否则报错：
  - `len`, `int_to_string`, `string_to_int`, `trim`, `upper`, `lower` → `from string import {...}`
  - `sqrt`, `abs`, `min`, `max` → `from math import {...}`
- 禁止通配符 `import *`。

### 标准库

| 模块 | 函数 | 签名 |
|---|---|---|
| prelude | `println(x)` / `print(x)` | `Any -> Unit` |
| string | `len(s)` | `String -> Int` |
| string | `int_to_string(n)` | `Int -> String` |
| string | `string_to_int(s)` | `String -> Int` 或 `Unit`（失败返回 `()`） |
| string | `trim(s)` / `upper(s)` / `lower(s)` | `String -> String` |
| math | `sqrt(x)` | `Float -> Float`（也接受 `Int`） |
| math | `abs(x)` | `Int -> Int` 或 `Float -> Float` |
| math | `min(a, b)` / `max(a, b)` | 同类型二选一 |

### 常见错误（务必避免）

1. 用花括号 `{}` — Lom 用 `end`，不用花括号。
2. 缺 `end` — 每个 `fn`/`if`/`while`/`for`/`match` 必须用 `end` 闭合。
3. 重新赋值不可变变量 — `let x = 3; x = 4` 报错。用 `let mut`。
4. 用 try/catch — Lom 用 `Result` + `?`，无异常。
5. 通配符导入 — `import *` 禁止。用 `from math import {sqrt, abs}`。
6. 用 class — Lom 用结构记录 `{x: Int, y: Int}`，不是 `class Point`。
7. 忘记 `?` — `let x = read_file(path)` 得 `Result<...>`，不是 `String`。加 `?` 解包。
8. match 不穷尽 — `match` 必须覆盖所有情况或用 `_`。
9. 用 `return` 作最后语句 — 不推荐。直接用尾表达式。
10. **未导入非 prelude 内置函数** — `len`/`int_to_string`/`sqrt` 等必须导入。`println`/`print` 是 prelude（自动可用）。

### 完整示例

```
from string import { trim, len }

fn greet(name: String) -> String
    "Hello, " + name
end

fn parse_name(s: String) -> Result<String, String>
    let cleaned = s |> trim
    if len(cleaned) == 0
        Err("empty name")
    else
        Ok(cleaned)
    end
end

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

## 任务列表

请为以下每个任务生成一份完整的 `.lom` 源文件。只输出代码，不要解释。

### Task 056

写函数 double(x: Int) -> Int 返回 x * 2。main 中 let result = 5 |> double，然后 println(result)。

### Task 057

写函数 inc(x: Int) -> Int 返回 x + 1。写函数 double(x: Int) -> Int 返回 x * 2。main 中 let result = 5 |> inc |> double，然后 println(result)（即 double(inc(5))=double(6)=12）。

### Task 058

写函数 add(x: Int, y: Int) -> Int 返回 x + y。main 中 let result = 10 |> add(3)，然后 println(result)（即 add(10, 3) = 13）。

### Task 059

main 中直接用 pipeline 调用 println：42 |> println。

### Task 060

写函数 double(x: Int) -> Int 返回 x*2。写函数 inc(x: Int) -> Int 返回 x+1。写函数 negate(x: Int) -> Int 返回 -x。main 中 let result = 3 |> negate |> double |> inc，println(result)（即 inc(double(negate(3)))=inc(double(-3))=inc(-6)=-5）。

### Task 061

写函数 add(x: Int, y: Int) -> Int 返回 x+y。main 中 let result = 1 |> add(2) |> add(3)，println(result)（即 add(add(1, 2), 3) = add(3, 3) = 6）。

### Task 062

写函数 double(x: Int) -> Int 返回 x*2。main 中 let result = 1 + 2 |> double，println(result)（即 double(1+2) = double(3) = 6，因 |> 优先级低于算术）。

### Task 063

写函数 double(x: Int) -> Int 返回 x*2。main 中 let result = 5 |> double == 10，println(result)（即 (5 |> double) == 10 => 10 == 10 => True，输出 true）。考察 pipeline 优先级高于比较。

### Task 064

from string import { int_to_string }。写函数 double(x: Int) -> Int 返回 x*2。写函数 stringify(n: Int) -> String 返回 int_to_string(n)。main 中 5 |> double |> stringify |> println（即 println(stringify(double(5))) = println("10")）。

### Task 065

写函数 add(x: Int, y: Int) -> Int 返回 x+y。写函数 square(x: Int) -> Int 返回 x*x。写函数 inc(x: Int) -> Int 返回 x+1。main 中 let result = 2 |> add(3) |> square |> inc，println(result)（即 inc(square(add(2, 3))) = inc(square(5)) = inc(25) = 26）。


---

## 输出格式

请严格按以下格式输出，每个任务用分隔符标记，便于自动提取：

```
=== 001.lom ===
fn add(a: Int, b: Int) -> Int
    a + b
end

fn main() -> Unit
    println(add(3, 4))
end
=== 002.lom ===
...下一份代码...
```

要求：
1. 每份代码用 `=== <id>.lom ===` 开始（id 为三位数字，如 001、002）。
2. 代码必须完整可运行（含必要的 `from ... import` 和 `fn main`）。
3. 只输出代码，不要加解释说明。
4. 按任务顺序输出，不要遗漏任何任务。
