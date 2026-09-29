#!/usr/bin/env python3
"""verify_selfcomp.py —— L2 自举编译器逐批验收（RFC-0004 L2.2/L2.3）。

对拍口径（RFC-0004 预设：行为级一致，不做字节级一致——编译器实现自由度）：
  宿主侧：lom build --target wasm（Rust 后端，tagged-i64 完整运行时）
          + eval/runner/run_wasm.mjs（宿主 harness）
  L2 侧： lom self_comp.lom -- <case> <hex>（宿主解释器跑 L2 编译器）
          + tools/hex2wasm.py（方案 A 宿主桩）
          + tools/selfcomp/run_selfcomp.mjs（L2 专用 harness）
  断言：两侧 stdout 逐字一致 + 退出码一致 + L2 编译输出 COMPILED 行。

包项目（L2.3 包批，tools/selfcomp/pkg_cases/）：宿主侧照旧 lom build；
L2 侧先 `lom pkg-expand <main.lom> --list` 取包名清单、`lom pkg-expand`
取展开单元，再 `self_comp.lom -- <展开单元> <hex> <包名逗号串>`（裁决 3 甲）。

负例集：tools/selfcomp/negative/ 下的子集外构造必须 COMPILE-ERROR
且不产 hex。L2.3-c 枚举/match/泛型负例另断言拒绝原因片段，防止错误
路径偶然产出同一个 COMPILE-ERROR 也被误判为校验已覆盖。

L2.3 record/tuple 批（designs/0010）：cases 116-126（record/tuple 编译、
file/env/math/io 内建）+ pkg_cases 105_pkg_alias_in_pkg（R90 包源内 as
别名 pkg-expand 保留）+ 负例 12（校验 #1-#7 与附面）；122_env_args 经
CASE_ARGS 向两侧 harness 透传 argv。

用法：python tools/verify_selfcomp.py [--lom-bin PATH]
      python tools/verify_selfcomp.py --bootstrap [--ci-smoke] [--lom-bin PATH]

--bootstrap（L2.4 自举闭环，designs/0010 §6，结构借 verify_selfhost.py --wasm）：
  ① 自举层：宿主跑 self_comp 编译自身 → hex2wasm → wasm self_comp 再编译
     最小用例（COMPILED + hex 非空 + 与宿主产 hex 逐字节对拍）；
  ② 三层对拍：wasm self_comp 编译代表子集（各批核心 11 例 + 1 pkg 展开
     单元带第三参）→ 产物 hex 与宿主跑 self_comp 产的 hex 逐字节对拍
     （确定性编译器应一致；不一致降级行为级并登记差异）→ hex2wasm →
     node 跑 → stdout+rc 与宿主 lom build 产物对拍；
  ③ 自施加（加分项）：wasm self_comp 编译 self_comp.lom 源码 → hex 与
     ① 的 selfcomp.hex 逐字节对拍（一致 = 强 quine 证明）。
  --ci-smoke 子档：仅 ① + ② 的 1 个最小用例（预算 <300s，CI 冒烟用；
  是否接入 CI 由规划者定）。
"""
import os
import subprocess
import sys
import tempfile
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CASES = os.path.join(ROOT, 'tools', 'selfcomp', 'cases')
PKG_CASES = os.path.join(ROOT, 'tools', 'selfcomp', 'pkg_cases')
NEGATIVES = os.path.join(ROOT, 'tools', 'selfcomp', 'negative')
SELF_COMP = os.path.join(ROOT, 'examples', 'selfhost', 'self_comp.lom')
HEX2WASM = os.path.join(ROOT, 'tools', 'hex2wasm.py')
RUN_SELFCOMP = os.path.join(ROOT, 'tools', 'selfcomp', 'run_selfcomp.mjs')
RUN_WASM = os.path.join(ROOT, 'eval', 'runner', 'run_wasm.mjs')

# L2.4 --bootstrap 代表子集（designs/0010 §6：各批核心形态各抽 1；
# 121 含 file 四件套真实落盘——与主循环同 cwd 语义）
BOOTSTRAP_SUBSET = [
    '01_arith_int.lom',        # L2.2 算术
    '06_if_stmt.lom',          # L2.3-a 控制流
    '17_closure_ctrlflow.lom', # L2.3-b 闭包
    '21_enum_guard.lom',       # L2.3-c 枚举 match（guard）
    '55_string_builtins.lom',  # String 批
    '67_list_map.lom',         # List 批 HOF
    '80_map_basic.lom',        # Map 批
    '91_json_roundtrip.lom',   # json 批
    '110_try_basic.lom',       # return/? 批
    '116_record_basic.lom',    # record/tuple 批
    '121_file_builtin.lom',    # file/env 批（真实 IO）
]
BOOTSTRAP_PKG = '101_pkg_single'  # pkg 展开 + 第三参包名透传

# L2.3 包批（designs/0008 §6.4）：需要向 self_comp 传第三参包名清单的负例
# （单文件模拟：import 命中包清单成员——否则会先命中"未知模块"拒绝）
NEG_PKGS = {
    'neg_pkg_missing_symbol.lom': 'mypkg',
    'neg_pkg_alias_arity.lom': 'mypkg',
}

# L2.3 record/tuple 批（designs/0010 §8）：对拍运行需要向两侧 harness 透传
# argv 的用例（env.args 消费——双侧同参，用例自身只消费非首元素：首元素
# 是各自 wasm 路径，打印会分叉）
CASE_ARGS = {
    '122_env_args.lom': ['alpha', 'beta'],
}

EXPECTED_NEGATIVE_MESSAGES = {
    # ---- L2.3 Map 批（designs/0006 §7.3 校验表 #1-#10 + 裁决 2/3 甲）----
    'neg_map_set_nonmap.lom': "调用 'map_set' 期望 Map 得 i64",
    'neg_map_key_type.lom': "第 2 参期望 String 键（得 i64）",
    'neg_map_val_mismatch.lom': "Map 值类型不符（期望 mp{i64} 得 f64）",
    'neg_map_annot_mismatch.lom': "let 注解类型 'mp{i64}' 与值类型 'i64' 不符",
    'neg_map_unknown_val.lom': '未知 Map 值类型——需注解或 set 上下文（map_get）',
    'neg_map_call_arity.lom': "调用 'map_set' 实数量不符",
    'neg_map_arith.lom': 'Map 参与算术',
    'neg_map_compare.lom': 'Map 大小比较',
    'neg_for_map.lom': 'for 迭代仅支持 Int/String/List',
    'neg_map_eq_enum_val.lom': 'Map 值相等比较暂不支持 枚举/闭包 值',
    'neg_map_remove_unit_bind.lom': 'let 绑定 void 值',
    # ---- L2.3 json 批（designs/0007 裁决 1 甲 + B1+B2 + 数字甲）----
    'neg_json_alias.lom': '暂不支持 as 别名',
    'neg_json_arith.lom': 'json 值参与算术/拼接',
    'neg_json_compare.lom': 'json 值参与比较',
    'neg_json_concat.lom': 'json 值参与算术/拼接',
    'neg_json_field_nonjs.lom': '该表达式形态',
    'neg_json_let_mismatch.lom': "let 注解类型 'js' 与值类型 'i64' 不符",
    'neg_json_map_consume.lom': '期望 Map 得 js',
    'neg_json_neg.lom': 'json 值参与一元负',
    'neg_json_parse_nonstr.lom': "第 1 参类型不符（期望 st 得 i64）",
    'neg_json_record_literal.lom': 'stringify 暂不支持 Record/Tuple 值',
    'neg_json_stringify_enum_val.lom': 'stringify 暂不支持 枚举/闭包 值',
    'neg_json_stringify_list_enum.lom': '暂不支持 枚举/闭包 值元素',
    'neg_json_stringify_unit.lom': 'json_stringify 实参不能为 Unit',
    'neg_json_unknown_builtin.lom': "未知内建 'json.json_dump'",
    # ---- R89（十六审 P3）：void 实参单列诊断（不再落容器显示兜底）----
    'neg_println_void.lom': 'println 实参求值为 void',
    # ---- L2.3 List 批（designs/0005 校验表 #1-#12 + 管道子集外登记）----
    # Map 批将 println/拼接提升的容器显示文案扩为 List/Map（neg_println_list
    # /neg_list_concat_promote 的锁定串随批更新）
    'neg_list_cons_elem_type.lom': "类型不符（'f64' vs 'i64'）",
    'neg_list_head_nonlist.lom': "调用 'list_head' 期望 List 得 i64",
    'neg_list_get_idx_type.lom': "调用 'list_get' 第 2 参类型不符",
    'neg_list_map_sig.lom': 'list_map 的 f 期望单参数闭包',
    'neg_list_fold_sig.lom': 'list_fold 的 f 期望双参数闭包',
    'neg_list_filter_pred_bool.lom': 'list_filter 谓词须返回 Bool',
    'neg_list_assign_elem.lom': "赋值 'xs' 类型不符（期望 ls{i64} 得 ls{f64}）",
    'neg_list_annot_mismatch.lom': "let 注解类型 'ls{i64}' 与值类型 'ls{f64}' 不符",
    'neg_list_arith.lom': 'List 参与算术',
    'neg_list_compare.lom': 'List 大小比较',
    'neg_list_unknown_elem.lom': '未知 List 元素类型——需注解或构造上下文（list_head）',
    'neg_range_not_int.lom': 'range 两端须为 Int',
    'neg_list_eq_enum_elem.lom': 'List 元素相等比较暂不支持 枚举/闭包 元素',
    'neg_pipe_syntax.lom': '该表达式形态',
    # ---- 第十五轮整改（R84/R85）----
    'neg_fold_unsupported_acc.lom': 'list_fold 累加器类型子集外',
    'neg_bool_param_flow.lom': '深层 Bool 流未被预扫覆盖',
    # ---- 此前批次 ----
    'neg_bool_arith.lom': 'Bool 参与算术',
    'neg_bool_mixed_compare.lom': 'Bool 与非 Bool 比较',
    'neg_bool_unary.lom': 'Bool 参与一元负',
    'neg_str_num_compare.lom': 'String 与非 String 比较',
    'neg_str_arith.lom': 'String 只参与 + 拼接',
    'neg_str_neg.lom': '闭包/枚举/String/Map 值参与一元负',
    'neg_str_annot_mismatch.lom': "let 注解类型 'i64' 与值类型 'st' 不符",
    'neg_str_call_value.lom': "调用非闭包值（得到 vt 'st'）",
    'neg_stoi_union.lom': 'untagged 表示下运行时不可区分',
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
    'neg_c2_generic_arity.lom': '类型参数数不符',
    'neg_c2_generic_conflict.lom': "变体 'Pair' 第 2 参类型不符",
    'neg_c2_generic_naked.lom': "泛型枚举类型 'Box' 期望 1 个类型参数",
    'neg_c2_generic_wrong_arg.lom': "调用 'take' 第 1 参类型不符",
    'neg_c2_nested_wrong_payload.lom': "调用 'take' 第 1 参类型不符",
    'neg_c2_nested_typevar_conflict.lom': "变体 'Both' 第 2 参类型不符",
    'neg_c2_none_arity.lom': "变体 'None' 实参数不符",
    'neg_c2_pattern_arity.lom': '子模式数不符',
    'neg_c2_pattern_wrong_enum.lom': "变体模式所属枚举 'Box' 与被测类型",
    'neg_c2_result_wrong_payload.lom': "调用 'take' 第 1 参类型不符",
    'neg_c2_recursive_wrong_payload.lom': "变体 'Node' 第 1 参类型不符",
    'neg_c2_some_arity.lom': "变体 'Some' 实参数不符",
    'neg_c2_type_param_builtin.lom': "类型参数名与内建类型冲突（'Int'）",
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
    # ---- L2.3 包批（designs/0008 §6.3 校验表 #1/#2/#3 + R74 别名口径）----
    'neg_pkg_unknown_module.lom': "未知模块 'ghost'",
    'neg_pkg_missing_symbol.lom': "包 'mypkg' 中无公开符号 'missing_fn'",
    'neg_pkg_enum_clash.lom': "跨包/主文件重名——宿主静默取首个定义，L2 明确拒绝；请重命名",
    'neg_pkg_alias_arity.lom': "调用 'aliased_fn' 实数量不符",
    # ---- L2.3 return 收官批（designs/0009 §6.3 校验表 #1-#4 + 裁决 3 甲）----
    # neg_return_stmt/neg_c2_try_deferred 本批转正删除（Git 历史可恢复）
    'neg_try_non_result.lom': "? 只能用于 Result/Option（得到 'i64'）",
    'neg_try_unit_operand.lom': '? 只能用于 Result/Option',
    'neg_try_ctx_mismatch.lom': '? 的 Err 载荷类型与所在函数返回类型不符',
    'neg_try_in_void_fn.lom': "? 所在函数返回类型须为 Result/Option（得到 'void'）",
    'neg_return_type_mismatch.lom': "return 值类型不符（期望 i64 得 st）",
    'neg_return_void_with_value.lom': 'void 函数的 return 不能带值',
    'neg_return_value_in_void_fn.lom': 'void 函数的 return 不能带值',
    'neg_return_missing_value.lom': "return 无值但函数返回 'i64'",
    # ---- L2.3 record/tuple 批（designs/0010 §7 校验 #1-#7 + 附面）----
    'neg_record_field_mismatch.lom': "let 注解类型 'rc{a:i64;b:i64}' 与值类型 'rc{a:i64;c:i64}' 不符",
    'neg_record_unknown_field.lom': "record 无字段 'y'（类型 rc{",
    'neg_tuple_oob.lom': "tuple 索引 2 越界（tp{",
    'neg_record_arith.lom': 'Record/Tuple 参与算术',
    'neg_record_compare.lom': 'Record/Tuple 相等与大小比较',
    'neg_file_arity.lom': "调用 'file_read' 实数量不符",
    'neg_tuple_destructure.lom': '该语句形态（match/解构留后续批次）',
    'neg_record_order.lom': "let 注解类型 'rc{x:i64;y:i64}' 与值类型 'rc{y:i64;x:i64}' 不符",
    'neg_record_neg.lom': 'Record/Tuple 值参与一元负',
    'neg_math_mixed.lom': "调用 'min' 期望同型 Int/Float 对",
    # ---- L2.3 容器显示批（designs/0011 §9）：9 枚预留负例转正删除；
    # 深层容器流（跨函数参数流转，预扫 scan_has_display 之外）锁 ibase
    # 兜底新文案（双防线乙；Map 深层流无形态——键类型 String 恒开设施）----
    'neg_println_deep_list.lom': 'println(容器值) 需要 String 设施（data/print_str）——深层容器流未被预扫覆盖',
    'neg_println_deep_rc.lom': 'println(容器值) 需要 String 设施（data/print_str）——深层容器流未被预扫覆盖',
    'neg_println_deep_enum.lom': 'println(容器值) 需要 String 设施（data/print_str）——深层容器流未被预扫覆盖',
    'neg_println_deep_enum_ls.lom': 'println(容器值) 需要 String 设施（data/print_str）——深层容器流未被预扫覆盖',
    # ---- R95 B甲：全终止路径转正；留下精确签名/可达路径负例 ----
    'neg_r95_closure_mixed_returns.lom': "类型不符（'i64' vs 'f64'）",
    'neg_r95_closure_param_leak.lom': "未定义变量 'secret'",
    'neg_r95_closure_partial_return.lom': 'void 函数的 return 不能带值',
    'neg_r95_closure_unknown_list_ret.lom': '全终止闭包返回类型不可精确推断',
    'neg_r95_closure_try_mixed_return.lom': '类型不符',
    'neg_r95_if_no_else_value.lom': 'let 绑定 void 值',
}


def run(cmd, **kw):
    # Windows 的子 Python（hex2wasm.py）向 pipe 写中文时默认走系统代码页；
    # 父进程明确按 UTF-8 解码，故统一固定子进程输出编码。
    kw.setdefault('env', {**os.environ, 'PYTHONIOENCODING': 'utf-8'})
    kw.setdefault('timeout', 600)
    return subprocess.run(cmd, capture_output=True, text=True,
                          encoding='utf-8', **kw)


def _read(path):
    with open(path, 'rb') as f:
        return f.read()


def run_bootstrap(lom, ci_smoke=False):
    """L2.4 自举闭环三层自证（designs/0010 §6）。返回 0/1。"""
    t_all = time.time()
    ok = fail = 0
    node_base = ['node', '--stack-size=60000', RUN_SELFCOMP]

    def note(passed, label, extra=''):
        nonlocal ok, fail
        if passed:
            ok += 1
            print('PASS-B %-26s %s' % (label, extra))
        else:
            fail += 1
            print('FAIL-B %-26s %s' % (label, extra))

    with tempfile.TemporaryDirectory() as td:
        # ---------- ① 自举层 ----------
        t1 = time.time()
        self_hex = os.path.join(td, 'selfcomp.hex')
        r = run([lom, SELF_COMP, '--', SELF_COMP, self_hex], cwd=ROOT, timeout=1200)
        if 'COMPILED' not in r.stdout or not os.path.exists(self_hex) or os.path.getsize(self_hex) == 0:
            note(False, 'L1 host self-compile', r.stdout.strip()[:200])
            print('RESULT: FAIL（自举层未通过：%d/%d）' % (ok, ok + fail))
            return 1
        nbytes = int([l for l in r.stdout.splitlines() if l.startswith('COMPILED')][0].split()[1])
        self_wasm = os.path.join(td, 'selfcomp.wasm')
        rg = run(['python', HEX2WASM, self_hex, self_wasm])
        if rg.returncode != 0:
            note(False, 'L1 hex2wasm', rg.stderr[:200])
            return 1
        t_host = time.time() - t1

        # wasm self_comp 编译最小用例（层 1 完成判定）+ 与宿主产 hex 逐字节对拍
        smoke_case = os.path.join(CASES, '01_arith_int.lom')
        w_smoke_hex = os.path.join(td, 'w01.hex')
        h_smoke_hex = os.path.join(td, 'h01.hex')
        rh = run([lom, SELF_COMP, '--', smoke_case, h_smoke_hex], cwd=ROOT)
        rw = run(node_base + [self_wasm, smoke_case, w_smoke_hex], timeout=900)
        t_wasm_small = None
        passed = ('COMPILED' in rw.stdout and os.path.exists(w_smoke_hex)
                  and os.path.getsize(w_smoke_hex) > 0)
        ident = passed and _read(w_smoke_hex) == _read(h_smoke_hex)
        if passed and not ident:
            print('  [登记] wasm 产 hex 与宿主产 hex 不一致——降级行为级（层 2 对拍以行为判定）')
        note(passed, 'L1 wasm compiles case',
             '%d bytes; hex %s; host-self %.1fs' %
             (os.path.getsize(w_smoke_hex) // 2 if passed else 0,
              'byte-identical' if ident else 'DIFFERS', t_host))

        # ---------- ② 三层对拍（代表子集） ----------
        t2 = time.time()
        subset = ['01_arith_int.lom'] if ci_smoke else BOOTSTRAP_SUBSET
        for name in subset:
            case = os.path.join(CASES, name)
            # 宿主产 hex（解释器跑 self_comp）
            h_hex = os.path.join(td, name + '.host.hex')
            rh = run([lom, SELF_COMP, '--', case, h_hex], cwd=ROOT)
            if 'COMPILED' not in rh.stdout:
                note(False, 'L2 %s host-hex' % name, rh.stdout.strip()[:150])
                continue
            # wasm self_comp 产 hex
            w_hex = os.path.join(td, name + '.wasm.hex')
            rw = run(node_base + [self_wasm, case, w_hex], timeout=900)
            if 'COMPILED' not in rw.stdout:
                note(False, 'L2 %s wasm-compile' % name, rw.stdout.strip()[:150])
                continue
            ident = _read(w_hex) == _read(h_hex)
            # 行为级：宿主 lom build 产物 vs wasm 产物的运行对拍
            host_wasm = os.path.join(td, name + '.host.wasm')
            rb = run([lom, 'build', case, '--target', 'wasm', '-o', host_wasm])
            if rb.returncode != 0:
                note(False, 'L2 %s host build' % name, rb.stderr[:150])
                continue
            rh_run = run(['node', RUN_WASM, host_wasm])
            l3_wasm = os.path.join(td, name + '.l3.wasm')
            rg = run(['python', HEX2WASM, w_hex, l3_wasm])
            if rg.returncode != 0:
                note(False, 'L2 %s hex2wasm' % name, rg.stderr[:150])
                continue
            rl_run = run(['node', RUN_SELFCOMP, l3_wasm])
            beh = (rh_run.stdout == rl_run.stdout and rh_run.returncode == rl_run.returncode)
            note(ident and beh, 'L2 %s' % name,
                 'hex %s; behavior %s (stdout %d lines, rc=%d)' %
                 ('identical' if ident else 'DIFFERS',
                  'match' if beh else 'MISMATCH',
                  len(rl_run.stdout.splitlines()), rl_run.returncode))
        t_subset = time.time() - t2

        # pkg 展开单元 + 第三参（仅全量档）
        if not ci_smoke:
            case_dir = os.path.join(PKG_CASES, BOOTSTRAP_PKG)
            main_lom = os.path.join(case_dir, 'main.lom')
            rl = run([lom, 'pkg-expand', main_lom, '--list'])
            pkgs = rl.stdout.strip()
            ru = run([lom, 'pkg-expand', main_lom])
            expanded = os.path.join(td, BOOTSTRAP_PKG + '.expanded.lom')
            with open(expanded, 'w', encoding='utf-8', newline='') as f:
                f.write(ru.stdout)
            h_hex = os.path.join(td, BOOTSTRAP_PKG + '.host.hex')
            w_hex = os.path.join(td, BOOTSTRAP_PKG + '.wasm.hex')
            rh = run([lom, SELF_COMP, '--', expanded, h_hex] + ([pkgs] if pkgs else []), cwd=ROOT)
            rw = run(node_base + [self_wasm, expanded, w_hex] + ([pkgs] if pkgs else []), timeout=900)
            okp = ('COMPILED' in rh.stdout and 'COMPILED' in rw.stdout)
            ident = okp and _read(w_hex) == _read(h_hex)
            note(okp and ident, 'L2-PKG %s' % BOOTSTRAP_PKG,
                 'pkgs=%s; hex %s' % (pkgs, 'identical' if ident else 'DIFFERS'))

        # ---------- ③ 自施加（加分项；全量档） ----------
        t3s = time.time()
        if not ci_smoke:
            quine_hex = os.path.join(td, 'selfcomp.quine.hex')
            rq = run(node_base + [self_wasm, SELF_COMP, quine_hex], timeout=1200)
            if 'COMPILED' not in rq.stdout or not os.path.exists(quine_hex):
                note(False, 'L3 self-apply', rq.stdout.strip()[:150])
            else:
                ident = _read(quine_hex) == _read(self_hex)
                note(ident, 'L3 self-apply (quine)',
                     '%d bytes; %s; %.1fs' % (os.path.getsize(quine_hex) // 2,
                                              'byte-identical' if ident else 'DIFFERS',
                                              time.time() - t3s))

    print('RESULT: %s（--bootstrap%s：%d/%d 项通过；总耗时 %.1fs）' %
          ('PASS' if fail == 0 else 'FAIL', ' --ci-smoke' if ci_smoke else '',
           ok, ok + fail, time.time() - t_all))
    return 0 if fail == 0 else 1


def main():
    args = sys.argv[1:]
    lom = args[args.index('--lom-bin') + 1] if '--lom-bin' in args else \
        os.path.join(ROOT, 'target', 'release', 'lom.exe')
    if not os.path.exists(lom):
        lom = os.path.join(ROOT, 'target', 'release', 'lom')
    if not os.path.exists(lom):
        print('lom binary not found; cargo build --release first')
        return 2
    if '--bootstrap' in args:
        return run_bootstrap(lom, ci_smoke=('--ci-smoke' in args))

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
            rh = run(['node', os.path.join(ROOT, 'eval', 'runner', 'run_wasm.mjs'), host_wasm]
                     + CASE_ARGS.get(name, []))
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
            rl = run(['node', os.path.join(ROOT, 'tools', 'selfcomp', 'run_selfcomp.mjs'), l2_wasm]
                     + CASE_ARGS.get(name, []))
            if rh.stdout == rl.stdout and rh.returncode == rl.returncode:
                ok += 1
                print('PASS %-22s %d bytes, stdout %d lines, rc=%d' %
                      (name, nbytes, len(rl.stdout.splitlines()), rl.returncode))
            else:
                fail += 1
                print('FAIL %s: 行为不一致\n  host: %r\n  L2:   %r\n  rc host=%d L2=%d' %
                      (name, rh.stdout[:300], rl.stdout[:300], rh.returncode, rl.returncode))

        # L2.3 包批（designs/0008 §6.4，裁决 1 丙+丁 / 裁决 3 甲）：包项目对拍。
        # 宿主侧照旧 lom build（包合并编译）；L2 侧先 pkg-expand 取包名清单
        # 与展开单元（两侧编译单元同为宿主包系统的合并产物——等价性由宿主
        # Rust 代码单点保证），展开单元 + 第三参包名清单喂 self_comp。
        pkg_dirs = sorted(d for d in glob.glob(os.path.join(PKG_CASES, '*'))
                          if os.path.isdir(d))
        for case_dir in pkg_dirs:
            name = os.path.basename(case_dir)
            main_lom = os.path.join(case_dir, 'main.lom')
            # 宿主侧
            host_wasm = os.path.join(td, name + '.host.wasm')
            r = run([lom, 'build', main_lom, '--target', 'wasm', '-o', host_wasm])
            if r.returncode != 0:
                print('FAIL %s: host build rc=%d %s' % (name, r.returncode, r.stderr[:200]))
                fail += 1
                continue
            rh = run(['node', os.path.join(ROOT, 'eval', 'runner', 'run_wasm.mjs'), host_wasm])
            # 包名清单（--list 单独出口）+ 展开单元（stdout 纯产物）
            rl = run([lom, 'pkg-expand', main_lom, '--list'])
            if rl.returncode != 0:
                print('FAIL %s: pkg-expand --list rc=%d %s' % (name, rl.returncode, rl.stderr[:200]))
                fail += 1
                continue
            pkgs = rl.stdout.strip()
            ru = run([lom, 'pkg-expand', main_lom])
            if ru.returncode != 0:
                print('FAIL %s: pkg-expand rc=%d %s' % (name, ru.returncode, ru.stderr[:200]))
                fail += 1
                continue
            expanded = os.path.join(td, name + '.expanded.lom')
            # newline=''：保持 stdout 的 LF 原样落盘（不触发 Windows CRLF 转换）
            with open(expanded, 'w', encoding='utf-8', newline='') as f:
                f.write(ru.stdout)
            # L2 侧：编译（第三参包名逗号清单——缺省不传，保持单文件 argv 形态）
            hex_out = os.path.join(td, name + '.l2.hex')
            cmd = [lom, SELF_COMP, '--', expanded, hex_out]
            if pkgs:
                cmd.append(pkgs)
            rc = run(cmd, cwd=ROOT)
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
            rl2 = run(['node', os.path.join(ROOT, 'tools', 'selfcomp', 'run_selfcomp.mjs'), l2_wasm])
            if rh.stdout == rl2.stdout and rh.returncode == rl2.returncode:
                ok += 1
                print('PASS-PKG %-22s pkgs=%s, %d bytes, stdout %d lines, rc=%d' %
                      (name, pkgs, nbytes, len(rl2.stdout.splitlines()), rl2.returncode))
            else:
                fail += 1
                print('FAIL %s: 行为不一致\n  host: %r\n  L2:   %r\n  rc host=%d L2=%d' %
                      (name, rh.stdout[:300], rl2.stdout[:300], rh.returncode, rl2.returncode))

        # 负例集：子集外构造必须 COMPILE-ERROR 且不产 hex（R66-R68 回归网）
        negatives = sorted(glob.glob(os.path.join(NEGATIVES, '*.lom')))
        for case in negatives:
            name = os.path.basename(case)
            hex_out = os.path.join(td, name + '.neg.hex')
            cmd = [lom, SELF_COMP, '--', case, hex_out]
            # L2.3 包批：带包名清单的负例（单文件模拟包 import 命中清单）
            if name in NEG_PKGS:
                cmd.append(NEG_PKGS[name])
            rc = run(cmd, cwd=ROOT)
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
