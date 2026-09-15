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
#   G. 测试数（Q4，2026-09-08）：源码 #[test] 静态计数 → HANDOVER §2.2 期望行 /
#      §9 检查单 / README 状态段（测试数同步教训三连——R 审查两抓 + N1 三处手改）
#   F. changelog 对账（N3，2026-09-07）：LANGUAGE_SPEC §13 条目 ↔ 版本/tag 双向——
#      当前 Cargo 版本必须有 §13 条目（防升版忘写 changelog，历史主腐坏形态）；
#      v1.0 冻结时代起 tag 与条目双向一致（v1.x 前的 0.x spec/工程两套编号是
#      历史结构，不追改——§13 是 spec 视角记录，纯工程版本明文豁免）
#   H. 宣称-证据对账（V1，2026-09-15）：tools/claims.json 每条 claim 核验
#      ①算术闭合 sum(parts)==claim（根治第五轮 R22 的 2600 型算术笔误）；
#      ②出现点同步——各文档现值捕获组 == claim（根治多文档"五处同错"漏改）
#   I. 源码计数（V2，2026-09-15）：diff_gen POOL_BASE 模板族数 / diff_test 探针
#      数 / SPEC_FOR_AI §11f 分歧条数 → 文档现值宣称（含中文数字形态）——
#      R27/R28 型口径漂移的钉子
#   Z. 自指项数（V2）：本工具总项数与 HANDOVER §2.2 宣称互锁——R16"监控不了
#      自身项数"盲区关闭
#
# 纪律：模式找不到也算 FAIL（文档措辞重构时必须同步更新本清单——对账清单本身也是文档）。
# 历史时点值（带日期/版本标签的快照，如 changelog 的 "eval 114/114 (v1.0.0)"、
# "113-task set as it stood then"）不在监控范围，只对现值声称；changelog 是天然时点日志，
# spec 的现值位置是 §12.3/§12.5。

import glob
import json
import os
import re
import subprocess
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
    # R42（七审整改）：README 门面 Current release 行——B 包升版漏改四个版本的
    # 存量腐坏位，E 类此前只盯 HANDOVER 不覆盖此处
    expect_all('README Current release', 'README.md',
               r'Current release: v([\d.]+)', [cargo_ver])

    # ---- F. changelog 对账（N3）----
    print('F. changelog 对账（LANGUAGE_SPEC §13）')
    entries = re.findall(r'(?m)^- \*\*(v[\d.]+)', read('LANGUAGE_SPEC.md'))
    check('当前版本有 changelog 条目', 'v' + cargo_ver in entries,
          'Cargo %s；§13 现有条目 %s' % (cargo_ver, ' '.join(entries)))
    tags = subprocess.run(['git', 'tag', '--list'], capture_output=True,
                          encoding='utf-8').stdout.split()
    tags_v1 = sorted(t for t in tags if t.startswith('v1.'))
    missing = [t for t in tags_v1 if t not in entries]
    check('v1.x tag 全有条目', not missing,
          '%d 个 v1.x tag（%s）；缺条目：%s' % (len(tags_v1), ' '.join(tags_v1),
                                               missing or '无'))
    # 当前发布中的版本豁免防虚构（升版流程是"先写条目→CI 绿→后打 tag"——
    # 与 tag 纪律的时序冲突，v1.1.2 升版时暴露过死锁：CI #96/#97）
    fictitious = [e for e in entries
                  if e.startswith('v1.') and e not in tags and e != 'v' + cargo_ver]
    check('v1.x 条目全有 tag（防虚构；当前发布版本豁免）', not fictitious,
          '无 tag 的条目：%s（当前版本 v%s 豁免）' % (fictitious or '无', cargo_ver))

    # ---- G. 测试数（Q4）----
    print('G. 测试数（源码 #[test] 静态计数）')
    test_count = 0
    for fp in sorted(glob.glob('src/**/*.rs', recursive=True)):
        test_count += read(fp).count('#[test]')
    expect_all('HANDOVER §2.2 测试行', 'docs/HANDOVER.md',
               r'cargo test --release\s+# 期望 (\d+)/(\d+)', [test_count, test_count])
    expect_all('HANDOVER §9 检查单', 'docs/HANDOVER.md',
               r'test --release` 确认 (\d+)/(\d+)', [test_count, test_count])
    expect_all('README 状态段测试数', 'README.md',
               r'(?m)^(\d+)/(\d+) Rust unit tests', [test_count, test_count])

    # ---- H. 宣称-证据对账（V1，2026-09-15：第五轮 R22 教训工程化）----
    # 每条 claim 核验两件事：①算术闭合 sum(parts)==claim（根治 2600 型算术笔误）；
    # ②出现点同步——各文档现值的所有捕获组 == claim（根治"五处同错"漏改）。
    # parts 的 evidence 指针供人审（指向 TODO 证据区原文），机器不核其真实性。
    print('H. 宣称-证据对账（tools/claims.json：分项回加 + 出现点同步）')
    claims = json.loads(read('tools/claims.json')).get('claims', [])
    check('claims.json 登记数 >= 1', len(claims) >= 1, '%d 条' % len(claims))
    for c in claims:
        s = sum(p['value'] for p in c['parts'])
        check('H %s 算术闭合' % c['id'], s == c['claim'],
              'sum(parts)=%d vs claim=%d（%s）'
              % (s, c['claim'], ' + '.join(str(p['value']) for p in c['parts'])))
        for occ in c['occurrences']:
            got = doc_number(occ['path'], occ['pattern'])
            if got is None:
                check('H %s @ %s' % (c['id'], occ['path']), False,
                      '模式未找到（措辞重构时同步 claims.json）: %r' % occ['pattern'])
            else:
                check('H %s @ %s' % (c['id'], occ['path']),
                      all(g == str(c['claim']) for g in got),
                      '现值 %s（期望每组 == %d）' % (list(got), c['claim']))

    # ---- I. 源码计数（V2：模板池/探针/分歧清单 → 文档现值）----
    # R27/R28 型口径漂移（探针条数/安全界多处三个数）的钉子：真值从源码/规范
    # 机械清点，文档宣称位（含中文数字形态）逐处比对。中文数字仅支持一至十
    # （§11f 分歧清单与探针集在冻结期不会超十，超出时本项 FAIL 提示人工扩表）。
    print('I. 源码计数（diff_gen 模板池 / diff_test 探针 / SPEC §11f 分歧条数）')
    CN = '一二三四五六七八九十'
    EN = ['one', 'two', 'three', 'four', 'five', 'six', 'seven', 'eight', 'nine', 'ten']

    def cn(n):
        return CN[n - 1] if 1 <= n <= 10 else None

    def en(n):
        return EN[n - 1] if 1 <= n <= 10 else None

    m_pool = re.search(r'POOL_BASE = \[(.*?)\n\]', read('tools/diff_gen.py'), re.S)
    pool_base = len(re.findall(r'Gen\.t_\w+', m_pool.group(1))) if m_pool else -1
    check('模板池可清点', pool_base > 0, 'POOL_BASE = %d 族' % pool_base)
    probe_n = len(set(re.findall(r'_pair_for_probe\("([\w-]+)"',
                                 read('tools/diff_test.py'))))
    m_11f = re.search(r'## 11f\..*?(?=\n## |\Z)', read('SPEC_FOR_AI.md'), re.S)
    div_n = len(re.findall(r'(?m)^  \d+\. \*\*', m_11f.group(0))) if m_11f else -1
    check('§11f 分歧清单可清点', div_n > 0, '§11f = %d 条' % div_n)
    expect_all('I §11f 条数 @ SPEC_FOR_AI §11f 导语（英文位）', 'SPEC_FOR_AI.md',
               r'the (one|two|three|four|five|six|seven|eight|nine|ten) known divergences',
               [en(div_n)])
    expect_all('I 模板族数 @ HANDOVER §1 下一步', 'docs/HANDOVER.md',
               r'模板族 (\d+)）', [pool_base])
    expect_all('I §11f 条数 @ diff_gen 头注释', 'tools/diff_gen.py',
               r'§11f (一|二|三|四|五|六|七|八|九|十)条已知分歧', [cn(div_n)])
    expect_all('I §11f 条数 @ diff_test 头注释', 'tools/diff_test.py',
               r'§11f (一|二|三|四|五|六|七|八|九|十)条已知分歧', [cn(div_n)])
    expect_all('I §11f/探针 @ diff_test run_probes docstring', 'tools/diff_test.py',
               r'(一|二|三|四|五|六|七|八|九|十)条白名单中的'
               r'(一|二|三|四|五|六|七|八|九|十)条可执行分歧', [cn(div_n), cn(probe_n)])
    expect_all('I §11f 条数 @ diff_test main 打印', 'tools/diff_test.py',
               r'§11f (一|二|三|四|五|六|七|八|九|十)条已知分歧', [cn(div_n)])
    expect_all('I §11f/探针 @ HANDOVER §2.2 --probe 行', 'docs/HANDOVER.md',
               r'§11f (一|二|三|四|五|六|七|八|九|十)条中'
               r'(一|二|三|四|五|六|七|八|九|十)条可执行分歧', [cn(div_n), cn(probe_n)])
    expect_all('I §11f/探针 @ HANDOVER §9-5', 'docs/HANDOVER.md',
               r'§11f (一|二|三|四|五|六|七|八|九|十)条分歧全档案，探针 (\d+)/(\d+)',
               [cn(div_n), probe_n, probe_n])

    # ---- Z. 自指项数（R16 盲区关闭：本工具总项数与 HANDOVER 宣称一致）----
    # R16 教训："doc_audit 监控不了自身项数的自指盲区"。本项把 §2.2 的
    # "文档数字 N 项"宣称与脚本实际产出项数互锁（len+1 计入本项自身）。
    m_z = doc_number('docs/HANDOVER.md', r'doc_audit\.py\s+# 对账：文档数字 (\d+) 项')
    total_all = len(RESULTS) + 1
    check('Z doc_audit 项数自指', m_z is not None and int(m_z[0]) == total_all,
          'HANDOVER 宣称 %s vs 本脚本总项数 %d（含本项）'
          % (m_z[0] if m_z else None, total_all))

    total, ok = len(RESULTS), sum(RESULTS)
    print('RESULT: %s（%d/%d 项通过）' % ('PASS' if ok == total else 'FAIL', ok, total))
    return 0 if ok == total else 1


if __name__ == '__main__':
    sys.exit(main())
