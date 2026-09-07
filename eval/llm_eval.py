#!/usr/bin/env python3
"""Lom LLM 复测管线（第四轮评审整改 P2 落地，2026-08-31）

按项目自订标准（guide §3.3：至少 3 个 LLM）补齐"AI 原生"宣称的证据链。
方法论对齐 2026-08-03 基线：把 eval/prompts/<分类>.md 整文件作为一条 user 消息发给模型
（相当于网页版粘贴），解析回复里的 === <id>.lom === 分隔块写到候选目录，
然后用 eval/runner/run.ps1 -CandidatesDir 评分。

密钥来源（脚本读取，不会出现在任何输出里）：
  1. 环境变量 DEEPSEEK_API_KEY / GLM_API_KEY，或
  2. eval/.api_keys.json（已 gitignore）：{"deepseek": "sk-...", "glm": "..."}

用法：
  python eval/llm_eval.py --provider deepseek --model deepseek-v4-pro --thinking
  python eval/llm_eval.py --provider glm --model glm-5.3
  python eval/llm_eval.py --provider deepseek --model deepseek-v4-pro --thinking \
      --samples 10 --temperature 1.0        # 多采样（pass@k 采集，L 工作包）

多采样布局（samples > 1，与单采样向后兼容）：
  eval/candidates_rerun/<provider>_<model>/s<k>/<id>.lom  — 第 k 个采样的候选代码
  eval/candidates_rerun/<provider>_<model>/raw/<cat>_s<k>.md — 第 k 个采样的原始回复
断点续跑：raw 文件已存在的采样跳过 API 调用（零成本恢复），提取步骤仍执行。

输出（samples = 1 时保持旧布局）：
  eval/candidates_rerun/<provider>_<model>/<id>.lom      — 提取的候选代码
  eval/candidates_rerun/<provider>_<model>/raw/*.md      — 原始回复（审计用）
  eval/candidates_rerun/<provider>_<model>/run_meta.json — 模型/参数/时间戳/提取统计

然后评分（示例）：
  powershell -ExecutionPolicy Bypass -File eval/runner/run.ps1 `
    -CandidatesDir eval/candidates_rerun/deepseek_deepseek-chat -LomBin ./target/release/lom.exe
"""

import argparse
import datetime
import http.client
import json
import os
import re
import sys
import time
import urllib.request
import urllib.error

PROVIDERS = {
    # https://api-docs.deepseek.com/zh-cn/ —— base_url https://api.deepseek.com（无 /v1）
    "deepseek": "https://api.deepseek.com/chat/completions",
    # https://docs.bigmodel.cn/cn/guide/develop/http/introduction —— Bearer API key 直接可用
    "glm": "https://open.bigmodel.cn/api/paas/v4/chat/completions",
    # https://docs.bigmodel.cn/cn/coding-plan/quick-start —— Coding Plan 订阅 key 的专属端点
    "glm-coding": "https://open.bigmodel.cn/api/coding/paas/v4/chat/completions",
}

# Coding Plan key 与平台按量 key 不通用；glm-coding 优先读 GLM_CODING_API_KEY / keys["glm-coding"]，
# 未单独配置时回落到 glm 的 key（个人版套餐 key 两处通用的情况）
KEY_FALLBACK = {"glm-coding": "glm"}

PROMPTS_DIR = os.path.join(os.path.dirname(__file__), "prompts")
TASKS_DIR = os.path.join(os.path.dirname(__file__), "tasks")

# 提取 === NNN.lom === 分隔块（与 _footer.md 的约定格式一致）
BLOCK_RE = re.compile(r"===\s*(\d{3})\.lom\s*===\s*\n(.*?)(?=\n===\s*\d{3}\.lom\s*===|\Z)", re.S)
FENCE_RE = re.compile(r"^```[a-zA-Z]*\n|\n```\s*$", re.M)


def load_key(provider: str) -> str:
    env_name = f"{provider.upper().replace('-', '_')}_API_KEY"
    if os.environ.get(env_name):
        return os.environ[env_name]
    keys_file = os.path.join(os.path.dirname(__file__), ".api_keys.json")
    if os.path.exists(keys_file):
        with open(keys_file, encoding="utf-8") as f:
            keys = json.load(f)
        if keys.get(provider):
            return keys[provider]
        # 回落（如 glm-coding → glm）
        fallback = KEY_FALLBACK.get(provider)
        if fallback and keys.get(fallback):
            return keys[fallback]
    sys.exit(f"找不到 {provider} 的 API key：设环境变量 {env_name} 或写 eval/.api_keys.json")


def expected_ids() -> dict:
    """分类文件 basename -> 该分类任务 id 列表"""
    out = {}
    for fn in sorted(os.listdir(TASKS_DIR)):
        if not fn.endswith(".json"):
            continue
        with open(os.path.join(TASKS_DIR, fn), encoding="utf-8") as f:
            tasks = json.load(f)
        out[fn[:-5]] = [t["id"] for t in tasks]
    return out


def sample_names(cat: str, samples: int) -> list:
    """某分类的 (raw 文件名, 候选子目录名) 序列。

    samples=1 保持旧布局（raw/<cat>.md、候选直接在 out_dir）——向后兼容上轮
    四模型存量目录；samples>1 用 raw/<cat>_s<k>.md 与 s<k>/ 子目录。
    """
    if samples == 1:
        return [(f"{cat}.md", ".")]
    return [(f"{cat}_s{k}.md", f"s{k}") for k in range(1, samples + 1)]


def call_api(base_url: str, key: str, model: str, prompt: str,
             temperature: float | None, max_tokens: int, thinking: bool,
             retries: int = 2) -> str:
    body = {
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": max_tokens,
        "stream": False,
    }
    if temperature is not None:
        body["temperature"] = temperature
    if thinking:
        # DeepSeek 官方参数（api-docs.deepseek.com）；GLM 4.5+ 同形
        body["thinking"] = {"type": "enabled"}
    req = urllib.request.Request(
        base_url,
        data=json.dumps(body).encode("utf-8"),
        headers={"Content-Type": "application/json", "Authorization": f"Bearer {key}"},
        method="POST",
    )
    for attempt in range(retries + 1):
        try:
            with urllib.request.urlopen(req, timeout=600) as resp:
                data = json.loads(resp.read().decode("utf-8"))
            content = data["choices"][0]["message"]["content"]
            finish = data["choices"][0].get("finish_reason", "?")
            if not content or not content.strip():
                # 空回复实测发生（deepseek 05_match_enum 返回空 content）——按可重试处理
                if attempt < retries:
                    print(f"  空回复（finish_reason={finish}），重试...", file=sys.stderr)
                    time.sleep(10)
                    continue
                sys.exit(f"API 反复返回空回复（finish_reason={finish}）")
            return content
        except urllib.error.HTTPError as e:
            detail = e.read().decode("utf-8", errors="replace")[:500]
            if attempt < retries and e.code in (429, 500, 502, 503, 504):
                wait = 10 * (attempt + 1)
                print(f"  HTTP {e.code}，{wait}s 后重试...", file=sys.stderr)
                time.sleep(wait)
            else:
                sys.exit(f"API 错误 HTTP {e.code}: {detail}")
        except (urllib.error.URLError, TimeoutError,
                http.client.HTTPException, ConnectionError) as e:
            # HTTPException/ConnectionError 覆盖 RemoteDisconnected 等服务端断连
            # （2026-09-07 L1 实测踩到：RemoteDisconnected 不是 URLError 子类，
            # 原始 catch 列表漏掉它会导致 75/100 时点整个进程崩溃退出）
            if attempt < retries:
                print(f"  网络/连接错误 {type(e).__name__}，10s 后重试...", file=sys.stderr)
                time.sleep(10)
            else:
                sys.exit(f"网络错误: {e}")


def extract_blocks(text: str) -> dict:
    """从回复中提取 === NNN.lom === 块；容忍块内包裹 ```lom 围栏和尾部孤立的 === 行"""
    out = {}
    for m in BLOCK_RE.finditer(text):
        code = m.group(2).strip("\n")
        code = FENCE_RE.sub("", code).strip("\n")
        # 模型偶尔在块尾多写一个孤立的 ===（把分隔符当成成对标记）
        lines = code.split("\n")
        while lines and lines[-1].strip() == "===":
            lines.pop()
        out[m.group(1)] = "\n".join(lines).strip("\n") + "\n"
    return out


def main():
    ap = argparse.ArgumentParser(description="Lom LLM 复测管线")
    ap.add_argument("--provider", required=True, choices=sorted(PROVIDERS))
    ap.add_argument("--model", required=True, help="模型名（如 deepseek-chat / glm-4.6）")
    ap.add_argument("--temperature", type=float, default=None,
                    help="不填则用服务商默认（会记录在 run_meta.json）")
    ap.add_argument("--max-tokens", type=int, default=16384)
    ap.add_argument("--thinking", action="store_true",
                    help="开启思考模式（DeepSeek/GLM 的 thinking.type=enabled）")
    ap.add_argument("--only", default=None, help="只跑某个分类（如 05_match_enum），调试用")
    ap.add_argument("--samples", type=int, default=1,
                    help="每分类采样次数（默认 1；>1 时走 s<k>/ 多采样布局，L 工作包 pass@k）")
    ap.add_argument("--from-raw", action="store_true",
                    help="不调 API，从已有 raw/ 回复重新提取（提取器修复后复用，零成本）")
    ap.add_argument("--out-root", default=os.path.join(os.path.dirname(__file__), "candidates_rerun"))
    ap.add_argument("--out-name", default=None,
                    help="覆盖输出目录名（默认 <provider>_<model>；断点续跑时对齐已有目录）")
    args = ap.parse_args()

    out_dir = os.path.join(args.out_root, args.out_name or f"{args.provider}_{args.model}")
    raw_dir = os.path.join(out_dir, "raw")
    os.makedirs(raw_dir, exist_ok=True)

    want = expected_ids()

    if args.from_raw:
        # 离线重提取：raw/*.md → 候选文件（兼容单/多采样两种 raw 命名）
        total_e = total_x = 0
        for fn in sorted(os.listdir(raw_dir)):
            if not fn.endswith(".md"):
                continue
            m = re.match(r"^(.*)_s(\d+)$", fn[:-3])
            cat, sub = (m.group(1), f"s{m.group(2)}") if m else (fn[:-3], ".")
            cand_dir = out_dir if sub == "." else os.path.join(out_dir, sub)
            os.makedirs(cand_dir, exist_ok=True)
            with open(os.path.join(raw_dir, fn), encoding="utf-8") as f:
                reply = f.read()
            blocks = extract_blocks(reply)
            for tid, code in blocks.items():
                with open(os.path.join(cand_dir, f"{tid}.lom"), "w", encoding="utf-8") as f:
                    f.write(code)
            missing = [i for i in want.get(cat, []) if i not in blocks]
            total_e += len(want.get(cat, []))
            total_x += len(blocks)
            msg = f"[{fn[:-3]}] 重提取 {len(blocks)}/{len(want.get(cat, []))}"
            if missing:
                msg += f" 缺 {missing}"
            print(msg)
        print(f"\n重提取完成：{total_x}/{total_e}")
        return

    if args.samples < 1:
        sys.exit("--samples 必须 >= 1")

    key = load_key(args.provider)

    stats = {}
    skipped_calls = 0
    t0 = datetime.datetime.now(datetime.timezone.utc)

    for fn in sorted(os.listdir(PROMPTS_DIR)):
        if not fn.endswith(".md") or fn.startswith("_") or fn == "README.md":
            continue
        cat = fn[:-3]
        if args.only and cat != args.only:
            continue
        with open(os.path.join(PROMPTS_DIR, fn), encoding="utf-8") as f:
            prompt = f.read()
        for raw_name, sub in sample_names(cat, args.samples):
            raw_path = os.path.join(raw_dir, raw_name)
            cand_dir = out_dir if sub == "." else os.path.join(out_dir, sub)
            os.makedirs(cand_dir, exist_ok=True)
            if os.path.exists(raw_path):
                # 断点续跑：raw 已留档的采样跳过调用，提取仍执行（提取器可能被修过）
                with open(raw_path, encoding="utf-8") as f:
                    reply = f.read()
                skipped_calls += 1
                print(f"[{raw_name[:-3]}] raw 已存在，跳过 API 调用（断点续跑）")
            else:
                print(f"[{raw_name[:-3]}] 调用 {args.model}（{len(prompt)} 字符）...")
                reply = call_api(PROVIDERS[args.provider], key, args.model, prompt,
                                 args.temperature, args.max_tokens, args.thinking)
                with open(raw_path, "w", encoding="utf-8") as f:
                    f.write(reply)
                time.sleep(2)  # 礼貌限速
            blocks = extract_blocks(reply)
            for tid, code in blocks.items():
                with open(os.path.join(cand_dir, f"{tid}.lom"), "w", encoding="utf-8") as f:
                    f.write(code)
            missing = [i for i in want.get(cat, []) if i not in blocks]
            stats[raw_name[:-3]] = {"extracted": len(blocks),
                                    "expected": len(want.get(cat, [])),
                                    "missing": missing}
            flag = "OK" if not missing else f"缺 {missing}"
            print(f"[{raw_name[:-3]}] 提取 {len(blocks)}/{len(want.get(cat, []))} {flag}")

    meta = {
        "provider": args.provider,
        "model": args.model,
        "temperature": args.temperature if args.temperature is not None else "provider-default",
        "thinking": args.thinking,
        "max_tokens": args.max_tokens,
        "samples": args.samples,
        "started_utc": t0.isoformat(),
        "finished_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "prompt_method": "整分类 prompt 文件作为单条 user 消息（对齐 2026-08-03 基线方法论）",
        "stats": stats,
    }
    with open(os.path.join(out_dir, "run_meta.json"), "w", encoding="utf-8") as f:
        json.dump(meta, f, ensure_ascii=False, indent=2)

    total_e = sum(s["expected"] for s in stats.values())
    total_x = sum(s["extracted"] for s in stats.values())
    print(f"\n完成：提取 {total_x}/{total_e}，跳过已有调用 {skipped_calls} 次，候选目录 {out_dir}")
    print("下一步：用 run.ps1 -CandidatesDir 评分（见文件头注释）；"
          "多采样目录用 eval/passk_summarize.py 汇总 pass@k")


if __name__ == "__main__":
    main()
