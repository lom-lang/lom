# tools/fuzz_smoke.py — 随机输入冲击冒烟（Q4 常态化，2026-09-08）
#
# 背景：第三轮审查（2026-09-03）做过一次性 80 轮随机 UTF-8 垃圾冲击
# （total parser 声称：0 崩溃）——本工具把该方法固化为可重复运行。
# 断言的口径：--check 对任意输入要么正常完成（exit 0/1），要么输出诊断；
# 崩溃（Windows 异常码 0xC0000005 访问违例 / 0xC00000FD 栈溢出，或 Rust
# panic 的 101）= FAIL。Q3 落地后深嵌套输入也应产结构化诊断（守卫路径）。
#
# 用法：
#   python tools/fuzz_smoke.py                  # 默认 80 个固定种子
#   python tools/fuzz_smoke.py --rounds 200     # 更多轮次
#   python tools/fuzz_smoke.py --lom <path>     # 指定二进制
#
# 种子固定（range(rounds)）——可复现；发现崩溃时用 --rounds N 重放定位。
# 无第三方依赖；临时文件写在项目内 .fuzz_tmp/（用完即删）。

import argparse
import os
import random
import shutil
import subprocess
import sys

TMP_DIR = '.fuzz_tmp'

# 破坏性尾缀/前缀（对齐上轮审查的冲击方法：把接近合法的输入弄坏）
MUTATORS = [
    lambda s: s + '"',                # 未闭合字符串
    lambda s: s + '(' * 500,          # 括号洪水（Q3 守卫路径）
    lambda s: s + '@' * 50,           # 非法字符洪水
    lambda s: s[: max(1, len(s) // 2)],  # 随机截断
    lambda s: s.replace('\n', '', 3),    # 删换行（语句分隔破坏）
]

# 种子语料片段（拼进随机垃圾，制造"接近合法"的输入）
SEED_SNIPPETS = [
    'fn main() -> Unit\n    println("x")\nend\n',
    'enum Shape\n    | Circle(Float)\nend\n',
    'let x = 1 + ',
    'match x\n    Ok(n) =>\n',
    'from string import {len}\n',
    '{"a": [1, 2, ',
]


def gen_input(rng):
    """一轮随机输入：种子片段 × 随机字节/字符混合 × 随机变异。"""
    parts = []
    for _ in range(rng.randint(1, 4)):
        if rng.random() < 0.5:
            parts.append(rng.choice(SEED_SNIPPETS))
        else:
            # 随机字节段：混合 ASCII、多字节 UTF-8 前导、非法孤立字节
            seg = bytes(rng.randint(0, 255) for _ in range(rng.randint(10, 200)))
            parts.append(seg.decode('utf-8', errors='replace'))
    s = ''.join(parts)
    if rng.random() < 0.6:
        s = rng.choice(MUTATORS)(s)
    return s


def main():
    ap = argparse.ArgumentParser(description='随机输入冲击冒烟')
    ap.add_argument('--rounds', type=int, default=80)
    ap.add_argument('--lom', default='./target/release/lom.exe' if sys.platform == 'win32'
                    else './target/release/lom')
    args = ap.parse_args()
    if not os.path.isfile(args.lom):
        print('错误：找不到 lom 二进制 %s' % args.lom)
        return 2

    os.makedirs(TMP_DIR, exist_ok=True)
    crashes = []
    for seed in range(args.rounds):
        rng = random.Random(seed)
        src = gen_input(rng)
        path = os.path.join(TMP_DIR, 'fuzz.lom')
        with open(path, 'w', encoding='utf-8', newline='') as f:
            f.write(src)
        try:
            r = subprocess.run([args.lom, path, '--check'], capture_output=True,
                               timeout=20)
        except subprocess.TimeoutExpired:
            crashes.append((seed, 'TIMEOUT'))
            continue
        # 合法终态：0（干净）/ 1（有诊断）。Windows 异常码与 Rust panic=101 是崩溃
        if r.returncode not in (0, 1):
            crashes.append((seed, 'exit=%d' % r.returncode))
            # 崩溃输入留档供复现
            shutil.copy(path, os.path.join(TMP_DIR, 'crash_%d.lom' % seed))
    shutil.rmtree(TMP_DIR, ignore_errors=True)

    if crashes:
        print('RESULT: FAIL（%d 轮中 %d 个崩溃输入）' % (args.rounds, len(crashes)))
        for seed, why in crashes[:10]:
            print('  seed=%d: %s' % (seed, why))
        return 1
    print('fuzz: %d 轮冲击零崩溃（固定种子 range(%d)，--check 全部正常终态）'
          % (args.rounds, args.rounds))
    print('RESULT: PASS')
    return 0


if __name__ == '__main__':
    sys.exit(main())
