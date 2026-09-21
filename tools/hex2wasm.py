#!/usr/bin/env python3
"""hex2wasm.py —— RFC-0004 方案 A 的宿主解码桩（L2.1 spike 交付件）。

Lom 的 file_write 是 UTF-8 文本语义，任意二进制流不可经 String 直接落盘
（UTF-8 空间不含孤立续字节/C0/C1/F5-FF，RFC-0004 预研实测）。方案 A：
L2 编译器产纯 ASCII hex 文本落盘，本桩转回二进制 .wasm。自举链条经一个
极小宿主桩——类比 C 编译器自举历史上必须经 as/ld 的常态。

用法：
    python tools/hex2wasm.py <in.hex> <out.wasm>

hex 格式：纯 ASCII（0-9a-f），允许任意空白（空格/换行/制表）作分隔。
"""
import re
import sys


def main() -> int:
    if len(sys.argv) != 3:
        sys.stderr.write("用法: python tools/hex2wasm.py <in.hex> <out.wasm>\n")
        return 2
    src = open(sys.argv[1], "r", encoding="ascii").read()
    compact = re.sub(r"\s+", "", src)
    if not compact or len(compact) % 2 != 0:
        sys.stderr.write("hex 长度必须为偶数（得到 %d 字符）\n" % len(compact))
        return 1
    if not re.fullmatch(r"[0-9a-fA-F]+", compact):
        sys.stderr.write("含非 hex 字符\n")
        return 1
    data = bytes.fromhex(compact)
    if data[:4] != b"\x00asm":
        sys.stderr.write("警告: 输出不是 wasm magic 开头（前 4 字节 %r）\n" % data[:4])
    with open(sys.argv[2], "wb") as f:
        f.write(data)
    print("hex2wasm: %d hex 字符 -> %d 字节 -> %s" % (len(compact), len(data), sys.argv[2]))
    return 0


if __name__ == "__main__":
    sys.exit(main())
