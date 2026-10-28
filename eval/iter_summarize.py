#!/usr/bin/env python3
"""B3 迭代修复汇总——轮数分布 + 删失率单列 + C(N) 收敛曲线（designs/0027 §1.3 双报口径）

读 iter_eval.py 轨迹目录（s<k>/<id>/rounds.jsonl，末行 final 事件）：

  每题（跨 k 条独立轨迹）：
    - 轮数分布：中位 / 均值 / P90（最近邻秩）——**仅收敛轨迹参与**；
      删失轨迹单列删失率，不折算 R+1 混入均值（供料②度量纪律）
    - C(N) = N 轮内收敛的轨迹占比（分母 = 全部轨迹，删失永不收敛）
      **C(1) ≡ pass@1**（round-1 prompt 与 pass@1 同构，同管线校准位）
  总体：per-task 均值口径（与 pass@k 报告同式）+ 轨迹池化口径（审计复算用）

用法：python eval/iter_summarize.py --dir eval/iter_runs/<run-name>
输出：stdout 人类可读表 + <dir>/iter_summary.json
"""

import argparse
import json
import math
import os
import statistics
import sys

SAMPLE_DIR_PREFIX = "s"


def load_trajectories(base):
    """返回 {task_id: [ {converged, rounds_to_pass, max_rounds}, ... ]}（按 s<k> 序）。"""
    sample_dirs = sorted((d for d in os.listdir(base)
                          if d.startswith(SAMPLE_DIR_PREFIX)
                          and os.path.isdir(os.path.join(base, d))),
                         key=lambda d: int(d[len(SAMPLE_DIR_PREFIX):]))
    if not sample_dirs:
        sys.exit(f"{base} 下没有 s<k> 采样子目录（iter_eval.py 轨迹目录？）")
    per_task = {}
    for sd in sample_dirs:
        for tid in sorted(os.listdir(os.path.join(base, sd))):
            rl = os.path.join(base, sd, tid, "rounds.jsonl")
            if not os.path.isfile(rl):
                continue
            final = None
            with open(rl, encoding="utf-8") as f:
                for line in f:
                    rec = json.loads(line)
                    if rec.get("event") == "final":
                        final = rec
            if final is None:
                sys.exit(f"{rl} 缺 final 行（轨迹损坏，重跑该采样）")
            per_task.setdefault(tid, []).append(final)
    if not per_task:
        sys.exit("未发现任何 rounds.jsonl")
    return per_task


def dist_stats(rounds):
    """收敛轮数分布（仅收敛轨迹调用）：中位/均值/P90（最近邻秩）。"""
    if not rounds:
        return {"median": None, "mean": None, "p90": None, "n": 0}
    s = sorted(rounds)
    return {
        "median": statistics.median(s),
        "mean": round(sum(s) / len(s), 2),
        "p90": s[max(0, math.ceil(0.9 * len(s)) - 1)],
        "n": len(s),
    }


def c_curve(finals, n_max):
    """C(N) = N 轮内收敛轨迹占比（N=1..n_max；分母=全部轨迹）。"""
    total = len(finals)
    return [sum(1 for fr in finals
                if fr["converged"] and fr["rounds_to_pass"] <= n) / total
            for n in range(1, n_max + 1)]


def fmt(v, pct=True):
    if v is None:
        return "-"
    return f"{v * 100:.1f}%" if pct else f"{v:g}"


def main():
    ap = argparse.ArgumentParser(description="B3 迭代修复汇总（轮数分布 + C(N)）")
    ap.add_argument("--dir", required=True, help="iter_eval.py 的 run 目录")
    args = ap.parse_args()

    per_task = load_trajectories(args.dir)
    n_all_tasks = len(per_task)

    print(f"任务 {n_all_tasks} 题；C(1) ≡ pass@1（同管线校准位）；"
          f"删失单列，不混入轮数均值\n")

    tasks_out = {}
    pooled_finals = []
    for tid in sorted(per_task):
        finals = per_task[tid]
        pooled_finals.extend(finals)
        n = len(finals)
        conv = [fr for fr in finals if fr["converged"]]
        cens = n - len(conv)
        r_max = max(fr["max_rounds"] for fr in finals)
        curve = c_curve(finals, r_max)
        stats = dist_stats([fr["rounds_to_pass"] for fr in conv])
        tasks_out[tid] = {
            "trajectories": n, "converged": len(conv), "censored": cens,
            "censored_rate": cens / n,
            "rounds_dist_converged_only": stats,
            "C": {str(i + 1): curve[i] for i in range(len(curve))},
        }
        cs = " ".join(f"C({i + 1})={fmt(curve[i])}" for i in range(len(curve)))
        print(f"  {tid} | 轨迹 {n} | 收敛 {len(conv)} | 删失 {cens}"
              f"（{fmt(cens / n)}）| 轮数(收敛) 中位 {fmt(stats['median'], False)}"
              f" / 均值 {fmt(stats['mean'], False)} / P90 {fmt(stats['p90'], False)}"
              f" | {cs}")

    # 总体：per-task 均值（pass@k 同式）+ 池化
    n_curve = max(len(t["C"]) for t in tasks_out.values())
    mean_c = [sum(t["C"][str(i + 1)] for t in tasks_out.values()) / n_all_tasks
              for i in range(n_curve)]
    mean_cens = sum(t["censored_rate"] for t in tasks_out.values()) / n_all_tasks
    pooled_curve = c_curve(pooled_finals, n_curve)
    pooled_stats = dist_stats([fr["rounds_to_pass"] for fr in pooled_finals
                               if fr["converged"]])
    overall = {
        "per_task_mean": {  # 与 pass@k 报告同式（各题 C(N) 的算术平均）
            "C": {str(i + 1): mean_c[i] for i in range(n_curve)},
            "censored_rate": mean_cens,
        },
        "pooled": {
            "trajectories": len(pooled_finals),
            "converged": sum(1 for fr in pooled_finals if fr["converged"]),
            "censored": sum(1 for fr in pooled_finals if not fr["converged"]),
            "censored_rate": sum(1 for fr in pooled_finals
                                 if not fr["converged"]) / len(pooled_finals),
            "rounds_dist_converged_only": pooled_stats,
            "C": {str(i + 1): pooled_curve[i] for i in range(n_curve)},
        },
    }
    print(f"\n总体（per-task 均值，pass@k 同式）：删失率 {fmt(mean_cens)} | "
          + " ".join(f"C({i + 1})={fmt(mean_c[i])}" for i in range(n_curve)))
    print(f"总体（轨迹池化，{len(pooled_finals)} 条）：删失率 "
          f"{fmt(overall['pooled']['censored_rate'])} | "
          + " ".join(f"C({i + 1})={fmt(pooled_curve[i])}" for i in range(n_curve)))

    meta_path = os.path.join(args.dir, "run_meta.json")
    meta_echo = json.load(open(meta_path, encoding="utf-8")) if os.path.exists(meta_path) else None
    out = {"tasks": tasks_out, "overall": overall, "run_meta": meta_echo}
    out_path = os.path.join(args.dir, "iter_summary.json")
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(out, f, ensure_ascii=False, indent=2)
    print(f"\n明细已写 {out_path}")


if __name__ == "__main__":
    main()
