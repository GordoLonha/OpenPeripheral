"""List (and optionally extract) files inside a Qt binary resource (.rcc).

usage: python -I tools/rcc_list.py FILE.rcc [--extract OUTDIR] [--match SUBSTR]

Used to read layout *data* (key positions / LED indices) for interoperability;
do not redistribute extracted vendor images or artwork.
"""
import argparse
import os
import struct
import zlib


def u16(b, o):
    return struct.unpack_from(">H", b, o)[0]


def u32(b, o):
    return struct.unpack_from(">I", b, o)[0]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("rcc")
    ap.add_argument("--extract")
    ap.add_argument("--match", default="")
    a = ap.parse_args()
    data = open(a.rcc, "rb").read()
    if data[:4] != b"qres":
        raise SystemExit("not an rcc file")
    version = u32(data, 4)
    tree, payload, names = u32(data, 8), u32(data, 12), u32(data, 16)
    node_size = 22 if version >= 2 else 14

    def name(off):
        n = u16(data, names + off)
        return data[names + off + 6: names + off + 6 + n * 2].decode("utf-16-be")

    def walk(idx, path):
        o = tree + idx * node_size
        nm = name(u32(data, o)) if idx else ""
        flags = u16(data, o + 4)
        full = f"{path}/{nm}" if nm else path
        if flags & 0x02:  # directory
            count, first = u32(data, o + 6), u32(data, o + 10)
            for i in range(count):
                yield from walk(first + i, full)
        else:
            yield full, flags, u32(data, o + 10)

    for path, flags, off in walk(0, ""):
        if a.match not in path:
            continue
        size = u32(data, payload + off)
        blob = data[payload + off + 4: payload + off + 4 + size]
        if flags & 0x01:  # zlib, first 4 bytes = uncompressed size
            blob = zlib.decompress(blob[4:])
        elif flags & 0x04:  # zstd: not supported by stdlib
            print(f"{path}\t{size}\t(zstd)")
            continue
        print(f"{path}\t{len(blob)}")
        if a.extract:
            dst = os.path.join(a.extract, path.lstrip("/"))
            os.makedirs(os.path.dirname(dst), exist_ok=True)
            open(dst, "wb").write(blob)


if __name__ == "__main__":
    main()
