#!/usr/bin/env python3
# tools/diff_gen.py — 双后端差分测试：随机 Lom 程序生成器（D 工作包 D1，2026-09-14）
#
# 设计（docs/TODO.md D 工作包登记）：
# - 类型感知的模板化生成：每个模板是一个手写正确的函数骨架（返回类型固定、
#   语法/类型/效应标注全部对齐 SPEC_FOR_AI），随机性在模板选择 + 参数填充
#   （常量/边界/分支数/变量名）上——不生成"自由拼接"的语法，保证零 parse/ type Error。
# - 确定性输出：纯计算 + println 为主；file/env 形态按纪律生成（file 三件套
#   "覆盖写先行"幂等闭环——append/exists 只在 write 重置基线后发生；env 只消费
#   args[1..] 永不触 argv[0]，见 docs/TODO.md D 包三期 D8），同一程序双后端
#   输出必须逐字一致。
# - 默认避开 SPEC_FOR_AI §11f 八条已知分歧形态：
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
# - --probe <kind> 探针模式：显式生成已知分歧形态（六类：mut-capture /
#   div-zero / large-float / json-number / int-range / deep-recursion，
#   全集见 tools/diff_test.py run_probes），供 diff_test 验证 §11f 白名单
#   仍然如档案所述（守护白名单本身不腐坏）。deep-recursion 探针验证双后端
#   结构化诊断 vs V8 trap——退出码都非 0，stdout 应一致为空/前缀。
# - 固定种子可复现（random.Random(seed)）；零第三方依赖（Python 标准库）。
# - 数值安全：Int 结果 |x| < 2^59（对齐 §11f-7 的 WASM 安全值域；乘法操作数 ≤10^4、
#   链深 ≤3、阶乘 n ≤15）；解释器全 i64 无此限，保守口径取双后端交集。
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
        # 变体名带 uniq 编号：同模板重复抽中时，同名变体跨 enum 会共享全局
        # 变体 idx（构造点按名解析到最后定义的那个），产生 TYPE003 warning 与
        # 类型歧义形态（D 三期顺手消除——变体名全局唯一是零歧义写法；
        # 实测双后端行为一致，非 bug，纯生成质量收敛）
        p = self.n
        cir, sq, rect, dot = "Cir_%d" % p, "Sq_%d" % p, "Rect_%d" % p, "Dot_%d" % p
        r = self.rng.choice([1.5, 2.0, 2.5, 3.0, 4.0])
        enum = f"""enum {en}
    | {cir}(Float)
    | {sq}(Float)
    | {rect}(Float, Float)
    | {dot}
end"""
        fn = f"""fn {v}(s: {en}) -> Float
    match s
        {cir}(r) => 3.0 * r * r
        {sq}(side) => side * side
        {rect}(w, h) => w * h
        {dot} => 0.0
    end
end"""
        calls = [
            f"println({v}({cir}({r})))",
            f"println({v}({sq}({r + 1.0})))",
            f"println({v}({rect}({r}, {r + 2.0})))",
            f"println({v}({dot}))",
        ]
        self.emit(enum + "\n\n" + fn, calls)

    def t_enum_guard_match(self):
        en = self.uniq("Level")
        v = self.uniq("lv")
        p = self.n
        low, mid, high, top = "Low_%d" % p, "Mid_%d" % p, "High_%d" % p, "Top_%d" % p
        k = self.rng.randint(0, 99)
        enum = f"enum {en} = {low} | {mid} | {high} | {top}"
        fn = f"""fn {v}(l: {en}, x: Int) -> String
    match l
        {low} if x < {max(k, 1)} => "low-small"
        {low} => "low-big"
        {mid} if x % 2 == 0 => "mid-even"
        {mid} => "mid-odd"
        {high} => "high"
        _ => "top-or-other"
    end
end"""
        calls = [
            f'println({v}({low}, {k}))',
            f'println({v}({low}, {k + 50}))',
            f'println({v}({mid}, {k}))',
            f'println({v}({mid}, {k + 1}))',
            f'println({v}({high}, {k}))',
            f'println({v}({top}, {k}))',
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

    # ===== 模板族 L：JSON（D6，2026-09-14）=====
    # 分歧 2 边界（D6a 实测）：安全区 = 纯整数字面量（无小数点/指数）+ 真小数值；
    # 分歧区 = "源语法 Float 但 JS 值为整"（30.0 / 1e2 / -0.0 / 0.0）——模板只
    # 用安全区数字，分歧形态由 --probe json-number 验证白名单。

    def t_json_parse_consume(self):
        v = self.uniq("jp")
        n1 = self.rng.randint(-99, 99)
        i1, i2 = self.rng.randint(0, 50), self.rng.randint(0, 50)
        f1 = self.rng.choice(["0.5", "1.25", "2.75", "3.5"])
        s1 = self.rng.choice(STR_ATOMS)
        doc = '{{"k1": {n1}, "name": "{s1}", "xs": [{i1}, {i2}, {f1}]}}'.format(
            n1=n1, s1=s1, i1=i1, i2=i2, f1=f1)
        # Lom 源内字符串转义：JSON 文档的引号写成 \"
        doc_lit = doc.replace('"', '\\"')
        fn = f'''fn {v}() -> Int
    let d = json_parse("{doc_lit}")
    let a = d.k1
    let xs = d.xs
    a + len(d.name) + list_length(xs) + list_get(xs, 0) + list_get(xs, 1)
end'''
        calls = [f'println({v}())']
        self.emit(fn, calls)

    def t_json_roundtrip(self):
        v = self.uniq("jr")
        n1 = self.rng.randint(-99, 99)
        i1, i2 = self.rng.randint(0, 50), self.rng.randint(0, 50)
        f1 = self.rng.choice(["0.5", "1.25", "2.75"])
        b1 = self.rng.choice(["True", "False"])  # Lom 布尔字面量大写（JSON 思维小写会 NAM003）
        s1 = self.rng.choice(STR_ATOMS)
        # 注意：宿主无 [..] List 字面量（list_demo 头注释），List 用 range 构造
        fn = f'''fn {v}() -> String
    let xs = {i1}..({i2} + 1)
    let d = {{ a: {n1}, tag: "{s1}", flags: {{ on: {b1} }}, tail: {f1}, xs: xs }}
    json_stringify(d)
end'''
        calls = [f'println({v}())']
        self.emit(fn, calls)

    def t_json_nested(self):
        v = self.uniq("jn")
        n1 = self.rng.randint(-50, 50)
        i1, i2 = self.rng.randint(0, 40), self.rng.randint(0, 40)
        f1 = self.rng.choice(["0.25", "0.75", "1.5"])
        doc_lit = '{{"outer": {{"inner": {{"v": {n1}, "xs": [{i1}, {i2}, {f1}]}}}}}}'.format(
            n1=n1, i1=i1, i2=i2, f1=f1).replace('"', '\\"')
        fn = f'''fn {v}() -> Int
    let d = json_parse("{doc_lit}")
    let inner = d.outer.inner
    let rt = json_stringify(inner)
    inner.v + list_get(inner.xs, 1) + len(rt)
end'''
        calls = [f'println({v}())']
        self.emit(fn, calls)


    # ===== 模板族 M：递归梯度（D7，2026-09-14）=====
    # 深度档位 <=8000：解释器 80k 软件守卫与 V8 默认栈（~1-3 万层）的安全带
    # 之内；更深形态属 §11f-6 已知分歧（int-range 同理不生成），由探针验证。

    def t_mutual_recursion(self):
        # 相互递归 a <-> b（even/odd 式）
        v1, v2 = self.uniq("ev"), self.uniq("od")
        n = self.rng.choice([11, 57, 433, 1001, 2500, 7997])
        fn = f"""fn {v1}(n: Int) -> Bool
    if n == 0
        True
    else
        {v2}(n - 1)
    end
end

fn {v2}(n: Int) -> Bool
    if n == 0
        False
    else
        {v1}(n - 1)
    end
end"""
        calls = [f'println({v1}({n}))', f'println({v2}({n}))']
        self.emit(fn, calls)

    def t_deep_recursion_gradient(self):
        # 深度梯度档位（百/千/八千级），值域守恒：n(n+1)/2 <= 8000*8001/2 < 2^59
        v = self.uniq("sd")
        n = self.rng.choice([100, 500, 1000, 2000, 4000, 8000])
        fn = f"""fn {v}(n: Int) -> Int
    if n == 0
        0
    else
        n + {v}(n - 1)
    end
end"""
        calls = [f'println({v}({n}))', f'println({v}({n // 7}))']
        self.emit(fn, calls)

    def t_recursion_early_return(self):
        # 递归体内的提前返回（命中即停 vs 沉降到底的对照）
        v = self.uniq("find")
        n = self.rng.choice([7, 33, 121, 500, 1500])
        k = self.rng.choice([3, 5, 7])
        fn = f"""fn {v}(n: Int) -> Int
    if n == 0
        0 - 1
    else
        if n % {k} == 0
            return n
        end
        {v}(n - 1)
    end
end"""
        calls = [f"println({v}({n}))", f"println({v}({n * 2 + 1}))"]
        self.emit(fn, calls)


    # ===== 模板族 N：file/env/深控制流/import 深形态（D8，D 包三期 2026-09-15）=====
    # file 三件套的确定性纪律：只生成"覆盖写先行重置基线"的幂等闭环——
    # 对拍顺序 interp 先跑、wasm 后跑，任何"裸 append/裸 exists"形态都会因
    # interp 侧留下的文件状态使两侧起点不同（§11.0 两侧同起点教训）。

    def t_file_roundtrip(self):
        # 覆盖写 → 读回 → 长度消费（file 四件套的核心路径；文件名 uniq，
        # write 先行保证同 seed 复跑与双后端顺序无关）
        v = self.uniq("frt")
        k = self.uniq("fk")
        fn = f"""fn {v}() -> Int ! [IO]
    let _ = file_write(".diff_tmp/{k}.txt", "base-{k}-content")
    let c = file_read(".diff_tmp/{k}.txt")
    len(c)
end"""
        calls = [f"println({v}())"]
        self.emit(fn, calls)

    def t_file_exists_gated(self):
        # 写后 exists=True 与"永不写入的唯一名"exists=False 的对照分支
        v = self.uniq("fex")
        k = self.uniq("fk")
        a = self.uniq("absent")
        fn = f"""fn {v}() -> Int ! [IO]
    let _ = file_write(".diff_tmp/{k}.txt", "x")
    let hit = file_exists(".diff_tmp/{k}.txt")
    let miss = file_exists(".diff_tmp/{a}.txt")
    if hit and !miss
        1
    else
        0
    end
end"""
        calls = [f"println({v}())"]
        self.emit(fn, calls)

    def t_file_append_accum(self):
        # 覆盖基线 → append 两次 → 读长（append 只在 write 重置后发生）
        v = self.uniq("fap")
        k = self.uniq("fk")
        fn = f"""fn {v}() -> Int ! [IO]
    let _ = file_write(".diff_tmp/{k}.txt", "head")
    let _ = file_append(".diff_tmp/{k}.txt", "-mid")
    let _ = file_append(".diff_tmp/{k}.txt", "-tail")
    let c = file_read(".diff_tmp/{k}.txt")
    len(c)
end"""
        calls = [f"println({v}())"]
        self.emit(fn, calls)

    def t_env_args_consume(self):
        # args() 尾部用户参数消费——diff_test 以 `-- d1 d2` 固定调用，两侧
        # 尾部逐字一致；argv[0] 双后端是不同路径（.lom vs .wasm，§11f-8
        # 结构性差异），模板永不消费 args[0]
        v = self.uniq("ea")
        fn = f"""fn {v}() -> Int ! [IO]
    let a = args()
    let n = list_length(a)
    let last = list_get(a, n - 1)
    let second = list_get(a, 1)
    n * 100 + len(last) * 10 + len(second)
end"""
        calls = [f"println({v}())"]
        self.emit(fn, calls)

    def t_while_match_return(self):
        # while × match 表达式值绑定 × Form B 臂内 if+return（三层控制流嵌套，
        # W 包 label 栈纪律的深覆盖）；Form B 臂独立 end（§4.1）
        v = self.uniq("wm")
        n = self.rng.randint(3, 12)
        stop = self.rng.randint(1, 4)
        fn = f"""fn {v}(n: Int, stop: Int) -> Int
    let mut total = 0
    let mut i = 0
    while i < n
        let d = match i % 3
            0 => 10
            1 => 20
            _ => 30
        end
        total += d
        match i % 4
            2 =>
                if i >= {stop}
                    return i * 1000 + {stop}
                end
            end
            _ => ()
        end
        i += 1
    end
    total
end"""
        calls = [f"println({v}({n}, {stop}))", f"println({v}({n}, 99))"]
        self.emit(fn, calls)

    def t_nested_for_closure(self):
        # 外层循环变量的不可变别名被内层闭包捕获（避开分歧 1 的 mut 捕获；
        # 闭包在循环内多次创建，每次捕获当轮值）
        v = self.uniq("nfc")
        a = self.rng.randint(2, 5)
        b = self.rng.randint(2, 5)
        fn = f"""fn {v}(a: Int, b: Int) -> Int
    let mut total = 0
    for i in a
        let base = i * 10
        let add_base = fn(x: Int) -> Int
            x + base
        end
        for j in b
            total += add_base(j)
        end
    end
    total
end"""
        calls = [f"println({v}({a}, {b}))"]
        self.emit(fn, calls)

    def t_enum_cross_match(self):
        # 两个 enum 的交叉嵌套 match（Form B 臂内 match 另一 enum——
        # §4.1 多 end 计数的高危形态定向覆盖）；变体名 uniq（D 三期纪律）
        e1 = self.uniq("Cx1")
        e2 = self.uniq("Cx2")
        v = self.uniq("xm")
        p = self.n
        enum1 = f"enum {e1} = A_{p} | B_{p} | C_{p}"
        enum2 = f"""enum {e2}
    | P_{p}(Int)
    | Q_{p}(Int, Int)
    | R_{p}
end"""
        x, y = self.rng.randint(0, 9), self.rng.randint(0, 9)
        fn = f"""fn {v}(a: {e1}, b: {e2}) -> Int
    match a
        A_{p} =>
            match b
                P_{p}(x) => x + 1
                Q_{p}(x, y) => x + y
                R_{p} => 0
            end
        end
        B_{p} => 100
        _ => 200
    end
end"""
        calls = [
            f"println({v}(A_{p}, P_{p}({x})))",
            f"println({v}(A_{p}, Q_{p}({x}, {y})))",
            f"println({v}(A_{p}, R_{p}))",
            f"println({v}(B_{p}, R_{p}))",
            f"println({v}(C_{p}, P_{p}({x})))",
        ]
        self.emit(enum1 + "\n\n" + enum2 + "\n\n" + fn, calls)

    def t_try_in_while(self):
        # while 体内 `?` 提前返回（t_try_in_for 的 while 版——label 栈的
        # 另一条降级路径）
        v = self.uniq("tw")
        w = self.uniq("whalf")
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
    let mut i = 0
    while i < n
        let h = {w}(i)?
        if h > 1
            return Ok(h * 50)
        end
        i += 1
    end
    Ok(0)
end"""
        calls = [f'println({v}({n}))', f'println({v}({bad}))']
        self.emit(fn_helper + "\n\n" + fn, calls)

    def t_mixed_iteration(self):
        # for-over-String 外层 × for-over-List 内层（两种迭代协议的混合嵌套）
        v = self.uniq("mi")
        s = self.rng.choice(["alpha", "banana", "foobar", "lomlang"])
        n = self.rng.randint(2, 5)
        fn = f"""fn {v}(s: String, n: Int) -> Int
    let mut total = 0
    for c in s
        if c == "a" or c == "o"
            for j in 1..{n}
                total += j
            end
        else
            total += 1
        end
    end
    total
end"""
        calls = [f'println({v}("{s}", {n}))']
        self.emit(fn, calls)

    def t_import_alias_stdlib(self):
        # stdlib 符号别名导入（v1.1.4 修的是包符号别名路径；stdlib 别名在
        # 单文件内的独立覆盖）——别名 import 行随模板 decls 走（非文件头）
        v = self.uniq("ia")
        s = self.rng.choice(STR_ATOMS)
        decl = f"""from string import {{ len as length_of, upper as up_case }}

fn {v}(s: String) -> Int
    length_of(up_case(s)) * 10 + length_of(s)
end"""
        calls = [f'println({v}("{s}"))']
        self.emit(decl, calls)

    def t_recursion_accumulator(self):
        # 递归带累积参数（acc 模式）+ 命中阈值提前返回（与沉降到底对照）
        v = self.uniq("racc")
        n = self.rng.randint(5, 40)
        stop = self.rng.randint(20, 200)
        fn = f"""fn {v}(n: Int, acc: Int, stop: Int) -> Int
    if n == 0
        acc
    else
        if acc > stop
            return acc * 100
        end
        {v}(n - 1, acc + n, stop)
    end
end"""
        calls = [f"println({v}({n}, 0, {stop}))", f"println({v}({n}, 0, 99999))"]
        self.emit(fn, calls)


    # ===== 模板族 O：字符串/数值深水（D9-a，D 包四期 2026-09-15）=====
    # 内建核对（interpreter module_of "string"）：len/int_to_string/string_to_int/
    # trim/upper/lower/split/contains/replace/starts_with/ends_with/char_from_code。
    # 数值纪律：除数非零字面量；Int 结果 |x| < 2^59（§11f-7）；ASCII only（分歧 4）。

    def t_string_contains_chain(self):
        v = self.uniq("scc")
        s = self.rng.choice(["hello-world", "lom-lang-test", "abc-def-ghi", "xx-yy"])
        fn = f"""fn {v}(s: String) -> String
    let a = starts_with(s, "h") or starts_with(s, "l")
    let b = ends_with(s, "t") or ends_with(s, "i")
    let c = contains(s, "-")
    if a and b and c
        "full"
    elif c and !a
        "mid-only"
    else
        "none"
    end
end"""
        calls = [f'println({v}("{s}"))', f'println({v}("plain"))']
        self.emit(fn, calls)

    def t_string_replace_ops(self):
        v = self.uniq("sro")
        s = self.rng.choice(["a-b-c", "x.y.z", "p_q_r", "m-n-o-p"])
        sep = s[1]
        fn = f"""fn {v}(s: String) -> Int
    let r1 = replace(s, "{sep}", "+")
    let r2 = replace(r1, "+{sep}", "++")
    len(r1) * 100 + len(r2) * 10 + len(s)
end"""
        calls = [f'println({v}("{s}"))']
        self.emit(fn, calls)

    def t_string_split_nested(self):
        # split 两层消费：外层 "-" 分段，段内 "," 计数累积
        v = self.uniq("ssn")
        segs = [
            "aa,bb,cc-dd,ee-ff",
            "x,xx-y,zzaa-bb,cc,dd",
            "p,qr-s,tu-vw,xyz",
        ]
        s = self.rng.choice(segs)
        fn = f"""fn {v}(s: String) -> Int
    let parts = split(s, "-")
    let mut total = 0
    for p in parts
        let sub = split(p, ",")
        total += list_length(sub) * 10 + len(p)
    end
    total
end"""
        calls = [f'println({v}("{s}"))']
        self.emit(fn, calls)

    def t_char_from_code_ops(self):
        # char_from_code 码点构造（33-126 ASCII 安全区）+ 相等/长度消费
        v = self.uniq("cfc")
        c1 = self.rng.randint(65, 90)
        c2 = self.rng.randint(97, 122)
        fn = f"""fn {v}(a: Int, b: Int) -> Int
    let x = char_from_code(a)
    let y = char_from_code(b)
    let joined = x + y + x
    let unit = if len(x) == 1 and len(y) == 1
        5
    else
        0
    end
    if x == y
        1
    else
        len(joined) * 10 + unit
    end
end"""
        calls = [f"println({v}({c1}, {c2}))", f"println({v}(65, 65))"]
        self.emit(fn, calls)

    def t_string_lex_compare(self):
        v = self.uniq("slc")
        a = self.rng.choice(STR_ATOMS)
        b = self.rng.choice(STR_ATOMS)
        fn = f"""fn {v}(a: String, b: String) -> String
    if a < b
        if b < a
            "unreachable"
        else
            "lt"
        end
    elif a > b
        "gt"
    else
        "eq"
    end
end"""
        calls = [f'println({v}("{a}", "{b}"))', f'println({v}("same", "same"))']
        self.emit(fn, calls)

    def t_string_case_pipeline(self):
        v = self.uniq("scp")
        s = self.rng.choice(["  MiXeD  ", " TeSt ", "  aBcDeF  ", " LoM-LaNg "])
        fn = f"""fn {v}(s: String) -> Int
    let t = trim(s)
    let u = upper(t)
    let l = lower(t)
    let base = if starts_with(u, "M") or starts_with(u, "T") or starts_with(u, "A") or starts_with(u, "L")
        10
    else
        0
    end
    base + len(u) + len(l) * 2 + len(t) * 3
end"""
        calls = [f'println({v}("{s}"))']
        self.emit(fn, calls)

    def t_int_negative_ops(self):
        v = self.uniq("ino")
        a = self.rng.randint(10, 999)
        b = self.rng.randint(2, 9)
        fn = f"""fn {v}(a: Int, b: Int) -> Int
    let n = 0 - a
    let q1 = n / {b}
    let q2 = (0 - n) / {b}
    let m = n % {b}
    let tag = if q1 <= 0
        1
    else
        2
    end
    q1 * 1000 + q2 * 100 + m * 10 + tag
end"""
        calls = [f"println({v}({a}, {b}))", f"println({v}(0, {b}))"]
        self.emit(fn, calls)

    def t_int_divmod_identity(self):
        # 代数恒等式 (a/b)*b + a%b == a 的双后端验证（截断除法双侧一致；
        # 负数域含负模——Rust/WASM i64 均向零截断）
        v = self.uniq("idi")
        a = self.rng.choice([self.rng.randint(100, 9999), -self.rng.randint(100, 9999)])
        b = self.rng.choice([3, 7, 12, 25, 99])
        fn = f"""fn {v}(a: Int, b: Int) -> Int
    let lhs = (a / b) * b + a % b
    if lhs == a
        1
    else
        0 - 1
    end
end"""
        calls = [f"println({v}({a}, {b}))", f"println({v}(0 - {abs(a)}, {b}))",
                 f"println({v}({abs(a)}, 0 - {b}))"]
        self.emit(fn, calls)

    def t_int_mul_boundary(self):
        # 2^50 量级乘法链（结果守恒 < 2^59，§11f-7 安全值域内的高位压力）
        v = self.uniq("imb")
        k = self.rng.randint(2, 31)
        fn = f"""fn {v}(k: Int) -> Int
    let base = 1125899906842624
    let scaled = base / {k}
    let back = scaled * {k}
    let adj = back + base % {k}
    if adj == base
        (scaled % 1000000007) + adj / 1000000000000000
    else
        0 - 1
    end
end"""
        calls = [f"println({v}({k}))", f"println({v}(1))"]
        self.emit(fn, calls)

    def t_int_sign_mixed(self):
        v = self.uniq("ism")
        a = self.rng.choice(INT_ATOMS)
        b = self.rng.choice(INT_ATOMS)
        fn = f"""fn {v}(a: Int, b: Int) -> Int
    let mut acc = 0
    if a > 0 and b < 0
        acc = a * (0 - 1) + b
    elif a < 0 and b > 0
        acc = b * 2 + a
    elif a == b
        acc = a + b
    else
        acc = a - b
    end
    let sign = if acc < 0
        0 - 1
    elif acc > 0
        1
    else
        0
    end
    let mag = if acc < 0
        0 - acc % 97
    else
        acc % 97
    end
    sign * 1000 + mag
end"""
        calls = [f"println({v}({a}, {b}))", f"println({v}(0, 0))"]
        self.emit(fn, calls)


    # ===== 模板族 P：List/Map/Record/Tuple 深组合（D9-b，D 包四期 2026-09-15）=====
    # Map 双后端均为引用语义（解释器 Rc<RefCell> / WASM 手写开放寻址堆对象），
    # 无已知分歧；map_keys/values 按键排序保确定性（5.20 纪律）。

    def t_list_hof_chain(self):
        # 四连链：map → filter → map → fold（HOF 组合深度）
        v = self.uniq("lhc")
        n = self.rng.randint(5, 12)
        fn = f"""fn {v}(n: Int) -> Int
    let xs = 1..n
    let ys = list_map(fn(x: Int) -> Int
        x * 3 + 1
    end, xs)
    let zs = list_filter(fn(x: Int) -> Bool
        x % 2 == 0
    end, ys)
    let ws = list_map(fn(x: Int) -> Int
        x / 2
    end, zs)
    list_fold(fn(acc: Int, x: Int) -> Int
        acc + x
    end, 0, ws) + list_length(xs) * 100 + list_length(zs)
end"""
        calls = [f"println({v}({n}))"]
        self.emit(fn, calls)

    def t_list_head_tail_recursion(self):
        # head/tail 递归求和与计数（结构递归经典形态；深度 = List 长度 ≤ 30）
        v = self.uniq("htr")
        w = self.uniq("cnt")
        n = self.rng.randint(4, 30)
        fn = f"""fn {v}(xs: List<Int>) -> Int
    if list_length(xs) == 0
        0
    else
        list_head(xs) + {v}(list_tail(xs))
    end
end

fn {w}(xs: List<Int>) -> Int
    if list_length(xs) == 0
        0
    else
        1 + {w}(list_tail(xs))
    end
end"""
        calls = [f"println({v}(1..{n}))", f"println({w}(1..{n}))",
                 f"println({v}(list_cons(99, 1..3)))"]
        self.emit(fn, calls)

    def t_list_closure_capture(self):
        # 闭包捕获 List（不可变绑定）+ 捕获 Int 阈值的组合
        v = self.uniq("lcc")
        n = self.rng.randint(4, 10)
        k = self.rng.randint(2, 5)
        fn = f"""fn {v}(n: Int, k: Int) -> Int
    let xs = 1..n
    let threshold = k * 2
    let count_over = fn(lst: List<Int>) -> Int
        list_fold(fn(acc: Int, x: Int) -> Int
            if x > threshold
                acc + 1
            else
                acc
            end
        end, 0, lst)
    end
    count_over(xs) * 10 + count_over(list_tail(xs))
end"""
        calls = [f"println({v}({n}, {k}))"]
        self.emit(fn, calls)

    def t_list_index_random(self):
        # list_get 多处随机访问（O(n) 走查路径的重复触发）
        v = self.uniq("lir")
        n = self.rng.randint(6, 15)
        i1 = self.rng.randint(0, n - 1)
        i2 = self.rng.randint(0, n - 1)
        fn = f"""fn {v}(n: Int) -> Int
    let xs = 10..(10 + n)
    list_get(xs, {i1}) * 100 + list_get(xs, {i2}) * 10 + list_get(xs, n - 1)
end"""
        calls = [f"println({v}({n}))", f"println({v}({n + 3}))"]
        self.emit(fn, calls)

    def t_list_nested_cons(self):
        # 多层 cons 嵌套构造 + 逐层展开（头插方向性验证）
        v = self.uniq("lnc")
        a, b, c = self.rng.randint(1, 50), self.rng.randint(1, 50), self.rng.randint(1, 50)
        fn = f"""fn {v}(a: Int, b: Int, c: Int) -> Int
    let l1 = list_cons(a, 1..3)
    let l2 = list_cons(b, l1)
    let l3 = list_cons(c, l2)
    list_head(l3) * 10000 + list_head(list_tail(l3)) * 1000
        + list_head(list_tail(list_tail(l3))) * 100 + list_length(l3)
end"""
        calls = [f"println({v}({a}, {b}, {c}))"]
        self.emit(fn, calls)

    def t_map_nested(self):
        # Map 值是 Map：嵌套 map_get 链（两层）
        v = self.uniq("mn")
        k1 = self.rng.randint(1, 9)
        k2 = self.rng.randint(1, 9)
        fn = f"""fn {v}(a: Int, b: Int) -> Int
    let inner = map_empty()
    let _ = map_set(inner, "x", a)
    let _ = map_set(inner, "y", b)
    let outer = map_empty()
    let _ = map_set(outer, "in", inner)
    let _ = map_set(outer, "tag", 7)
    let got = map_get(outer, "in")
    match got
        Some(m) =>
            let xv = map_get(m, "x")
            match xv
                Some(n) => n * 10 + map_size(outer)
                None => 0 - 1
            end
        end
        None => 0 - 2
    end
end"""
        calls = [f"println({v}({k1}, {k2}))", f"println({v}(0, 0))"]
        self.emit(fn, calls)

    def t_map_reference_alias(self):
        # Map 引用语义：let 别名共享，经别名 set 经本名读（双后端均引用语义）
        v = self.uniq("mra")
        k = self.rng.randint(1, 99)
        fn = f"""fn {v}(k: Int) -> Int
    let m1 = map_empty()
    let _ = map_set(m1, "base", 10)
    let m2 = m1
    let _ = map_set(m2, "added", k)
    let _ = map_set(m1, "base", 20)
    match map_get(m2, "base")
        Some(b) => b * 100 + map_size(m1) * 10 + (if map_has(m1, "added")
            1
        else
            0
        end)
        None => 0 - 1
    end
end"""
        calls = [f"println({v}({k}))"]
        self.emit(fn, calls)

    def t_map_sorted_keys_consume(self):
        # map_keys 按键排序消费（确定性遍历）
        v = self.uniq("msk")
        fn = f"""fn {v}() -> Int
    let m = map_empty()
    let _ = map_set(m, "zeta", 1)
    let _ = map_set(m, "alpha", 2)
    let _ = map_set(m, "mid", 3)
    let _ = map_set(m, "beta", 4)
    let mut total = 0
    let mut idx = 0
    for k in map_keys(m)
        total += idx * 10 + len(k)
        idx += 1
    end
    total * 100 + map_size(m) + list_length(map_keys(m))
end"""
        calls = [f"println({v}())"]
        self.emit(fn, calls)

    def t_map_remove_rehash(self):
        # set 多键 → remove 中间 → set 新键 → 再读（墓碑与探测路径压力）
        v = self.uniq("mrr")
        k = self.rng.randint(1, 6)
        fn = f"""fn {v}(k: Int) -> Int
    let m = map_empty()
    let mut i = 1
    while i <= 9
        let _ = map_set(m, "k" + int_to_string(i * 7), i * i)
        i += 1
    end
    let mut j = 1
    while j <= 5
        map_remove(m, "k" + int_to_string(j * 7))
        j += 1
    end
    let _ = map_set(m, "new1", 100)
    let _ = map_set(m, "new2", 200)
    match map_get(m, "k" + int_to_string({k} * 7))
        Some(x) => x * 10 + map_size(m)
        None => map_size(m) * 100 + (if map_has(m, "new2")
            5
        else
            6
        end)
    end
end"""
        calls = [f"println({v}({k}))", f"println({v}(9))"]
        self.emit(fn, calls)

    def t_record_rebuild_chain(self):
        # Record 三级嵌套重建链（不可变更新模式）
        v = self.uniq("rrc")
        a = self.rng.randint(-9, 9)
        fn = f"""fn {v}(a: Int) -> Int
    let r1 = {{v: a, w: 1}}
    let r2 = {{inner: r1, tag: "mid"}}
    let r3 = {{holder: r2, top: r2.inner.v * 2}}
    let rebuilt = {{holder: {{inner: {{v: r3.top + 1, w: r3.holder.inner.w + 1}}, tag: r3.holder.tag}}, top: r3.top}}
    rebuilt.holder.inner.v * 1000 + rebuilt.holder.inner.w * 100
        + rebuilt.top + len(r3.holder.tag)
end"""
        calls = [f"println({v}({a}))"]
        self.emit(fn, calls)

    def t_tuple_destructure_swap(self):
        # 元组解构交换 + .0/.1 访问 + 嵌套元组（经中间变量——链式 t3.0.0 是
        # PARSE001：'.0' 与浮点词法歧义，语言面不支持链式元组下标，见 §4.5b）
        v = self.uniq("tds")
        a = self.rng.randint(1, 99)
        b = self.rng.randint(1, 99)
        fn = f"""fn {v}(a: Int, b: Int) -> Int
    let t1 = (a, b)
    let (x, y) = t1
    let t2 = (y, x)
    let t3 = (t1, t2, x + y)
    let fst = t3.0
    let snd = t3.1
    fst.0 * 1000 + snd.0 * 100 + t3.2 * 10 + len(int_to_string(snd.1))
end"""
        calls = [f"println({v}({a}, {b}))"]
        self.emit(fn, calls)

    def t_record_tuple_eq(self):
        # 结构相等组合：Record == Record（字段序无关）/ Tuple == Tuple
        v = self.uniq("rte")
        x, y = self.rng.randint(-50, 50), self.rng.randint(-50, 50)
        fn = f"""fn {v}(x: Int, y: Int) -> Int
    let p = {{x: x, y: y}}
    let q = {{y: y, x: x}}
    let t = (x, y)
    let u = (x, y)
    let score = if p == q
        1
    else
        0
    end
    let score2 = if t == u
        score + 10
    else
        score
    end
    let mixed = if (x, y) == (x, y + 1)
        score2 + 100
    else
        score2 + 200
    end
    mixed + p.x + q.y
end"""
        calls = [f"println({v}({x}, {y}))"]
        self.emit(fn, calls)


    # ===== 模板族 Q：enum 深水（D9-c，D 包四期 2026-09-15）=====
    # 变体名一律 uniq 编号（D 三期纪律）；递归 enum 深度控制 ≤40（栈安全带）。

    def t_enum_wide_variant(self):
        # 3-4 字段宽变体 + 多变体分派
        en = self.uniq("Wide")
        v = self.uniq("wd")
        p = self.n
        tri, quad = "Tri_%d" % p, "Quad_%d" % p
        one, nil = "One_%d" % p, "Nil_%d" % p
        a, b, c, d = (self.rng.randint(1, 30) for _ in range(4))
        enum = f"""enum {en}
    | {tri}(Int, Int, Int)
    | {quad}(Int, Int, Int, Int)
    | {one}(Int)
    | {nil}
end"""
        fn = f"""fn {v}(e: {en}) -> Int
    match e
        {tri}(a, b, c) => a * 100 + b * 10 + c
        {quad}(a, b, c, d) => a + b * 2 + c * 3 + d * 4
        {one}(x) => x * 1000
        {nil} => 0 - 1
    end
end"""
        calls = [
            f"println({v}({tri}({a}, {b}, {c})))",
            f"println({v}({quad}({a}, {b}, {c}, {d})))",
            f"println({v}({one}({a})))",
            f"println({v}({nil}))",
        ]
        self.emit(enum + "\n\n" + fn, calls)

    def t_enum_payload_enum(self):
        # 变体载荷是另一个 enum（嵌套 enum 分派）
        inner = self.uniq("In")
        outer = self.uniq("Out")
        v = self.uniq("ne")
        p = self.n
        ia, ib = "Ia_%d" % p, "Ib_%d" % p
        oa, ob = "Wrap_%d" % p, "Plain_%d" % p
        k = self.rng.randint(1, 99)
        enum_i = f"enum {inner} = {ia} | {ib}"
        enum_o = f"""enum {outer}
    | {oa}({inner})
    | {ob}(Int)
end"""
        fn = f"""fn {v}(o: {outer}) -> Int
    match o
        {oa}(i) =>
            match i
                {ia} => 10
                {ib} => 20
            end
        end
        {ob}(n) => n * 3
    end
end"""
        calls = [
            f"println({v}({oa}({ia})))",
            f"println({v}({oa}({ib})))",
            f"println({v}({ob}({k})))",
        ]
        self.emit(enum_i + "\n\n" + enum_o + "\n\n" + fn, calls)

    def t_enum_recursive_tree(self):
        # 递归 enum（AST 式树形——stmt_interp 同款形态的差分覆盖）：
        # Lit / Neg / Add 节点，eval 递归求值，深度 ≤12
        en = self.uniq("Expr")
        v = self.uniq("ev")
        lit, neg, add = "Lit_%d" % self.n, "Neg_%d" % self.n, "Add_%d" % self.n
        enum = f"""enum {en}
    | {lit}(Int)
    | {neg}({en})
    | {add}({en}, {en})
end"""
        fn = f"""fn {v}(e: {en}) -> Int
    match e
        {lit}(n) => n
        {neg}(inner) => 0 - {v}(inner)
        {add}(l, r) => {v}(l) + {v}(r)
    end
end"""
        # 固定形状的树形（深度 3-5，字面量嵌入；值域守恒）
        a, b, c = (self.rng.randint(1, 50) for _ in range(3))
        t1 = f"{add}({lit}({a}), {neg}({lit}({b})))"
        t2 = f"{neg}({add}({lit}({c}), {neg}({lit}({a}))))"
        t3 = f"{add}({add}({lit}({a}), {lit}({b})), {neg}({neg}({lit}({c}))))"
        calls = [
            f"println({v}({t1}))",
            f"println({v}({t2}))",
            f"println({v}({t3}))",
        ]
        self.emit(enum + "\n\n" + fn, calls)

    def t_enum_guard_chain(self):
        # 同变体多 guard 臂的穿透链（guard False 落到下一臂）
        en = self.uniq("Gc")
        v = self.uniq("gc")
        p = self.n
        lo, hi = "Lo_%d" % p, "Hi_%d" % p
        k = self.rng.randint(1, 99)
        enum = f"enum {en} = {lo} | {hi}"
        fn = f"""fn {v}(e: {en}, x: Int) -> String
    match e
        {lo} if x < {k} => "lo-a"
        {lo} if x < {k * 2 + 10} => "lo-b"
        {lo} => "lo-c"
        {hi} if x > {k + 50} => "hi-a"
        {hi} => "hi-b"
    end
end"""
        calls = [
            f'println({v}({lo}, 0))',
            f'println({v}({lo}, {k}))',
            f'println({v}({lo}, {k * 2 + 50}))',
            f'println({v}({hi}, {k + 100}))',
            f'println({v}({hi}, 0))',
        ]
        self.emit(enum + "\n\n" + fn, calls)

    def t_enum_nested_match_guard(self):
        # Form B 臂内嵌 match + guard 组合（控制流 × enum 交叉深水）
        en = self.uniq("Nm")
        v = self.uniq("nmg")
        p = self.n
        a1, a2 = "Na_%d" % p, "Nb_%d" % p
        k = self.rng.randint(1, 40)
        enum = f"enum {en} = {a1} | {a2}"
        x_mid = k * 3 + 1 + max(k, 1) * 2
        fn = f"""fn {v}(e: {en}, x: Int) -> Int
    match e
        {a1} =>
            match x % 3
                0 => x * 10
                1 if x > {k} => x * 100
                _ => x + 1
            end
        end
        {a2} =>
            if x % 2 == 0
                return x * 1000
            end
            0 - x
        end
    end
end"""
        calls = [
            f"println({v}({a1}, {k * 3}))",
            f"println({v}({a1}, {x_mid}))",
            f"println({v}({a1}, 2))",
            f"println({v}({a2}, 8))",
            f"println({v}({a2}, 7))",
        ]
        self.emit(enum + "\n\n" + fn, calls)

    def t_enum_build_consume_chain(self):
        # enum 值经函数链传递/重构（构造→变换→消费三段）
        en = self.uniq("Bc")
        mk = self.uniq("mk")
        xf = self.uniq("xf")
        v = self.uniq("run")
        p = self.n
        s1, s2, z0 = "B1_%d" % p, "B2_%d" % p, "Z_%d" % p
        k = self.rng.randint(1, 99)
        enum = f"""enum {en}
    | {s1}(Int)
    | {s2}(String)
    | {z0}
end"""
        fn_mk = f"""fn {mk}(n: Int) -> {en}
    if n % 3 == 0
        {s1}(n * 2)
    elif n % 3 == 1
        {s2}("m" + int_to_string(n))
    else
        {z0}
    end
end"""
        fn_xf = f"""fn {xf}(e: {en}) -> {en}
    match e
        {s1}(n) => {s1}(n + 1)
        {s2}(s) => {s2}(s + "x")
        {z0} => {s1}(7)
    end
end"""
        fn_v = f"""fn {v}(n: Int) -> Int
    let e1 = {mk}(n)
    let e2 = {xf}(e1)
    let e3 = {xf}(e2)
    match e3
        {s1}(n) => n * 10
        {s2}(s) => len(s) * 100
        {z0} => 0 - 1
    end
end"""
        calls = [f"println({v}({k}))", f"println({v}({k + 1}))", f"println({v}({k + 2}))"]
        self.emit(enum + "\n\n" + fn_mk + "\n\n" + fn_xf + "\n\n" + fn_v, calls)

    def t_enum_variant_dict(self):
        # enum → Map 载荷交换：变体解包写 Map 再读回（enum × 容器交叉）
        en = self.uniq("Vd")
        v = self.uniq("vd")
        p = self.n
        kv, ks = "Kv_%d" % p, "Ks_%d" % p
        a = self.rng.randint(1, 99)
        enum = f"""enum {en}
    | {kv}(Int)
    | {ks}(String)
end"""
        fn = f"""fn {v}(e: {en}) -> Int
    let m = map_empty()
    match e
        {kv}(n) =>
            let _ = map_set(m, "num", n * 3)
        end
        {ks}(s) =>
            let _ = map_set(m, "len", len(s))
            let _ = map_set(m, "tag", 1)
        end
    end
    match map_get(m, "num")
        Some(x) => x + map_size(m) * 10
        None =>
            match map_get(m, "len")
                Some(l) => l * 100 + map_size(m)
                None => 0 - 1
            end
        end
    end
end"""
        calls = [f"println({v}({kv}({a})))", f'println({v}({ks}("abcd")))', f'println({v}({ks}("x")))']
        self.emit(enum + "\n\n" + fn, calls)

    def t_enum_wildcard_positions(self):
        # 通配臂的多样位置：`_` 带 guard 中位/尾位兜底（绑定臂 other=> 不计
        # 穷尽性——§4.5b 已知坑，需穷尽时一律 `_` 兜底）
        en = self.uniq("Wp")
        v = self.uniq("wp")
        p = self.n
        w1, w2, w3 = "W1_%d" % p, "W2_%d" % p, "W3_%d" % p
        k = self.rng.randint(0, 99)
        enum = f"enum {en} = {w1} | {w2} | {w3}"
        fn = f"""fn {v}(e: {en}, x: Int) -> String
    match e
        {w1} => "first"
        _ if x > {max(k, 1)} => "wild-guard"
        {w2} => "second"
        _ => "rest"
    end
end"""
        calls = [
            f'println({v}({w1}, 0))',
            f'println({v}({w2}, {k + 10}))',
            f'println({v}({w2}, 0))',
            f'println({v}({w3}, 0))',
        ]
        self.emit(enum + "\n\n" + fn, calls)


    # ===== 模板族 R：闭包/HOF/管道深组合（D9-d，D 包四期 2026-09-15）=====

    def t_closure_return_closure2(self):
        # 三层闭包工厂：make(a)(b)(c) 链式调用（捕获层层嵌套）
        v = self.uniq("crc")
        a, b = self.rng.randint(1, 9), self.rng.randint(1, 9)
        fn = f"""fn {v}(a: Int) -> Fn
    fn(b: Int) -> Fn
        fn(c: Int) -> Int
            a * 100 + b * 10 + c
        end
    end
end"""
        calls = [f"println({v}({a})({b})(7))", f"println({v}({b})({a})(3))"]
        self.emit(fn, calls)

    def t_closure_as_arg(self):
        # 闭包作参数：用户 HOF apply_twice / apply_if + 组合调用
        v = self.uniq("caa")
        x = self.rng.randint(1, 30)
        fn = f"""fn {v}(x: Int) -> Int
    let apply_twice = fn(f: Fn, n: Int) -> Int
        f(f(n))
    end
    let apply_if = fn(cond: Bool, f: Fn, n: Int) -> Int
        if cond
            f(n)
        else
            n
        end
    end
    let inc = fn(n: Int) -> Int
        n + 1
    end
    let dbl = fn(n: Int) -> Int
        n * 2
    end
    apply_twice(inc, {x}) * 100 + apply_if({x % 2 == 0}, dbl, {x}) * 10 + apply_if(True, inc, 0)
end"""
        calls = [f"println({v}({x}))"]
        self.emit(fn, calls)

    def t_named_fn_as_value(self):
        # 具名函数当值（v0.4.2）：let 绑定 + 经参数传递给 HOF
        v = self.uniq("nfv")
        h = self.uniq("triple")
        a = self.rng.randint(2, 20)
        fn_helper = f"""fn {h}(n: Int) -> Int
    n * 3 - 2
end"""
        fn = f"""fn {v}(a: Int) -> Int
    let f = {h}
    let via_var = f(a)
    let apply3 = fn(g: Fn, n: Int) -> Int
        g(g(g(n)))
    end
    via_var * 10 + apply3({h}, a)
end"""
        calls = [f"println({v}({a}))"]
        self.emit(fn_helper + "\n\n" + fn, calls)

    def t_closure_capture_record(self):
        # 闭包捕获 Record（不可变绑定）+ 字段读取组合
        v = self.uniq("ccr")
        a, b = self.rng.randint(1, 50), self.rng.randint(1, 50)
        fn = f"""fn {v}(a: Int, b: Int) -> Int
    let cfg = {{scale: 3, offset: b}}
    let read = fn(n: Int) -> Int
        n * cfg.scale + cfg.offset
    end
    read(a) * 10 + read(0)
end"""
        calls = [f"println({v}({a}, {b}))"]
        self.emit(fn, calls)

    def t_closure_capture_list(self):
        # 闭包捕获 List + 内部 HOF 消费（捕获容器的深组合）
        v = self.uniq("ccl")
        n = self.rng.randint(3, 9)
        k = self.rng.randint(2, 5)
        fn = f"""fn {v}(n: Int, k: Int) -> Int
    let xs = 1..n
    let summarize = fn() -> Int
        list_fold(fn(acc: Int, x: Int) -> Int
            if x % {k} == 0
                acc + x
            else
                acc + 1
            end
        end, 0, xs)
    end
    summarize() * 10 + list_length(xs)
end"""
        calls = [f"println({v}({n}, {k}))"]
        self.emit(fn, calls)

    def t_pipeline_deep_chain(self):
        # 六段管道链（helper 全 uniq——重复抽中防 NAM002）
        v = self.uniq("pdc")
        hs = [self.uniq("p%d" % i) for i in range(6)]
        a = self.rng.randint(1, 30)
        parts = "\n".join(
            "fn %s(x: Int) -> Int\n    x %s %d\nend" % (h, op, k)
            for h, op, k in zip(hs, ["+", "*", "-", "/", "%", "+"],
                                [3, 2, 5, 2, 97, 11])
        )
        chain = " |> ".join(hs)
        fn = f"""fn {v}(x: Int) -> Int
    x |> {chain}
end"""
        calls = [f"println({v}({a}))", f"println({v}(100))"]
        self.emit(parts + "\n\n" + fn, calls)

    def t_pipeline_closure_mix(self):
        # 管道右操作数是闭包变量（任意 callee 形态）+ 具名混合
        v = self.uniq("pcm")
        h = self.uniq("step1")
        a = self.rng.randint(1, 40)
        fn_helper = f"""fn {h}(x: Int) -> Int
    x + 10
end"""
        fn = f"""fn {v}(x: Int) -> Int
    let g = fn(n: Int) -> Int
        n * 2 - 1
    end
    x |> {h} |> g |> {h}
end"""
        calls = [f"println({v}({a}))"]
        self.emit(fn_helper + "\n\n" + fn, calls)

    def t_compose_higher_order(self):
        # compose 组合子：compose(f, g)(x) = g(f(x))
        v = self.uniq("cmp")
        a = self.rng.randint(1, 20)
        fn = f"""fn {v}(a: Int) -> Int
    let compose = fn(f: Fn, g: Fn) -> Fn
        fn(x: Int) -> Int
            g(f(x))
        end
    end
    let inc = fn(n: Int) -> Int
        n + 3
    end
    let sqm = fn(n: Int) -> Int
        n * n % 1000
    end
    let h = compose(inc, sqm)
    h({a}) * 10 + h(0)
end"""
        calls = [f"println({v}({a}))"]
        self.emit(fn, calls)


    # ===== 模板族 S：Result/Option/效应深组合（D9-e，D 包四期 2026-09-15）=====

    def t_result_chain_propagate(self):
        # 链式 ? 传播：三层函数链，中段 Err 短路
        v1, v2, v3 = self.uniq("ra"), self.uniq("rb"), self.uniq("rc")
        n = self.rng.randint(10, 99)
        bad = self.rng.choice(["3", "33", "7"])
        fn1 = f"""fn {v1}(n: Int) -> Result<Int, String>
    if n % 2 == 0
        Ok(n / 2)
    else
        Err("odd")
    end
end"""
        fn2 = f"""fn {v2}(n: Int) -> Result<Int, String>
    let h = {v1}(n)?
    if h > 10
        Ok(h * 10)
    else
        Err("small")
    end
end"""
        fn3 = f"""fn {v3}(n: Int) -> Result<Int, String>
    let x = {v2}(n)?
    let y = {v1}(x)?
    Ok(y + 1)
end"""
        fn = fn1 + "\n\n" + fn2 + "\n\n" + fn3
        calls = [f"println({v3}({n}))", f"println({v3}({bad}))", f"println({v3}(4))"]
        self.emit(fn, calls)

    def t_option_nested_consume(self):
        # 嵌套 Option：Option<Option<Int>> 形态的解包消费
        v = self.uniq("onc")
        w = self.uniq("wrap")
        n = self.rng.randint(0, 99)
        fn_helper = f"""fn {w}(n: Int) -> Option<Option<Int>>
    if n > 50
        Some(Some(n * 2))
    elif n > 10
        Some(None)
    else
        None
    end
end"""
        fn = f"""fn {v}(n: Int) -> Int
    let outer = {w}(n)
    match outer
        Some(inner) =>
            match inner
                Some(x) => x + 1000
                None => 100
            end
        end
        None => 0
    end
end"""
        calls = [f"println({v}({n}))", f"println({v}(30))", f"println({v}(5))"]
        self.emit(fn_helper + "\n\n" + fn, calls)

    def t_try_in_complex_expr(self):
        # ? 在复杂表达式内（二元运算操作数 + 函数调用实参）
        v = self.uniq("tce")
        w = self.uniq("safe_div")
        a = self.rng.randint(10, 99)
        b = self.rng.randint(2, 9)
        fn_helper = f"""fn {w}(a: Int, b: Int) -> Result<Int, String>
    if b != 0
        Ok(a / b)
    else
        Err("zero")
    end
end"""
        fn = f"""fn {v}(a: Int, b: Int) -> Result<Int, String>
    let sum = {w}(a, b)? + {w}(a * 2, b)?
    let scaled = {w}(sum, 2)?
    Ok(scaled * 10 + a % b)
end"""
        calls = [f"println({v}({a}, {b}))", f"println({v}({a}, 0))"]
        self.emit(fn_helper + "\n\n" + fn, calls)

    def t_effect_propagation_chain(self):
        # 多函数效应传播链：纯函数调 IO 函数 → EFF001 warning 形态的正确标注
        # （全链如实标注 ! [IO] 则零 warning——effect 推导的深层覆盖）
        io1, io2, pure = self.uniq("w1"), self.uniq("w2"), self.uniq("pp")
        v = self.uniq("epc")
        n = self.rng.randint(2, 8)
        fn1 = f"""fn {io1}(n: Int) -> Int ! [IO]
    print(int_to_string(n))
    n * 2
end"""
        fn2 = f"""fn {io2}(n: Int) -> Int ! [IO]
    let d = {io1}(n)
    let e = {io1}(d)
    println("")
    d + e
end"""
        fn3 = f"""fn {pure}(n: Int) -> Int
    n + 1
end"""
        fn = f"""fn {v}(n: Int) -> Int ! [IO]
    let p = {pure}(n)
    let q = {io2}(p)
    q
end"""
        calls = [f"println({v}({n}))"]
        self.emit(fn1 + "\n\n" + fn2 + "\n\n" + fn3 + "\n\n" + fn, calls)

    def t_match_three_level(self):
        # match 消费三层嵌套（Result 内 Option 内值）
        v = self.uniq("m3")
        w = self.uniq("fetch")
        k = self.rng.randint(0, 99)
        fn_helper = f"""fn {w}(k: Int) -> Result<Option<Int>, String>
    if k % 15 == 0
        Err("fizzbuzz")
    elif k % 3 == 0
        Ok(Some(k))
    elif k % 5 == 0
        Ok(None)
    else
        Ok(Some(k * 2))
    end
end"""
        fn = f"""fn {v}(k: Int) -> String
    match {w}(k)
        Err(e) => "E:" + e
        Ok(None) => "none"
        Ok(Some(x)) =>
            if x > 100
                "big"
            else
                "small"
            end
        end
    end
end"""
        calls = [f'println({v}({k}))', f'println({v}(30))', f'println({v}(10))', f'println({v}(7))']
        self.emit(fn_helper + "\n\n" + fn, calls)

    def t_result_map_pattern(self):
        # Result 载荷 Tuple 的消费（变体模式内嵌元组模式不支持——§11 v0.27 坑；
        # 用绑定 + .0/.1 访问）
        v = self.uniq("rmp")
        w = self.uniq("mkt")
        a = self.rng.randint(1, 99)
        fn_helper = f"""fn {w}(a: Int) -> Result<(Int, Int), String>
    if a % 2 == 0
        Ok((a, a * 2))
    else
        Err("odd-pair")
    end
end"""
        fn = f"""fn {v}(a: Int) -> Int
    match {w}(a)
        Ok(t) => t.0 * 100 + t.1
        Err(_) => 0 - 1
    end
end"""
        calls = [f"println({v}({a}))", f"println({v}({a + 1}))"]
        self.emit(fn_helper + "\n\n" + fn, calls)

    def t_try_in_loop_complex(self):
        # 循环内 ? + 双 helper 交替传播 + 提前返回对照
        v = self.uniq("tlc")
        w1, w2 = self.uniq("h1"), self.uniq("h2")
        n = self.rng.randint(3, 9)
        fn1 = f"""fn {w1}(n: Int) -> Result<Int, String>
    if n % 2 == 0
        Ok(n * 3)
    else
        Err("e" + int_to_string(n))
    end
end"""
        fn2 = f"""fn {w2}(n: Int) -> Result<Int, String>
    if n < 10
        Ok(n + 100)
    else
        Err("overflow")
    end
end"""
        fn = f"""fn {v}(n: Int) -> Result<Int, String>
    let mut i = 0
    let mut total = 0
    while i < n
        let a = {w1}(i)?
        let b = {w2}(a)?
        if b > 105
            return Ok(total + b)
        end
        total += b
        i += 1
    end
    Ok(total)
end"""
        calls = [f"println({v}({n}))", f"println({v}(2))"]
        self.emit(fn1 + "\n\n" + fn2 + "\n\n" + fn, calls)


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
    Gen.t_shadowing, Gen.t_json_parse_consume, Gen.t_json_roundtrip, Gen.t_json_nested,
    Gen.t_mutual_recursion, Gen.t_deep_recursion_gradient, Gen.t_recursion_early_return,
    Gen.t_file_roundtrip, Gen.t_file_exists_gated, Gen.t_file_append_accum,
    Gen.t_env_args_consume, Gen.t_while_match_return, Gen.t_nested_for_closure,
    Gen.t_enum_cross_match, Gen.t_try_in_while, Gen.t_mixed_iteration,
    Gen.t_import_alias_stdlib, Gen.t_recursion_accumulator,
    Gen.t_string_contains_chain, Gen.t_string_replace_ops, Gen.t_string_split_nested,
    Gen.t_char_from_code_ops, Gen.t_string_lex_compare, Gen.t_string_case_pipeline,
    Gen.t_int_negative_ops, Gen.t_int_divmod_identity, Gen.t_int_mul_boundary,
    Gen.t_int_sign_mixed,
    Gen.t_list_hof_chain, Gen.t_list_head_tail_recursion, Gen.t_list_closure_capture,
    Gen.t_list_index_random, Gen.t_list_nested_cons,
    Gen.t_map_nested, Gen.t_map_reference_alias, Gen.t_map_sorted_keys_consume,
    Gen.t_map_remove_rehash,
    Gen.t_record_rebuild_chain, Gen.t_tuple_destructure_swap, Gen.t_record_tuple_eq,
    Gen.t_enum_wide_variant, Gen.t_enum_payload_enum, Gen.t_enum_recursive_tree,
    Gen.t_enum_guard_chain, Gen.t_enum_nested_match_guard, Gen.t_enum_build_consume_chain,
    Gen.t_enum_variant_dict, Gen.t_enum_wildcard_positions,
    Gen.t_closure_return_closure2, Gen.t_closure_as_arg, Gen.t_named_fn_as_value,
    Gen.t_closure_capture_record, Gen.t_closure_capture_list, Gen.t_pipeline_deep_chain,
    Gen.t_pipeline_closure_mix, Gen.t_compose_higher_order,
    Gen.t_result_chain_propagate, Gen.t_option_nested_consume, Gen.t_try_in_complex_expr,
    Gen.t_effect_propagation_chain, Gen.t_match_three_level, Gen.t_result_map_pattern,
    Gen.t_try_in_loop_complex,
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
    # 每程序模板数 4-10（D 包四期密度提升：更长 main、更密函数交互）
    count = rng.randint(4, 10)
    for _ in range(count):
        rng.choice(POOL)(g)
    header = [
        "# diff_gen 自动生成（D 工作包差分测试）—— seed=%d" % seed,
        "from string import { len, upper, lower, trim, int_to_string, string_to_int, split, contains, replace, starts_with, ends_with, char_from_code }",
        "from math import { sqrt, abs, min, max }",
        "from list import { list_map, list_filter, list_fold, list_cons, list_head, list_tail, list_length, list_get }",
        "from map import { map_empty, map_set, map_get, map_has, map_remove, map_keys, map_size }",
        "from json import { json_parse, json_stringify }",
        "from file import { file_read, file_write, file_append, file_exists }",
        "from env import { args }",
        "",
    ]
    body = "\n\n".join(g.decls)
    main_text = "fn main() -> Unit\n" + "\n".join("    " + s for s in g.main) + "\n    ()\nend"
    return "\n".join(header) + body + "\n\n" + main_text + "\n", ""


# ---------- 包模式（D5）：多文件包项目的随机生成 ----------
# 覆盖面：lom.toml 依赖图（扁平 + 链式 libB→libA）、跨包导入（含 as 别名）、
# 包符号在主文件的组合调用、WASM merge（包前主后）与解释器 load_packages 的
# 行为对齐。包内函数**只保留 Int 单参/双参两种签名**——调用形态二值化消灭
# 签名分派错误（首版 Float/String/enum 分派出过错；那些类型的覆盖由单文件
# 模式的模板池承担，池大小见 doc_audit I 类监控），包模式的独特价值在依赖图与合并语义）。包内函数保持
# 自包含（仅 prelude 纯运算——包源码的 import 在 WASM merge 时只收集 items）。

_PKG_BODY_1 = [
    "fn {name}(x: Int) -> Int\n    x * {k} + {m}\nend",
    "fn {name}(x: Int) -> Int\n    if x % 2 == 0\n        x / 2 + {m}\n    else\n        x * 3 - {m}\n    end\nend",
    # 注：不放阶乘/大数增长模板——WASM 后端 Int 安全值域 ±2^59（§11f 第 7 条，
    # tagged i64 低 4 位 tag 的结构性限制），包模式结果控制在百量级；
    # 阶乘覆盖由单文件模式承担（n ≤ 12，结果 < 2^59）
    "fn {name}(n: Int) -> Int\n    let mut total = 0\n    for i in n\n        total += i * {k}\n    end\n    total\nend",
]
_PKG_BODY_2 = [
    "fn {name}(x: Int, y: Int) -> Int\n    x * {k} - y + {m}\nend",
    "fn {name}(x: Int, y: Int) -> Int\n    if x > y\n        x - y + {k}\n    else\n        y - x + {m}\n    end\nend",
]
# D 包四期包模式深化：包内签名多样化（Float/String/Bool 单参——仅 prelude
# 能力，包源码保持自包含；跨包桥接仍只用 Int 单参，D5"消灭签名分派错误"的
# 纪律对桥接层保留，多样化只在包内与主文件直调层）
_PKG_BODY_F1 = [
    "fn {name}(x: Float) -> Float\n    x * {k}.5 + 1.25\nend",
    "fn {name}(x: Float) -> Float\n    if x > 10.0\n        x / 2.0\n    else\n        x + {m}.5\n    end\nend",
]
_PKG_BODY_S1 = [
    'fn {name}(s: String) -> String\n    s + "-" + s\nend',
    'fn {name}(s: String) -> Int\n    if s > "m"\n        {k}\n    else\n        0 - {m}\n    end\nend',
]
_PKG_BODY_B1 = [
    "fn {name}(b: Bool) -> Int\n    if b\n        {k}\n    else\n        {m}\n    end\nend",
]


def gen_pkg_project(seed: int) -> dict:
    """生成一个包项目（包模式，D5 + D 包四期深化）。返回 {相对路径: 内容} 的文件树。

    结构：主目录 lom.toml + main.lom；libA 必有（含包内 enum 消费/互助函数的
    概率增强）；libB 50%；有 libB 时 libC 35%（三包链 C→B→A）。跨包桥接只走
    Int 单参；包内与主文件直调层签名多样化（Int 单/双参 + Float/String/Bool）。
    """
    rng = random.Random(seed)
    files: dict[str, str] = {}
    n = 0

    def mkfn() -> tuple[str, str, str]:
        """产出一个包内函数：返回 (导出名, 函数源码, kind ∈ i1/i2/f1/s1/b1)。"""
        nonlocal n
        n += 1
        name = "pf_%d" % n
        r = rng.random()
        if r < 0.40:
            kind, tpl = "i1", rng.choice(_PKG_BODY_1)
        elif r < 0.60:
            kind, tpl = "i2", rng.choice(_PKG_BODY_2)
        elif r < 0.73:
            kind, tpl = "f1", rng.choice(_PKG_BODY_F1)
        elif r < 0.87:
            kind, tpl = "s1", rng.choice(_PKG_BODY_S1)
        else:
            kind, tpl = "b1", rng.choice(_PKG_BODY_B1)
        src = tpl.format(name=name, k=rng.randint(2, 9), m=rng.randint(0, 50))
        return name, src, kind

    def call(name: str, kind: str) -> str:
        """按签名类型生成调用表达式（字面量参数，确定性）。"""
        if kind == "i1":
            return "%s(%d)" % (name, rng.randint(1, 30))
        if kind == "i2":
            return "%s(%d, %d)" % (name, rng.randint(1, 30), rng.randint(1, 30))
        if kind == "f1":
            return "%s(%d.5)" % (name, rng.randint(1, 20))
        if kind == "s1":
            return '%s("s%d")' % (name, rng.randint(1, 99))
        return "%s(%s)" % (name, rng.choice(["True", "False"]))

    def mkname() -> str:
        nonlocal n
        n += 1
        return "pf_%d" % n

    # --- libA（必有，2-3 个函数 + 概率增强：包内 enum 消费 / 同包互助）---
    liba_fns = [mkfn() for _ in range(rng.randint(2, 3))]
    liba_extra = ""
    if rng.random() < 0.5:
        # 包内 enum：定义 + 构造 + match 消费（不跨包暴露变体——主文件只调 Int 入口）
        p = n
        en = "Pe_%d" % p
        pe_fn = mkname()
        liba_extra = (
            "enum %s = Pa_%d | Pb_%d\n\nfn %s(x: Int) -> Int\n    let e = if x %% 2 == 0\n        Pa_%d\n    else\n        Pb_%d\n    end\n    match e\n        Pa_%d => x * 2\n        Pb_%d => 0 - x\n    end\nend"
            % (en, p, p, pe_fn, p, p, p, p)
        )
        liba_fns.append((pe_fn, liba_extra, "i1"))
    i1s = [nm for nm, _, kd in liba_fns if kd == "i1"]
    if len(i1s) >= 2 and rng.random() < 0.5:
        # 同包互助：一个 fn 调用同包另一个 fn
        helper = rng.choice(i1s)
        coop = mkname()
        coop_src = "fn %s(x: Int) -> Int\n    %s(x) + %d\nend" % (coop, helper, rng.randint(1, 50))
        liba_fns.append((coop, coop_src, "i1"))
    files["liba/lom.toml"] = 'name = "liba"\nversion = "0.1.0"\n'
    files["liba/a.lom"] = "\n\n".join(src for _, src, _ in liba_fns) + "\n"

    # --- libB（50%；60% 概率链式依赖 libA，bridge 调用 liba 单参 Int 符号）---
    libb_fns: list[tuple[str, str, str]] = []
    has_b = rng.random() < 0.5
    chain = has_b and rng.random() < 0.6
    if has_b:
        libb_fns = [mkfn() for _ in range(rng.randint(2, 3))]
        if chain:
            cands = [nm for nm, _, kd in liba_fns if kd == "i1"]
            if cands:
                liba_name = rng.choice(cands)
                bridge = mkname()
                bridge_src = (
                    "from liba import { %s }\n\nfn %s(x: Int) -> Int\n    %s(x) + %d\nend"
                    % (liba_name, bridge, liba_name, rng.randint(1, 99))
                )
                libb_fns.append((bridge, bridge_src, "i1"))
        files["libb/lom.toml"] = (
            'name = "libb"\nversion = "0.1.0"\n\n[dependencies]\nliba = { path = "../liba" }\n'
            if chain else 'name = "libb"\nversion = "0.1.0"\n'
        )
        files["libb/b.lom"] = "\n\n".join(src for _, src, _ in libb_fns) + "\n"

    # --- libC（有 libB 时 35%：三包链 C→B→A，bridge 调用 libb 单参 Int 符号）---
    libc_fns: list[tuple[str, str, str]] = []
    has_c = has_b and rng.random() < 0.35
    if has_c:
        libc_fns = [mkfn() for _ in range(rng.randint(1, 2))]
        cands = [nm for nm, _, kd in libb_fns if kd == "i1"]
        if cands:
            libb_name = rng.choice(cands)
            bridge2 = mkname()
            bridge2_src = (
                "from libb import { %s }\n\nfn %s(x: Int) -> Int\n    %s(x) * 2 + %d\nend"
                % (libb_name, bridge2, libb_name, rng.randint(1, 30))
            )
            libc_fns.append((bridge2, bridge2_src, "i1"))
        files["libc/lom.toml"] = 'name = "libc"\nversion = "0.1.0"\n\n[dependencies]\nlibb = { path = "../libb" }\n'
        files["libc/c.lom"] = "\n\n".join(src for _, src, _ in libc_fns) + "\n"

    # --- 主清单 ---
    deps = ['liba = { path = "liba" }']
    if has_b:
        deps.append('libb = { path = "libb" }')
    if has_c:
        deps.append('libc = { path = "libc" }')
    files["lom.toml"] = 'name = "app"\nversion = "0.1.0"\n\n[dependencies]\n' + "\n".join(deps) + "\n"

    # --- 主文件：导入（含别名）+ 本地组合 + main ---
    # liba 选 2 个符号（第二个用别名；别名不改变签名类型）
    picks = rng.sample(range(len(liba_fns)), 2)
    (nm1, _, kd1) = liba_fns[picks[0]]
    (nm2, _, kd2) = liba_fns[picks[1]]
    imports = ["from liba import { %s, %s as aliased_fn }" % (nm1, nm2)]
    call1 = call(nm1, kd1)
    call2 = call("aliased_fn", kd2)
    call_b = None
    if has_b:
        b_nm, _, b_kd = rng.choice(libb_fns)
        imports.append("from libb import { %s }" % b_nm)
        call_b = call(b_nm, b_kd)
    call_c = None
    if has_c and libc_fns:
        c_nm, _, c_kd = rng.choice(libc_fns)
        imports.append("from libc import { %s }" % c_nm)
        call_c = call(c_nm, c_kd)

    # 本地组合函数：仅当 liba 有 Int 单参符号时做跨符号组合（其余类型在 main 直接调用）
    n += 1
    combo = "local_%d" % n
    single_syms = [s for s in [(nm1, kd1), ("aliased_fn", kd2)] if s[1] == "i1"]
    if single_syms:
        s1 = single_syms[0][0]
        combo_lines = ["fn %s(x: Int) -> Int" % combo, "    %s(x) + %s(x * 2)" % (s1, s1), "end"]
    else:
        combo_lines = ["fn %s(x: Int) -> Int" % combo, "    x * 2 + 1", "end"]

    main_lines = ["fn main() -> Unit"]
    main_lines.append("    println(%s)" % call1)
    main_lines.append("    println(%s)" % call2)
    main_lines.append("    println(%s(%d))" % (combo, rng.randint(1, 30)))
    if call_b:
        main_lines.append("    println(%s)" % call_b)
    if call_c:
        main_lines.append("    println(%s)" % call_c)
    main_lines.append("    ()")
    main_lines.append("end")

    files["main.lom"] = (
        "# diff_gen 包模式（D5）—— seed=%d\n" % seed
        + "\n".join(imports)
        + "\n\n"
        + "\n".join(combo_lines)
        + "\n\n"
        + "\n".join(main_lines)
        + "\n"
    )
    return files

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
    if kind == "json-number":
        # 分歧 2：JSON 数字按 JS 宿主值切分 Int/Float 而非源语法——
        # "30.0"/"1e2"/"-0.0" 源语法是 Float、JS 值为整 → 双后端分叉
        return """# probe: json-number（§11f 分歧 2——预期 30.0 类形态双后端分叉）
from json import { json_parse }

fn main() -> Unit
    let v = json_parse("[30, 30.0, 1e2, -0.0, 0.5]")
    println(v)
end
"""
    if kind == "int-range":
        # 分歧 7（D 包二期 2026-09-14 实测发现）：WASM tagged i64 低 4 位 tag
        # 只能无损承载 60 位载荷——2^59 起撞符号位变负、2^60 起高位静默截断；
        # 解释器全 i64。预期双侧 stdout 不同且 wasm 值 = interp 值 mod 2^60 的符号解释
        return """# probe: int-range（§11f 分歧 7——预期 wasm 侧静默截断/翻符号）
fn main() -> Unit
    println(576460752303423488)
    println(1152921504606846976)
    println(1152921504606846977)
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
