"""Convert raw RGBA dumps (square) into one PNG strip, scaled up, on a checkered
background, to eyeball tray icons.

usage: python -I tools/rgba_to_png.py OUT.png FILE.rgba [FILE.rgba ...] [--scale 6]
"""
import math
import struct
import sys
import zlib


def main():
    args = sys.argv[1:]
    scale = 6
    if "--scale" in args:
        i = args.index("--scale")
        scale = int(args[i + 1])
        del args[i : i + 2]
    out, files = args[0], args[1:]
    imgs = [open(f, "rb").read() for f in files]
    n = int(math.isqrt(len(imgs[0]) // 4))
    gap = 8
    w = len(imgs) * (n * scale + gap) + gap
    h = n * scale + 2 * gap
    rows = []
    for y in range(h):
        row = bytearray(b"\x00")
        for x in range(w):
            # checkerboard: dark + light halves to check both taskbar themes
            bg = (32, 32, 36) if y < h // 2 else (230, 230, 232)
            col = bg
            k, xr = divmod(x - gap, n * scale + gap)
            yr = y - gap
            if 0 <= k < len(imgs) and 0 <= xr < n * scale and 0 <= yr < n * scale:
                px, py = xr // scale, yr // scale
                o = (py * n + px) * 4
                r, g, b, a = imgs[k][o : o + 4]
                a /= 255
                col = tuple(round(c * a + bgc * (1 - a)) for c, bgc in zip((r, g, b), bg))
            row += bytes(col)
        rows.append(bytes(row))

    def chunk(t, d):
        return struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)

    data = zlib.compress(b"".join(rows), 9)
    png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0)) + chunk(b"IDAT", data) + chunk(b"IEND", b"")
    open(out, "wb").write(png)
    print("wrote", out)


if __name__ == "__main__":
    main()
