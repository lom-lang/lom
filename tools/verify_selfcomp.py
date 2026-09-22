#!/usr/bin/env python3
"""verify_selfcomp.py —— L2 自举编译器逐批验收（RFC-0004 L2.2/L2.3）。

对拍口径（RFC-0004 预设：行为级一致，不做字节级一致——编译器实现自由度）：
  宿主侧：lom build --target wasm（Rust 后端，tagged-i64 完整运行时）
          + eval/runner/run_wasm.mjs（宿主 harness）
  L2 侧： lom self_comp.lom -- <case> <hex>（宿主解释器跑 L2 编译器）
          + tools/hex2wasm.py（方案 A 宿主桩）
          + tools/selfcomp/run_selfcomp.mjs（L2 专用 harness）
  断言：两侧 stdout 逐字一致 + 退出码一致 + L2 编译输出 COMPILED 行。

负例集：tools/selfcomp/negative/ 下的子集外构造必须 COMPILE-ERROR
且不产 hex。L2.3-c 新增枚举/match 负例另断言拒绝原因片段，防止错误
路径偶然产出同一个 COMPILE-ERROR 也被误判为校验已覆盖。

用法：python tools/verify_selfcomp.py [--lom-bin PATH]
"""
import os
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CASES = os.path.join(ROOT, 'tools', 'selfcomp', 'cases')
NEGATIVES = os.path.join(ROOT, 'tools', 'selfcomp', 'negative')
SELF_COMP = os.path.join(ROOT, 'examples', 'selfhost', 'self_comp.lom')

EXPECTED_NEGATIVE_MESSAGES = {
    'neg_for_block_let_leak.lom': "未定义变量 'y'",
    'neg_if_block_let_leak.lom': "未定义变量 'y'",
    'neg_if_sibling_let_leak.lom': "未定义变量 'y'",
    'neg_builtin_none_pattern.lom': "内建 Result/Option 模式留后续子批（'None'）",
    'neg_builtin_result_type.lom': '泛型 enum/Result/Option 留后续批次',
    'neg_builtin_some.lom': "内建 Result/Option 变体留后续子批（'Some'）",
    'neg_closure_assign_signature.lom': '闭包签名不符',
    'neg_enum_assign_type.lom': "赋值 'e' 类型不符",
    'neg_enum_builtin_type_name.lom': '枚举名与内建类型冲突',
    'neg_enum_ctor_arity.lom': "变体 'Both' 实参数不符",
    'neg_enum_ctor_type.lom': "变体 'V' 第 1 参类型不符",
    'neg_enum_duplicate_variant.lom': '重复变体名',
    'neg_enum_eq.lom': '闭包/枚举值参与比较',
    'neg_enum_generic.lom': '泛型 enum 留后续批次',
    'neg_enum_pattern_arity.lom': '子模式数不符',
    'neg_enum_pattern_unknown.lom': "未知变体模式 'Missing'",
    'neg_enum_pattern_wrong_type.lom': "无参变体模式 'B1' 与被测类型",
    'neg_enum_print.lom': 'println 只接受 Int/Float',
    'neg_enum_unit_payload.lom': '变体载荷不能为 Unit',
    'neg_enum_unknown_type.lom': "未知或子集外枚举类型 'Ghost'",
    'neg_match_arm_type.lom': 'match 臂值类型不一致',
    'neg_match_binder_outer_leak.lom': "未定义变量 'x'",
    'neg_match_binder_sibling_leak.lom': "未定义变量 'x'",
    'neg_match_block_let_leak.lom': "未定义变量 'z'",
    'neg_match_guard_type.lom': 'match guard 须为 Bool',
    'neg_match_no_arms.lom': 'match 至少需要一个臂',
    'neg_match_string_pattern.lom': 'String 字面量模式留 String 批',
    'neg_variant_shadow_call.lom': '调用非闭包值',
    'neg_while_block_let_leak.lom': "未定义变量 'y'",
}


def run(cmd, **kw):
    # Windows 的子 Python（hex2wasm.py）向 pipe 写中文时默认走系统代码页；
    # 父进程明确按 UTF-8 解码，故统一固定子进程输出编码。
    kw.setdefault('env', {**os.environ, 'PYTHONIOENCODING': 'utf-8'})
    return subprocess.run(cmd, capture_output=True, text=True,
                          encoding='utf-8', timeout=600, **kw)


def main():
    args = sys.argv[1:]
    lom = args[args.index('--lom-bin') + 1] if '--lom-bin' in args else \
        os.path.join(ROOT, 'target', 'release', 'lom.exe')
    if not os.path.exists(lom):
        lom = os.path.join(ROOT, 'target', 'release', 'lom')
    if not os.path.exists(lom):
        print('lom binary not found; cargo build --release first')
        return 2

    import glob
    cases = sorted(glob.glob(os.path.join(CASES, '*.lom')))
    if not cases:
        print('no cases found in', CASES)
        return 2

    ok = fail = 0
    with tempfile.TemporaryDirectory() as td:
        for case in cases:
            name = os.path.basename(case)
            # 宿主侧
            host_wasm = os.path.join(td, name + '.host.wasm')
            r = run([lom, 'build', case, '--target', 'wasm', '-o', host_wasm])
            if r.returncode != 0:
                print('FAIL %s: host build rc=%d %s' % (name, r.returncode, r.stderr[:200]))
                fail += 1
                continue
            rh = run(['node', os.path.join(ROOT, 'eval', 'runner', 'run_wasm.mjs'), host_wasm])
            # L2 侧：编译
            hex_out = os.path.join(td, name + '.l2.hex')
            rc = run([lom, SELF_COMP, '--', case, hex_out], cwd=ROOT)
            if 'COMPILED' not in rc.stdout:
                print('FAIL %s: L2 compile: %s' % (name, rc.stdout.strip()[:300]))
                fail += 1
                continue
            nbytes = int([l for l in rc.stdout.splitlines() if l.startswith('COMPILED')][0].split()[1])
            l2_wasm = os.path.join(td, name + '.l2.wasm')
            rg = run(['python', os.path.join(ROOT, 'tools', 'hex2wasm.py'), hex_out, l2_wasm])
            if rg.returncode != 0:
                print('FAIL %s: hex2wasm: %s' % (name, rg.stderr[:200]))
                fail += 1
                continue
            rl = run(['node', os.path.join(ROOT, 'tools', 'selfcomp', 'run_selfcomp.mjs'), l2_wasm])
            if rh.stdout == rl.stdout and rh.returncode == rl.returncode:
                ok += 1
                print('PASS %-22s %d bytes, stdout %d lines, rc=%d' %
                      (name, nbytes, len(rl.stdout.splitlines()), rl.returncode))
            else:
                fail += 1
                print('FAIL %s: 行为不一致\n  host: %r\n  L2:   %r\n  rc host=%d L2=%d' %
                      (name, rh.stdout[:300], rl.stdout[:300], rh.returncode, rl.returncode))

        # 负例集：子集外构造必须 COMPILE-ERROR 且不产 hex（R66-R68 回归网）
        negatives = sorted(glob.glob(os.path.join(NEGATIVES, '*.lom')))
        for case in negatives:
            name = os.path.basename(case)
            hex_out = os.path.join(td, name + '.neg.hex')
            rc = run([lom, SELF_COMP, '--', case, hex_out], cwd=ROOT)
            if 'COMPILE-ERROR' not in rc.stdout:
                print('FAIL-NEG %s: 期望 COMPILE-ERROR，实际: %s' % (name, rc.stdout.strip()[:200]))
                fail += 1
                continue
            expected = EXPECTED_NEGATIVE_MESSAGES.get(name)
            if expected and expected not in rc.stdout:
                print('FAIL-NEG %s: 拒绝原因应含 %r，实际: %s' %
                      (name, expected, rc.stdout.strip()[:300]))
                fail += 1
                continue
            if os.path.exists(hex_out):
                print('FAIL-NEG %s: 拒绝输入仍产出 hex' % name)
                fail += 1
                continue
            ok += 1
            print('PASS-NEG %-24s COMPILE-ERROR, no hex' % name)

    print('RESULT: %s（%d/%d 项通过：正例对拍 + 负例拒绝）' %
          ('PASS' if fail == 0 else 'FAIL', ok, ok + fail))
    return 0 if fail == 0 else 1


if __name__ == '__main__':
    sys.exit(main())
