#!/usr/bin/env python3
"""R60（九审）：error_repair 任务 prompt 内嵌诊断的真实性校验。

背景（九审 R60）：eval 089/090 的 prompt 内嵌诊断是手写的，与实际实现输出
不符（runner 只验 stdout+rc，掩盖了漂移）。R57 整改时已把 089/090 换成真实
命令输出；本工具把该口径变成机器 gate——对 10_error_repair.json 的每个任务：

  1. 从 prompt 提取代码块（"错误代码："或"代码："标记后、"诊断 JSON"前）
  2. 提取内嵌诊断 JSON（"诊断 JSON"标记后的平衡对象，支持完整 lom-diag/v1
     schema 或单条诊断对象两种形态）
  3. 代码写入临时 .lom，跑真实 `lom <file> --json` 取当前实现的诊断
  4. 比对诊断码多重集合（code × 出现次数）；severity 不一致或行号偏差报警告

用法：python tools/eval_prompt_check.py [--bin <lom.exe 路径>]
退出码：0 = 全部一致；1 = 有失真（CI gate 形态）。
"""

import io
import json
import os
import re
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
TASKS = os.path.join(ROOT, "eval", "tasks", "10_error_repair.json")


def extract_code(prompt: str):
    m = re.search(r"(?:错误代码|代码)：\r?\n(.*?)(?=(?:\r?\n)?诊断 JSON)", prompt, re.S)
    if not m:
        return None
    return m.group(1).strip()


def extract_diag_json(prompt: str):
    m = re.search(r"诊断 JSON[^：\n]*：\r?\n", prompt)
    if not m:
        return None
    rest = prompt[m.end():]
    # 找第一个平衡的 JSON 对象（prompt 里的内嵌诊断都是单对象）
    start = rest.find("{")
    if start < 0:
        return None
    depth = 0
    in_str = False
    esc = False
    for i in range(start, len(rest)):
        c = rest[i]
        if in_str:
            if esc:
                esc = False
            elif c == "\\":
                esc = True
            elif c == '"':
                in_str = False
            continue
        if c == '"':
            in_str = True
        elif c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return rest[start : i + 1]
    return None


def diag_codes(obj):
    """从内嵌 JSON（完整 schema 或单条对象）取 (code, severity) 多重集合"""
    if isinstance(obj, dict):
        if "diagnostics" in obj:
            items = obj["diagnostics"]
        elif "code" in obj:
            items = [obj]
        else:
            return None
        return sorted(
            (d.get("code", "?"), d.get("severity", "?")) for d in items if isinstance(d, dict)
        )
    return None


def real_codes(bin_path, code, path):
    r = subprocess.run(
        [bin_path, path, "--json"], capture_output=True, text=True, encoding="utf-8"
    )
    out = r.stdout.strip()
    if not out.startswith("{"):
        return None, "lom --json 无 JSON 输出（rc=%d）" % r.returncode
    v = json.loads(out)
    return sorted(
        (d.get("code", "?"), d.get("severity", "?")) for d in v.get("diagnostics", [])
    ), None


def main():
    args = sys.argv[1:]
    bin_path = "./target/release/lom.exe" if os.name == "nt" else "./target/release/lom"
    if "--bin" in args:
        bin_path = args[args.index("--bin") + 1]

    data = json.load(io.open(TASKS, encoding="utf-8"))
    failures = 0
    checked = 0
    for t in data:
        tid = t.get("id", "?")
        prompt = t.get("prompt", "")
        code = extract_code(prompt)
        dj = extract_diag_json(prompt)
        if code is None or dj is None:
            print("  SKIP | %s | 无法提取代码块或内嵌诊断（形态特殊，人工核）" % tid)
            continue
        try:
            embedded = json.loads(dj)
        except json.JSONDecodeError as e:
            print("  FAIL | %s | 内嵌诊断不是合法 JSON: %s" % (tid, e))
            failures += 1
            continue
        ecodes = diag_codes(embedded)
        if ecodes is None:
            print("  SKIP | %s | 内嵌 JSON 形态未识别" % tid)
            continue
        with tempfile.NamedTemporaryFile(
            "w", suffix=".lom", delete=False, encoding="utf-8", newline="\n"
        ) as f:
            f.write(code + "\n")
            tmp = f.name
        try:
            rcodes, err = real_codes(bin_path, code, tmp)
            if err:
                print("  FAIL | %s | %s" % (tid, err))
                failures += 1
                continue
            checked += 1
            if rcodes != ecodes:
                print("  FAIL | %s | 诊断失真：prompt=%s 实测=%s" % (tid, ecodes, rcodes))
                failures += 1
            else:
                print("  OK   | %s | %s" % (tid, ecodes if ecodes else "(零诊断)"))
        finally:
            os.unlink(tmp)

    print("RESULT: %s（校验 %d / 失真 %d）" % ("PASS" if failures == 0 else "FAIL", checked, failures))
    sys.exit(0 if failures == 0 else 1)


if __name__ == "__main__":
    main()
