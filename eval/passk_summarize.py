#!/usr/bin/env python3
"""pass@k 多采样汇总（L 工作包，2026-09-07）

对 llm_eval.py --samples N 产出的多采样目录（s1..sN）逐采样调用 run.ps1 评分
（评分事实源不二设——stdout 比对 + 退出码 0 的既有口径），构建逐任务通过矩阵，
计算 pass@k 无偏估计：

    pass@k = E[ 1 - C(n-c, k) / C(n, k) ]

n = 采样数、c = 该任务通过的采样数（HumanEval / Codex 论文同式；Fraction
整数精确计算，无浮点漂移）。missing 候选（模型未产出该任务块）计为不通过。

用法：
  python eval/passk_summarize.py --dir eval/candidates_rerun/deepseek_deepseek-v4-pro \
      --ks 1 5 10 --lom-bin ./target/release/lom.exe

输出：stdout 汇总表 + <dir>/passk_summary.json（逐任务矩阵 + 分类汇总 +
run_meta 参数回显，报告审计用）。
"""

import argparse
import json
import os
import re
import subprocess
import sys
from fractions import Fraction

EVAL_DIR = os.path.dirname(os.path.abspath(__file__))
RUNNER = os.path.join(EVAL_DIR, "runner", "run.ps1")
TASKS_DIR = os.path.join(EVAL_DIR, "tasks")

# run.ps1 -Verbose 的逐任务行：`  [001] PASS (arithmetic)` / FAIL / MISSING candidate
RESULT_RE = re.compile(r"^\s*\[(\d{3})\] (PASS|FAIL|MISSING)", re.M)
SAMPLE_DIR_RE = re.compile(r"^s(\d+)$")


def load_tasks() -> dict:
    """任务全集：id -> category（与 run.ps1 相同的遍历源）"""
    out = {}
    for fn in sorted(os.listdir(TASKS_DIR)):
        if not fn.endswith(".json"):
            continue
        with open(os.path.join(TASKS_DIR, fn), encoding="utf-8") as f:
            tasks = json.load(f)
        category = fn[:-5].split("_", 1)[1] if "_" in fn[:-5] else fn[:-5]
        for t in tasks:
            out[t["id"]] = category
    return out


def score_sample(sample_dir: str, lom_bin: str) -> dict:
    """跑一次 run.ps1 评分，返回 {task_id: bool}。

    退出码 0/1 都是正常终局（1 = 存在失败任务）；解析不到行或任务数对不上才报错。
    """
    cmd = ["powershell", "-ExecutionPolicy", "Bypass", "-File", RUNNER,
           "-CandidatesDir", sample_dir, "-LomBin", lom_bin, "-Verbose"]
    # run.ps1 输出 UTF-8（含中文任务的 expected/actual 回显）——必须显式指定解码，
    # 否则 Windows Python 默认按 GBK 解码直接 UnicodeDecodeError（HANDOVER §3.3 家族坑）
    proc = subprocess.run(cmd, capture_output=True, timeout=1800,
                          encoding="utf-8", errors="replace")
    if proc.returncode not in (0, 1):
        sys.exit(f"run.ps1 异常退出 code={proc.returncode}\nstderr: {proc.stderr[:800]}")
    results = {tid: m == "PASS" for tid, m in RESULT_RE.findall(proc.stdout)}
    return results


def pass_at_k(n: int, c: int, k: int) -> Fraction:
    """单任务 pass@k 无偏估计：1 - C(n-c,k)/C(n,k)，整数精确。

    n-c < k（失败采样不够填 k 个槽）时必然至少一个通过，值为 1。
    """
    if k < 1 or k > n:
        sys.exit(f"k={k} 越界（采样数 n={n}）")
    if n - c < k:
        return Fraction(1)
    # C(n-c,k)/C(n,k) = prod_{i=0..k-1} (n-c-i) / (n-i)
    num, den = 1, 1
    for i in range(k):
        num *= (n - c - i)
        den *= (n - i)
    return Fraction(1) - Fraction(num, den)


def fmt(fr: Fraction) -> str:
    return f"{float(fr) * 100:.1f}%"


def main():
    ap = argparse.ArgumentParser(description="pass@k 多采样汇总")
    ap.add_argument("--dir", required=True, help="模型候选目录（含 s1..sN 子目录）")
    ap.add_argument("--ks", type=int, nargs="+", default=[1, 5, 10],
                    help="要计算的 k 值（须全部 <= 采样数）")
    ap.add_argument("--lom-bin", default="./target/release/lom.exe")
    args = ap.parse_args()

    base = args.dir
    samples = sorted(
        (int(m.group(1)), os.path.join(base, m.group(0)))
        for m in (SAMPLE_DIR_RE.match(d) for d in os.listdir(base))
        if m
    )
    if not samples:
        sys.exit(f"{base} 下没有 s<数字> 采样子目录")
    n = len(samples)
    for k in args.ks:
        if k > n:
            sys.exit(f"k={k} 大于采样数 n={n}")

    tasks = load_tasks()
    print(f"采样数 n={n}，任务总数 {len(tasks)}，k={args.ks}\n")

    # 逐采样评分 → 矩阵 matrix[tid] = [bool] * n
    matrix = {tid: [] for tid in tasks}
    for idx, (sno, sdir) in enumerate(samples):
        results = score_sample(sdir, args.lom_bin)
        scored = sorted(results)
        if not scored:
            sys.exit(f"[{os.path.basename(sdir)}] run.ps1 没有回出任何任务行（候选目录损坏？）")
        unknown = sorted(set(results) - set(tasks))
        if unknown:
            sys.exit(f"[{os.path.basename(sdir)}] 出现未知任务 id {unknown[:10]}")
        # 局部采集（llm_eval.py --only）是合法场景——任务数对不上降级为警告
        if len(scored) != len(tasks):
            missing = sorted(set(tasks) - set(results))
            print(f"  警告：s{sno} 只覆盖 {len(scored)}/{len(tasks)} 任务，"
                  f"缺 {missing[:10]}{'...' if len(missing) > 10 else ''}（按缺失=不通过计）")
        for tid in scored:
            matrix[tid].append(results[tid])
        for tid in sorted(set(tasks) - set(scored)):
            matrix[tid].append(False)  # 该采样无候选 = 不通过，矩阵保持每任务 n 格
        passed = sum(results.values())
        print(f"[s{sno}] 评分完成：{passed}/{len(scored)}")

    # 汇总：总体 + 分类
    cats = sorted(set(tasks.values()))
    overall = {k: sum((pass_at_k(n, sum(matrix[tid]), k) for tid in tasks),
                      Fraction(0)) / len(tasks) for k in args.ks}
    by_cat = {
        cat: {k: sum((pass_at_k(n, sum(matrix[tid]), k) for tid in tasks
                      if tasks[tid] == cat), Fraction(0))
              / sum(1 for tid in tasks if tasks[tid] == cat)
              for k in args.ks}
        for cat in cats
    }

    print("\n===== pass@k（无偏估计） =====")
    print(f"{'k':>4} | {'总体':>8}")
    for k in args.ks:
        print(f"{k:>4} | {fmt(overall[k]):>8}")
    print(f"\n===== 按分类 pass@{' / pass@'.join(str(k) for k in args.ks)} =====")
    for cat in cats:
        row = " / ".join(fmt(by_cat[cat][k]) for k in args.ks)
        cnt = sum(1 for tid in tasks if tasks[tid] == cat)
        print(f"  {cat:<16} ({cnt:>2})  {row}")

    # 非 10/10 满分任务（报告的失败分析素材）
    print("\n===== 通过采样数 < n 的任务 =====")
    imperfect = [(tid, sum(matrix[tid])) for tid in sorted(tasks)
                 if sum(matrix[tid]) < n]
    if not imperfect:
        print("（无——全部任务 n/n 满分通过）")
    for tid, c in imperfect:
        print(f"  {tid}（{tasks[tid]}）：{c}/{n}")

    # run_meta 参数回显（审计）
    meta_path = os.path.join(base, "run_meta.json")
    meta_echo = None
    if os.path.exists(meta_path):
        with open(meta_path, encoding="utf-8") as f:
            meta_echo = json.load(f)

    summary = {
        "samples": n,
        "ks": args.ks,
        "task_count": len(tasks),
        "overall": {str(k): float(overall[k]) for k in args.ks},
        "by_category": {cat: {str(k): float(by_cat[cat][k]) for k in args.ks}
                        for cat in cats},
        "matrix": {tid: matrix[tid] for tid in sorted(tasks)},
        "run_meta": meta_echo,
    }
    out_path = os.path.join(base, "passk_summary.json")
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(summary, f, ensure_ascii=False, indent=2)
    print(f"\n明细已写 {out_path}")


if __name__ == "__main__":
    main()
