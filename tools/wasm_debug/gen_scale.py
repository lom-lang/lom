# W0 规模标定：生成递增规模的合法 Lom 程序（三种形态）
# 用法: python .w0/gen_scale.py <shape> <target_bytes> > out.lom
# shape: fn = 重复函数声明 / expr = 长加法链 / nest = 深括号嵌套
import sys

shape = sys.argv[1]
target = int(sys.argv[2])

lines = []
if shape == 'fn':
    lines.append('fn f0(a: Int) -> Int')
    lines.append('    a + 0')
    lines.append('end')
    n = 1
    while sum(len(l) + 1 for l in lines) < target - 140:
        lines.append(f'fn f{n}(a: Int) -> Int')
        lines.append(f'    a + {n}')
        lines.append('end')
        n += 1
    last = n - 1
    lines.append('fn main() -> Unit')
    lines.append(f'    println(f0({last}))')
    lines.append(f'    println(f{last}(1))')
    lines.append('end')
elif shape == 'expr':
    lines.append('fn main() -> Unit')
    body = '    let x = 1'
    while len(body) < target - 40:
        body += ' + 1'
    lines.append(body)
    lines.append('    println(x)')
    lines.append('end')
elif shape == 'nest':
    lines.append('fn main() -> Unit')
    depth = max(2, (target - 60) // 2)
    body = '    let x = ' + '(' * depth + '"ok"' + ')' * depth
    lines.append(body)
    lines.append('    println(x)')
    lines.append('end')
else:
    sys.exit('unknown shape')

prog = '\n'.join(lines) + '\n'
sys.stdout.write(prog)
sys.stderr.write(f'{shape} {len(prog)}B\n')
