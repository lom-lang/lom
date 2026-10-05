// Lom Playground 内置示例集（designs/0019 §4.2）。
//
// 每枚 source 都在 playground harness（wasm32-wasip1 + 手写 WASI 绑定）
// 上实跑验证过：repair-loop 期望 RUNTIME002 诊断（点 Fix 看修复闭环），
// 其余五枚 exitCode === 0 且 stdout 非空。语法要点见仓库 LANGUAGE_SPEC.md：
// 布尔 True/False 大写；无 List 字面量（list_empty/list_cons 构造）；
// match Form B 每臂以 end 闭合；块尾 end。

export const EXAMPLES = [
  {
    id: "repair-loop",
    name: "修复闭环（README 门面同款）",
    note: "坏代码：len 未导入。先 Run 看 RUNTIME002 / NAM005 结构化诊断，再点 Fix —— fix --apply 会自动补上 from string import 并写回编辑器（附行级 diff）。",
    source: `fn main() -> Unit
    println(len("hello"))
end
`,
  },
  {
    id: "hello",
    name: "Hello",
    note: "最小可运行程序：fn ... end 块 + println。",
    source: `# 最小门面
fn main() -> Unit
    println("hello, lom")
end
`,
  },
  {
    id: "fib",
    name: "Fibonacci（递归）",
    note: "递归版 fib，打印前 10 项。递归深度远低于 300 帧守卫，可放心运行。",
    source: `# 斐波那契 — 递归实现，打印前 10 项
fn fib(n: Int) -> Int
    if n < 2
        n
    else
        fib(n - 1) + fib(n - 2)
    end
end

fn main() -> Unit
    let mut i = 0
    while i < 10
        println(fib(i))
        i = i + 1
    end
end
`,
  },
  {
    id: "closures",
    name: "闭包 — 计数器与捕获",
    note: "闭包一等值：make_counter 返回捕获外部变量的闭包。诊断栏的 MUT002 警告是语言有意为之 —— 它提示闭包捕获 mut 绑定在双后端语义相反（解释器=共享作用域，WASM=创建时拷贝），建议避免依赖，这正是“诊断是产品”的样例。",
    source: `# 闭包 — 计数器（捕获可变变量）与值传递
fn make_counter() -> Fn
    let mut count = 0
    fn(step: Int) -> Int
        count = count + step
        count
    end
end

fn apply(f: Fn, x: Int) -> Int
    f(x)
end

fn main() -> Unit
    let c = make_counter()
    println(c(1))
    println(c(1))
    println(c(1))

    # 闭包作为参数传递（高阶函数）
    let double = fn(n: Int) -> Int
        n * 2
    end
    println(apply(double, 21))
end
`,
  },
  {
    id: "match-enum",
    name: "enum + match + Result/Option",
    note: "自定义枚举（带参数变体）+ match 模式匹配 + 内建 Result/Option。match 单行臂（Form A）在这里就够。",
    source: `# enum + match + Result/Option
enum Shape
    | Circle(Float)
    | Square(Float)
end

fn area(s: Shape) -> Float
    match s
        Circle(r) => 3.14 * r * r
        Square(side) => side * side
    end
end

fn safe_div(x: Int, y: Int) -> Result<Int, String>
    if y == 0
        Err("div by zero")
    else
        Ok(x / y)
    end
end

fn main() -> Unit
    println(area(Circle(2.0)))
    println(area(Square(3.0)))

    match safe_div(10, 2)
        Ok(n) => println(n)
        Err(e) => println("error: " + e)
    end

    match safe_div(10, 0)
        Ok(n) => println(n)
        Err(e) => println("error: " + e)
    end

    let maybe = Some(5)
    match maybe
        Some(v) => println(v)
        None => println("nothing")
    end
end
`,
  },
  {
    id: "effects",
    name: "显式效应 ! [IO]",
    note: "纯函数不能调用带效应的函数（EFF001 警告）；main 隐式拥有全部效应。效应是编译期注解，不改变运行行为。",
    source: `# 显式效应 — 纯函数与 ! [IO] 带效应函数
from string import { int_to_string }

fn double(x: Int) -> Int
    x * 2
end

fn print_double(x: Int) -> Unit ! [IO]
    println(double(x))
end

fn now() -> Int ! [Clock]
    1234567890
end

fn log_msg(msg: String) -> Unit ! [IO, Clock]
    print("[")
    print(int_to_string(now()))
    print("] ")
    println(msg)
end

fn main() -> Unit
    print_double(21)
    log_msg("hello effects")
end
`,
  },
];
