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

### Task 031

写函数 make_adder(n: Int) -> Fn 返回一个闭包 fn(x: Int) -> Int 返回 x + n。main 中 let add5 = make_adder(5)，然后 println(add5(10)) 和 println(add5(20))。

### Task 032

main 中直接定义闭包 let double = fn(x: Int) -> Int 返回 x * 2 end。然后 println(double(5)) 和 println(double(21))。

### Task 033

写函数 apply(f: Fn, x: Int) -> Int 返回 f(x)。main 中 let inc = fn(n: Int) -> Int n + 1 end，然后 println(apply(inc, 5)) 和 println(apply(inc, 100))。

### Task 034

写函数 apply_twice(f: Fn, x: Int) -> Int 返回 f(f(x))。main 中 let inc = fn(n: Int) -> Int n + 1 end，然后 println(apply_twice(inc, 5))。

### Task 035

写函数 compose(f: Fn, g: Fn, x: Int) -> Int 返回 f(g(x))。main 中 let double = fn(n: Int) -> Int n * 2 end，let inc = fn(n: Int) -> Int n + 1 end，println(compose(double, inc, 5))（即 double(inc(5))=12）和 println(compose(inc, double, 5))（即 inc(double(5))=11）。

### Task 036

写函数 sum_of_squares(n: Int) -> Int 用 while 循环对 0..n-1 的平方求和。即 0² + 1² + ... + (n-1)²。main 调用 println(sum_of_squares(5))（0+1+4+9+16=30）和 println(sum_of_squares(10))（0+1+4+9+16+25+36+49+64+81=285）。

### Task 037

写函数 counter(start: Int) -> Fn 返回一个闭包 fn() -> Int，每次调用返回 start 累加 1 的值（使用捕获的 mut 变量）。提示：闭包内用 let mut count = start，返回 count，然后 count = count + 1。但因为闭包不能有状态，简化为：返回 fn() -> Int 直接返回 start（无状态）。main 中 let c = counter(10)，println(c())。

### Task 038

写函数 map_sum(f: Fn, n: Int) -> Int 用 while 对 0..n-1 应用 f 并求和。即 f(0) + f(1) + ... + f(n-1)。main 中 let square = fn(n: Int) -> Int n * n end，println(map_sum(square, 4))（0+1+4+9=14）。

### Task 039

写函数 filter_sum(f: Fn, n: Int) -> Int 用 while 对 0..n-1 中满足 f(i) 为 True 的 i 求和。main 中 let is_even = fn(n: Int) -> Bool if n % 2 == 0 True else False end end，println(filter_sum(is_even, 10))（0+2+4+6+8=20）。

### Task 040

写函数 pipeline3(f: Fn, g: Fn, h: Fn, x: Int) -> Int 返回 h(g(f(x)))。main 中 let inc = fn(n: Int) -> Int n + 1 end，let double = fn(n: Int) -> Int n * 2 end，let negate = fn(n: Int) -> Int -n end，println(pipeline3(inc, double, negate, 5))（即 negate(double(inc(5)))=negate(double(6))=negate(12)=-12）。

### Task 106

写具名函数 double(x: Int) -> Int 返回 x * 2,写高阶函数 apply_twice(f: Fn, x: Int) -> Int 返回 f(f(x))。main 调用 println(apply_twice(double, 3))(具名函数直接当参数传),再 let inc = fn(x: Int) -> Int x + 1 end,println(apply_twice(inc, 10))。

### Task 107

用 list 模块的高阶函数(需 from list import {list_map, list_filter, list_fold}):写具名函数 double(x: Int) -> Int 返回 x * 2;main 里 let xs = 1..6,然后 println(list_map(double, xs))、println(list_filter(fn(x: Int) -> Bool x % 2 == 0 end, xs))、println(list_fold(fn(acc: Int, x: Int) -> Int acc + x end, 0, xs))。

### Task 116

写 let 绑定的递归闭包:main 里 let fact = fn(n: Int) -> Int,若 n <= 1 返回 1,否则返回 n * fact(n - 1)(闭包体内自引用,fact 尚未绑定完成时引用自身),end 闭合;然后 println(fact(5)) 和 println(fact(10))。


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
