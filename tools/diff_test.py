#!/usr/bin/env python3
# tools/diff_test.py — 双后端差分测试对拍器（D 工作包 D2，2026-09-14）
#
# 用法：
#   python tools/diff_test.py --rounds 500 [--seed-base 1000]   # 正常模式：随机程序对拍
#   python tools/diff_test.py --probe                            # 探针模式：验证 §11f 白名单仍如档案所述
#   python tools/diff_test.py --rounds 20 --ci                   # CI 冒烟（固定小轮次）
#
# 正常模式纪律：diff_gen 默认避开 SPEC_FOR_AI §11f 七条已知分歧形态，
# 因此**任何 stdout/退出码差异都是新发现**（差异即 FAIL，留档 .diff_failures/
# 供人工调查——bug 则修复 + 回归测试，未记录分歧则补 §11f 白名单）。
# 探针模式纪律：显式生成已知分歧形态，验证差异**确实出现且形态符合档案**——
# 探针无差异（如 mut-capture 输出相同）意味着白名单腐坏（分歧消失未更新文档），同样 FAIL。
#
# 双后端执行口径（对齐 eval/runner/run.ps1 -Backend wasm）：
#   解释器：lom <file>
#   WASM  ：lom build <file> --target wasm -o <file>.wasm && node eval/runner/run_wasm.mjs <file>.wasm
# 比对：stdout 逐字（UTF-8）+ 退出码。零第三方依赖；临时文件在项目内 .diff_tmp（Git Bash
# /tmp 与 Windows 进程不通的既有坑），用完即删。
import argparse
import os
import shutil
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
LOM = os.path.join(ROOT, "target", "release", "lom.exe")
if not os.path.exists(LOM):
    LOM = os.path.join(ROOT, "target", "release", "lom")
HARNESS = os.path.join(ROOT, "eval", "runner", "run_wasm.mjs")
TMP = os.path.join(ROOT, ".diff_tmp")
FAILDIR = os.path.join(ROOT, ".diff_failures")

sys.path.insert(0, HERE)
import diff_gen  # noqa: E402


def run_interp(path: str, timeout=25):
    p = subprocess.run([LOM, path], capture_output=True, timeout=timeout)
    return p.stdout.decode("utf-8", errors="replace"), p.returncode


def run_wasm(path: str, timeout=25):
    wasm_path = path + ".wasm"
    b = subprocess.run([LOM, "build", path, "--target", "wasm", "-o", wasm_path],
                       capture_output=True, timeout=timeout)
    if b.returncode != 0:
        return ("<wasm build 失败>\n" + b.stderr.decode("utf-8", errors="replace")), b.returncode
    p = subprocess.run(["node", HARNESS, wasm_path], capture_output=True, timeout=timeout)
    return p.stdout.decode("utf-8", errors="replace"), p.returncode


def one_normal(seed: int) -> tuple[bool, str]:
    """生成 + 对拍一个正常程序。返回 (pass?, 摘要)。

    前置闸门：--check 非 0（Error 级）= 生成器产出非法程序，与后端差异
    分开报告（生成器 bug 不污染后端对拍信号——D3 首轮 43 个误报的教训）。
    """
    text, _ = diff_gen.gen_program(seed)
    path = os.path.join(TMP, "r%d.lom" % seed)
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(text)
    c = subprocess.run([LOM, path, "--check"], capture_output=True, timeout=25)
    if c.returncode != 0:
        # --check 诊断走 stdout（run 路径才走 stderr）——两流都看
        out = c.stdout.decode("utf-8", errors="replace") + c.stderr.decode("utf-8", errors="replace")
        first = [l for l in out.splitlines() if l.strip()][:1]
        os.makedirs(FAILDIR, exist_ok=True)
        shutil.copyfile(path, os.path.join(FAILDIR, "seed%d.lom" % seed))
        os.remove(path)
        return False, "GENERATOR-BUG（--check rc=%d）: %s" % (c.returncode, first)
    i_out, i_rc = run_interp(path)
    w_out, w_rc = run_wasm(path)
    if i_out == w_out and i_rc == w_rc:
        os.remove(path)
        if os.path.exists(path + ".wasm"):
            os.remove(path + ".wasm")
        return True, ""
    # 差异即新发现（正常模式默认避开全部已知分歧）——留档
    os.makedirs(FAILDIR, exist_ok=True)
    base = os.path.join(FAILDIR, "seed%d" % seed)
    shutil.copyfile(path, base + ".lom")
    with open(base + ".interp.out", "w", encoding="utf-8", newline="\n") as f:
        f.write("rc=%d\n---\n%s" % (i_rc, i_out))
    with open(base + ".wasm.out", "w", encoding="utf-8", newline="\n") as f:
        f.write("rc=%d\n---\n%s" % (w_rc, w_out))
    os.remove(path)
    if os.path.exists(path + ".wasm"):
        os.remove(path + ".wasm")
    # 摘要：首个差异行
    il, wl = i_out.splitlines(), w_out.splitlines()
    detail = ""
    for k in range(max(len(il), len(wl))):
        a = il[k] if k < len(il) else "<missing>"
        b = wl[k] if k < len(wl) else "<missing>"
        if a != b:
            detail = "line %d: interp=%r wasm=%r" % (k + 1, a[:60], b[:60])
            break
    return False, "rc %d vs %d; %s" % (i_rc, w_rc, detail)


def _pair_for_probe(kind: str, seed: int):
    text = diff_gen.gen_probe(kind, seed)
    path = os.path.join(TMP, "probe_%s.lom" % kind)
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(text)
    return path


def run_probes() -> int:
    """探针模式：验证 §11f 七条白名单中的六条可执行分歧仍如档案所述。

    （分歧 4 trim Unicode 需非 ASCII 输入，由文档 + eval 既有用例覆盖，不在探针集；
    分歧 2 json-number 在集内。）
    """
    failures = 0

    def report(kind, ok, msg):
        nonlocal failures
        print("  probe %-14s %s %s" % (kind, "OK  " if ok else "FAIL", msg))
        if not ok:
            failures += 1

    # 1. mut-capture：预期 stdout 不同（解释器共享作用域=新值 / WASM 值拷贝=旧值）
    p = _pair_for_probe("mut-capture", 42)
    i_out, i_rc = run_interp(p)
    w_out, w_rc = run_wasm(p)
    if i_out != w_out and i_rc == 0 and w_rc == 0:
        report("mut-capture", True, "分歧存在（§11f-1）：interp=%r wasm=%r" % (i_out.strip(), w_out.strip()))
    elif i_out == w_out:
        report("mut-capture", False, "分歧消失——白名单可能腐坏（需复核 §11f-1）")
    else:
        report("mut-capture", False, "意外退出码：interp rc=%d wasm rc=%d" % (i_rc, w_rc))

    # 2. div-zero：预期双侧 rc=1（stdout 前缀一致）
    p = _pair_for_probe("div-zero", 42)
    i_out, i_rc = run_interp(p)
    w_out, w_rc = run_wasm(p)
    if i_rc == 1 and w_rc == 1 and i_out == w_out:
        report("div-zero", True, "双侧 rc=1 且 stdout 一致（诊断文本差异在 stderr，§11f-3）")
    else:
        report("div-zero", False, "预期双侧 rc=1 stdout 一致，实际 interp rc=%d wasm rc=%d out_eq=%s"
               % (i_rc, w_rc, i_out == w_out))

    # 3. large-float：预期 stdout 不同但数值等价
    p = _pair_for_probe("large-float", 42)
    i_out, i_rc = run_interp(p)
    w_out, w_rc = run_wasm(p)
    def _pf(x):
        # harness 的 fmtFloat 会给无点形态补 ".0"——科学计数法形态是 "2e+30.0"，
        # 剥掉 e 记法后的 ".0" 尾才能被 Python float() 解析
        x = x.strip()
        if x.endswith(".0") and ("e" in x or "E" in x):
            x = x[:-2]
        return float(x)
    try:
        num_eq = abs(_pf(i_out) - _pf(w_out)) <= abs(_pf(i_out)) * 1e-12
    except ValueError:
        num_eq = False
    if i_out != w_out and num_eq and i_rc == 0 and w_rc == 0:
        report("large-float", True, "格式分歧数值等价（§11f-5）：%r vs %r" % (i_out.strip(), w_out.strip()))
    elif i_out == w_out:
        report("large-float", False, "分歧消失——白名单可能腐坏（需复核 §11f-5）")
    else:
        report("large-float", False, "意外形态：interp=%r wasm=%r rc=%d/%d" % (i_out.strip()[:40], w_out.strip()[:40], i_rc, w_rc))

    # 4. json-number：预期 stdout 不同（Int/Float 切分按 JS 值 vs 源语法，§11f-2）
    p = _pair_for_probe("json-number", 42)
    i_out, i_rc = run_interp(p)
    w_out, w_rc = run_wasm(p)
    if i_out != w_out and i_rc == 0 and w_rc == 0 and "30.0" in i_out and "30.0" not in w_out:
        report("json-number", True, "分歧存在（§11f-2）：interp=%r wasm=%r" % (i_out.strip(), w_out.strip()))
    elif i_out == w_out:
        report("json-number", False, "分歧消失——白名单可能腐坏（需复核 §11f-2）")
    else:
        report("json-number", False, "意外形态：interp=%r wasm=%r rc=%d/%d" % (i_out.strip()[:40], w_out.strip()[:40], i_rc, w_rc))

    # 5. int-range：预期 stdout 不同（WASM ±2^59 外静默截断/翻符号，§11f-7）
    p = _pair_for_probe("int-range", 42)
    i_out, i_rc = run_interp(p)
    w_out, w_rc = run_wasm(p)
    il = [l for l in i_out.splitlines() if l.strip()]
    wl = [l for l in w_out.splitlines() if l.strip()]
    mod_ok = False
    if len(il) == len(wl) == 3 and i_rc == 0 and w_rc == 0 and i_out != w_out:
        try:
            a, b = int(il[1]), int(wl[1])
            # wasm 值应等于 interp 值对 2^64 的带符号折叠（60 位载荷截断的下游形态）
            mod_ok = ((a - b) % (1 << 64) == 0) or ((b - a) % (1 << 64) == 0) or (a % (1 << 60) == (b % (1 << 60)))
        except ValueError:
            pass
    if mod_ok:
        report("int-range", True, "分歧存在且为 2^60 截断形态（§11f-7）：2^60 行 interp=%r wasm=%r" % (il[1], wl[1]))
    elif i_out == w_out:
        report("int-range", False, "分歧消失——白名单可能腐坏（需复核 §11f-7）")
    else:
        report("int-range", False, "意外形态：interp=%r wasm=%r rc=%d/%d" % (il[:1], wl[:1], i_rc, w_rc))

    # 6. deep-recursion：预期双侧失败（rc!=0），stdout 前缀一致
    p = _pair_for_probe("deep-recursion", 42)
    i_out, i_rc = run_interp(p, timeout=60)
    w_out, w_rc = run_wasm(p, timeout=60)
    if i_rc != 0 and w_rc != 0 and i_out == w_out:
        report("deep-recursion", True, "双侧失败 stdout 一致（§11f-6：RUNTIME000 vs V8 trap 在 stderr）")
    else:
        report("deep-recursion", False, "预期双侧 rc!=0 stdout 一致，实际 interp rc=%d wasm rc=%d out_eq=%s"
               % (i_rc, w_rc, i_out == w_out))

    return failures


def one_pkg(seed: int) -> tuple[bool, str]:
    """包模式（D5）：生成多文件包项目 + 双后端对拍。返回 (pass?, 摘要)。

    包发现规则修复（v1.1.4）：解释器与 build 均按 main.lom 所在目录发现
    lom.toml——对拍从任意 cwd 都有效（修复前 build 按 cwd 发现会假失败）。
    """
    files = diff_gen.gen_pkg_project(seed)
    root = os.path.join(TMP, "pkg%d" % seed)
    for rel, content in files.items():
        p = os.path.join(root, rel)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, "w", encoding="utf-8", newline="\n") as f:
            f.write(content)
    main = os.path.join(root, "main.lom")
    # 注意：--check 的诊断走 stdout（run 路径才走 stderr）——rc 非 0 时两流都看
    c = subprocess.run([LOM, main, "--check"], capture_output=True, timeout=25)
    if c.returncode != 0:
        out = c.stdout.decode("utf-8", errors="replace") + c.stderr.decode("utf-8", errors="replace")
        first = [l for l in out.splitlines() if l.strip()][:1]
        return False, "GENERATOR-BUG（--check rc=%d）: %s" % (c.returncode, first)
    i_out, i_rc = run_interp(main)
    w_out, w_rc = run_wasm(main)
    ok = i_out == w_out and i_rc == w_rc
    if not ok:
        os.makedirs(FAILDIR, exist_ok=True)
        base = os.path.join(FAILDIR, "pkgseed%d" % seed)
        os.makedirs(base, exist_ok=True)
        for rel, content in files.items():
            out = os.path.join(base, *rel.split("/"))
            os.makedirs(os.path.dirname(out), exist_ok=True)
            with open(out, "w", encoding="utf-8", newline="\n") as f:
                f.write(content)
        with open(base + ".interp.out", "w", encoding="utf-8", newline="\n") as f:
            f.write("rc=%d\n---\n%s" % (i_rc, i_out))
        with open(base + ".wasm.out", "w", encoding="utf-8", newline="\n") as f:
            f.write("rc=%d\n---\n%s" % (w_rc, w_out))
        detail = "rc %d vs %d" % (i_rc, w_rc)
        il, wl = i_out.splitlines(), w_out.splitlines()
        for k in range(max(len(il), len(wl))):
            a = il[k] if k < len(il) else "<missing>"
            b = wl[k] if k < len(wl) else "<missing>"
            if a != b:
                detail += "; line %d: interp=%r wasm=%r" % (k + 1, a[:60], b[:60])
                break
        return False, detail
    return True, ""


def main():
    ap = argparse.ArgumentParser(description="双后端差分测试对拍器（D 工作包）")
    ap.add_argument("--rounds", type=int, default=100)
    ap.add_argument("--seed-base", type=int, default=1)
    ap.add_argument("--probe", action="store_true", help="探针模式：验证 §11f 白名单")
    ap.add_argument("--pkg", action="store_true", help="包模式（D5）：多文件包项目对拍")
    ap.add_argument("--ci", action="store_true", help="CI 冒烟口径（失败时退出码 1 不变，输出精简）")
    args = ap.parse_args()

    if not os.path.exists(LOM):
        print("lom 二进制不存在：%s（先 cargo build --release）" % LOM)
        sys.exit(2)
    os.makedirs(TMP, exist_ok=True)
    try:
        if args.probe:
            print("probe 模式：验证 §11f 已知分歧白名单（6 条可执行探针）")
            fails = run_probes()
            n = 6
            print("probe: %d/%d 验证通过" % (n - fails, n))
            print("RESULT: %s" % ("PASS" if fails == 0 else "FAIL"))
            sys.exit(0 if fails == 0 else 1)

        fails = 0
        one = one_pkg if args.pkg else one_normal
        label = "包项目" if args.pkg else "程序"
        for i in range(args.rounds):
            seed = args.seed_base + i
            ok, detail = one(seed)
            if not ok:
                fails += 1
                print("  seed %d: 新发现差异 — %s" % (seed, detail))
            if not args.ci and (i + 1) % 50 == 0:
                print("  ... %d/%d（fail %d）" % (i + 1, args.rounds, fails))
        print("diff[%s]: %d/%d 双后端一致（差异即新发现，正常模式默认避开 §11f 七条已知分歧）"
              % (label, args.rounds - fails, args.rounds))
        if fails:
            print("失败留档：%s/*.lom + .interp.out + .wasm.out" % FAILDIR)
        print("RESULT: %s" % ("PASS" if fails == 0 else "FAIL"))
        sys.exit(0 if fails == 0 else 1)
    finally:
        shutil.rmtree(TMP, ignore_errors=True)


if __name__ == "__main__":
    main()
