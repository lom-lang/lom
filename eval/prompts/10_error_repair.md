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

### Task 086

以下 .lom 代码有错误（lom-diag/v1 诊断如下）。请修复代码使其正确运行并输出 7。

错误代码：
fn add(a: Int, b: Int) -> Int
    a + b
end

fn main() -> Unit
    println(add(3, 4)
end

诊断 JSON：
{"schema":"lom-diag/v1","ok":false,"diagnostics":[{"severity":"error","stage":"parse","code":"PARSE001","message":"期望 ')' 或参数，得到 'end'","line":7,"col":5,"hint":"检查括号是否匹配"}]}

请输出修复后的完整代码。

### Task 087

以下 .lom 代码有错误。请修复使其输出 hello world。

错误代码：
fn main() -> Unit
    let s = "hello world
    println(s)
end

诊断 JSON：
{"schema":"lom-diag/v1","ok":false,"diagnostics":[{"severity":"error","stage":"lex","code":"LEX001","message":"未闭合的字符串","line":2,"col":12,"hint":"字符串需要在同行用 \" 闭合"}]}

请输出修复后的完整代码。

### Task 088

以下 .lom 代码有错误。请修复使其输出 42。

错误代码：
fn main() -> Unit
    println(42
end

诊断 JSON：
{"schema":"lom-diag/v1","ok":false,"diagnostics":[{"severity":"error","stage":"parse","code":"PARSE001","message":"期望 ')' 或参数，得到 'end'","line":3,"col":1,"hint":"检查括号是否匹配"}]}

请输出修复后的完整代码。

### Task 089

以下 .lom 代码有错误。请修复使其输出 5。

错误代码：
fn main() -> Unit
    let x = 5
    println(x)

诊断 JSON：
{"schema":"lom-diag/v1","ok":false,"diagnostics":[{"severity":"error","stage":"parse","code":"PARSE001","message":"期望 'end' 结束函数体，得到 EOF","line":3,"col":1,"hint":"函数体需要以 end 闭合"}]}

请输出修复后的完整代码。

### Task 090

以下 .lom 代码有错误。请修复使其输出 10。

错误代码：
fn double(x: Int) -> Int
    x * 2
end

fn main() -> Unit
    let result = double(5)
    println(result)

诊断 JSON：
{"schema":"lom-diag/v1","ok":false,"diagnostics":[{"severity":"error","stage":"parse","code":"PARSE001","message":"期望 'end' 结束函数体，得到 EOF","line":7,"col":1,"hint":"函数体需要以 end 闭合"}]}

请输出修复后的完整代码。

### Task 091

以下 .lom 代码有错误。请修复使其输出 true。

错误代码：
fn main() -> Unit
    let x = 5
    let y = 5
    println(x == y)
end

诊断 JSON：
{"schema":"lom-diag/v1","ok":true,"diagnostics":[]}

注意：此代码实际上没有错误（诊断 ok=true）。但若运行时输出不对，请检查逻辑。正确输出应为 true。

### Task 092

以下 .lom 代码有错误。请修复使其输出 30。

错误代码：
fn main() -> Unit
    let mut total = 0
    let mut i = 0
    while i < 5
        total = total + i
        i = i + 1
    end
    println(total)
end

诊断 JSON：
{"schema":"lom-diag/v1","ok":true,"diagnostics":[]}

注意：代码无语法错误，但逻辑可能有误。期望输出 30（0+1+2+3+4=10... 不对，应是 0+1+2+3+4=10）。等等，请确认：i 从 0 加到 4，总和是 10。但题目要求输出 30。请修改逻辑使其输出 30（提示：0²+1²+2²+3²+4²=0+1+4+9+16=30，即 total = total + i * i）。

### Task 093

以下 .lom 代码有错误。请修复使其正确输出 Monday / Tuesday / Other。

错误代码：
fn day_name(n: Int) -> String
    match n
        1 => "Monday"
        2 => "Tuesday"
    end
end

fn main() -> Unit
    println(day_name(1))
    println(day_name(2))
    println(day_name(5))
end

诊断 JSON（--check 模式）：
{"schema":"lom-diag/v1","ok":true,"diagnostics":[{"severity":"warning","stage":"type","code":"MAT001","message":"match 不穷尽：缺少 _ 通配符或所有分支","line":2,"col":5,"hint":"添加 _ => ... 分支处理剩余情况"}]}

请输出修复后的完整代码。

### Task 094

以下 .lom 代码有错误。请修复使其正确输出 IO 效应函数的结果。

错误代码：
fn greet(name: String) -> Unit
    println("Hello, " + name)
end

fn main() -> Unit
    greet("Lom")
end

诊断 JSON（--check 模式）：
{"schema":"lom-diag/v1","ok":true,"diagnostics":[{"severity":"warning","stage":"type","code":"EFF001","message":"纯函数或未声明效应 [] 的函数调用了带效应 [IO] 的函数 'println'","line":2,"col":5,"hint":"在函数签名返回类型后添加效应注解: ! [IO]"}]}

请输出修复后的完整代码（消除 EFF001 警告）。

### Task 095

以下 .lom 代码有错误。请修复使其输出 5。

错误代码：
from string import { len }

fn main() -> Unit
    println(lenght("hello"))
end

诊断 JSON：
{"schema":"lom-diag/v1","ok":true,"diagnostics":[]}

运行时错误（实际运行时）：
未定义变量: 'lenght'（拼写错误，应为 len）

请输出修复后的完整代码。

### Task 096

以下 .lom 代码有错误。请修复使其输出 11。

错误代码：
fn add(x: Int, y: Int) -> Int
    x + y
end

fn main() -> Unit
    println(add(5))
end

诊断 JSON（--check 模式）：
{"schema":"lom-diag/v1","ok":true,"diagnostics":[{"severity":"warning","stage":"type","code":"TYPE003","message":"函数 'add' 期望 2 个参数，得到 1 个","line":6,"col":5,"hint":"检查函数调用参数数量"}]}

请输出修复后的完整代码。

### Task 097

以下 .lom 代码有错误。请修复使其正确处理 Result 并输出 5 和 Error: div by zero。

错误代码：
fn safe_divide(a: Int, b: Int) -> Result<Int, String>
    if b == 0
        Err("div by zero")
    else
        Ok(a / b)
    end
end

fn main() -> Unit
    match safe_divide(10, 2)
        Ok(n) => println(n)
    end

    match safe_divide(10, 0)
        Err(e) => println("Error: " + e)
    end
end

诊断 JSON（--check 模式）：
{"schema":"lom-diag/v1","ok":true,"diagnostics":[{"severity":"warning","stage":"type","code":"MAT001","message":"match 不穷尽：Result 缺少 Err 分支","line":10,"col":5},{"severity":"warning","stage":"type","code":"MAT001","message":"match 不穷尽：Result 缺少 Ok 分支","line":14,"col":5}]}

请输出修复后的完整代码（两个 match 都需穷尽 Ok 和 Err）。

### Task 098

以下 .lom 代码有错误。请修复使其输出正确的斐波那契数列前 5 项：0 1 1 2 3。

错误代码：
fn fib(n: Int) -> Int
    if n == 0
        0
    elif n == 1
        1
    else
        fib(n - 1) + fib(n - 2)
    end
end

fn main() -> Unit
    let mut i = 0
    while i < 5
        println(fib(i))
        i = i + 1
    end
end

诊断 JSON：
{"schema":"lom-diag/v1","ok":true,"diagnostics":[]}

注意：代码无语法/类型错误，但请确认输出是否为 0 1 1 2 3。若是则无需修改；若不是请修复。期望输出每行一个数字：0 1 1 2 3。

### Task 099

以下 .lom 代码有多个错误。请修复使其正确输出 OK 和 Not Found。

错误代码：
fn main() -> Unit
    match 200
        200 => println("OK")
        404 => println("Not Found")
    end

    match 404
        200 => println("OK")
        404 => println("Not Found")
    end
end

诊断 JSON：
{"schema":"lom-diag/v1","ok":true,"diagnostics":[]}

注意：代码无语法错误且能运行。但若需要处理未知状态码（如 500），应加 _ 分支。当前代码对 200 和 404 的输出应为 OK 和 Not Found。请确认并输出代码。

### Task 100

以下 .lom 代码有错误。请修复使其正确输出结果。

错误代码：
from string import { int_to_string }

fn double(x: Int) -> Int
    x * 2
end

fn main() -> Unit
    let n = double(21)
    let s = int_tostring(n)
    println(s)
end

诊断 JSON：
{"schema":"lom-diag/v1","ok":true,"diagnostics":[]}

运行时错误（实际运行时）：
未定义变量: 'int_tostring'（应为 int_to_string，下划线缺失）

请输出修复后的完整代码。期望输出 42。

### Task 109

以下 .lom 代码有错误（lom-diag/v1 诊断如下）。请修复代码使其正确运行并输出 3。

错误代码：
fn add(a: Int, b: Int) -> Int
    a + b
end

fn main() -> Unit
    let s = "abc
    println(add(1, 2)
end
诊断 JSON：
{"schema":"lom-diag/v1","ok":false,"diagnostics":[{"severity":"error","stage":"lex","code":"LEX001","message":"未闭合的字符串","line":6,"col":13,"hint":"在字符串末尾添加 \" 闭合"},{"severity":"error","stage":"parse","code":"PARSE001","message":"期望 ')'，得到 End","line":8,"col":1,"hint":"检查语法结构是否完整，关键字/分隔符是否匹配"}]}

请输出修复后的完整代码。

### Task 110

以下 .lom 代码有错误（lom-diag/v1 诊断如下）。请修复代码使其正确运行并输出 42。

错误代码：
fn main() -> Unit
    let count = 41
    println(cont + 1)
end
诊断 JSON：
{"schema":"lom-diag/v1","ok":false,"diagnostics":[{"severity":"error","stage":"type","code":"NAM003","message":"未定义变量 'cont'","line":0,"col":0,"hint":"是否想用 'count'？"}]}

请输出修复后的完整代码。

### Task 111

以下 .lom 代码有错误（lom-diag/v1 诊断如下）。请修复代码使其正确运行并输出 lom。

错误代码：
fn main() -> Unit
    let user = {name: "lom", age: 1}
    println(user.nam)
end
诊断 JSON：
{"schema":"lom-diag/v1","ok":false,"diagnostics":[{"severity":"error","stage":"type","code":"NAM004","message":"记录无字段 'nam'","line":0,"col":0,"hint":"是否想用 'name'？"}]}

请输出修复后的完整代码。

### Task 112

以下 .lom 代码能运行但有两个效应警告（lom-diag/v1 诊断如下）。请补全效应注解消除警告，保持输出不变（boot 然后 1）。

代码：
fn log_line(s: String) -> Unit
    println(s)
end

fn show(n: Int) -> Unit
    println(n)
end

fn main() -> Unit
    log_line("boot")
    show(1)
end

诊断 JSON：
{"schema":"lom-diag/v1","ok":true,"diagnostics":[{"severity":"warning","stage":"type","code":"EFF001","message":"纯函数或未声明效应 [] 的函数调用了带效应 [IO] 的函数 'println'","line":1,"col":1,"hint":null},{"severity":"warning","stage":"type","code":"EFF001","message":"纯函数或未声明效应 [] 的函数调用了带效应 [IO] 的函数 'println'","line":5,"col":1,"hint":null}]}

请输出修复后的完整代码。

### Task 113

以下 .lom 代码有错误（lom-diag/v1 诊断如下）。请修复代码使其正确运行并输出 x。

错误代码：
fn main() -> Unit
    let x = 1
    match x
        _ => println("x")
诊断 JSON：
{"schema":"lom-diag/v1","ok":false,"diagnostics":[{"severity":"error","stage":"parse","code":"PARSE001","message":"期望 'end' 闭合 match","line":5,"col":1,"hint":"检查语法结构是否完整，关键字/分隔符是否匹配"}]}

请输出修复后的完整代码。


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
