# tools/verify_selfhost.py — Phase 8.1 自举前端验收脚本（RFC-0003 §8.1）
#
# 用法：
#   python tools/verify_selfhost.py            # 默认：dump 比对（examples + bootstrap + eval 113 参考解）
#   python tools/verify_selfhost.py --tokens   # token 流比对（lexer 对账，宿主 --dump-tokens）
#   python tools/verify_selfhost.py --diags    # 诊断比对（LEX/PARSE 码|行|列 + 消息折叠，坏文件集）
#   python tools/verify_selfhost.py --static   # 8.2 静态检查对齐（NAM003/TYPE003/EFF001/MAT001：坏文件 + 干净集误报检查）
#   python tools/verify_selfhost.py --run      # 8.3 examples 运行验收（stdout 对齐；v0.27.0 起 json×2 解除豁免，RFC-0003 修订 24）
#   python tools/verify_selfhost.py --wasm     # 8.4 wasm 载体三段验收：第二层全量 stdout 对齐 / 第三层 stmt_interp golden / 自施加 dump（v1.1.1 起规模限制解除，RFC-0003 修订 25）
#
# 验收口径（RFC-0003 修订记录 3 + 8.1 节）：
#   - dump / token：逐字一致；
#   - v1.0.0 起宿主 lexer 对非 ASCII 字面量按完整 UTF-8 解码（修复历史 Latin-1 字节展开），
#     下方 Latin-1 折叠等价逻辑降级为防御网（正常应 0 计数）；
#   - 诊断：码 + 位置逐字一致；消息经 Latin-1 折叠后等价（自举消息字面量经宿主
#     解释器读入时被 Latin-1 化，输出侧折叠还原）。
#
# 无第三方依赖（纯标准库 Python）。

import glob
import json
import os
import re
import subprocess
import sys
import tempfile

LOM = './target/release/lom.exe' if sys.platform == 'win32' else './target/release/lom'
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


FOUR_CODES = ('NAM003', 'TYPE003', 'EFF001', 'MAT001')


def mode_static(files):
    """8.2 口径：四类静态检查诊断对齐。
    - 坏文件集（tools/selfhost_cases + effects_bad + apply_test + fix_corpus）：
      宿主 --json 过滤四类码 vs 自举 --check 过滤四类码——码+位置+消息（折叠）逐字；
      注意双方对 parse 有错的文件都不产四类（宿主 diags.ok 才 typecheck，自举同款）。
    - 干净集（files 参数）：双方四类诊断都应为空（误报检查）。
    """
    bad_files = sorted(glob.glob('tools/selfhost_cases/*.bad.lom')) \
        + ['examples/effects_bad.lom', 'examples/apply_test.lom'] \
        + sorted(glob.glob('eval/fix_corpus/*.bad.lom'))
    ok = fail = 0
    fails = []
    for path in bad_files:
        host_json = run_lom([path, '--json'])
        try:
            diags = [d for d in json.loads(host_json)['diagnostics'] if d['code'] in FOUR_CODES]
        except (json.JSONDecodeError, KeyError):
            fails.append((path, '宿主 --json 解析失败'))
            fail += 1
            continue
        host_list = [(d['code'], str(d['line']), str(d['col'])) for d in diags]
        host_msgs = [fold_latin1(d['message']) or d['message'] for d in diags]
        try:
            self_txt = run_lom([SELF, '--', path, '--check'])
        except subprocess.TimeoutExpired:
            fails.append((path, 'TIMEOUT'))
            fail += 1
            continue
        self_list = []
        self_msgs = []
        for line in self_txt.splitlines():
            parts = line.split('|', 3)
            if len(parts) == 4 and parts[0] in FOUR_CODES:
                self_list.append((parts[0], parts[1], parts[2]))
                self_msgs.append(parts[3])
        if host_list == self_list and all(
            hm == sm or fold_latin1(sm) == hm for hm, sm in zip(host_msgs, self_msgs)
        ):
            ok += 1
        else:
            fails.append((path, f'不一致\n    宿主: {host_list}\n    自举: {self_list}'))
            fail += 1

    clean_ok = clean_bad = 0
    for path, _ in files:
        host_json = run_lom([path, '--json'])
        try:
            h = [(d['code'], str(d['line']), str(d['col']))
                 for d in json.loads(host_json)['diagnostics'] if d['code'] in FOUR_CODES]
        except (json.JSONDecodeError, KeyError):
            h = ['?']
        try:
            self_txt = run_lom([SELF, '--', path, '--check'])
            s2 = [tuple(line.split('|', 3)[:3]) for line in self_txt.splitlines()
                  if line.split('|', 3)[0] in FOUR_CODES]
        except subprocess.TimeoutExpired:
            s2 = ['TIMEOUT']
        if h == s2:
            clean_ok += 1
        else:
            clean_bad += 1
            fails.append((path, f'干净集误报 host={h} self={s2}'))
    print(f'static: 坏文件 PASS {ok} / FAIL {fail}；干净集 ALIGNED {clean_ok} / DIFF {clean_bad}')
    for f, why in fails:
        print(f'  FAIL {f}: {why}')
    return fail == 0 and clean_bad == 0



# --run 模式：examples 运行验收（8.3 口径固化 + v0.27.0 json×2 解除豁免）。
# 排除：apply_test（故意坏文件）；bench（args 驱动——argv 透传已落地，可手动
#   `lom self_interp.lom -- bench.lom --run <bench> <n>` 验证，不进默认集控时长）。
RUN_EXCLUDE = {'apply_test.lom', 'bench.lom'}

# 有状态示例的运行时产物——每侧运行前清理，保证宿主/自举两次运行同起点
# （file_demo 首行输出 file_exists 的 False：不清理则宿主留下的文件让自举侧
#   打印 True——首跑 CI 抓到，本地曾因陈旧文件假通过）。
RUN_ARTIFACTS = ['examples/_file_demo_tmp.txt', 'examples/_todo_data.json']


def _clean_run_artifacts():
    for a in RUN_ARTIFACTS:
        try:
            os.remove(a)
        except FileNotFoundError:
            pass


def mode_run(files):
    ok = fail = folded = 0
    fails = []
    for path, _ in files:
        p = path.replace('\\', '/')
        if not p.startswith('examples/'):
            continue
        if os.path.basename(p) in RUN_EXCLUDE:
            continue
        _clean_run_artifacts()
        host = run_lom([path])
        _clean_run_artifacts()
        try:
            self_out = run_lom([SELF, '--', path, '--run'])
        except subprocess.TimeoutExpired:
            fails.append((path, 'TIMEOUT'))
            fail += 1
            continue
        if host == self_out:
            ok += 1
        else:
            # 防御网：v1.0.0 修复宿主 lexer 非 ASCII 解码后，正常应为 0 计数折叠等价
            # （两侧逐字一致直接走上面分支）；若此路径再次命中，说明宿主/自举的
            # 非 ASCII 处理又分叉了——按 dump/tokens 模式同款 Latin-1 折叠判定等价
            hl, sl = host.splitlines(), self_out.splitlines()
            if len(hl) == len(sl) and all(
                h == s or fold_latin1(h) == s for h, s in zip(hl, sl)
            ):
                ok += 1
                folded += 1
            else:
                fails.append((path, f'输出不一致 host={len(host)}B self={len(self_out)}B'))
                fail += 1
    print(f'run: PASS {ok} / FAIL {fail}（examples+bootstrap 运行验收，含 stmt_interp 三层自证；Latin-1 折叠等价 {folded}）')
    for f, why in fails:
        print(f'  FAIL {f}: {why}')
    return fail == 0


# 8.4 第二层：wasm 载体跑自举解释器。
# v1.1.1（W 工作包）起规模限制解除：根治两处 codegen bug——
#   ① 变体 Binder 臂的载荷 i64.load 与臂测试平铺急切求值，0 参枚举贴线性内存
#     末尾时越界（8.4 挂账的 tok_disc 野值真相，RFC-0003 修订 25）；
#   ② for 三个分派 if 未压 Label::If → for 体内 return/? 的 br $ret 深度少算、
#     提前返回被吞（eval 020 在自举层暴露；7.6 起潜伏）。
# 第二层 = examples+bootstrap 全量（RUN_EXCLUDE 豁免；stmt_interp 走第三层）+ eval 参考解。
# 深链形态（长表达式链/深嵌套 parse 递归）仍受 V8 默认栈限制——已知、可
# --stack-size 调（RFC-0002 退出标准 4 口径），非本层验收面。
WASM_NODE_STACK = 60000  # 第三层/自施加用的 V8 栈深（KB；自举套自举递归深度相乘）


def mode_wasm(files):
    """编译 self_interp → wasm；三段验收：
    ① 第二层：wasm 载体自举解释器执行全量文件，stdout 与宿主直接运行对齐；
    ② 第三层：stmt_interp 39 条 golden 逐字（--stack-size）；
    ③ 自施加：self_interp --dump-ast 自身源码，与宿主原生 dump 逐字一致。
    """
    wasm_path = os.path.join(tempfile.gettempdir(), 'self_interp_84.wasm')
    try:
        r = subprocess.run([LOM, 'build', SELF, '--target', 'wasm', '-o', wasm_path],
                           capture_output=True, text=True, encoding='utf-8', errors='replace', timeout=600)
        if not os.path.exists(wasm_path):
            print(f'wasm 编译失败: {r.stdout} {r.stderr}')
            return False
    except subprocess.TimeoutExpired:
        print('wasm 编译 TIMEOUT')
        return False

    def run_wasm_self(args, stack=None, timeout=900):
        cmd = ['node'] + ([f'--stack-size={stack}'] if stack else []) \
            + ['eval/runner/run_wasm.mjs', wasm_path, '--'] + args
        return subprocess.run(cmd, capture_output=True, text=True,
                              encoding='utf-8', errors='replace', timeout=timeout)

    # ---- ① 第二层：examples+bootstrap（豁免 apply_test/bench；stmt_interp 走 ②）+ eval 参考解 ----
    ok = fail = folded = 0
    fails = []
    for path, _ in files:
        p = path.replace('\\', '/')
        base = os.path.basename(p)
        if base in RUN_EXCLUDE or base == 'stmt_interp.lom':
            continue
        _clean_run_artifacts()
        host = run_lom([path])
        _clean_run_artifacts()
        try:
            r = run_wasm_self([path, '--run'])
        except subprocess.TimeoutExpired:
            fails.append((p, 'TIMEOUT')); fail += 1; continue
        if host == r.stdout:
            ok += 1
        else:
            # 防御网：todo.lom 的非 ASCII 输出走 Latin-1 折叠（宿主 lexer 字节展开，
            # wasm 侧经 harness utf-8 解码是干净侧）——同 mode_run 口径
            hl, wl = host.splitlines(), r.stdout.splitlines()
            if len(hl) == len(wl) and all(
                h == w or fold_latin1(h) == w for h, w in zip(hl, wl)
            ):
                ok += 1
                folded += 1
            else:
                fails.append((p, f'host={len(host)}B wasm={len(r.stdout)}B rc={r.returncode}'))
                fail += 1
    print(f'wasm-layer2: PASS {ok} / FAIL {fail}（examples+bootstrap+eval 参考解全量，'
          f'规模限制已解除；Latin-1 折叠等价 {folded}；RFC-0003 修订 25）')
    for f, why in fails:
        print(f'  FAIL {f}: {why}')

    # ---- ② 第三层：stmt_interp golden（V8 栈深可调的已知限制）----
    l3_ok = False
    l3_why = ''
    try:
        r = run_wasm_self(['examples/bootstrap/stmt_interp.lom', '--run'], stack=WASM_NODE_STACK)
        with open('examples/bootstrap/stmt_interp.expected.txt', encoding='utf-8') as f:
            golden = f.read().replace('\r\n', '\n')
        if r.returncode == 0 and r.stdout == golden:
            l3_ok = True
        else:
            l3_why = f'rc={r.returncode} wasm={len(r.stdout)}B golden={len(golden)}B'
    except subprocess.TimeoutExpired:
        l3_why = 'TIMEOUT'
    print(f"wasm-layer3: {'PASS' if l3_ok else 'FAIL ' + l3_why}"
          f'（wasm 自举跑 stmt_interp，39 条 golden 逐字；--stack-size={WASM_NODE_STACK}）')

    # ---- ③ 自施加：self_interp --dump-ast 自身 5703 行源码 ----
    sa_ok = False
    sa_why = ''
    try:
        host_dump = run_lom([SELF, '--dump-ast'], timeout=600)
        r = run_wasm_self([SELF, '--dump-ast'], stack=WASM_NODE_STACK, timeout=600)
        if r.returncode == 0 and r.stdout == host_dump:
            sa_ok = True
        else:
            sa_why = f'rc={r.returncode} wasm={len(r.stdout)}B host={len(host_dump)}B'
    except subprocess.TimeoutExpired:
        sa_why = 'TIMEOUT'
    print(f"wasm-selfapply: {'PASS' if sa_ok else 'FAIL ' + sa_why}"
          f'（wasm 自举解析自身源码 dump，与宿主原生逐字一致）')

    return fail == 0 and l3_ok and sa_ok


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
        elif mode == '--wasm':
            all_ok = mode_wasm(real_files)
        elif mode == '--static':
            all_ok = mode_static(real_files)
        elif mode == '--run':
            all_ok = mode_run(real_files)
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
