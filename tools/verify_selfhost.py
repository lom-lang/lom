# tools/verify_selfhost.py — Phase 8.1 自举前端验收脚本（RFC-0003 §8.1）
#
# 用法：
#   python tools/verify_selfhost.py            # 默认：dump 比对（examples + bootstrap + eval 113 参考解）
#   python tools/verify_selfhost.py --tokens   # token 流比对（lexer 对账，宿主 --dump-tokens）
#   python tools/verify_selfhost.py --diags    # 诊断比对（码|行|列 + 消息 Latin-1 折叠，用坏文件集）
#
# 验收口径（RFC-0003 修订记录 3 + 8.1 节）：
#   - dump / token：逐字一致；
#   - todo.lom 的 Str 内容：宿主 lexer 按字节 Latin-1 展开（历史行为）vs 自举按字符——
#     结构/位置必须一致，Str 值经 Latin-1 折叠后逐字等价（单独计数，不计入 FAIL）；
#   - 诊断：码 + 位置逐字一致；消息经 Latin-1 折叠后等价（自举消息字面量经宿主
#     解释器读入时被 Latin-1 化，输出侧折叠还原）。
#
# 无第三方依赖（纯标准库 Python）。

import glob
import json
import re
import subprocess
import sys

LOM = './target/release/lom.exe'
SELF = 'examples/selfhost/self_interp.lom'
STR_RE = re.compile(r'^(\s*)Str "(.*)"$')
# token 行形态：Str("...") @ln:cl（--dump-tokens 的载荷）
TOKSTR_RE = re.compile(r'^(\s*Str\(")(.*)("\) @\d+:\d+)$')


def run_lom(args, timeout=300):
    r = subprocess.run([LOM] + args, capture_output=True, text=True,
                       encoding='utf-8', errors='replace', timeout=timeout)
    return r.stdout


def unescape_rust_debug(s):
    # Rust Debug 字符串转义：\" \\ \n \r \t \u{XX}
    out = []
    i = 0
    while i < len(s):
        if s[i] == '\\' and i + 1 < len(s):
            c = s[i + 1]
            if c == 'n':
                out.append('\n'); i += 2
            elif c == 'r':
                out.append('\r'); i += 2
            elif c == 't':
                out.append('\t'); i += 2
            elif c == '"':
                out.append('"'); i += 2
            elif c == '\\':
                out.append('\\'); i += 2
            elif c == 'u' and i + 2 < len(s) and s[i + 2] == '{':
                j = s.index('}', i)
                out.append(chr(int(s[i + 3:j], 16))); i = j + 1
            else:
                out.append(c); i += 2
        else:
            out.append(s[i]); i += 1
    return ''.join(out)


def fold_latin1(s):
    try:
        return s.encode('latin-1').decode('utf-8')
    except (UnicodeEncodeError, UnicodeDecodeError):
        return None


def dump_equiv_with_latin1(host_text, self_text):
    """逐字比对；Str 行允许 Latin-1 折叠等价。返回 (ok, reason)。"""
    hl = host_text.splitlines()
    sl = self_text.splitlines()
    if len(hl) != len(sl):
        return False, f'行数不一致 {len(hl)} vs {len(sl)}'
    for i, (h, s2) in enumerate(zip(hl, sl)):
        if h == s2:
            continue
        mh = STR_RE.match(h)
        ms = STR_RE.match(s2)
        if mh and ms and mh.group(1) == ms.group(1):
            folded = fold_latin1(unescape_rust_debug(mh.group(2)))
            if folded == unescape_rust_debug(ms.group(2)):
                continue
        th = TOKSTR_RE.match(h)
        ts = TOKSTR_RE.match(s2)
        if th and ts and th.group(1) == ts.group(1) and th.group(3) == ts.group(3):
            folded = fold_latin1(unescape_rust_debug(th.group(2)))
            if folded == unescape_rust_debug(ts.group(2)):
                continue
        return False, f'第 {i + 1} 行不一致: {h!r} vs {s2!r}'
    return True, ''


def load_refset():
    """从 eval/tasks/*.json 动态提取参考解（与 eval/runner 的对账口径一致）。"""
    out = []
    for jf in sorted(glob.glob('eval/tasks/*.json')):
        for t in json.load(open(jf, encoding='utf-8')):
            out.append((f"eval#{t['id']}", t['solution']))
    return out


def mode_dump(files):
    ok = fail = folded_ok = 0
    fails = []
    for path, _ in files:
        host = run_lom([path, '--dump-ast'])
        try:
            self_out = run_lom([SELF, '--', path])
        except subprocess.TimeoutExpired:
            fails.append((path, 'TIMEOUT'))
            fail += 1
            continue
        if host == self_out:
            ok += 1
        else:
            eq, why = dump_equiv_with_latin1(host, self_out)
            if eq:
                ok += 1
                folded_ok += 1
            else:
                fails.append((path, why))
                fail += 1
    print(f'dump: PASS {ok} / FAIL {fail}（其中 Latin-1 折叠等价 {folded_ok}）')
    for f, why in fails:
        print(f'  FAIL {f}: {why}')
    return fail == 0


def mode_tokens(files):
    ok = fail = known = 0
    fails = []
    for path, _ in files:
        host = run_lom([path, '--dump-tokens'])
        try:
            self_out = run_lom([SELF, '--', path, '--tokens'])
        except subprocess.TimeoutExpired:
            fails.append((path, 'TIMEOUT'))
            fail += 1
            continue
        # token 行含 Str 载荷——todo.lom 走折叠等价
        if host == self_out:
            ok += 1
        else:
            eq, why = dump_equiv_with_latin1(host, self_out)
            if eq:
                ok += 1
            elif path.replace('\\', '/') == 'examples/todo.lom' and _token_coords_only_diff(host, self_out):
                # RFC-0003 修订记录 3：宿主列是字节列、自举是字符列，
                # 非 ASCII 行的 token 列必然分叉（token 流含位置，dump 不含）。
                # 序列+行号+载荷已核对一致，仅列差异 → 记为已知差异
                ok += 1
                known += 1
            else:
                fails.append((path, why))
                fail += 1
    print(f'tokens: PASS {ok} / FAIL {fail}（其中列坐标系已知差异 {known}）')
    for f, why in fails:
        print(f'  FAIL {f}: {why}')
    return fail == 0


def _token_coords_only_diff(host_text, self_text):
    """todo.lom 专用：token 序列/行号/载荷全一致（Str 载荷允许折叠等价），
    仅非 ASCII 行的列不同。"""
    hl = host_text.splitlines()
    sl = self_text.splitlines()
    if len(hl) != len(sl):
        return False
    for h, s2 in zip(hl, sl):
        if h == s2:
            continue
        # Str 载荷行：折叠等价 + 行号一致（列在非 ASCII 行分叉，忽略）
        th = TOKSTR_RE.match(h)
        ts = TOKSTR_RE.match(s2)
        if th and ts:
            folded = fold_latin1(unescape_rust_debug(th.group(2)))
            if folded == unescape_rust_debug(ts.group(2)):
                hn = re.search(r'@(\d+):', th.group(3))
                sn = re.search(r'@(\d+):', ts.group(3))
                if hn and sn and hn.group(1) == sn.group(1):
                    continue
            return False
        # 其他 token：载荷与行号一致、仅列不同（非 ASCII 行的字节列/字符列）
        mh = re.match(r'^(.*) @(\d+):(\d+)$', h)
        ms = re.match(r'^(.*) @(\d+):(\d+)$', s2)
        if not (mh and ms):
            return False
        if mh.group(1) != ms.group(1) or mh.group(2) != ms.group(2):
            return False
    return True


def mode_diags():
    # 坏文件集：fix_corpus 的 .bad.lom + apply_test（故意坏示例）
    files = sorted(glob.glob('eval/fix_corpus/*.bad.lom')) + ['examples/apply_test.lom']
    ok = fail = 0
    fails = []
    for path in files:
        host_json = run_lom([path, '--json'])
        try:
            diags = json.loads(host_json)['diagnostics']
        except (json.JSONDecodeError, KeyError):
            fails.append((path, '宿主 --json 解析失败'))
            fail += 1
            continue
        # 8.1 口径：只比对词法/语法阶段（LEX*/PARSE*）；NAM/TYPE/MAT/EFF 是 8.2 的范围
        diags = [d for d in diags if d['code'].startswith(('LEX', 'PARSE'))]
        host_list = [(d['code'], str(d['line']), str(d['col'])) for d in diags]
        try:
            self_txt = run_lom([SELF, '--', path, '--diags'])
        except subprocess.TimeoutExpired:
            fails.append((path, 'TIMEOUT'))
            fail += 1
            continue
        self_list = []
        self_msgs = []
        for line in self_txt.splitlines():
            parts = line.split('|', 3)
            if len(parts) == 4:
                self_list.append((parts[0], parts[1], parts[2]))
                self_msgs.append(parts[3])
        if host_list == self_list:
            # 码+位置对齐；消息折叠等价抽查（lex/parse 消息均为宿主中文，折叠后应等价）
            host_msgs = [fold_latin1(d['message']) or d['message'] for d in diags]
            msgs_eq = all(
                hm == sm or fold_latin1(sm) == hm
                for hm, sm in zip(host_msgs, self_msgs)
            )
            if msgs_eq:
                ok += 1
            else:
                fails.append((path, '码+位置一致但消息折叠后不等价'))
                fail += 1
        else:
            fails.append((path, f'诊断不一致\n    宿主: {host_list}\n    自举: {self_list}'))
            fail += 1
    print(f'diags: PASS {ok} / FAIL {fail}（坏文件 {len(files)} 个）')
    for f, why in fails:
        print(f'  FAIL {f}: {why}')
    return fail == 0


def main():
    mode = sys.argv[1] if len(sys.argv) > 1 else '--dump'
    files = sorted(
        [(p, None) for p in glob.glob('examples/*.lom')]
        + [(p, None) for p in glob.glob('examples/bootstrap/*.lom')]
        + load_refset()
    )
    # 参考解写到临时文件跑（宿主 CLI 需要文件路径；用项目内临时目录，跑完即删）
    import os
    import tempfile
    tmpdir = tempfile.mkdtemp(prefix='_selfhost_ref_', dir='.')
    real_files = []
    try:
        for name, src in files:
            if src is None:
                real_files.append((name, None))
            else:
                p = os.path.join(tmpdir, name.replace('#', '_') + '.lom')
                with open(p, 'w', encoding='utf-8', newline='') as f:
                    f.write(src)
                real_files.append((p, None))
        if mode == '--tokens':
            all_ok = mode_tokens(real_files)
        elif mode == '--diags':
            all_ok = mode_diags()
        else:
            all_ok = mode_dump(real_files)
    finally:
        for p in os.listdir(tmpdir):
            os.remove(os.path.join(tmpdir, p))
        os.rmdir(tmpdir)
    print('RESULT:', 'PASS' if all_ok else 'FAIL')
    sys.exit(0 if all_ok else 1)


if __name__ == '__main__':
    main()
