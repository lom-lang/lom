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

### Task 131

以下 .lom 代码在宿主 lom 下能运行（输出 2）但有一条警告（lom-diag/v1 诊断如下）；同时它无法通过实验性 L2 子集编译器（examples/selfhost/self_comp.lom）的编译。请修复代码：消除宿主警告、保持宿主输出 2 不变，并使代码能通过 L2 子集编译。修复须保留对 Map 的实际读取——**通过 map_get 取出键 "a" 对应的值参与算术得到输出 2**（不得把输出写死为常量；也不得保留任何 Map 直接参与算术的代码——即使它在不会执行的分支里也会被 L2 编译器拒绝，因为 L2 对全程序做静态检查）。

代码：
from io import {println}
from map import { map_empty, map_set }

fn main() -> Unit ! [IO]
    let m = map_empty()
    map_set(m, "a", 1)
    if False
        println(m + 1)
    else
        println(2)
    end
end
诊断 JSON：
{"schema": "lom-diag/v1", "file": "bad.lom", "ok": true, "summary": {"total": 1, "errors": 0, "warnings": 1, "holes": 0}, "diagnostics": [{"severity": "warning", "stage": "type", "code": "TYPE001", "message": "'+' 不支持 Generic(\"Map\", [Named(\"_Any\")]) 和 Int", "file": "bad.lom", "line": 0, "col": 0, "source_line": null, "is_hole": false, "hint": "检查运算符两侧类型是否一致"}]}

实验性 L2 子集编译器（examples/selfhost/self_comp.lom）拒绝如下：
codegen error: [L2V001] L2.2 子集不支持: Map 参与算术（L2 子集外；仅支持 ==/!= 比较）
COMPILE-ERROR

请输出修复后的完整代码。

### Task 132

以下 .lom 代码静态检查有一条警告（lom-diag/v1 诊断如下），且运行时会失败（[RUNTIME002] 符号 'upper' 未导入——警告正是对运行时失败的预警）。请修复代码消除该警告并使程序正常运行。

代码：
fn shout(s: String) -> String
    upper(s) + "!"
end

fn main() -> Unit
    println(shout("lom"))
end
诊断 JSON：
{"severity": "warning", "stage": "type", "code": "NAM005", "message": "内建 'upper' 未导入——需在文件顶部声明：from string import {upper}", "file": "bad.lom", "line": 2, "col": 5, "source_line": "    upper(s) + \"!\"", "is_hole": false, "hint": null}

请输出修复后的完整代码。

### Task 133

以下 .lom 代码静态检查有一条警告（lom-diag/v1 诊断如下），且运行时会失败（[RUNTIME001] 期望 Bool，得到 String——警告正是对运行时失败的预警）。请修复代码消除该警告并使程序正常运行。

代码：
from io import {println}
fn main() -> Unit
    let name = "lom"
    if name
        println("yes")
    else
        println("no")
    end
end
诊断 JSON：
{"severity": "warning", "stage": "type", "code": "TYPE002", "message": "if 条件应为 Bool，得到 String", "file": "bad.lom", "line": 4, "col": 8, "source_line": "    if name", "is_hole": false, "hint": "条件表达式应为 Bool"}

请输出修复后的完整代码。

### Task 134

以下 .lom 代码在宿主 lom 下完全正确（静态检查零诊断、运行输出 2），但无法通过实验性 L2 子集编译器（examples/selfhost/self_comp.lom）的编译——L2 是宿主语言的实验性子集，不支持所有表达式形态。请把管道写法改写为等价的普通函数调用，保持输出 2 不变，使代码能通过 L2 子集编译。

代码：
from io import {println}
from list import { list_length }

fn main() -> Unit
    let xs = 1..3
    println(xs |> list_length)
end
诊断 JSON：
{"schema": "lom-diag/v1", "file": "bad.lom", "ok": true, "summary": {"total": 0, "errors": 0, "warnings": 0, "holes": 0}, "diagnostics": []}

实验性 L2 子集编译器（examples/selfhost/self_comp.lom）拒绝如下：
codegen error: [L2S001] L2.2 子集不支持: 该表达式形态
COMPILE-ERROR

请输出修复后的完整代码。

### Task 135

以下 .lom 代码静态检查有一条警告（lom-diag/v1 诊断如下），且运行时会失败（[RUNTIME002] 符号 'len' 未导入——警告正是对运行时失败的预警）；同时它无法通过实验性 L2 子集编译器（examples/selfhost/self_comp.lom）的编译。请修复代码消除该警告并使程序正常运行，且能通过 L2 子集编译。

代码：
from io import {println}

fn main() -> Unit
    let s = "hello"
    println(len(s))
end
诊断 JSON：
{"schema": "lom-diag/v1", "file": "bad.lom", "ok": true, "summary": {"total": 1, "errors": 0, "warnings": 1, "holes": 0}, "diagnostics": [{"severity": "warning", "stage": "type", "code": "NAM005", "message": "内建 'len' 未导入——需在文件顶部声明：from string import {len}", "file": "bad.lom", "line": 5, "col": 13, "source_line": "    println(len(s))", "is_hole": false, "hint": null}]}

实验性 L2 子集编译器（examples/selfhost/self_comp.lom）拒绝如下：
codegen error: 5:13 [L2U001] 未定义变量 'len'
COMPILE-ERROR

请输出修复后的完整代码。

### Task 136

以下 .lom 代码静态检查完全干净（lom-diag/v1 零诊断），但运行时失败——错误只在运行期出现（[RUNTIME000] 递归深度超限：函数缺少终止条件）。请修复代码使程序正常运行并输出正确结果。

代码：
fn sum_to(n: Int) -> Int
    n + sum_to(n - 1)
end

fn main() -> Unit
    println(sum_to(10))
end
诊断 JSON（--check 模式，零诊断）：
{"schema": "lom-diag/v1", "file": "bad.lom", "ok": true, "summary": {"total": 0, "errors": 0, "warnings": 0, "holes": 0}, "diagnostics": []}

运行时输出（lom bad.lom，stderr，退出码 1）：
[runtime] error (1:1): [RUNTIME000] 递归深度超过 60000 层（256MB 栈的安全上限）：函数 'sum_to' 疑似缺少终止条件——检查递归出口或改写为 while 循环
    | fn sum_to(n: Int) -> Int
    | ^
共 1 个诊断（1 错误，0 警告，0 代码洞）。

期望输出：55（sum_to(10) = 10+9+…+1）。修复须真实计算（不得直接输出常量 55）。

请输出修复后的完整代码。

### Task 137

以下 .lom 代码静态检查完全干净（lom-diag/v1 零诊断），但运行时失败——错误只在运行期出现（具体运行时错误信息未提供，需实际运行才能发现）。请修复代码使程序正常运行并输出正确结果。

代码：
fn is_even(n: Int) -> Bool
    is_odd(n - 1)
end

fn is_odd(n: Int) -> Bool
    if n == 0
        False
    else
        is_even(n - 1)
    end
end

fn main() -> Unit
    println(is_even(10))
end
诊断 JSON（--check 模式，零诊断）：
{
  "schema": "lom-diag/v1",
  "file": "bad.lom",
  "ok": true,
  "summary": {
    "total": 0,
    "errors": 0,
    "warnings": 0,
    "holes": 0
  },
  "diagnostics": []
}

期望输出：true（is_even(10)）。修复须保留 is_even/is_odd 相互递归结构（这正是考查点——不得合并为单一函数，也不得直接输出常量 true）。

请输出修复后的完整代码。

### Task 138

以下 .lom 代码静态检查完全干净（lom-diag/v1 零诊断），但运行时失败——错误只在运行期出现（具体运行时错误信息未提供，需实际运行才能发现）。请修复代码使程序正常运行并输出正确结果。

代码：
from io import {println}

fn main() -> Unit
    let total = 100
    let count = 4 - 4
    println(total / count)
end
诊断 JSON（--check 模式，零诊断）：
{
  "schema": "lom-diag/v1",
  "file": "bad.lom",
  "ok": true,
  "summary": {
    "total": 0,
    "errors": 0,
    "warnings": 0,
    "holes": 0
  },
  "diagnostics": []
}

期望输出：25（total / count 的正确结果）。修复须真实计算 total / count（不得直接输出常量 25）。

请输出修复后的完整代码。

### Task 139

以下 .lom 代码静态检查完全干净（lom-diag/v1 零诊断），但运行时失败——错误只在运行期出现（具体运行时错误信息未提供，需实际运行才能发现）。请修复代码使程序正常运行并输出正确结果。

代码：
from io import {println}
from string import {string_to_int}

fn main() -> Unit
    let n = string_to_int("12x")
    println(n + 30)
end
诊断 JSON（--check 模式，零诊断）：
{
  "schema": "lom-diag/v1",
  "file": "bad.lom",
  "ok": true,
  "summary": {
    "total": 0,
    "errors": 0,
    "warnings": 0,
    "holes": 0
  },
  "diagnostics": []
}

期望输出：42。修复须保留 string_to_int 调用参与计算（不得直接输出常量 42）。

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
