# W1 堆语义图对比 v2：解码对象，指针归一化为 owner 分配序号，按分配序对齐 diff。
# 用法: python .w0/heapdiff.py <trap_snap> <trap_alloc_csv> <pass_snap> <pass_alloc_csv> [first_n]
import struct, sys

trap = open(sys.argv[1], 'rb').read()
pas = open(sys.argv[3], 'rb').read()
def allocs(p):
    return [tuple(map(int, l.split(','))) for l in open(p).read().splitlines() if l]
ta, pa = allocs(sys.argv[2]), allocs(sys.argv[4])
FIRST_N = int(sys.argv[5]) if len(sys.argv) > 5 else 40

def rngs(al):
    return [(e - s, s, e) for s, e in al]  # (start, size, end)

tr, pr = rngs(ta), rngs(pa)

def owner_of(rl, addr):
    for i, (st, sz, e) in enumerate(rl):
        if st <= addr < e:
            return i
    return None

def norm_val(rl, mem, v):
    """tagged i64 -> 语义元组：立即数或 ('ptr', owner, +off, tag)。"""
    tag = v & 0xF
    body = v >> 4
    if tag in (4, 5, 6, 7, 8, 9, 10) and body != 0:
        o = owner_of(rl, int(body))
        if o is not None:
            return ('ptr', o, int(body) - rl[o][0], tag)
    return ('imm', v)

def decode_obj(rl, mem, idx):
    st, sz, e = rl[idx]
    u32 = lambda o: struct.unpack_from('<I', mem, o)[0]
    i64 = lambda o: struct.unpack_from('<q', mem, o)[0]
    if sz > 4 and 4 + u32(st) == sz:
        ln = u32(st)
        if st + 4 + ln > len(mem):
            return ('str-overflow', ln)
        if ln < 8192:
            try:
                return ('str', mem[st+4:st+4+ln].decode('utf-8'))
            except Exception:
                return ('str?', mem[st+4:st+4+ln].hex())
    if sz == 16:
        return ('cons', norm_val(rl, mem, i64(st)), norm_val(rl, mem, i64(st + 8)))
    if sz == 12:
        bo = u32(st)
        bo_o = owner_of(rl, bo)
        return ('map', ('ptr', bo_o, bo - rl[bo_o][0], 4) if bo_o is not None else bo,
                u32(st + 4), u32(st + 8))
    if sz >= 8 and sz == 8 + 8 * u32(st + 4):
        return ('enum', u32(st), u32(st + 4),
                [norm_val(rl, mem, i64(st + 8 + 8 * i)) for i in range(u32(st + 4))])
    if sz >= 12 and sz == 4 + 8 * u32(st) and u32(st) <= 64:
        n = u32(st)
        return ('tuple', [norm_val(rl, mem, i64(st + 4 + 8 * i)) for i in range(n)])
    if sz >= 16 and sz == 4 + 12 * u32(st) and u32(st) <= 64:
        n = u32(st)
        flds = []
        for i in range(n):
            no = u32(st + 4 + 12 * i)
            no_o = owner_of(rl, no)
            flds.append((('ptr', no_o, no - rl[no_o][0], 4) if no_o is not None else no,
                         norm_val(rl, mem, i64(st + 8 + 12 * i))))
        return ('rec', flds)
    if sz % 16 == 0:
        cells = []
        for i in range(sz // 16):
            b = st + 16 * i
            ko = u32(b + 4)
            ko_o = owner_of(rl, ko)
            kn = ('ptr', ko_o, ko - rl[ko_o][0], 4) if ko_o is not None else ko
            cells.append((u32(b), kn, norm_val(rl, mem, i64(b + 8))))
        return ('cells', cells)
    return ('raw', mem[st:e].hex())

n = min(len(tr), len(pr))
shown = 0
first = None
for i in range(6, n):
    if tr[i][0] + tr[i][1] > len(trap) or pr[i][0] + pr[i][1] > len(pas):
        print(f'snapshot boundary at #{i}（后续不可比）')
        break
    if tr[i][1] != pr[i][1]:
        print(f'SIZE DIVERGE at #{i}: trap={tr[i][1]} pass={pr[i][1]}')
        break
    dt = decode_obj(tr, trap, i)
    dp = decode_obj(pr, pas, i)
    if dt != dp:
        if first is None:
            first = i
            print(f'FIRST semantic diverge @alloc #{i} (size={tr[i][1]} trap@{tr[i][0]} pass@{pr[i][0]}):')
            print('  trap:', dt)
            print('  pass:', dp)
        shown += 1
        if shown >= FIRST_N:
            break
if first is None:
    print('common prefix: 语义级完全一致（指针归一化后）')
else:
    print(f'diverging allocs shown: {shown}')
