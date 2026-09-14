#!/usr/bin/env python3
# tools/diff_gen.py — 双后端差分测试：随机 Lom 程序生成器（D 工作包 D1，2026-09-14）
#
# 设计（docs/TODO.md D 工作包登记）：
# - 类型感知的模板化生成：每个模板是一个手写正确的函数骨架（返回类型固定、
#   语法/类型/效应标注全部对齐 SPEC_FOR_AI），随机性在模板选择 + 参数填充
#   （常量/边界/分支数/变量名）上——不生成"自由拼接"的语法，保证零 parse/ type Error。
# - 确定性输出：只用纯计算 + println（无 Clock/file/env/随机），同一程序
#   双后端输出必须逐字一致。
# - 默认避开 SPEC_FOR_AI §11f 六条已知分歧形态：
#   1. 闭包捕获 mut（模板只捕获不可变绑定）
#   2. JSON 数字 Int/Float 切分（不生成 json_parse）
#   3. 除零/模零（除数为非零字面量或加 != 0 守卫）
#   4. trim Unicode 空白（字符串只用 ASCII 可打印字符）
#   5. 大浮点显示（Float 结果值域控制在 |x| < 1e12；乘除链深 ≤3、操作数 ≤1000）
#   6. 深递归/深嵌套（递归深度 ≤200、表达式嵌套 ≤8、循环边界 ≤64）
#   → 正常模式下任何 stdout/退出码差异都是新发现（tools/diff_test.py 对拍）。
# - 定向覆盖 W 包事故模式（for-return 潜伏 7 版本）：提前返回族模板
#   （for 体内 return / for 体内 ? / while 内 return / 块尾 if 内 return /
#   match Form B 臂内 return / 嵌套循环内层 return）在模板池中加权。
# - --probe <kind> 探针模式：显式生成已知分歧形态（mut-capture / div-zero /
#   large-float），供 diff_test 验证 §11f 白名单仍然如档案所述（守护白名单
#   本身不腐坏）。deep-recursion 探针见 --probe deep-recursion（深递归双后端
#   结构化诊断 vs V8 trap——退出码都非 0，stdout 应一致为空/前缀）。
# - 固定种子可复现（random.Random(seed)）；零第三方依赖（Python 标准库）。
# - 数值安全：Int 结果 |x| < 2^62（乘法操作数 ≤10^4、链深 ≤3、阶乘 n ≤15、
#   累乘值 ≤8 次）；Int 溢出虽双后端一致回绕（i64 语义），仍保守控制可读性。
import argparse
import os
import random
import sys

# ---------- 常量池（全部 ASCII，避开分歧 4 的 Unicode 空白） ----------
INT_ATOMS = [0, 1, 2, 3, 5, 7, 8, 9, 11, 12, 13, 16, 20, 21, 25, 28, 33, 40, 47, 50,
             60, 64, 77, 88, 99, 100, 111, 128, 200, 256, 300, 400, 500, 512, 777, 999,
             -1, -2, -5, -7, -13, -20, -33, -50, -99, -100, -128, -500]
FLOAT_ATOMS = [0.5, 1.5, 2.5, 3.14, 4.0, 6.25, 7.5, 8.5, 9.99, 10.0, 12.5, 15.5,
               20.0, 25.5, 31.5, 40.0, 50.5, 64.0, 99.9, 100.0, 128.5,
               -0.5, -1.5, -2.25, -7.5, -10.0, -33.3, -50.0, -99.9]
STR_ATOMS = ["a", "b", "abc", "hello", "world", "lom", "test", "xyz", "data", "key",
             "value", "word", "text", "name", "item", "ok", "err", "foo", "bar", "baz",
             "ab", "cd", "123", "42", "9x8", "mid", "tail", "longer-string-sample"]
BOOL_STR = ["True", "False"]


class Gen:
    """一个随机程序的生成上下文：顶层声明收集 + main 语句收集 + 名字去重。"""

    def __init__(self, rng: random.Random):
        self.rng = rng
        self.decls: list[str] = []   # enum / fn 声明（按序）
        self.main: list[str] = []    # main 体语句（含打印）
        self.n = 0                   # 名字去重计数

    def uniq(self, prefix: str) -> str:
        self.n += 1
        return f"{prefix}_{self.n}"

    def rint(self, lo=None, hi=None) -> int:
        if lo is None:
            return self.rng.choice(INT_ATOMS)
        return self.rng.randint(lo, hi)

    def rfloat(self) -> float:
        return self.rng.choice(FLOAT_ATOMS)

    def rstr(self) -> str:
        return self.rng.choice(STR_ATOMS)

    def emit(self, fn_text: str, call_stmts: list[str]):
        """登记一个辅助函数与其在 main 中的调用/打印语句。"""
        self.decls.append(fn_text.rstrip())
        self.main.extend(call_stmts)

    # ===== 模板族 A：纯 Int 计算 =====

    def t_factorial(self):
        v = self.uniq("n")
        r = self.uniq("acc")
        n = self.rng.randint(2, 12)
        fn = f"""fn {v}(n: Int) -> Int
    if n <= 1
        1
    else
        n * {v}(n - 1)
    end
end"""
        calls = [f"println({v}({n}))"]
        self.emit(fn, calls)

    def t_digit_sum(self):
        v = self.uniq("ds")
        n = abs(self.rng.randint(100, 999999))
        fn = f"""fn {v}(n: Int) -> Int
    let mut m = n
    let mut total = 0
    while m > 0
        total += m % 10
        m = m / 10
    end
    total
end"""
        calls = [f"println({v}({n}))"]
        self.emit(fn, calls)

    def t_range_sum_filtered(self):
        v = self.uniq("rs")
        n = self.rng.randint(4, 40)
        k = self.rng.randint(2, 5)
        op = self.rng.choice(["% {k} == 0", "% {k} == 1", "> {k}", "< {k} * 2"])
        cond = op.format(k=k)
        fn = f"""fn {v}(n: Int) -> Int
    let mut total = 0
    for i in 1..n
        if i {cond}
            total += i
        end
    end
    total
end"""
        calls = [f"println({v}({n}))"]
        self.emit(fn, calls)

    def t_nested_loops(self):
        v = self.uniq("grid")
        a = self.rng.randint(2, 6)
        b = self.rng.randint(2, 6)
        fn = f"""fn {v}(a: Int, b: Int) -> Int
    let mut count = 0
    for i in a
        for j in b
            if (i + 1) * (j + 1) % 2 == 0
                count += 1
            else
                count += 2
            end
        end
    end
    count
end"""
        calls = [f"println({v}({a}, {b}))"]
        self.emit(fn, calls)

    def t_grading(self):
        v = self.uniq("grade")
        n = self.rng.choice([95, 85, 75, 65, 55, 45, 100, 0, 77, 82])
        fn = f"""fn {v}(score: Int) -> String
    if score >= 90
        "A"
    elif score >= 80
        "B"
    elif score >= 70
        "C"
    elif score >= 60
        "D"
    else
        "F"
    end
end"""
        calls = [f"println({v}({n}))"]
        self.emit(fn, calls)

    def t_compound_assign(self):
        v = self.uniq("ca")
        x = self.rng.randint(1, 100)
        fn = f"""fn {v}(x: Int) -> Int
    let mut a = x
    a += 10
    a -= 3
    a *= 4
    a /= 2
    a = a % 97
    a
end"""
        calls = [f"println({v}({x}))"]
        self.emit(fn, calls)

    def t_minmax(self):
        v = self.uniq("mm")
        a, b, c = self.rng.choice(INT_ATOMS), self.rng.choice(INT_ATOMS), self.rng.choice(INT_ATOMS)
        fn = f"""fn {v}(a: Int, b: Int, c: Int) -> Int
    max(a, max(b, c)) - min(a, min(b, c)) + abs(a - b)
end"""
        calls = [f"println({v}({a}, {b}, {c}))"]
        self.emit(fn, calls)

    def t_fib_iter(self):
        v = self.uniq("fib")
        n = self.rng.randint(5, 30)
        fn = f"""fn {v}(n: Int) -> Int
    let mut a = 0
    let mut b = 1
    let mut i = 0
    while i < n
        let t = a + b
        a = b
        b = t
        i += 1
    end
    a
end"""
        calls = [f"println({v}({n}))"]
        self.emit(fn, calls)

    # ===== 模板族 B：提前返回（W 包事故模式定向覆盖，加权池核心） =====

    def t_return_in_for_list(self):
        v = self.uniq("ff")
        n = self.rng.randint(4, 12)
        k = self.rng.randint(2, 5)
        default = self.rng.choice([-1, -99, 0])
        fn = f"""fn {v}(n: Int) -> Int
    for x in 1..n
        if x % {k} == 0
            return x
        end
    end
    {default}
end"""
        calls = [f"println({v}({n}))", f"println({v}({k}))", f"println({v}(2))"]
        self.emit(fn, calls)

    def t_return_in_for_int(self):
        v = self.uniq("fi")
        n = self.rng.randint(3, 15)
        thr = self.rng.randint(2, 9)
        fn = f"""fn {v}(n: Int) -> Int
    let mut found = {self.rng.choice([-1, 0])}
    for i in n
        if i > {thr}
            return i * 10
        end
    end
    found
end"""
        calls = [f"println({v}({n}))"]
        self.emit(fn, calls)

    def t_return_in_nested_for(self):
        v = self.uniq("nf")
        a = self.rng.randint(2, 6)
        b = self.rng.randint(2, 6)
        fn = f"""fn {v}(a: Int, b: Int) -> Int
    for i in a
        for j in b
            if i * j % 3 == 0
                return i * 100 + j
            end
        end
    end
    -1
end"""
        calls = [f"println({v}({a}, {b}))"]
        self.emit(fn, calls)

    def t_return_in_if_expr(self):
        # 块内 if 语句 + return + 尾表达式（if 后有尾表达式故为语句形态，
        # True 路径 return 穿透 / False 路径落到尾表达式——8.1 修复语义的定向覆盖）
        v = self.uniq("ri")
        x = self.rng.choice(INT_ATOMS)
        fn = f"""fn {v}(x: Int) -> Int
    if x > 0
        if x % 2 == 0
            return x / 2
        end
        return x * 3 + 1
    end
    0 - x
end"""
        calls = [f"println({v}({x}))"]
        self.emit(fn, calls)

    def t_return_in_match_arm(self):
        # match Form B 臂块内 return（8.1 宿主修复的形态之一定向回归）；
        # 每臂独立闭合 end（§4.1 坑），`_ => ()` 兜底防 MAT001（guard/单臂不穷尽）
        v = self.uniq("mr")
        n = self.rng.randint(1, 12)
        fn = f"""fn {v}(n: Int) -> Int
    match n % 3
        0 =>
            return 100
        end
        _ => ()
    end
    match n % 4
        1 =>
            return 200
        end
        _ => ()
    end
    n
end"""
        calls = [f"println({v}({n}))"]
        self.emit(fn, calls)

    def t_return_in_while(self):
        v = self.uniq("rw")
        n = self.rng.randint(3, 64)
        step = self.rng.choice([2, 3, 5])
        fn = f"""fn {v}(n: Int) -> Int
    let mut i = 0
    while i < n
        if i * i >= n
            return i
        end
        i += {step}
    end
    -1
end"""
        calls = [f"println({v}({n}))"]
        self.emit(fn, calls)

    def t_try_in_for(self):
        # for 体内 ? 提前返回（W 包修复的第二个形态：br 深度少算吞返回值）
        v = self.uniq("tf")
        w = self.uniq("half")
        n = self.rng.randint(2, 9)
        bad = self.rng.choice(["-3", "-7", "0"])
        fn_helper = f"""fn {w}(n: Int) -> Result<Int, String>
    if n % 2 == 0
        Ok(n / 2)
    else
        Err("odd: " + n)
    end
end"""
        fn = f"""fn {v}(n: Int) -> Result<Int, String>
    for i in n
        let h = {w}(i)?
        if h > 1
            return Ok(h * 100)
        end
    end
    Ok(0)
end"""
        calls = [f'println({v}({n}))', f'println({v}({bad}))']
        self.emit(fn_helper + "\n\n" + fn, calls)

    def t_match_consume_result(self):
        v = self.uniq("mc")
        w = self.uniq("dv")
        a = self.rng.randint(10, 999)
        b = self.rng.choice([2, 3, 4, 5, 7])
        fn_helper = f"""fn {w}(a: Int, b: Int) -> Result<Int, String>
    if b != 0
        Ok(a / b)
    else
        Err("div by zero")
    end
end"""
        fn = f"""fn {v}(a: Int, b: Int) -> Int
    match {w}(a, b)
        Ok(q) => q + 1
        Err(e) => 0 - 1
    end
end"""
        calls = [f"println({v}({a}, {b}))"]
        self.emit(fn_helper + "\n\n" + fn, calls)

    # ===== 模板族 C：String =====

    def t_string_ops(self):
        v = self.uniq("so")
        s = self.rng.choice(STR_ATOMS)
        fn = f"""fn {v}(s: String) -> String
    upper(lower(s)) + "-" + int_to_string(len(s)) + "-" + s
end"""
        calls = [f'println({v}("{s}"))']
        self.emit(fn, calls)

    def t_string_walk(self):
        v = self.uniq("sw")
        s = self.rng.choice(STR_ATOMS)
        fn = f"""fn {v}(s: String) -> Int
    let mut total = 0
    for c in s
        if c == "a" or c == "e" or c == "i" or c == "o" or c == "u"
            total += 1
        else
            total += 2
        end
    end
    total
end"""
        calls = [f'println({v}("{s}"))']
        self.emit(fn, calls)

    def t_string_roundtrip(self):
        v = self.uniq("rt")
        n = self.rng.choice([0, 7, 42, 255, 1000, 9999])
        fn = f"""fn {v}(n: Int) -> Bool
    let s = int_to_string(n)
    let back = string_to_int(s)
    if back == ()
        False
    else
        back == n
    end
end"""
        calls = [f"println({v}({n}))"]
        self.emit(fn, calls)

    def t_split_join(self):
        v = self.uniq("sj")
        parts = "-".join(self.rng.sample(STR_ATOMS, 3))
        fn = f"""fn {v}(s: String) -> Int
    let xs = split(s, "-")
    let mut total = 0
    for p in xs
        total += len(p) + 1
    end
    total
end"""
        calls = [f'println({v}("{parts}"))']
        self.emit(fn, calls)

    # ===== 模板族 D：Float =====

    def t_float_arith(self):
        v = self.uniq("fa")
        a = self.rng.choice(FLOAT_ATOMS)
        fn = f"""fn {v}(x: Float) -> Float
    (x + 2.5) * 4.0 / 8.0 - 0.25
end"""
        calls = [f"println({v}({a}))"]
        self.emit(fn, calls)

    def t_float_mixed(self):
        # Int 与 Float 混合算术提升（v0.6.x 修复对齐的定向回归）
        v = self.uniq("fm")
        a = self.rng.choice(FLOAT_ATOMS)
        k = self.rng.randint(2, 9)
        fn = f"""fn {v}(x: Float, k: Int) -> Float
    x * k + 1.5 - k / 4
end"""
        calls = [f"println({v}({a}, {k}))"]
        self.emit(fn, calls)

    def t_float_sqrt(self):
        v = self.uniq("fs")
        a = self.rng.choice([4.0, 9.0, 16.0, 25.0, 2.25, 6.25, 100.0, 1.0])
        fn = f"""fn {v}(x: Float) -> Float
    sqrt(x) + sqrt(x + 4.0) * 2.0
end"""
        calls = [f"println({v}({a}))"]
        self.emit(fn, calls)

    def t_float_accum(self):
        v = self.uniq("facc")
        n = self.rng.randint(3, 12)
        fn = f"""fn {v}(n: Int) -> Float
    let mut total = 0.5
    for i in n
        total = total + 0.25
        total = total * 2.0
        total = total / 2.0
    end
    total
end"""
        calls = [f"println({v}({n}))"]
        self.emit(fn, calls)

    # ===== 模板族 E：List =====

    def t_list_hof(self):
        v = self.uniq("lh")
        w = self.uniq("dbl")
        n = self.rng.randint(3, 9)
        fn_helper = f"""fn {w}(x: Int) -> Int
    x * 3 - 1
end"""
        fn = f"""fn {v}(n: Int) -> Int
    let xs = 1..n
    let ys = list_map({w}, xs)
    let zs = list_filter(fn(x: Int) -> Bool
        x % 2 == 0
    end, ys)
    list_fold(fn(acc: Int, x: Int) -> Int
        acc + x
    end, 0, zs)
end"""
        calls = [f"println({v}({n}))", f"println(list_length(1..{n}))"]
        self.emit(fn_helper + "\n\n" + fn, calls)

    def t_list_ops(self):
        v = self.uniq("lo")
        n = self.rng.randint(3, 8)
        fn = f"""fn {v}(n: Int) -> Int
    let xs = list_cons(99, 1..n)
    let head = list_head(xs)
    let tail_len = list_length(list_tail(xs))
    head * 100 + tail_len + list_get(xs, 1)
end"""
        calls = [f"println({v}({n}))"]
        self.emit(fn, calls)

    def t_list_walk(self):
        v = self.uniq("lw")
        n = self.rng.randint(2, 7)
        fn = f"""fn {v}(n: Int) -> String
    let mut out = "["
    for x in 1..n
        if x > 1
            out = out + ","
        end
        out = out + int_to_string(x * x)
    end
    out + "]"
end"""
        calls = [f"println({v}({n}))"]
        self.emit(fn, calls)

    # ===== 模板族 F：enum + match =====

    def t_enum_dispatch(self):
        en = self.uniq("Shape")
        v = self.uniq("area")
        r = self.rng.choice([1.5, 2.0, 2.5, 3.0, 4.0])
        enum = f"""enum {en}
    | Cir(Float)
    | Sq(Float)
    | Rect(Float, Float)
    | Dot
end"""
        fn = f"""fn {v}(s: {en}) -> Float
    match s
        Cir(r) => 3.0 * r * r
        Sq(side) => side * side
        Rect(w, h) => w * h
        Dot => 0.0
    end
end"""
        calls = [
            f"println({v}(Cir({r})))",
            f"println({v}(Sq({r + 1.0})))",
            f"println({v}(Rect({r}, {r + 2.0})))",
            f"println({v}(Dot))",
        ]
        self.emit(enum + "\n\n" + fn, calls)

    def t_enum_guard_match(self):
        en = self.uniq("Level")
        v = self.uniq("lv")
        k = self.rng.randint(0, 99)
        enum = f"enum {en} = Low | Mid | High | Top"
        fn = f"""fn {v}(l: {en}, x: Int) -> String
    match l
        Low if x < {max(k, 1)} => "low-small"
        Low => "low-big"
        Mid if x % 2 == 0 => "mid-even"
        Mid => "mid-odd"
        High => "high"
        _ => "top-or-other"
    end
end"""
        calls = [
            f'println({v}(Low, {k}))',
            f'println({v}(Low, {k + 50}))',
            f'println({v}(Mid, {k}))',
            f'println({v}(Mid, {k + 1}))',
            f'println({v}(High, {k}))',
            f'println({v}(Top, {k}))',
        ]
        self.emit(enum + "\n\n" + fn, calls)

    def t_option_chain(self):
        # `?` 要求函数返回 Result/Option（TYPE020）——返回类型如实标 Option
        v = self.uniq("oc")
        w = self.uniq("fnd")
        s = self.rng.choice(STR_ATOMS)
        fn_helper = f"""fn {w}(s: String) -> Option<String>
    if len(s) > 3
        Some(upper(s))
    else
        None
    end
end"""
        fn = f"""fn {v}(s: String) -> Option<String>
    let t = {w}(s)?
    Some(t + "!")
end"""
        calls = [
            f'match {v}("{s}")',
            '    Some(r) => println(r)',
            '    None => println("none")',
            'end',
            f'match {v}("ab")',
            '    Some(r) => println(r)',
            '    None => println("none")',
            'end',
        ]
        self.emit(fn_helper + "\n\n" + fn, calls)

    def t_nested_match(self):
        # Form B 臂内嵌 match / if：每个 Form B 臂独立闭合 end（§4.1 坑）
        v = self.uniq("nm")
        a = self.rng.randint(0, 9)
        b = self.rng.randint(0, 9)
        fn = f"""fn {v}(a: Int, b: Int) -> String
    match a
        0 =>
            match b
                0 => "both-zero"
                _ => "a-zero"
            end
        end
        1 => "one"
        _ =>
            if b > 0
                "pos-pos"
            else
                "pos-neg"
            end
        end
    end
end"""
        calls = [f'println({v}({a}, {b}))', f'println({v}(0, 0))', f'println({v}(0, 5))']
        self.emit(fn, calls)

    # ===== 模板族 G：Record / Tuple =====

    def t_record_ops(self):
        v = self.uniq("rect")
        x, y = self.rng.randint(-50, 50), self.rng.randint(-50, 50)
        fn = f"""fn {v}(x: Int, y: Int) -> Int
    let p = {{x: x, y: y}}
    let q = {{y: y, x: x}}
    if p == q
        p.x * 10 + p.y
    else
        -1
    end
end"""
        calls = [f"println({v}({x}, {y}))"]
        self.emit(fn, calls)

    def t_tuple_ops(self):
        v = self.uniq("tup")
        a = self.rng.choice(STR_ATOMS)
        k = self.rng.randint(0, 99)
        fn = f"""fn {v}(s: String, k: Int) -> Int
    let t = (k, s, len(s))
    let (n, w, l) = t
    n + l * 2 + len(w)
end"""
        calls = [f'println({v}("{a}", {k}))']
        self.emit(fn, calls)

    def t_record_nested(self):
        v = self.uniq("recn")
        a = self.rng.randint(-9, 9)
        b = self.rng.randint(-9, 9)
        fn = f"""fn {v}(a: Int, b: Int) -> Int
    let outer = {{inner: {{v: a}}, w: b}}
    let sum = outer.inner.v + outer.w
    let rebuilt = {{inner: {{v: sum}}, w: 1}}
    rebuilt.inner.v * 100 + rebuilt.w
end"""
        calls = [f"println({v}({a}, {b}))"]
        self.emit(fn, calls)

    # ===== 模板族 H：闭包 =====

    def t_closure_capture(self):
        # 只捕获不可变绑定（避开分歧 1）
        v = self.uniq("cl")
        base = self.rng.randint(1, 50)
        x = self.rng.randint(1, 50)
        fn = f"""fn {v}(base: Int, x: Int) -> Int
    let add_base = fn(n: Int) -> Int
        n + base
    end
    let mul = fn(n: Int) -> Int
        n * 3
    end
    mul(add_base(x)) + add_base(0)
end"""
        calls = [f"println({v}({base}, {x}))"]
        self.emit(fn, calls)

    def t_recursive_closure(self):
        v = self.uniq("rc")
        n = self.rng.randint(2, 10)
        fn = f"""fn {v}(n: Int) -> Int
    let f = fn(k: Int) -> Int
        if k <= 1
            1
        else
            k * f(k - 1)
        end
    end
    f(n)
end"""
        calls = [f"println({v}({n}))"]
        self.emit(fn, calls)

    def t_closure_maker(self):
        v = self.uniq("cm")
        k = self.rng.randint(1, 9)
        fn = f"""fn {v}(k: Int) -> Int
    let make = fn(step: Int) -> Fn
        fn(x: Int) -> Int
            x + step
        end
    end
    let add_k = make(k)
    let add_double = make(k * 2)
    add_k(10) * 100 + add_double(10)
end"""
        calls = [f"println({v}({k}))"]
        self.emit(fn, calls)

    def t_higher_order(self):
        v = self.uniq("ho")
        a = self.rng.randint(1, 30)
        fn = f"""fn {v}(a: Int) -> Int
    let apply2 = fn(f: Fn, x: Int) -> Int
        f(f(x))
    end
    let g = fn(n: Int) -> Int
        n * n - 1
    end
    apply2(g, a)
end"""
        calls = [f"println({v}({a}))"]
        self.emit(fn, calls)

    # ===== 模板族 I：管道 =====

    def t_pipeline_int(self):
        # helper 名必须 uniq 化：同模板被抽中两次时固定名会 NAM002 重复定义
        v = self.uniq("pipei")
        h1, h2, h3 = self.uniq("add3"), self.uniq("pdbl"), self.uniq("sub1")
        a = self.rng.randint(1, 40)
        fn = f"""fn {v}(x: Int) -> Int
    x |> {h1} |> {h2} |> {h3}
end"""
        helper = f"""fn {h1}(x: Int) -> Int
    x + 3
end

fn {h2}(x: Int) -> Int
    x * 2
end

fn {h3}(x: Int) -> Int
    x - 1
end"""
        calls = [f"println({v}({a}))"]
        self.emit(helper + "\n\n" + fn, calls)

    def t_pipeline_string(self):
        v = self.uniq("pipes")
        s = self.rng.choice(STR_ATOMS)
        fn = f"""fn {v}(s: String) -> String
    s |> trim |> upper
end"""
        calls = [f'println({v}("  {s}  "))']
        self.emit(fn, calls)

    # ===== 模板族 J：Map =====

    def t_map_ops(self):
        v = self.uniq("mo")
        k = self.rng.randint(1, 9)
        fn = f"""fn {v}(k: Int) -> Int
    let m = map_empty()
    let mut i = 1
    while i <= 6
        let _ = map_set(m, "k" + int_to_string(i), i * i)
        i += 1
    end
    let hit = map_get(m, "k" + int_to_string(k))
    match hit
        Some(v) => v + map_size(m) * 10
        None => 0 - map_size(m)
    end
end"""
        calls = [f"println({v}({k}))", f"println({v}(99))"]
        self.emit(fn, calls)

    def t_map_keys(self):
        v = self.uniq("mk")
        fn = f"""fn {v}() -> Int
    let m = map_empty()
    let _ = map_set(m, "alpha", 1)
    let _ = map_set(m, "beta", 2)
    let _ = map_set(m, "gamma", 3)
    if map_has(m, "beta")
        map_remove(m, "beta")
        list_length(map_keys(m)) * 100 + map_size(m)
    else
        -2
    end
end"""
        calls = [f"println({v}())"]
        self.emit(fn, calls)

    # ===== 模板族 K：杂项组合 =====

    def t_bool_logic(self):
        v = self.uniq("bl")
        a, b = self.rng.randint(0, 20), self.rng.randint(0, 20)
        fn = f"""fn {v}(a: Int, b: Int) -> Bool
    (a > 0 and b > 0) or (a < 0 and !(b < 0)) or (a == b and a % 2 == 0)
end"""
        calls = [f"println({v}({a}, {b}))"]
        self.emit(fn, calls)

    def t_mixed_io(self):
        v = self.uniq("mix")
        n = self.rng.randint(2, 8)
        fn = f"""fn {v}(n: Int) -> Int ! [IO]
    for i in n
        print(int_to_string(i % 10))
        if i % 3 == 2
            print("-")
        end
    end
    println("")
    n * 7
end"""
        calls = [f"println({v}({n}))"]
        self.emit(fn, calls)

    def t_string_escapes(self):
        v = self.uniq("esc")
        fn = f"""fn {v}() -> String
    "tab:\\t" + int_to_string(len("a\\\\b")) + " quote:\\" end:\\n"
end"""
        calls = [f"println({v}())"]
        self.emit(fn, calls)

    def t_compare_ops(self):
        v = self.uniq("cmp")
        a, b = self.rng.choice(INT_ATOMS), self.rng.choice(INT_ATOMS)
        fn = f"""fn {v}(a: Int, b: Int) -> String
    if a < b
        "lt"
    elif a > b
        "gt"
    elif a == b
        "eq"
    else
        "unreachable"
    end
end"""
        calls = [f'println({v}({a}, {b}))']
        self.emit(fn, calls)

    def t_shadowing(self):
        v = self.uniq("sh")
        a = self.rng.randint(0, 50)
        fn = f"""fn {v}(a: Int) -> Int
    let x = a + 1
    let r1 = x * 2
    let x = r1 + 100
    let r2 = x * 2
    r1 + r2
end"""
        calls = [f"println({v}({a}))"]
        self.emit(fn, calls)


# 模板池：提前返回族（B）加权 ×3 —— W 包事故模式的定向覆盖
POOL_BASE = [
    Gen.t_factorial, Gen.t_digit_sum, Gen.t_range_sum_filtered, Gen.t_nested_loops,
    Gen.t_grading, Gen.t_compound_assign, Gen.t_minmax, Gen.t_fib_iter,
    Gen.t_string_ops, Gen.t_string_walk, Gen.t_string_roundtrip, Gen.t_split_join,
    Gen.t_float_arith, Gen.t_float_mixed, Gen.t_float_sqrt, Gen.t_float_accum,
    Gen.t_list_hof, Gen.t_list_ops, Gen.t_list_walk,
    Gen.t_enum_dispatch, Gen.t_enum_guard_match, Gen.t_option_chain, Gen.t_nested_match,
    Gen.t_record_ops, Gen.t_tuple_ops, Gen.t_record_nested,
    Gen.t_closure_capture, Gen.t_recursive_closure, Gen.t_closure_maker, Gen.t_higher_order,
    Gen.t_pipeline_int, Gen.t_pipeline_string,
    Gen.t_map_ops, Gen.t_map_keys,
    Gen.t_bool_logic, Gen.t_mixed_io, Gen.t_string_escapes, Gen.t_compare_ops,
    Gen.t_shadowing,
]
POOL_EARLY_RETURN = [
    Gen.t_return_in_for_list, Gen.t_return_in_for_int, Gen.t_return_in_nested_for,
    Gen.t_return_in_if_expr, Gen.t_return_in_match_arm, Gen.t_return_in_while,
    Gen.t_try_in_for, Gen.t_match_consume_result,
]
POOL = POOL_BASE + POOL_EARLY_RETURN * 3


def gen_program(seed: int) -> tuple[str, str]:
    """返回 (程序文本, 形态标签集合)。标签供探针/统计用。"""
    rng = random.Random(seed)
    g = Gen(rng)
    count = rng.randint(3, 7)
    for _ in range(count):
        rng.choice(POOL)(g)
    header = [
        "# diff_gen 自动生成（D 工作包差分测试）—— seed=%d" % seed,
        "from string import { len, upper, lower, trim, int_to_string, string_to_int, split }",
        "from math import { sqrt, abs, min, max }",
        "from list import { list_map, list_filter, list_fold, list_cons, list_head, list_tail, list_length, list_get }",
        "from map import { map_empty, map_set, map_get, map_has, map_remove, map_keys, map_size }",
        "",
    ]
    body = "\n\n".join(g.decls)
    main_text = "fn main() -> Unit\n" + "\n".join("    " + s for s in g.main) + "\n    ()\nend"
    return "\n".join(header) + body + "\n\n" + main_text + "\n", ""


# ---------- 探针模式：显式生成 §11f 已知分歧形态 ----------

def gen_probe(kind: str, seed: int) -> str:
    rng = random.Random(seed)
    n = rng.randint(1, 9)
    if kind == "mut-capture":
        # 分歧 1：闭包捕获 mut——WASM 创建时值拷贝 vs 解释器共享作用域
        return f"""# probe: mut-capture（§11f 分歧 1——预期双后端输出不同）
fn main() -> Unit
    let mut x = {n}
    let f = fn() -> Int
        x
    end
    x = {n + 10}
    println(f())
end
"""
    if kind == "div-zero":
        # 分歧 3：除零——双侧 exit 1，诊断消息形态不同（stderr，stdout 前缀一致）
        return f"""# probe: div-zero（§11f 分歧 3——预期双侧 exit 1）
fn main() -> Unit
    println("before")
    println({n} / 0)
end
"""
    if kind == "large-float":
        # 分歧 5：大浮点显示——全展开 vs 科学计数法。
        # 注意 Lom 无科学计数法字面量（T4 警示），大数用全展开字面量构造
        return f"""# probe: large-float（§11f 分歧 5——预期 stdout 数值等价但格式不同）
fn main() -> Unit
    println({n}.0 * 1000000000000000000000000000000.0)
end
"""
    if kind == "deep-recursion":
        # 分歧 6：深递归——解释器 RUNTIME000 结构化诊断 exit 1 vs WASM V8 trap
        depth = 200000
        return f"""# probe: deep-recursion（§11f 分歧 6——预期双侧失败但形态不同）
fn down(n: Int) -> Int
    down(n + 1)
end

fn main() -> Unit
    println("start")
    println(down(0))
end
"""
    raise SystemExit("unknown probe kind: %s (mut-capture|div-zero|large-float|deep-recursion)" % kind)


def main():
    ap = argparse.ArgumentParser(description="双后端差分测试程序生成器（D 工作包）")
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--count", type=int, default=10)
    ap.add_argument("--out", default=".diff_gen")
    ap.add_argument("--probe", default=None,
                    help="探针模式：mut-capture|div-zero|large-float|deep-recursion")
    args = ap.parse_args()

    os.makedirs(args.out, exist_ok=True)
    if args.probe:
        text = gen_probe(args.probe, args.seed)
        path = os.path.join(args.out, "probe_%s.lom" % args.probe)
        with open(path, "w", encoding="utf-8", newline="\n") as f:
            f.write(text)
        print("probe written: %s" % path)
        return
    for i in range(args.count):
        seed = args.seed + i
        text, _ = gen_program(seed)
        path = os.path.join(args.out, "g%05d.lom" % seed)
        with open(path, "w", encoding="utf-8", newline="\n") as f:
            f.write(text)
    print("generated %d programs (seed %d..%d) in %s" % (args.count, args.seed, args.seed + args.count - 1, args.out))


if __name__ == "__main__":
    main()
