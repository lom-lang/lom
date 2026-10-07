# -*- coding: utf-8 -*-
"""仓库内相对链接断链机检（常驻 CI doc-gates 步骤）。

沿革：2026-10-06 防腐巡检批的一次性脚本（target/link_check/check_links.py，
gitignored）两轮实战（防腐批 + 二十七审）后升格入库（2026-10-07 用户裁决，
TODO 菜单⑥），接线 .github/workflows/ci.yml doc-gates job。

扫描面：git 跟踪的全部 .md + docs/ 下全部 .html（含 archive/、reviews/、顶层）。
提取 markdown 链接与 html href/src 中的仓库内相对路径（外链/锚点/协议前缀
跳过；围栏代码块与行内代码 span 剥除，避免把代码示例里的伪链接当真链接），
检查目标存在性。

分类与退出码（CI gate 约定）：
  A 类 = 现行文档断链（来源文件不在 archive/ 与 reviews/ 下）→ **拦截**（exit 1）
  B 类 = 历史原文断链（来源为 docs/archive/*.md 或 docs/reviews/*.html，
        审查纪律"报告原文一经落盘不回写"）→ 仅报告，不拦截
  大小写漂移 = Windows 磁盘存在但 git 索引内大小写不符（在 Linux CI 上
        实际就是断链）→ **拦截**（exit 1）
用法：python tools/link_check.py [--raw]
  --raw = 额外把机检原始结果落盘 target/link_check/broken_raw.txt（本地
  巡检用；CI 不传，不依赖 target/ 目录存在）。
"""
import os
import re
import subprocess
import sys
from urllib.parse import unquote

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RAW_OUT = os.path.join(REPO, "target", "link_check", "broken_raw.txt")

# ---------- 收集 git 跟踪文件 ----------
out = subprocess.run(
    ["git", "-C", REPO, "ls-files"], capture_output=True, text=True,
    encoding="utf-8", check=True,
).stdout.splitlines()

tracked_files = set(out)
md_files = sorted(f for f in tracked_files if f.endswith(".md"))
# docs/ 下全部 html（递归：含 archive/*.html、reviews/*.html、顶层三个 html）
html_files = sorted(
    f for f in tracked_files if f.startswith("docs/") and f.endswith(".html")
)
scan_files = md_files + html_files

# git 索引内路径集合（用于大小写精确比对）
tracked_norm = tracked_files          # 原样
tracked_lower = {f.lower() for f in tracked_files}

SKIP_PREFIX = ("http://", "https://", "mailto:", "tel:", "data:", "javascript:", "ftp://")

MD_LINK = re.compile(r"\[([^\[\]]*)\]\(\s*<?([^)<>\s]+)>?(?:\s+[\"'][^\"']*[\"'])?\s*\)")
HTML_ATTR = re.compile(r"""(?:href|src)\s*=\s*["']([^"']+)["']""", re.IGNORECASE)


def strip_non_link_text(text: str) -> str:
    """去掉围栏代码块与行内代码 span，避免把代码示例里的伪链接当真链接。"""
    lines = text.split("\n")
    keep = []
    in_fence = False
    fence_marker = ""
    for ln in lines:
        stripped = ln.lstrip()
        if not in_fence and (stripped.startswith("```") or stripped.startswith("~~~")):
            in_fence = True
            fence_marker = stripped[:3]
            keep.append("")
            continue
        if in_fence:
            if stripped.startswith(fence_marker):
                in_fence = False
            keep.append("")
            continue
        keep.append(ln)
    text = "\n".join(keep)
    # 行内代码 span（非贪婪，逐段剥除）
    text = re.sub(r"`[^`\n]*`", lambda m: " " * len(m.group(0)), text)
    return text


def classify_target(link: str):
    """返回 (kind, path) ：kind in {'skip','repo'}"""
    link = link.strip()
    if not link:
        return ("skip", None)
    low = link.lower()
    if low.startswith(SKIP_PREFIX):
        return ("skip", None)
    if link.startswith("#"):
        return ("skip", None)
    # 去掉锚点与查询
    path = link.split("#", 1)[0].split("?", 1)[0]
    if not path:
        return ("skip", None)  # 纯锚点
    path = unquote(path)
    return ("repo", path)


def resolve(base_file: str, path: str) -> str:
    """相对路径基于所在文件目录解析；以 / 开头按仓库根。返回仓库相对 posix 路径。"""
    if path.startswith("/"):
        cand = path[1:]
    else:
        base_dir = os.path.dirname(base_file)
        cand = os.path.normpath(os.path.join(base_dir, path))
    return cand.replace(os.sep, "/")


def exists_check(rel: str):
    """返回 (exists_bool, case_flag)。case_flag=2 表示磁盘存在但大小写与 git 索引
    不一致（Windows 大小写不敏感能感知到磁盘存在；Linux CI 上该形态直接是断链，
    走 exists=False 路径，两侧拦截口径一致）。"""
    full = os.path.join(REPO, rel)
    if os.path.exists(full):
        # Windows 大小写不敏感；再对 git 索引做精确比对（仅对具体文件，目录不比）
        case_flag = 0
        rel_norm = rel.replace("\\", "/")
        if os.path.isfile(full):
            if rel_norm not in tracked_norm and rel_norm.lower() in tracked_lower:
                case_flag = 2
        return (True, case_flag)
    return (False, 0)


broken = []   # (file, line, raw, resolved, target_in_archive, source_hist)
case_issues = []  # 大小写漂移（磁盘存在但索引内大小写不同）
total_links = 0

for f in scan_files:
    try:
        with open(os.path.join(REPO, f), "r", encoding="utf-8", errors="replace") as fh:
            text = fh.read()
    except OSError as e:
        print(f"!! 读取失败 {f}: {e}")
        continue

    is_html = f.endswith(".html")
    scan_text = text if is_html else strip_non_link_text(text)

    # 逐行定位：把匹配落到行号
    for m in (HTML_ATTR.finditer(scan_text) if is_html else MD_LINK.finditer(scan_text)):
        raw = m.group(2) if not is_html else m.group(1)  # md: group2=路径；html: group1=属性值
        kind, path = classify_target(raw)
        if kind != "repo":
            continue
        total_links += 1
        rel = resolve(f, path)
        ok, case_flag = exists_check(rel)
        if ok and case_flag == 2:
            line = scan_text.count("\n", 0, m.start()) + 1
            case_issues.append((f, line, raw, rel))
            continue
        if not ok:
            line = scan_text.count("\n", 0, m.start()) + 1
            target_in_archive = rel.startswith("docs/archive/")
            source_hist = f.startswith("docs/archive/") or f.startswith("docs/reviews/")
            broken.append((f, line, raw, rel, target_in_archive, source_hist))

# ---------- 输出 ----------
print(f"扫描文件数（md+docs html） = {len(scan_files)}  （md={len(md_files)}, docs html={len(html_files)}）")
print(f"仓库内相对链接总数 = {total_links}")
print(f"断链总数 = {len(broken)}")
print()


def cat_A(e):  # 现行文档类：来源不是历史原文文件
    return not e[5]


def cat_B(e):  # 历史原文类：来源为 archive/ 或 reviews/
    return e[5]


A = [e for e in broken if cat_A(e)]
B = [e for e in broken if cat_B(e)]
print(f"A 断链-现行文档类 = {len(A)}  （拦截项）")
for f, line, raw, rel, tia, sh in A:
    print(f"  {f}:{line} | {raw} | {rel}")
print()
print(f"B 断链-历史原文类 = {len(B)}  （不拦截；其中断链目标在 archive 内 = {sum(1 for e in B if e[4])}）")
for f, line, raw, rel, tia, sh in B:
    tag = "target∈archive" if tia else "target∉archive"
    print(f"  {f}:{line} | {raw} | {rel} | {tag}")
print()
print(f"大小写漂移（磁盘存在但 git 索引大小写不符） = {len(case_issues)}  （拦截项）")
for f, line, raw, rel in case_issues:
    print(f"  {f}:{line} | {raw} | {rel}")

# ---------- 退出码（CI gate 约定）----------
if A or case_issues:
    print()
    print(f"RESULT: FAIL（A 类断链 {len(A)} + 大小写漂移 {len(case_issues)}；B 类历史原文 {len(B)} 不拦）")
    if "--raw" in sys.argv:
        os.makedirs(os.path.dirname(RAW_OUT), exist_ok=True)
        with open(RAW_OUT, "w", encoding="utf-8") as fh:
            fh.write(f"total_links={total_links}\nbroken={len(broken)}\nA={len(A)}\nB={len(B)}\ncase={len(case_issues)}\n")
            for f, line, raw, rel, tia, sh in broken:
                cat = "B" if sh else "A"
                fh.write(f"[{cat}] {f}:{line} | {raw} | {rel} | target_in_archive={tia}\n")
            for f, line, raw, rel in case_issues:
                fh.write(f"[CASE] {f}:{line} | {raw} | {rel}\n")
        print(f"OK -> {os.path.relpath(RAW_OUT, REPO)}")
    sys.exit(1)

print()
print(f"RESULT: PASS（A 类断链 0 + 大小写漂移 0；B 类历史原文 {len(B)} 不拦）")
sys.exit(0)
