#!/usr/bin/env python3
"""B3 迭代修复效率采集——单步反馈链协议（designs/0027 §1.2 裁决 0-2 方案 B，2026-10-28）

协议（"代码即记忆"，非对话式）：
  每轮 = 单条 user 消息调用。
  round 1 = 标准题 prompt（与 pass@1 同构：_context.md + "### Task <id>" + 题面
    + _footer.md——即 eval/prompts/_generate.ps1 的重组形态）。
  round N>1 = 语言速览 + 原题面 + 上轮候选完整代码 + 上轮实跑真实诊断：
      - 静态：`lom <file> --check --json` 原文
      - 运行时：`lom <file>` 的退出码 / stdout / stderr 原文
    （双真实采集纪律：诊断文本一律实跑采集，禁手写——eval/README §iterative）
  模型每轮回写完整代码（=== <id>.lom === 块提取，与 llm_eval.py 同式）。
  通过判定与 runner 同口径：stdout 逐字（CRLF 归一）+ 退出码 0；
  通过即早停；达 max_rounds 未通过记删失（删失单列报，不折算轮数）。

轨迹布局（<out-root>/<out-name>/ 下）：
  s<k>/<id>/roundN.lom          第 N 轮候选代码（模型产出或 dry-run 退化候选）
  s<k>/<id>/roundN_prompt.txt   当轮发出的完整 prompt（审计用）
  s<k>/<id>/roundN_reply.txt    当轮模型原始回复（dry-run 无此文件）
  s<k>/<id>/roundN_check.json   上轮候选的静态诊断原文（失败轮采集）
  s<k>/<id>/roundN_run.txt      上轮候选运行记录（rc/stdout/stderr 原文）
  s<k>/<id>/rounds.jsonl        每轮一行摘要 + 末行 final 事件（汇总器唯一输入）
  run_meta.json                 参数/时间戳/任务面/收敛删失计数

用法：
  # 正式采集（端点与密钥同 llm_eval.py：DEEPSEEK_API_KEY / GLM*_API_KEY 或 eval/.api_keys.json）
  python eval/iter_eval.py --provider glm-coding --model glm-5.3 --samples 10 --temperature 1.0
  # 管线自验（零 API）：round1 恒回传题面内嵌 broken、round2 起恒回传参考解
  python eval/iter_eval.py --dry-run --samples 2 --out-name dryrun --out-root target/probes/b3_impl/iter
  # 删失路径自验：R=1 下 broken 不通过的题全部删失
  python eval/iter_eval.py --dry-run --samples 2 --max-rounds 1 --out-name dryrun_cens --out-root target/probes/b3_impl/iter

汇总：python eval/iter_summarize.py --dir <上述 run 目录>
"""

import argparse
import datetime
import json
import os
import re
import subprocess
import sys
import tempfile
import time

# 复用 llm_eval 的端点配置/密钥读取/调用重试/块提取（单文件模块，import 零副作用）
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import llm_eval  # noqa: E402

EVAL_DIR = os.path.dirname(os.path.abspath(__file__))
PROMPTS_DIR = os.path.join(EVAL_DIR, "prompts")
TASKS_FILE = os.path.join(EVAL_DIR, "tasks", "11_iterative_repair.json")
DEFAULT_R = 5  # 题面 max_rounds 缺省（designs/0027 裁决 0-4）

# 与 tools/eval_prompt_check.py 同锚：题面内嵌 broken 代码提取（dry-run 退化候选用）
BROKEN_RE = re.compile(r"(?:错误代码|代码)：\r?\n(.*?)(?=(?:\r?\n)?诊断 JSON)", re.S)
RUNTIME_CODE_RE = re.compile(r"\[(RUNTIME\d{3})\]")


def read(path):
    with open(path, encoding="utf-8") as f:
        return f.read()


def broken_from_prompt(prompt):
    m = BROKEN_RE.search(prompt)
    return m.group(1).strip() + "\n" if m else None


def round1_prompt(task, context, footer):
    return context + f"### Task {task['id']}\n\n{task['prompt']}\n\n" + footer


def roundN_prompt(task, rnd, prev_code, static_txt, run_rec, context, footer):
    """round N>1：原题面 + 上轮代码 + 上轮真实诊断（静态 JSON 原文 + 运行时三件套）。"""
    head = context + f"### Task {task['id']}\n\n{task['prompt']}\n\n"
    fb = (
        f"---\n\n## 第 {rnd} 轮修复（上轮真实运行反馈）\n\n"
        f"你上一轮（round {rnd - 1}）提交的代码经真实工具链运行未通过。"
        f"以下是上轮完整代码与真实输出（与题面诊断同源纪律：均为实跑采集原文）：\n\n"
    )
    if prev_code is not None:
        fb += f"上轮代码：\n```\n{prev_code}```\n\n"
    else:
        fb += "上轮未产出任何 === <id>.lom === 代码块。\n\n"
    fb += f"上轮静态诊断（lom <file> --check --json 原文）：\n{static_txt or '(无输出)'}\n\n"
    fb += (
        f"上轮运行（lom <file>，退出码 {run_rec['rc']}）：\n"
        f"stdout:\n{run_rec['stdout'] or '(空)'}\n"
        f"stderr:\n{run_rec['stderr'] or '(空)'}\n\n"
        "请依据以上真实诊断继续修复，输出修复后的完整代码"
        "（要求与原题面相同：正确运行并输出期望结果）。\n\n"
    )
    return head + fb + footer


def run_candidate(lom_bin, code, expected, path):
    """评分口径与 run.ps1 Test-OneTask 一致：CRLF 归一后 stdout 逐字比对 + rc==0。"""
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(code)
    proc = subprocess.run([lom_bin, path], capture_output=True, timeout=120,
                          encoding="utf-8", errors="replace")
    ok = proc.returncode == 0 and proc.stdout.replace("\r\n", "\n") == expected
    return {"pass": ok, "rc": proc.returncode, "stdout": proc.stdout, "stderr": proc.stderr}


def static_check(lom_bin, path):
    proc = subprocess.run([lom_bin, path, "--check", "--json"], capture_output=True,
                          timeout=120, encoding="utf-8", errors="replace")
    return proc.stdout.strip()


def diag_codes(static_txt):
    try:
        v = json.loads(static_txt)
    except (json.JSONDecodeError, TypeError):
        return []
    if not isinstance(v, dict):
        return []
    return [d.get("code", "?") for d in v.get("diagnostics", []) if isinstance(d, dict)]


def main():
    ap = argparse.ArgumentParser(description="B3 迭代修复采集（单步反馈链协议）")
    ap.add_argument("--provider", default=None, choices=sorted(llm_eval.PROVIDERS))
    ap.add_argument("--model", default=None)
    ap.add_argument("--samples", type=int, default=1, help="每题独立轨迹数 k")
    ap.add_argument("--max-rounds", type=int, default=None,
                    help="全局轮数上限 R（缺省用题面 max_rounds 字段，再缺省 5）")
    ap.add_argument("--temperature", type=float, default=None)
    ap.add_argument("--thinking", action="store_true")
    ap.add_argument("--max-tokens", type=int, default=16384)
    ap.add_argument("--only", default=None, help="只跑指定题（逗号分隔 id，调试用）")
    ap.add_argument("--dry-run", action="store_true",
                    help="零 API 管线自验：round1 恒回传题面 broken、round2 起恒回传参考解")
    ap.add_argument("--lom-bin", default="./target/release/lom.exe")
    ap.add_argument("--out-root", default=os.path.join(EVAL_DIR, "iter_runs"))
    ap.add_argument("--out-name", default=None,
                    help="输出目录名（默认 dryrun 或 <provider>_<model>）")
    args = ap.parse_args()

    if not args.dry_run and (not args.provider or not args.model):
        sys.exit("正式采集需 --provider 与 --model（或用 --dry-run 自验管线）")
    if args.samples < 1:
        sys.exit("--samples 必须 >= 1")

    tasks = json.loads(read(TASKS_FILE))
    if args.only:
        want = {s.strip() for s in args.only.split(",")}
        tasks = [t for t in tasks if t["id"] in want]
        missing = want - {t["id"] for t in tasks}
        if missing:
            sys.exit(f"--only 含未知任务 id：{sorted(missing)}")

    context = read(os.path.join(PROMPTS_DIR, "_context.md"))
    footer = read(os.path.join(PROMPTS_DIR, "_footer.md"))
    out_dir = os.path.join(args.out_root,
                           args.out_name or ("dryrun" if args.dry_run
                                             else f"{args.provider}_{args.model}"))
    os.makedirs(out_dir, exist_ok=True)

    key = None if args.dry_run else llm_eval.load_key(args.provider)
    t0 = datetime.datetime.now(datetime.timezone.utc)
    totals = {"trajectories": 0, "converged": 0, "censored": 0, "round1_pass": 0}

    for k in range(1, args.samples + 1):
        for task in tasks:
            tid = task["id"]
            R = args.max_rounds or task.get("max_rounds", DEFAULT_R)
            traj = os.path.join(out_dir, f"s{k}", tid)
            os.makedirs(traj, exist_ok=True)
            rounds_log = open(os.path.join(traj, "rounds.jsonl"), "w",
                              encoding="utf-8", newline="\n")
            prev_code = None
            static_txt = None
            run_rec = None
            converged = False
            rounds_run = 0
            for rnd in range(1, R + 1):
                rounds_run = rnd
                if rnd == 1:
                    prompt = round1_prompt(task, context, footer)
                else:
                    prompt = roundN_prompt(task, rnd, prev_code, static_txt,
                                           run_rec or {"rc": None, "stdout": "", "stderr": ""},
                                           context, footer)
                with open(os.path.join(traj, f"round{rnd}_prompt.txt"), "w",
                          encoding="utf-8", newline="\n") as f:
                    f.write(prompt)

                if args.dry_run:
                    # 退化候选：round1 = 题面内嵌 broken（原样回传）；round>=2 = 参考解
                    code = (broken_from_prompt(task["prompt"]) if rnd == 1
                            else task["solution"])
                else:
                    reply = llm_eval.call_api(
                        llm_eval.PROVIDERS[args.provider], key, args.model, prompt,
                        args.temperature, args.max_tokens, args.thinking)
                    with open(os.path.join(traj, f"round{rnd}_reply.txt"), "w",
                              encoding="utf-8", newline="\n") as f:
                        f.write(reply)
                    code = llm_eval.extract_blocks(reply).get(tid)
                    time.sleep(2)  # 礼貌限速（同 llm_eval）

                cand_path = os.path.join(traj, f"round{rnd}.lom")
                if code is None:
                    passed, rc, note = False, None, "模型未产出 === %s.lom === 块" % tid
                    scodes, rcodes = [], []
                else:
                    with open(cand_path, "w", encoding="utf-8", newline="\n") as f:
                        f.write(code)
                    run_rec = run_candidate(args.lom_bin, code, task["expected"], cand_path)
                    passed, rc, note = run_rec["pass"], run_rec["rc"], None
                    static_txt = static_check(args.lom_bin, cand_path)
                    with open(os.path.join(traj, f"round{rnd}_check.json"), "w",
                              encoding="utf-8", newline="\n") as f:
                        f.write(static_txt + "\n")
                    with open(os.path.join(traj, f"round{rnd}_run.txt"), "w",
                              encoding="utf-8", newline="\n") as f:
                        f.write("rc=%s\n--- stdout ---\n%s--- stderr ---\n%s"
                                % (run_rec["rc"], run_rec["stdout"], run_rec["stderr"]))
                    scodes = diag_codes(static_txt)
                    rcodes = sorted(set(RUNTIME_CODE_RE.findall(run_rec["stderr"])))
                    prev_code = code
                rounds_log.write(json.dumps({
                    "round": rnd, "pass": passed, "rc": rc, "note": note,
                    "static_codes": scodes, "runtime_codes": rcodes,
                }, ensure_ascii=False) + "\n")
                rounds_log.flush()
                if passed:
                    converged = True
                    if rnd == 1:
                        totals["round1_pass"] += 1
                    break
            rounds_log.write(json.dumps({
                "event": "final", "max_rounds": R, "rounds_run": rounds_run,
                "converged": converged,
                "rounds_to_pass": rounds_run if converged else None,
            }, ensure_ascii=False) + "\n")
            rounds_log.close()
            totals["trajectories"] += 1
            totals["converged" if converged else "censored"] += 1
            print(f"[s{k}/{tid}] {'收敛于 round %d' % rounds_run if converged else '删失（%d 轮未过）' % R}")

    meta = {
        "protocol": "single-step feedback chain (designs/0027 §1.2, adjudication 0-2=B)",
        "provider": args.provider, "model": args.model, "dry_run": args.dry_run,
        "samples": args.samples, "max_rounds_override": args.max_rounds,
        "temperature": args.temperature if args.temperature is not None else "provider-default",
        "thinking": args.thinking, "max_tokens": args.max_tokens,
        "lom_bin": args.lom_bin, "tasks_file": os.path.relpath(TASKS_FILE, EVAL_DIR),
        "task_ids": [t["id"] for t in tasks],
        "pass_criterion": "stdout verbatim (CRLF-normalized) + exit code 0 (same as runner)",
        "started_utc": t0.isoformat(),
        "finished_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "totals": totals,
    }
    with open(os.path.join(out_dir, "run_meta.json"), "w", encoding="utf-8") as f:
        json.dump(meta, f, ensure_ascii=False, indent=2)
    print(f"\n轨迹：{totals['converged']}/{totals['trajectories']} 收敛，"
          f"{totals['censored']} 删失；目录 {out_dir}")
    print("下一步：python eval/iter_summarize.py --dir " + out_dir)


if __name__ == "__main__":
    main()
