# -*- coding: utf-8 -*-
"""tools/l2fix.py — L2 码进修复闭环通道（docs/designs/0018 裁决点 1 甲形态：纯工具面，零宿主改动、不升版）

管线：读 <file.lom> → 临时目录写副本 → 子进程 `<lom> <self_comp.lom> -- <tmp-in> <tmp-out>`
（self_comp 实为 L2 子集编译器，判定靠 stdout 码行——COMPILE-ERROR 恒 rc=0）
→ 解析码行 → 七族建议映射 → 输出。

码行两形态（第二形态为实测补充：LEX 负例走 lex error 行而非 codegen error 行）：
  codegen：`codegen error: [(\\d+:\\d+ )]?\\[(L2[A-Z]\\d{3}|LEX\\d{3})\\] (.*)`
  （0020 行号批：位置段 ln:cl 可选前缀于码前——名字锚命中才有；miss 无位置）
  词法/语法层：`lex error L:C [LEXnnn] msg` / `parse error L:C [PARSExxx] msg`
  非 L2 码（LEX/PARSE 等）透传原样，给"语法层错误，参考输出"提示——本工具的
  建议映射只覆盖 L2 codegen 码族。

用法：
  python tools/l2fix.py <file.lom> [--json] [--self-check] [--lom-bin PATH] [--l2 PATH]

  --json        输出 l2-fix/v1 轻量 JSON（独立 schema，不撞宿主 lom-fix/v1——L2 工具码不进宿主协议）
  --self-check  质量锁定（designs/0018 §2B）：对 tools/selfcomp/negative/ 全部负例
                + eval 127/128/129 三题 broken（prompt 中提取）跑全链断言；首次交付自验用，不入 CI 常驻
  --lom-bin     宿主二进制（默认 target/release/lom，Windows 探测 .exe）
  --l2          L2 编译器源（默认 examples/selfhost/self_comp.lom）

退出码：0 = 分析完成（有建议条目或"L2 编译通过"）；1 = --self-check 断言失败；2 = 工具面错误。
"""

import argparse
import json
import re
import shutil
import subprocess
import sys
import tempfile
from collections import Counter
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

# ---------------------------------------------------------------------------
# 内建 名字→模块 表（L2U 族 High import 建议的查表依据；designs/0018 裁决点 2 甲：
# 工具自带双源）。源 = src/interpreter.rs module_of（L2202 区）单一事实源；
# §14.4 内建冻结（43 个）为漂移依据；与宿主 NAM005 口径差：L2 中 io 的
# println/print 需显式 import。
# ---------------------------------------------------------------------------
NAME_TO_MODULE = {
    # io
    "println": "io",
    "print": "io",
    # string
    "len": "string",
    "int_to_string": "string",
    "string_to_int": "string",
    "trim": "string",
    "upper": "string",
    "lower": "string",
    "split": "string",
    "contains": "string",
    "replace": "string",
    "starts_with": "string",
    "ends_with": "string",
    "char_from_code": "string",
    # math
    "sqrt": "math",
    "abs": "math",
    "min": "math",
    "max": "math",
    # list
    "list_empty": "list",
    "list_length": "list",
    "list_get": "list",
    "list_is_empty": "list",
    "list_head": "list",
    "list_tail": "list",
    "list_cons": "list",
    "list_map": "list",
    "list_filter": "list",
    "list_fold": "list",
    # json
    "json_parse": "json",
    "json_stringify": "json",
    # map
    "map_empty": "map",
    "map_set": "map",
    "map_get": "map",
    "map_has": "map",
    "map_remove": "map",
    "map_keys": "map",
    "map_values": "map",
    "map_size": "map",
    # file
    "file_read": "file",
    "file_write": "file",
    "file_append": "file",
    "file_exists": "file",
    # env
    "args": "env",
}

# L2S/L2V 族级静态模板（designs/0018 §2A：文案多自带方向；128"该表达式形态"
# 零细节——模板必列常见触发物清单；127 正解 map_get 不在文案、由模板携带）。
L2S_TEMPLATE = (
    "L2 子集外形态——常见触发物清单：管道 x |> f（改写为普通函数调用 f(x)）/ "
    "match 或解构语句 / 复杂表达式（嵌套调用、容器字面量、导入 as 别名、非 Int 的 "
    "range 端、stringify 容器/枚举值等）——逐个简化重试定位触发物；另按原文自带"
    "方向改形态（如改用局部比较、显式 from io import、补类型注解等替代写法）。"
)
L2V_TEMPLATE = (
    "L2 值域运算规则（族级模板）：\n"
    "- Map/List 不能整体参与算术或大小比较：用 map_get/list_get 等取出元素后再运算；"
    "容器仅 ==/!= 可直接比较（元素/值为枚举或闭包时连 ==/!= 也不支持，需取载荷逐个比较）。\n"
    "- Bool/enum（及闭包）不参与算术与一元负；Bool 只用于逻辑运算与条件，不与非 Bool 混合比较。\n"
    "- String 只参与 + 拼接（不做算术/大小比较/一元负）；json 值仅支持 .field 访问、"
    "println、stringify 与 list 消费；Record/Tuple 不参与算术与比较。\n"
    "- 算术要求 Int/Float 同型（Int 与 Float 混合同样拒绝——先统一类型再运算）。"
)

CODEGEN_LINE = re.compile(r"codegen error: (?:(\d+:\d+) )?\[(L2[A-Z]\d{3}|LEX\d{3})\] (.*)")
SYNTAX_LINE = re.compile(r"(?:lex|parse) error \d+:\d+ \[(\w+\d{3})\] (.*)")


def default_lom_bin():
    base = REPO / "target" / "release"
    for cand in ("lom.exe", "lom"):  # Windows 探测 .exe，非 Windows 先探 lom.exe 也无害
        if (base / cand).is_file():
            return str(base / cand)
    return str(base / ("lom.exe" if sys.platform == "win32" else "lom"))


def run_l2(lom_bin, l2_path, src):
    """子进程跑 L2 编译：临时目录写副本 → <lom> <self_comp> -- <in> <out> → 返回 stdout。"""
    tmp = tempfile.mkdtemp(prefix="l2fix-")
    try:
        tin, tout = Path(tmp) / "in.lom", Path(tmp) / "out.hex"
        shutil.copyfile(src, tin)
        try:
            proc = subprocess.run(
                [lom_bin, str(l2_path), "--", str(tin), str(tout)],
                capture_output=True, text=True, encoding="utf-8",
                errors="replace", timeout=300)
        except FileNotFoundError:
            sys.exit("错误：宿主二进制不可用：%s（用 --lom-bin 指定）" % lom_bin)
        return proc.stdout
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def parse_codes(stdout):
    """stdout → [(code, message)]，保持输出顺序。

    0020 行号批后 codegen 行带可选位置前缀（group1=ln:cl、group2=码、
    group3=文案）——位置不进建议面（message 维持纯文案，七族映射零改动）。
    """
    out = []
    for line in stdout.splitlines():
        m = CODEGEN_LINE.search(line)
        if m:
            out.append((m.group(2), m.group(3).strip()))
            continue
        m = SYNTAX_LINE.search(line)
        if m and not m.group(1).startswith("L2"):
            out.append((m.group(1), m.group(2).strip()))
    return out


def _expected_got(message):
    """提取中文形态的期望/实得（含数量）——`期望 X` / `得 X`（要求后随空白，避开"得到"）。"""
    exp = re.search(r"期望\s+([^ ）,，;；]+)", message)
    got = re.search(r"(?<![得具])得\s+([^ ）,，;；]+)", message)
    return (exp.group(1) if exp else None, got.group(1) if got else None)


def suggest(code, message):
    """七族建议映射 → {code, message, suggestion, confidence, action}。"""
    family = code[:3]
    if family == "L2U":
        m = re.search(r"'([^']+)'", message)
        name = m.group(1) if m else None
        if "未定义变量" in message and name in NAME_TO_MODULE:
            mod = NAME_TO_MODULE[name]
            return {"code": code, "message": message, "confidence": "high",
                    "action": "insert",
                    "suggestion": ("在文件顶部插入导入语句：from %s import {%s}"
                                   "（插至文件顶部 1:1）。确认后手动应用，或经宿主 "
                                   "`lom fix` 的 NAM005 通道自动应用——宿主侧同形态"
                                   "（内建未导入）会被 NAM005 捕获。" % (mod, name))}
        label = "'%s'" % name if name else "（原文未提取到名字）"
        return {"code": code, "message": message, "confidence": "low",
                "action": "hint",
                "suggestion": ("名字 %s 不在内建名字→模块表中——真未定义或拼写错误，"
                               "不建议盲补 import：核对拼写、let/参数作用域（块内与"
                               "分支内 let 不外泄）、闭包捕获，以及 import 的模块名"
                               "与符号名（含 as 别名限制）。" % label)}
    if family == "L2P":
        if "缺少 main" in message:
            return {"code": code, "message": message, "confidence": "medium",
                    "action": "hint",
                    "suggestion": ("补 main 骨架（L2 程序入口必需）：\n"
                                   "fn main() -> Unit\n    println(0)\nend")}
        m = re.search(r"'([^']+)'", message)
        name = m.group(1) if m else None
        if "不得命名" in message:
            return {"code": code, "message": message, "confidence": "medium",
                    "action": "hint",
                    "suggestion": ("改名方向：用户函数不得命名为 '%s'（该名字保留给"
                                   "内建）——换一个函数名。" % name)}
        label = "'%s'" % name if name else "重名处"
        return {"code": code, "message": message, "confidence": "medium",
                "action": "hint",
                "suggestion": ("重命名方向：%s 与既有声明/内建重名（重复变体/枚举/"
                               "类型参数/导入，或与内建类型冲突）——为其换名，或删除"
                               "重复的声明/导入。" % label)}
    if family in ("L2T", "L2C"):
        exp, got = _expected_got(message)
        if family == "L2T":
            head = "核对注解与值类型" if exp is None else "期望 %s、实得 %s——核对注解与值类型" % (exp, got)
            tail = ("：let 注解改型或改值、模式子模式数与被测类型匹配、"
                    "match 臂/return 类型同型、条件与谓词须为 Bool。")
        else:
            head = "核对该调用的实参" if exp is None else "期望 %s、实得 %s——核对该调用的实参" % (exp, got)
            tail = ("：实参数量与类型逐一对照；闭包实参注意签名（list_map 的 f "
                    "单参、list_fold 的 f 双参 (acc, elem)；print/println 单参）。")
        return {"code": code, "message": message, "confidence": "medium",
                "action": "hint", "suggestion": head + tail}
    if family == "L2V":
        return {"code": code, "message": message, "confidence": "medium",
                "action": "hint", "suggestion": L2V_TEMPLATE}
    if family == "L2S":
        return {"code": code, "message": message, "confidence": "medium",
                "action": "hint", "suggestion": L2S_TEMPLATE}
    if family == "L2E":
        return {"code": code, "message": message, "confidence": "n/a",
                "action": "report",
                "suggestion": "L2 编译器内部错误——请报维护者（附完整输出）。"}
    if family == "L2G":  # classify_l2 兜底码（语料外构造性可达——二十六审
        # R114：Map 键类型注解/tuple 非数字索引等约 29 条组合文案落此兜底；
        # 码非空、文案完整，建议面低置信不误导）
        return {"code": code, "message": message, "confidence": "low",
                "action": "hint",
                "suggestion": ("未归类到七族的 L2 拒绝（兜底码）——按原文文案方向"
                               "修改；如认为归类有误，请连同完整输出反馈维护者。")}
    # LEX/PARSE 等语法层码：透传原样
    return {"code": code, "message": message, "confidence": "n/a",
            "action": "hint",
            "suggestion": ("语法层错误（词法/解析阶段）——l2fix 的建议映射只覆盖 "
                           "L2 codegen 码族；请参考原始输出的行列号与文案修复语法"
                           "后重试。")}


def analyze(path, lom_bin, l2_path):
    """全链：返回 (entries, passed, stdout)。passed = 无码条目且 stdout 含 COMPILED。"""
    src = Path(path)
    if not src.is_file():
        sys.exit("错误：输入文件不存在：%s" % src)
    if not Path(l2_path).is_file():
        sys.exit("错误：L2 编译器源不存在：%s（用 --l2 指定）" % l2_path)
    stdout = run_l2(lom_bin, l2_path, src)
    entries = [suggest(c, m) for c, m in parse_codes(stdout)]
    return entries, (not entries and "COMPILED" in stdout), stdout


def print_human(entries, passed, stdout, fname):
    if passed:
        print("L2 编译通过")
        return
    if not entries:
        print("未解析到任何码行（输出形态未知）——L2 原始输出：")
        for line in stdout.splitlines():
            print("  " + line)
        return
    print("L2 拒绝 %d 处（%s）：" % (len(entries), fname))
    for i, e in enumerate(entries, 1):
        print("[%d] %s | %s" % (i, e["code"], e["message"]))
        for ln in e["suggestion"].split("\n"):
            print("      " + ln)
        print("      置信度 %s | 动作 %s" % (e["confidence"], e["action"]))


def broken_from_prompt(prompt):
    """eval prompt → broken 代码段（"代码："行与"诊断 JSON："行之间）。"""
    lines = prompt.splitlines()
    start = end = None
    for i, ln in enumerate(lines):
        if start is None and ln.strip() == "代码：":
            start = i + 1
        elif start is not None and ln.startswith("诊断 JSON："):
            end = i
            break
    if start is None or end is None:
        return None
    return "\n".join(lines[start:end]).rstrip() + "\n"


def self_check(lom_bin, l2_path):
    """质量锁定（designs/0018 §2B）：六断言，全过返回 True。"""
    negs = sorted((REPO / "tools" / "selfcomp" / "negative").glob("*.lom"))
    fams, no_code, empty_sug, n_entries = Counter(), [], [], 0
    for f in negs:
        entries, passed, _ = analyze(f, lom_bin, l2_path)
        if passed or not entries:
            no_code.append(f.name)
            continue
        for e in entries:
            n_entries += 1
            fams[e["code"]] += 1
            if not e["suggestion"].strip() and e["code"] != "L2E001":
                empty_sug.append("%s:%s" % (f.name, e["code"]))
    ok = []

    def judge(idx, desc, cond):
        ok.append(cond)
        print("%s %s：%s" % (idx, desc, "PASS" if cond else "FAIL"))

    judge("①", "码解析 100%%（%d 负例每枚 ≥1 码条目）" % len(negs),
          len(negs) > 0 and not no_code)
    if no_code:
        print("   未解析到码：%s" % ", ".join(no_code))
    judge("②", "建议非空（L2E 除外为专属文案）", not empty_sug)
    if empty_sug:
        print("   空建议：%s" % ", ".join(empty_sug))

    tasks = json.loads((REPO / "eval" / "tasks" / "10_error_repair.json")
                       .read_text(encoding="utf-8"))
    tmp = tempfile.mkdtemp(prefix="l2fix-selfcheck-")
    try:
        results = {}
        for t in tasks:
            if str(t.get("id")) in ("127", "128", "129"):
                broken = broken_from_prompt(t["prompt"])
                p = Path(tmp) / ("broken_%s.lom" % t["id"])
                p.write_text(broken or "", encoding="utf-8")
                results[str(t["id"])] = analyze(p, lom_bin, l2_path)[0]
        judge("③", "129 得 high import 建议（from string import {len}）",
              any(e["confidence"] == "high" and e["action"] == "insert"
                  and "from string import {len}" in e["suggestion"]
                  for e in results.get("129", [])))
        judge("④", "127 建议含 map_get",
              any("map_get" in e["suggestion"] for e in results.get("127", [])))
        judge("⑤", "128 建议含管道触发物提示",
              any("管道" in e["suggestion"] for e in results.get("128", [])))
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    pos = REPO / "tools" / "selfcomp" / "cases" / "01_arith_int.lom"
    entries, passed, _ = analyze(pos, lom_bin, l2_path)
    judge("⑥", "干净正例（%s）输出\"L2 编译通过\"" % pos.name,
          passed and not entries)

    dist = " ".join("%s=%d" % (c, n) for c, n in sorted(fams.items()))
    print("self-check：%d/%d 断言通过（负例 %d 枚、码条目 %d 条，分布 %s）"
          % (sum(ok), len(ok), len(negs), n_entries, dist or "无"))
    return all(ok)


def main():
    try:  # Windows 控制台中文输出兜底
        sys.stdout.reconfigure(encoding="utf-8")
    except Exception:
        pass
    ap = argparse.ArgumentParser(
        description="L2 码进修复闭环通道（designs/0018 甲形态：L2 码→七族修复建议）")
    ap.add_argument("file", nargs="?", help="待诊断的 .lom 文件（--self-check 时可省略）")
    ap.add_argument("--json", action="store_true", help="输出 l2-fix/v1 JSON")
    ap.add_argument("--self-check", action="store_true",
                    help="质量锁定：全部负例 + eval 127/128/129 全链断言")
    ap.add_argument("--lom-bin", default=default_lom_bin(), help="宿主 lom 二进制路径")
    ap.add_argument("--l2", default=str(REPO / "examples" / "selfhost" / "self_comp.lom"),
                    help="L2 子集编译器源路径")
    args = ap.parse_args()

    if args.self_check:
        sys.exit(0 if self_check(args.lom_bin, args.l2) else 1)
    if not args.file:
        ap.error("缺少 <file.lom>（或改用 --self-check）")
    entries, passed, stdout = analyze(args.file, args.lom_bin, args.l2)
    if args.json:
        print(json.dumps({"schema": "l2-fix/v1", "file": str(args.file),
                          "entries": entries}, ensure_ascii=False, indent=2))
    else:
        print_human(entries, passed, stdout, args.file)
    sys.exit(0)


if __name__ == "__main__":
    main()
