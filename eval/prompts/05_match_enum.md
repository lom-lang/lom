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

### Task 041

写函数 day_name(n: Int) -> String 用 match 返回星期几名称：0 => "Sun", 1 => "Mon", 2 => "Tue", 3 => "Wed", 4 => "Thu", 5 => "Fri", 6 => "Sat", _ => "invalid"。main 调用 println(day_name(0))、println(day_name(3))、println(day_name(6))、println(day_name(9))。

### Task 042

写函数 classify_num(n: Int) -> String 用 match：0 => "zero", _ => "nonzero"。main 调用 println(classify_num(0)) 和 println(classify_num(42))。

### Task 043

声明枚举 enum Color = Red | Green | Blue。写函数 color_name(c: Color) -> String 用 match：Red => "red", Green => "green", Blue => "blue"。main 调用 println(color_name(Red))、println(color_name(Green))、println(color_name(Blue))。

### Task 044

写函数 safe_divide(a: Int, b: Int) -> Result<Int, String>：b == 0 返回 Err("div by zero")，否则返回 Ok(a / b)。main 用 match 处理 safe_divide(10, 2)（打印 5）和 safe_divide(10, 0)（打印 "Error: div by zero"）。match 用 Ok(n) => println(n) 和 Err(e) => println("Error: " + e)。

### Task 045

写函数 first_char(s: String) -> Option<String>：如果 s 为空字符串返回 None，否则返回 Some(s)。提示：用 if len(s) == 0 判空。main 用 match 处理 first_char("hello")（打印 "got: hello"）和 first_char("")（打印 "empty"）。match 用 Some(s) => println("got: " + s) 和 None => println("empty")。需要 from string import { len }。

### Task 046

声明枚举 enum Shape = Circle(Float) | Rect(Float, Float)。写函数 area(s: Shape) -> Float 用 match：Circle(r) => 3.14159 * r * r，Rect(w, h) => w * h。main 调用 println(area(Circle(2.0)))（≈12.56636）和 println(area(Rect(3.0, 4.0)))（=12.0，Float 输出为 12.0）。

### Task 047

声明枚举 enum Expr = Num(Int) | Add(Expr, Expr)（注：Expr 递归类型，但 Lom 不支持真正的递归类型，此处仅测试语法声明）。改为简单版：enum Op = Add | Sub | Mul | Div。写函数 apply_op(op: Op, a: Int, b: Int) -> Int 用 match：Add => a + b, Sub => a - b, Mul => a * b, Div => a / b。main 调用 println(apply_op(Add, 3, 4))、println(apply_op(Mul, 3, 4))、println(apply_op(Sub, 10, 3))。

### Task 048

写函数 parse_status(code: Int) -> String 用 match：200 => "OK", 404 => "Not Found", 500 => "Server Error", _ => "Unknown"。main 调用 println(parse_status(200))、println(parse_status(404))、println(parse_status(500))、println(parse_status(403))。

### Task 049

写函数 half(n: Int) -> Option<Int>：n 为偶数返回 Some(n / 2)，奇数返回 None。用 if n % 2 == 0 判偶。main 用 match 处理 half(10)（打印 5）和 half(7)（打印 "odd"）。match 用 Some(v) => println(v) 和 None => println("odd")。

### Task 050

写函数 grade_to_score(grade: String) -> Result<Int, String>：grade == "A" 返回 Ok(90)，grade == "B" 返回 Ok(80)，grade == "C" 返回 Ok(70)，其他返回 Err("unknown grade")。main 用 match 处理 grade_to_score("A")（打印 90）和 grade_to_score("X")（打印 "err: unknown grade"）。match 用 Ok(s) => println(s) 和 Err(e) => println("err: " + e)。

### Task 051

写函数 fizzbuzz(n: Int) -> String 用 match 和 if/elif 组合：n 能被 15 整除返回 "FizzBuzz"，能被 3 整除返回 "Fizz"，能被 5 整除返回 "Buzz"，否则返回 n 转字符串。需要 from string import { int_to_string }。main 调用 println(fizzbuzz(15))、println(fizzbuzz(9))、println(fizzbuzz(10))、println(fizzbuzz(7))。

### Task 052

声明多行枚举 enum Direction
    North
    South
    East
    West
end。写函数 opposite(d: Direction) -> Direction 用 match：North => South, South => North, East => West, West => East。main 调用 println(opposite(North) == South)（应为 true）和 println(opposite(East) == West)（应为 true）。

### Task 053

写函数 unwrap_or(r: Result<Int, String>, default: Int) -> Int 用 match：Ok(n) => n, Err(_) => default。main 调用 println(unwrap_or(Ok(42), 0))（打印 42）和 println(unwrap_or(Err("err"), 99))（打印 99）。

### Task 054

写函数 classify_age(age: Int) -> String 用 match：0 => "newborn", n（若 n < 13）... 提示：match 只能匹配字面量，不能匹配不等式。所以用 if/elif/else 实现：age < 1 返回 "baby", age < 13 返回 "child", age < 20 返回 "teen", age < 65 返回 "adult", 否则返回 "senior"。main 调用 println(classify_age(0))、println(classify_age(5))、println(classify_age(15))、println(classify_age(30))、println(classify_age(70))。

### Task 055

声明 enum LogLevel = Debug | Info | Warn | Error。写函数 log_prefix(level: LogLevel) -> String 用 match：Debug => "[DBG]", Info => "[INF]", Warn => "[WRN]", Error => "[ERR]"。写函数 log_message(level: LogLevel, msg: String) -> String 返回 log_prefix(level) + " " + msg。main 调用 println(log_message(Info, "started"))、println(log_message(Error, "crashed"))、println(log_message(Debug, "x=42"))。


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
