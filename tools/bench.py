# -*- coding: utf-8 -*-
"""tools/bench.py — Lom 性能基准批量计时工具（designs/0022 §2.4 B 件：基准资产入库）

用途：对 examples/bench.lom 的十负载（既有四负载 + 0022 新增六负载）逐负载逐 n 档
跑 `<lom> examples/bench.lom -- <bench> <n>`，每点 --runs 次（默认 3）取中位
wall-clock 毫秒，并与内置期望函数表（Python 独立实现，与 .lom 侧负载算法对照）
逐字比对 stdout——计时与正确性一次收口。

用法：
  python tools/bench.py                              # quick 档（默认）：十负载全矩阵
  python tools/bench.py --full                       # full 档：更大的 n 矩阵
  python tools/bench.py --bench arith_loop           # 只跑单负载（跑该档全部 n）
  python tools/bench.py --bench arith_loop --n 200000  # 单负载单 n 覆盖（--n 需配 --bench）
  python tools/bench.py --runs 5                     # 每点 5 次取中位（默认 3）
  python tools/bench.py --lom-bin path/to/lom(.exe)  # 宿主二进制覆盖
  python tools/bench.py --out result.json            # 结果落 JSON（供下次 --baseline 对照）
  python tools/bench.py --baseline old.json          # 与前次结果逐点算倍率列

输出行式：`负载 n=值 run=t1/t2/t3 ms 中位=值 out=期望✓`（不符时 out=实际(期望 X)✗，
并附 rc/stderr 诊断行）。结尾 `RESULT: PASS/FAIL`——任何点输出不符（rc≠0 或
stdout≠期望）即 FAIL，退出码 1。--baseline 的倍率列是逐点对照信息，不影响 PASS/FAIL。

性能证据边界声明（designs/0022 §6 纪律）：
  - 只报同机实测 wall-clock（subprocess 计时，含进程启动开销 ~15ms——与
    target/probes/perf_supply/timeit.py 供料 harness 同口径）；
  - 每点 --runs 次逐次列原始值、取中位，不隐藏离散度；
  - 不作"全面提速"类总括宣称——倍率列仅逐点陈述，结论由读者按点判读。

纪律：本工具纯只读（不跑任何 git 命令、不写仓库文件；--out 落盘路径由调用方
指定，结果文件建议放 target/ 下 gitignored 区）；不进 CI（designs/0022 裁决点 3
已裁：入库 + 手动批验收形态）；零第三方依赖（标准库 subprocess/time/statistics/
json/argparse）。
"""

import argparse
import json
import os
import statistics
import subprocess
import sys
import time
from datetime import datetime
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

# ---------------------------------------------------------------------------
# n 档常量表（quick / full 两档；档位量级参照 target/probes/perf_supply 供料实测：
#   arith_loop ~0.72ms/千次、list_get_nth 与 str_concat 平方、lookup 平方等）。
# recurse 上限 30000：实测 v1.9.3 在 n=50000 栈溢出（递归深度为登记在案的既定
# 边界，基准不贴边界值）。
# ---------------------------------------------------------------------------
LEVELS_QUICK = {
    "list_build":   [1000, 8000],
    "lookup":       [200, 400],
    "recurse":      [10000, 20000],
    "map_lookup":   [1000, 5000],
    "arith_loop":   [200000, 500000],
    "call_density": [100000, 250000],
    "closure_loop": [100000, 250000],
    "list_get_nth": [2000, 5000],
    "map_hit_far":  [50000, 100000],
    "str_concat":   [10000, 25000],
}
LEVELS_FULL = {
    "list_build":   [1000, 2000, 4000],
    "lookup":       [200, 400, 800],
    "recurse":      [10000, 20000, 30000],
    "map_lookup":   [1000, 5000, 20000],
    "arith_loop":   [200000, 500000, 1000000],
    "call_density": [100000, 250000, 500000],
    "closure_loop": [100000, 250000, 500000],
    "list_get_nth": [2000, 5000, 10000],
    "map_hit_far":  [50000, 100000, 250000],
    "str_concat":   [10000, 25000, 50000],
}
LEVELS = {"quick": LEVELS_QUICK, "full": LEVELS_FULL}

# ---------------------------------------------------------------------------
# 期望值函数表（Python 独立实现；与 .lom 侧负载算法逐条对照）
#   list_build   : sum(1..n) = n(n+1)/2                 （cons 头插后 for-in 求和）
#   lookup       : n 次查 key=1,每次得 1*2 → 2n
#   recurse      : countdown 终值 0
#   map_lookup   : n 次查 "1",每次得 1*2 → 2n
#   arith_loop   : sum_{i=1..n} ( i%7 - (i//3)%5 )      （Lom Int 截断除法,i≥1 与 // 一致）
#   call_density : 每轮 acc = acc+i+1 → sum(1..n) + n
#   closure_loop : 每轮 acc = f(acc) = acc+i → sum(1..n)
#   list_get_nth : n 次 list_get(xs, n-1),每次得最先插入的 1 → n
#   map_hit_far  : sum_{i=1..n} 2i = n(n+1)
#   str_concat   : n 次拼接 "x" 后 len(s) → n
# ---------------------------------------------------------------------------
_ARITH_PERIOD = 105  # lcm(7,15)：f(i+105)=f(i)（105%7==0 且 105//3=35、35%5==0）


def _arith_f(i):
    return (i % 7) - ((i // 3) % 5)


_ARITH_FULL_CYCLE = sum(_arith_f(i) for i in range(1, _ARITH_PERIOD + 1))


def _arith_loop(n):
    # 周期化加速（n 到 1M 时朴素循环偏慢）；模块加载时对 0..399 与朴素式自检互锁
    q, r = divmod(n, _ARITH_PERIOD)
    return q * _ARITH_FULL_CYCLE + sum(_arith_f(i) for i in range(1, r + 1))


for _n in range(0, 400):
    assert _arith_loop(_n) == sum(_arith_f(i) for i in range(1, _n + 1)), _n

EXPECTED = {
    "list_build":   lambda n: n * (n + 1) // 2,
    "lookup":       lambda n: 2 * n,
    "recurse":      lambda n: 0,
    "map_lookup":   lambda n: 2 * n,
    "arith_loop":   _arith_loop,
    "call_density": lambda n: n * (n + 1) // 2 + n,
    "closure_loop": lambda n: n * (n + 1) // 2,
    "list_get_nth": lambda n: n,
    "map_hit_far":  lambda n: n * (n + 1),
    "str_concat":   lambda n: n,
}

DEFAULT_LOM = REPO / "target" / "release" / ("lom.exe" if os.name == "nt" else "lom")
BENCH_LOM = REPO / "examples" / "bench.lom"
SUBPROC_TIMEOUT = 600  # 秒；单点最大档（lookup n=800 full ~2.5s）留足余量


def lom_version(lom_bin):
    try:
        r = subprocess.run([str(lom_bin), "--version"], capture_output=True,
                           text=True, timeout=30)
        blob = (r.stdout or r.stderr).strip()
        return blob.splitlines()[0] if blob else "unknown"
    except Exception:
        return "unknown"


def run_point(lom_bin, bench, n, runs):
    """跑一 benchmark 点 runs 次。返回 (times_ms, last_out, last_rc, last_stderr)。"""
    cmd = [str(lom_bin), str(BENCH_LOM), "--", bench, str(n)]
    times, out, rc, err = [], "", -1, ""
    for _ in range(runs):
        t0 = time.perf_counter()
        try:
            r = subprocess.run(cmd, capture_output=True, text=True,
                               timeout=SUBPROC_TIMEOUT)
            ms = (time.perf_counter() - t0) * 1000.0
            out, rc, err = r.stdout.strip(), r.returncode, r.stderr.strip()
        except subprocess.TimeoutExpired:
            ms = (time.perf_counter() - t0) * 1000.0
            out, rc, err = "(timeout)", -1, "subprocess timeout"
        times.append(ms)
    return times, out, rc, err


def main():
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")

    ap = argparse.ArgumentParser(
        description="Lom 性能基准批量计时（designs/0022 §2.4 B 件；只读工具，不跑 git）")
    g = ap.add_mutually_exclusive_group()
    g.add_argument("--quick", action="store_true", help="quick 档（默认）")
    g.add_argument("--full", action="store_true", help="full 档（更大 n 矩阵）")
    ap.add_argument("--bench", help="只跑单负载（名：%s）" % "|".join(EXPECTED))
    ap.add_argument("--n", type=int, help="覆盖 n（需与 --bench 同用）")
    ap.add_argument("--runs", type=int, default=3, help="每点次数，取中位（默认 3）")
    ap.add_argument("--lom-bin", default=str(DEFAULT_LOM), help="宿主二进制路径")
    ap.add_argument("--baseline", help="前次结果 JSON（逐点算倍率列；信息列不影响 PASS/FAIL）")
    ap.add_argument("--out", help="本次结果落 JSON（供下次 --baseline 对照）")
    args = ap.parse_args()

    if args.runs < 1:
        print("ERROR: --runs 至少为 1", file=sys.stderr)
        return 2
    if args.n is not None and not args.bench:
        print("ERROR: --n 需与 --bench 同用（全矩阵覆盖单 n 无意义且部分负载会超时）",
              file=sys.stderr)
        return 2
    if args.bench and args.bench not in EXPECTED:
        print("ERROR: 未知负载 %r；可用：%s" % (args.bench, " ".join(EXPECTED)),
              file=sys.stderr)
        return 2

    lom_bin = Path(args.lom_bin)
    if not lom_bin.exists():
        print("ERROR: 宿主二进制不存在：%s（先 cargo build --release）" % lom_bin,
              file=sys.stderr)
        return 2

    mode = "full" if args.full else "quick"
    if args.bench:
        levels = {args.bench: [args.n] if args.n is not None else LEVELS[mode][args.bench]}
    else:
        levels = LEVELS[mode]

    base = {}
    if args.baseline:
        with open(args.baseline, encoding="utf-8") as f:
            for row in json.load(f).get("results", []):
                base[(row["bench"], row["n"])] = row["median_ms"]

    print("# tools/bench.py 档=%s runs=%d lom=%s bench=%s"
          % (mode, args.runs, lom_version(lom_bin), BENCH_LOM.name))

    rows, fails, total = [], 0, 0
    for bench, ns in levels.items():
        for n in ns:
            total += 1
            expected = str(EXPECTED[bench](n))
            times, out, rc, err = run_point(lom_bin, bench, n, args.runs)
            ok = (rc == 0 and out == expected)
            if not ok:
                fails += 1
            med = statistics.median(times)
            runs_str = "/".join("%.1f" % t for t in times)
            if ok:
                out_field = "%s✓" % out
            else:
                out_field = "%s(期望 %s)✗" % (out, expected)
            line = "%-13s n=%-7d run=%-27s ms 中位=%-9.1f out=%s" % (
                bench, n, runs_str, med, out_field)
            b = base.get((bench, n))
            if args.baseline:
                if b is not None:
                    line += "  基准=%.1f 倍率=%.3f" % (b, med / b)
                else:
                    line += "  基准=N/A"
            print(line, flush=True)
            if not ok:
                print("      ! rc=%d stderr=%s" % (rc, err[:200]), flush=True)
            rows.append({"bench": bench, "n": n, "runs_ms": times,
                         "median_ms": med, "expected": expected, "out": out,
                         "ok": ok})

    if args.out:
        payload = {
            "meta": {"date": datetime.now().isoformat(timespec="seconds"),
                     "mode": mode, "runs": args.runs,
                     "lom_bin": str(lom_bin), "lom_version": lom_version(lom_bin)},
            "results": rows,
        }
        with open(args.out, "w", encoding="utf-8") as f:
            json.dump(payload, f, ensure_ascii=False, indent=1)
        print("# 结果已落 %s" % args.out)

    print("RESULT: %s（%d/%d 点输出比对通过）"
          % ("PASS" if fails == 0 else "FAIL", total - fails, total))
    return 0 if fails == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
