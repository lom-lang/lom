#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""specs/acri/consumer.py — ACRI v0.1 参考消费器（规范 §4 的可运行实装）

不依赖 Lom 语言知识的最小 ACRI 消费者：引导码表 → 消费 check --json →
按公理 1 过滤可应用建议 → 驱动 plan 闭环。它证明规范接口可以被一个
对实现语言一无所知的第三方程序消费（ACRI §0 实现方中立）。

用法（对参考实现 Lom）：
  python specs/acri/consumer.py <file.lom> [--lom-bin ./target/release/lom.exe] [--apply]
不带 --apply 只呈现闭环（AXIOM 3：plan 与 apply 分离）；带 --apply 时按
AXIOM 1 客户侧再过滤一遍（纵深防御——不信任工具侧的过滤）。
退出码：0 = 分析完成（无论有无诊断——与 ACRI §2.2 fix 语义一致）。
"""

import argparse
import json
import subprocess
import sys


def run(bin_path, args):
    """跑一条 ACRI L1 命令，返回 (stdout, exit_code)。"""
    r = subprocess.run([bin_path] + args, capture_output=True, text=True)
    return r.stdout, r.returncode


def main():
    ap = argparse.ArgumentParser(description="ACRI v0.1 minimal reference consumer")
    ap.add_argument("file", help="target source file")
    ap.add_argument("--lom-bin", default="./target/release/lom.exe",
                    help="path to the ACRI L1 tool (default: ./target/release/lom.exe)")
    ap.add_argument("--apply", action="store_true",
                    help="drive fix --apply after planning (AXIOM 1 enforced client-side)")
    ns = ap.parse_args()

    # ① 引导码表（ACRI §2.1 codes —— agent 的引导文件）
    table_out, rc = run(ns.lom_bin, ["codes", "--json"])
    if rc != 0:
        print(f"[consumer] codes 出口不可用（rc={rc}）——不是 L1-conformant 工具？", file=sys.stderr)
        return 2
    table = {c["code"]: c for c in json.loads(table_out)["codes"]}
    print(f"[bootstrap] 码表 {len(table)} 条（命名空间: "
          f"{sorted({c['category'] for c in table.values()})}）")

    # ② 消费诊断（ACRI §2.1 check --json；rc 遵 §2.2 矩阵）
    check_out, rc = run(ns.lom_bin, [ns.file, "--check", "--json"])
    envelope = json.loads(check_out)
    print(f"[check] ok={envelope['ok']} 诊断 {envelope['summary']['total']} 条 "
          f"(error {envelope['summary']['errors']} / warning {envelope['summary']['warnings']})，rc={rc}")
    for d in envelope["diagnostics"]:
        meta = table.get(d["code"], {})
        print(f"  - {d['code']} [{d['severity']}] {d.get('line')}:{d.get('line') and d.get('col')}"
              f" — {meta.get('summary', '?')}")
        if d.get("hint"):
            print(f"      hint: {d['hint']}")

    # ③ 驱动修复计划（ACRI §2.1 fix --plan；AXIOM 3：此处零副作用）
    plan_out, _ = run(ns.lom_bin, ["fix", ns.file, "--plan", "--json"])
    plan = json.loads(plan_out)
    applicable = []
    for entry in plan.get("plans", []):
        for f in entry.get("fixes", []):
            ok = f.get("confidence") == "high" and f.get("action") != "hint"  # AXIOM 1
            print(f"  [plan] {entry['diagnostic']['code']}: {f.get('action')}/"
                  f"{f.get('confidence')} {'✓可自动' if ok else '—仅建议'}")
            if ok:
                applicable.append(entry)
                break

    # ④ 可选应用（AXIOM 1 客户侧过滤后的显式门）
    if ns.apply and applicable:
        _, rc = run(ns.lom_bin, ["fix", ns.file, "--apply"])
        print(f"[apply] rc={rc}（应用后请重跑本消费器复核）")
    elif ns.apply:
        print("[apply] 无公理 1 可应用项——不调用")
    return 0


if __name__ == "__main__":
    sys.exit(main())
