#!/usr/bin/env python3
"""verify_selfcomp.py —— L2.2 最小子集编译器验收（RFC-0004 L2.2）。

对拍口径（RFC-0004 预设：行为级一致，不做字节级一致——编译器实现自由度）：
  宿主侧：lom build --target wasm（Rust 后端，tagged-i64 完整运行时）
          + eval/runner/run_wasm.mjs（宿主 harness）
  L2 侧： lom self_comp.lom -- <case> <hex>（宿主解释器跑 L2 编译器）
          + tools/hex2wasm.py（方案 A 宿主桩）
          + tools/selfcomp/run_selfcomp.mjs（L2 专用 harness）
  断言：两侧 stdout 逐字一致 + 退出码一致 + L2 编译输出 COMPILED 行。

负例集（R66-R68/十一审）：tools/selfcomp/negative/ 下的子集外构造
（if/while/for/return 语句、闭包、比较、String、Float %、let 注解不符、
match、缺 main、未知函数、嵌套 println）必须 COMPILE-ERROR 且不产 hex
——语句级拒绝曾是死臂（Err 值在语句位置被丢弃，R66），负例集是它的
回归网。

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


def run(cmd, **kw):
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
