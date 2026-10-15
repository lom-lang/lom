#!/usr/bin/env python3
"""build_selfboot.py —— playground 二期自举演示分发物构建（designs/0024 B-only）。

构建三件分发物（部署时构建快照：随仓库 self_comp.lom 演进需重建；构建脚本
产物，不手工维护）：
  selfcomp.wasm          L2 编译器产物（宿主自编译产 hex 文本，经 tools/hex2wasm.py 解码）
  self_comp.lom          编译器源码副本（examples/selfhost/ 同构副本）
  selfboot-baseline.hex  预置基准（= 本次宿主自编译产出的 hex 文本，quine 比对目标）

产物链口径（对齐 tools/verify_selfcomp.py --bootstrap 的 L1 段）：
  宿主 lom 跑 self_comp.lom 编译自身 -> selfcomp.hex（stdout 含 "COMPILED N bytes" 行）
  -> tools/hex2wasm.py 解码 -> selfcomp.wasm

用法：
    python tools/build_selfboot.py [--lom-bin PATH] [--out DIR]

默认：--lom-bin target/release/lom(.exe)；--out playground/dist-selfboot/
（相对仓库根解析；gitignored 不入库）。
"""
import argparse
import os
import shutil
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SELF_COMP = os.path.join(ROOT, 'examples', 'selfhost', 'self_comp.lom')
HEX2WASM = os.path.join(ROOT, 'tools', 'hex2wasm.py')
QUINE_BYTES = 272925  # 仓库现行基线（README / verify_selfcomp 一致）


def run(cmd, **kw):
    # 对齐 verify_selfcomp.run：子进程输出统一按 UTF-8 解码
    # （Windows 子 Python 向 pipe 写中文时默认走系统代码页）
    kw.setdefault('env', {**os.environ, 'PYTHONIOENCODING': 'utf-8'})
    kw.setdefault('timeout', 1800)
    return subprocess.run(cmd, capture_output=True, text=True,
                          encoding='utf-8', **kw)


def resolve_lom(explicit):
    if explicit:
        return explicit if os.path.exists(explicit) else None
    for cand in (os.path.join(ROOT, 'target', 'release', 'lom.exe'),
                 os.path.join(ROOT, 'target', 'release', 'lom')):
        if os.path.exists(cand):
            return cand
    return None


def main() -> int:
    ap = argparse.ArgumentParser(
        description='playground self-hosting demo 分发物构建（designs/0024 B-only）')
    ap.add_argument('--lom-bin', default=None,
                    help='宿主 lom 二进制（默认 target/release/lom[.exe]）')
    ap.add_argument('--out', default=os.path.join('playground', 'dist-selfboot'),
                    help='输出目录（默认 playground/dist-selfboot/，相对仓库根，gitignored）')
    args = ap.parse_args()

    lom = resolve_lom(args.lom_bin)
    if not lom:
        print('lom binary not found; cargo build --release first (或传 --lom-bin)')
        return 2
    if not os.path.exists(SELF_COMP):
        print('self_comp.lom not found:', SELF_COMP)
        return 2

    outdir = args.out if os.path.isabs(args.out) else os.path.join(ROOT, args.out)
    os.makedirs(outdir, exist_ok=True)
    hex_path = os.path.join(outdir, 'selfcomp.hex')
    wasm_path = os.path.join(outdir, 'selfcomp.wasm')
    baseline_path = os.path.join(outdir, 'selfboot-baseline.hex')
    src_out = os.path.join(outdir, 'self_comp.lom')

    # ① 宿主自编译：lom self_comp.lom -- self_comp.lom <out.hex>
    r = run([lom, SELF_COMP, '--', SELF_COMP, hex_path], cwd=ROOT)
    if 'COMPILED' not in r.stdout or not os.path.exists(hex_path) or os.path.getsize(hex_path) == 0:
        print('宿主自编译失败：\n' + r.stdout.strip()[:400] + '\n' + r.stderr.strip()[:400])
        return 1
    nbytes = int([l for l in r.stdout.splitlines() if l.startswith('COMPILED')][0].split()[1])
    if nbytes != QUINE_BYTES:
        # 分发物漂移防御：与仓库基线不符即失败——链漂移需先有意识地重锚
        # 基线（README/文档/断言同步），不由本脚本静默跟随
        print('COMPILED %d bytes != 仓库基线 %d —— self_comp.lom 或编译器已漂移，'
              '请先同步 README/docs 基线再重建分发物' % (nbytes, QUINE_BYTES))
        return 1

    # ② hex -> wasm（与 verify_selfcomp 同一解码桩；sys.executable 保证同解释器）
    rg = run([sys.executable, HEX2WASM, hex_path, wasm_path])
    if rg.returncode != 0:
        print('hex2wasm 失败：\n' + rg.stderr.strip()[:400])
        return 1
    if os.path.getsize(wasm_path) != nbytes:
        print('解码后 wasm 尺寸 %d != COMPILED %d' % (os.path.getsize(wasm_path), nbytes))
        return 1

    # ③ 基准与源码副本（基准 = 本次宿主自编译的 hex 原文）
    shutil.copyfile(hex_path, baseline_path)
    shutil.copyfile(SELF_COMP, src_out)

    print('build_selfboot: COMPILED %d bytes' % nbytes)
    print('  %s (%d bytes)' % (wasm_path, os.path.getsize(wasm_path)))
    print('  %s (%d chars)' % (baseline_path, os.path.getsize(baseline_path)))
    print('  %s (%d bytes)' % (src_out, os.path.getsize(src_out)))
    print('  (中间产物) %s' % hex_path)
    return 0


if __name__ == '__main__':
    sys.exit(main())
