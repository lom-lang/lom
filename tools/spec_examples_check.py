# tools/spec_examples_check.py — SPEC_FOR_AI 代码示例实测对账（第二轮审查整改 R7，2026-09-05）
#
# 背景：历次文档清扫以 LANGUAGE_SPEC/README 为主战场，SPEC_FOR_AI（喂给 LLM 的唯一规范）
# 只在评审点名时被扫——第二轮审查的 P1（type alias 假特性）正是漏网结果。本工具把
# "lexer/parser 实测对账"制度化到 SPEC_FOR_AI 的每个 fenced 代码块。
#
# 用法：
#   python tools/spec_examples_check.py                     # 默认 SPEC_FOR_AI.md + 本地 lom 二进制
#   python tools/spec_examples_check.py --spec <file>       # 指定规范文件
#   python tools/spec_examples_check.py --lom <path>        # 指定 lom 二进制
#
# 分类与断言（对每个 fenced 块）：
#   - skip-json / skip-diag：块首为 `{`（JSON schema 示例）或 `[lex]`/`[parse]`/...
#     前缀（人类可读诊断输出示例）——不是 Lom 代码，跳过（计数报告）；
#   - skip-marker：围栏上方最近的非空行含 `spec-check: skip`（HTML 注释形态，
#     如概念性内建类型定义——真实代码里重定义是 NAM002）；
#   - 反例（counterexample）：围栏上方最近的非空行含 ❌ 或 "Do not write"/「不要写」
#     ——断言其确实产生 ≥1 条 Error 诊断（反知识声明一旦失效即红，倒逼文档更新）；
#   - 正例（positive）——三层断言：
#     ① 解析层：`--check --json` 无 parse 阶段 Error（抓假特性：未实现的语法必在此暴露）；
#     ② 导入层：含 `from X import {Y}` 的块实际运行，输出不得含 RUNTIME005
#       （模块导入在求值前处理，假符号必然先于此暴露——抓 sin/cos 类）；
#     ③ 完整层：`--check` 零 Error 的自含块实跑退出码 0。
#       若唯一 Error 是 NAM003（引用散文上下文定义的变量，如 score/xs/n）视为教学片段，
#       只过 ①②——片段级 NAM003 与"教了不存在的函数"不可机械区分，是本工具的已知边界。
#
# 片段包裹：无顶层项（fn/enum/from 行首）的块包进 `fn main() -> Unit ... end`（缩进 4），
# 反例同样包裹（保证失败原因是反例本身而非顶层语句限制）。
#
# 无第三方依赖（纯标准库 Python）；临时文件写在项目内 .spec_check_tmp/（用完即删，
# Git Bash 的 /tmp 对 Windows 进程不可见）。

import argparse
import json
import os
import re
import shutil
import subprocess
import sys

DEFAULT_SPEC = 'SPEC_FOR_AI.md'
TMP_DIR = '.spec_check_tmp'
TIMEOUT_RUN = 10          # 正例实跑超时（秒）——文档示例不应有长循环
TOPLEVEL_RE = re.compile(r'^(fn |enum |from )')
DIAG_PREFIX_RE = re.compile(r'^\[(lex|parse|type|runtime|eff|mat|nam|pkg)\]')


def default_lom():
    return './target/release/lom.exe' if sys.platform == 'win32' else './target/release/lom'


def extract_blocks(text):
    """返回 [(start_line_1based, [行...])]——start_line 是围栏开行（``` 那行）。"""
    lines = text.split('\n')
    blocks = []
    i = 0
    while i < len(lines):
        if lines[i].strip().startswith('```'):
            j = i + 1
            content = []
            while j < len(lines) and lines[j].strip() != '```':
                content.append(lines[j])
                j += 1
            if j < len(lines):  # 找到闭合围栏
                blocks.append((i + 1, content))
            i = j + 1
        else:
            i += 1
    return blocks


def nearest_text_above(lines, fence_line, lookback=5):
    """围栏开行上方最近的非空行（1-based fence_line）。"""
    idx = fence_line - 2  # 上一行（0-based）
    for _ in range(lookback):
        if idx < 0:
            return ''
        s = lines[idx].strip()
        if s:
            return s
        idx -= 1
    return ''


def classify(content, above):
    first = next((l for l in content if l.strip()), '')
    if first.lstrip().startswith('{'):
        return 'skip-json'
    if DIAG_PREFIX_RE.match(first.lstrip()):
        return 'skip-diag'
    if 'spec-check: skip' in above:
        return 'skip-marker'
    if '❌' in above or 'Do not write' in above or '不要写' in above:
        return 'counterexample'
    return 'positive'


def wrap_if_fragment(content):
    """无顶层项的块包进 main（返回 (code, wrapped_bool)）。"""
    if any(TOPLEVEL_RE.match(l) for l in content):
        return '\n'.join(content) + '\n', False
    body = '\n'.join(('    ' + l if l.strip() else '') for l in content)
    return 'fn main() -> Unit\n' + body + '\nend\n', True


def run_lom(lom, args, timeout=TIMEOUT_RUN):
    return subprocess.run([lom] + args, capture_output=True, text=True,
                          encoding='utf-8', errors='replace', timeout=timeout)


def check_json(lom, path):
    """返回 --check --json 的诊断列表（解析失败返回 None）。"""
    r = run_lom(lom, [path, '--check', '--json'])
    try:
        return json.loads(r.stdout).get('diagnostics', [])
    except json.JSONDecodeError:
        return None


def errors_of(diags):
    return [d for d in diags if d.get('severity') == 'error']


# 值位置的大写标识符：Lom 惯例类型/变体大写、值小写。教学片段里 NAM003 的
# 未定义"变量"若是大写名（Int/Float/UserId…），几乎必然是外来语法（type alias
# 的 RHS、注解泄漏等）——合法片段的散文上下文名都是小写。
UPPER_VAR_NAM003_RE = re.compile(r"(?:^|赋值给)未定义变量 '([A-Z][A-Za-z0-9_]*)")


def check_block(lom, tag, content, kind, tmpdir):
    """正例/反例断言。返回 (ok, 明细行)。"""
    rel = os.path.join(tmpdir, tag + '.lom')
    code, wrapped = wrap_if_fragment(content)
    with open(rel, 'w', encoding='utf-8', newline='\n') as f:
        f.write(code)

    diags = check_json(lom, rel)
    if diags is None:
        return False, '  %s: --json 输出不可解析' % tag
    errs = errors_of(diags)
    codes = [d.get('code', '?') for d in errs]

    if kind == 'counterexample':
        # 反例必须产生诊断（反知识声明失效——如特性将来被实现——则红）
        if errs:
            return True, '  %s: 反例产诊断 %s ✓' % (tag, codes)
        return False, '  %s: 反例零诊断——❌ 声明已失效，需更新文档' % tag

    # 正例三层断言
    parse_errs = [d for d in errs if d.get('stage') == 'parse']
    if parse_errs:
        return False, '  %s: 解析失败 %s（假特性类）' % (tag, [d.get('code') for d in parse_errs])

    has_import = any(l.startswith('from ') for l in content)
    if has_import:
        try:
            r = run_lom(lom, [rel])
            out = (r.stdout or '') + (r.stderr or '')
            if 'RUNTIME005' in out:
                return False, '  %s: RUNTIME005 假导入符号（sin/cos 类）' % tag
        except subprocess.TimeoutExpired:
            return False, '  %s: 导入层运行超时' % tag

    if not errs:
        try:
            r = run_lom(lom, [rel])
            if r.returncode != 0:
                last = (r.stderr or r.stdout or '').strip().split('\n')[-1][:80]
                return False, '  %s: 自含块运行退出码 %d：%s' % (tag, r.returncode, last)
            return True, '  %s: check 零 Error + 运行退出 0 ✓%s' % (tag, '（包 main）' if wrapped else '')
        except subprocess.TimeoutExpired:
            return False, '  %s: 运行超时（>%ds）' % (tag, TIMEOUT_RUN)

    non_nam = [c for c in codes if c != 'NAM003']
    if not non_nam:
        upper_vars = sorted({m for d in errs if d.get('code') == 'NAM003'
                             for m in UPPER_VAR_NAM003_RE.findall(d.get('message', ''))})
        if upper_vars:
            return False, ('  %s: 值位置出现大写名 %s —— 疑似外来语法'
                           '（type alias RHS/注解泄漏类）' % (tag, upper_vars))
        return True, '  %s: 教学片段（仅 NAM003 散文上下文名）✓%s' % (tag, '（包 main）' if wrapped else '')
    return False, '  %s: 非 NAM003 错误 %s' % (tag, non_nam)


def main():
    ap = argparse.ArgumentParser(description='SPEC_FOR_AI 代码示例实测对账（R7）')
    ap.add_argument('--spec', default=DEFAULT_SPEC)
    ap.add_argument('--lom', default=default_lom())
    args = ap.parse_args()

    if not os.path.isfile(args.lom):
        print('错误：找不到 lom 二进制 %s（先 cargo build --release）' % args.lom)
        return 2

    with open(args.spec, encoding='utf-8') as f:
        text = f.read()
    lines = text.split('\n')
    blocks = extract_blocks(text)
    if not blocks:
        print('错误：%s 中没有 fenced 代码块' % args.spec)
        return 2

    os.makedirs(TMP_DIR, exist_ok=True)
    stats = {'skip-json': 0, 'skip-diag': 0, 'skip-marker': 0,
             'counterexample': 0, 'positive': 0}
    failures = []
    print('SPEC_FOR_AI 示例对账：%d 个 fenced 块（%s）' % (len(blocks), args.spec))
    for n, (fence_line, content) in enumerate(blocks):
        tag = 'L%d' % fence_line
        kind = classify(content, nearest_text_above(lines, fence_line))
        if kind.startswith('skip'):
            stats[kind] += 1
            print('  %s: %s（跳过）' % (tag, kind))
            continue
        stats[kind] += 1
        try:
            ok, detail = check_block(args.lom, tag, content, kind, TMP_DIR)
        except subprocess.TimeoutExpired:
            ok, detail = False, '  %s: 超时' % tag
        print(detail)
        if not ok:
            failures.append(detail.strip())

    shutil.rmtree(TMP_DIR, ignore_errors=True)

    print('分类：正例 %d / 反例 %d / skip(json %d + diag %d + marker %d)'
          % (stats['positive'], stats['counterexample'],
             stats['skip-json'], stats['skip-diag'], stats['skip-marker']))
    if failures:
        print('RESULT: FAIL（%d 个块断言失败）' % len(failures))
        return 1
    print('RESULT: PASS')
    return 0


if __name__ == '__main__':
    sys.exit(main())
