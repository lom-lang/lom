# tools/doc_audit.py — 文档数字自动对账 gate（第二轮审查整改 R8，2026-09-05）
#
# 背景：本次审查的陈旧数字呈"总账更新、边角漏网"模式（§9 的 454/147、spec §12.4/12.5、
# eval/README 三处、README 拆分），其中两处是 HANDOVER:355 白纸黑字记录过的复发教训——
# 人工簿记清单天然漏项。本工具把"机械可核的现值数字"收敛为 CI gate。
#
# 用法：
#   python tools/doc_audit.py            # 仓库根目录运行（默认相对路径）
#
# 监控清单（计算真值 → 文档现值逐处比对；真值变了只改数字不再靠人肉清单）：
#   A. eval 任务总数：tasks/*.json 求和 → manifest total_tasks + eval/README ×3
#      + README + LANGUAGE_SPEC ×3 + HANDOVER ×2
#   B. 自举 dump 计数：examples 顶层 + bootstrap + eval 总数 → HANDOVER §2.2 + ci.yml 步骤名
#   C. .lom 文件拆分：glob 实数（总数/顶层/bootstrap/pkg_demo/selfhost）→ README 状态段
#   D. self_interp 行数：wc 口径（换行符计数）→ HANDOVER §9
#   E. 版本号：Cargo.toml（+ Cargo.lock 一致性）→ HANDOVER §1/§9
#
# 纪律：模式找不到也算 FAIL（文档措辞重构时必须同步更新本清单——对账清单本身也是文档）。
# 历史时点值（带日期/版本标签的快照，如 changelog 的 "eval 114/114 (v1.0.0)"、
# "113-task set as it stood then"）不在监控范围，只对现值声称；changelog 是天然时点日志，
# spec 的现值位置是 §12.3/§12.5。

import glob
import json
import os
import re
import sys

RESULTS = []


def read(path):
    with open(path, encoding='utf-8') as f:
        return f.read()


def check(name, ok, detail):
    RESULTS.append(ok)
    print(('  OK  ' if ok else '  FAIL') + ' | ' + name + ' | ' + detail)


def doc_number(path, pattern, flags=0):
    """返回文档中首个匹配的全部捕获组（str 元组）；未匹配返回 None。"""
    m = re.search(pattern, read(path), flags)
    return m.groups() if m else None


def expect_all(name, path, pattern, expected, flags=0):
    """expected: 与捕获组等长的期望值列表（str 比较）。"""
    got = doc_number(path, pattern, flags)
    if got is None:
        check(name, False, '%s 中模式未找到（措辞变了？同步更新 tools/doc_audit.py 清单）: %r'
              % (path, pattern))
        return
    exp = [str(e) for e in expected]
    check(name, list(got) == exp, '%s 现值 %s（期望 %s）' % (path, list(got), exp))


def main():
    root = os.getcwd()
    fail_early = [p for p in ('eval/manifest.json', 'README.md', 'LANGUAGE_SPEC.md',
                              'docs/HANDOVER.md', '.github/workflows/ci.yml',
                              'Cargo.toml', 'Cargo.lock') if not os.path.isfile(p)]
    if fail_early:
        print('错误：缺少文件 %s——请在仓库根目录运行' % fail_early)
        return 2

    # ---- 计算真值 ----
    eval_total = 0
    for fp in sorted(glob.glob('eval/tasks/*.json')):
        data = json.loads(read(fp))
        eval_total += len(data if isinstance(data, list) else data.get('tasks', []))

    top_lom = sorted(glob.glob('examples/*.lom'))
    boot_lom = sorted(glob.glob('examples/bootstrap/*.lom'))
    pkg_lom = sorted(glob.glob('examples/pkg_demo/**/*.lom', recursive=True))
    self_lom = sorted(glob.glob('examples/selfhost/*.lom'))
    dump_expect = len(top_lom) + len(boot_lom) + eval_total

    self_interp = 'examples/selfhost/self_interp.lom'
    self_lines = read(self_interp).count('\n')

    cargo_ver = re.search(r'^version\s*=\s*"([^"]+)"', read('Cargo.toml'),
                          re.M).group(1)
    lock_ver = None
    m = re.search(r'name = "lom"\s*\nversion = "([^"]+)"', read('Cargo.lock'))
    if m:
        lock_ver = m.group(1)

    print('真值：eval %d 任务 | dump %d 文件（%d 顶层 + %d bootstrap + %d eval）| '
          '.lom 拆分 %d=%d+%d+%d+%d | self_interp %d 行 | 版本 %s（lock %s）'
          % (eval_total, dump_expect, len(top_lom), len(boot_lom), eval_total,
             len(top_lom) + len(boot_lom) + len(pkg_lom) + len(self_lom),
             len(top_lom), len(boot_lom), len(pkg_lom), len(self_lom),
             self_lines, cargo_ver, lock_ver))

    # ---- A. eval 任务总数 ----
    print('A. eval 任务总数')
    manifest = json.loads(read('eval/manifest.json'))
    check('manifest total_tasks', manifest.get('total_tasks') == eval_total,
          'manifest %s（期望 %d）' % (manifest.get('total_tasks'), eval_total))
    expect_all('eval/README 标题计数', 'eval/README.md',
               r'(?m)^(\d+)-task benchmark', [eval_total])
    expect_all('eval/README 覆盖计数', 'eval/README.md',
               r'(\d+) tasks across 10 categories', [eval_total])
    expect_all('eval/README 期望通过', 'eval/README.md',
               r'\*\*(\d+)/(\d+) pass\*\*', [eval_total, eval_total])
    expect_all('README 状态段', 'README.md',
               r'`eval/` (\d+)/(\d+) reference solutions', [eval_total, eval_total])
    expect_all('LANGUAGE_SPEC §12.3', 'LANGUAGE_SPEC.md',
               r'\*\*(\d+)/(\d+) pass on both backends', [eval_total, eval_total])
    expect_all('LANGUAGE_SPEC §12.5', 'LANGUAGE_SPEC.md',
               r'Reference solutions: (\d+)/(\d+) pass', [eval_total, eval_total])
    expect_all('HANDOVER §1 评测集行', 'docs/HANDOVER.md',
               r'评测集 \| \*\*(\d+)/(\d+)\*\*', [eval_total, eval_total])
    # HANDOVER §2.2 的 eval 行锚定 run.ps1（同文件的 "期望 456/456" 是 cargo test 行，测试数不在监控范围）
    expect_all('HANDOVER §2.2 期望', 'docs/HANDOVER.md',
               r'run\.ps1 -Verify[^\n]*# 期望 (\d+)/(\d+)', [eval_total, eval_total])

    # ---- B. 自举 dump 计数 ----
    print('B. 自举 dump 计数')
    expect_all('HANDOVER §2.2 dump', 'docs/HANDOVER.md',
               r'dump (\d+)/(\d+)（另', [dump_expect, dump_expect])
    expect_all('ci.yml 步骤名', '.github/workflows/ci.yml',
               r'Selfhost dump \((\d+) 文件逐字', [dump_expect])

    # ---- C. .lom 文件拆分 ----
    print('C. .lom 文件拆分')
    expect_all('README 状态段拆分', 'README.md',
               r'(\d+) `\.lom` files \((\d+) examples \+ (\d+) bootstrap[^+]*'
               r'\+ (\d+) in the `pkg_demo` package \+ (\d+) self-hosted',
               [len(top_lom) + len(boot_lom) + len(pkg_lom) + len(self_lom),
                len(top_lom), len(boot_lom), len(pkg_lom), len(self_lom)])

    # ---- D. self_interp 行数 ----
    print('D. self_interp 行数')
    expect_all('HANDOVER §9-5', 'docs/HANDOVER.md',
               r'self_interp\.lom（(\d+) 行', [self_lines])

    # ---- E. 版本号 ----
    print('E. 版本号')
    check('Cargo.lock 一致', lock_ver == cargo_ver,
          'Cargo.toml %s vs Cargo.lock %s' % (cargo_ver, lock_ver))
    expect_all('HANDOVER §1 版本行', 'docs/HANDOVER.md',
               r'\| 版本 \| \*\*v([\d.]+)\*\*', [cargo_ver])
    expect_all('HANDOVER §9 版本显示', 'docs/HANDOVER.md',
               r'--version` 显示 ([\d.]+)', [cargo_ver])

    total, ok = len(RESULTS), sum(RESULTS)
    print('RESULT: %s（%d/%d 项通过）' % ('PASS' if ok == total else 'FAIL', ok, total))
    return 0 if ok == total else 1


if __name__ == '__main__':
    sys.exit(main())
