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
且不产 hex。L2.3-c 枚举/match/泛型负例另断言拒绝原因片段，防止错误
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
    'neg_bool_arith.lom': 'Bool 参与算术',
    'neg_bool_mixed_compare.lom': 'Bool 与非 Bool 比较',
    'neg_bool_unary.lom': 'Bool 参与一元负',
    'neg_str_num_compare.lom': 'String 与非 String 比较',
    'neg_str_arith.lom': 'String 只参与 + 拼接',
    'neg_str_neg.lom': 'String 值参与一元负',
    'neg_str_annot_mismatch.lom': "let 注解类型 'i64' 与值类型 'st' 不符",
    'neg_str_call_value.lom': "调用非闭包值（得到 vt 'st'）",
    'neg_stoi_union.lom': 'untagged 表示下运行时不可区分',
    'neg_split_list.lom': '随 List 批交付',
    'neg_str_builtin_arity.lom': "调用 'contains' 实数量不符",
    'neg_str_builtin_type.lom': "调用 'len' 第 1 参类型不符（期望 st 得 i64）",
    'neg_str_builtin_not_imported.lom': "未定义变量 'len'",
    'neg_match_num_str_pattern.lom': '字面量模式类型不符（被测 i64 模式 st）',
    'neg_builtin_import_clash.lom': '与用户函数/重复导入同名',
    'neg_str_condition.lom': 'if 条件须为 Bool',
    'neg_str_assign_mismatch.lom': "赋值 'n' 类型不符",
    'neg_for_block_let_leak.lom': "未定义变量 'y'",
    'neg_if_block_let_leak.lom': "未定义变量 'y'",
    'neg_if_branch_local_outer_leak.lom': "未定义变量 'x'",
    'neg_if_branch_local_sibling_leak.lom': "未定义变量 'x'",
    'neg_if_sibling_let_leak.lom': "未定义变量 'y'",
    'neg_builtin_none_pattern.lom': "无参变体模式 'None' 与被测类型 'i64' 不符",
    'neg_c2_assign_wrong_instance.lom': "赋值 'x' 类型不符",
    'neg_c2_duplicate_param.lom': "重复类型参数 'T'",
    'neg_c2_generic_arity.lom': "类型参数数不符",
    'neg_c2_generic_conflict.lom': "变体 'Pair' 第 2 参类型不符",
    'neg_c2_generic_naked.lom': "泛型枚举类型 'Box' 期望 1 个类型参数",
    'neg_c2_generic_wrong_arg.lom': "调用 'take' 第 1 参类型不符",
    'neg_c2_nested_wrong_payload.lom': "调用 'take' 第 1 参类型不符",
    'neg_c2_nested_typevar_conflict.lom': "变体 'Both' 第 2 参类型不符",
    'neg_c2_none_arity.lom': "变体 'None' 实参数不符",
    'neg_c2_pattern_arity.lom': "子模式数不符",
    'neg_c2_pattern_wrong_enum.lom': "变体模式所属枚举 'Box' 与被测类型",
    'neg_c2_result_wrong_payload.lom': "调用 'take' 第 1 参类型不符",
    'neg_c2_recursive_wrong_payload.lom': "变体 'Node' 第 1 参类型不符",
    'neg_c2_print_generic.lom': 'println 只接受 Int/Float',
    'neg_c2_some_arity.lom': "变体 'Some' 实参数不符",
    'neg_c2_type_param_builtin.lom': "类型参数名与内建类型冲突（'Int'）",
    'neg_c2_try_deferred.lom': "? 提前返回留 return 批",
    'neg_c2_unbound_field_type.lom': "未知或子集外枚举类型 'U'",
    'neg_c2_unit_type_arg.lom': "Option 类型参数暂不支持 Unit/Fn",
    'neg_c2_unknown_payload_pattern.lom': "模式载荷类型不可推断",
    'neg_c2_unknown_type_arg.lom': "未知或子集外枚举类型 'Ghost'",
    'neg_closure_assign_signature.lom': '闭包签名不符',
    'neg_enum_assign_type.lom': "赋值 'e' 类型不符",
    'neg_enum_builtin_type_name.lom': '枚举名与内建类型冲突',
    'neg_enum_ctor_arity.lom': "变体 'Both' 实参数不符",
    'neg_enum_ctor_type.lom': "变体 'V' 第 1 参类型不符",
    'neg_enum_duplicate_variant.lom': '重复变体名',
    'neg_enum_eq.lom': '闭包/枚举值参与比较',
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
    'neg_match_if_local_outer_leak.lom': "未定义变量 'z'",
    'neg_match_no_arms.lom': 'match 至少需要一个臂',
    'neg_match_string_pattern.lom': '字面量模式类型不符（被测 i64 模式 st）',
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
