"""Render the OpenPeripheral app icon (1024x1024 PNG) with the standard library only.

usage: python -I tools/make_icon.py app/src-tauri/app-icon.png
then:  npx tauri icon app/src-tauri/app-icon.png   (from app/)

Design: rounded square filled with a conic gradient (cyan -> violet -> pink ->
amber), a dark inset "keycap", and three glowing level bars on the cap.
"""
import math
import struct
import sys
import zlib

N = 1024
STOPS = [(0.0, (57, 208, 240)), (0.3, (139, 92, 246)), (0.55, (255, 79, 163)), (0.8, (255, 179, 71)), (1.0, (57, 208, 240))]
BG = (11, 13, 18)
CAP = (21, 25, 33)


def conic(x, y):
    a = (math.atan2(y - N / 2, x - N / 2) / (2 * math.pi) + 0.5 + 0.15) % 1.0
    for (p0, c0), (p1, c1) in zip(STOPS, STOPS[1:]):
        if p0 <= a <= p1:
            t = (a - p0) / (p1 - p0)
            return tuple(c0[i] + (c1[i] - c0[i]) * t for i in range(3))
    return STOPS[0][1]


def linear(x, y):
    """Diagonal gradient (bottom-left cyan -> top-right amber) for the bars."""
    a = min(0.999, max(0.0, ((x - 330) + (700 - y)) / 760)) * 0.8
    for (p0, c0), (p1, c1) in zip(STOPS, STOPS[1:]):
        if p0 <= a <= p1:
            t = (a - p0) / (p1 - p0)
            return tuple(c0[i] + (c1[i] - c0[i]) * t for i in range(3))
    return STOPS[-1][1]


def rrect_sdf(x, y, cx, cy, hw, hh, r):
    """Signed distance to a rounded rectangle (negative inside)."""
    dx, dy = abs(x - cx) - (hw - r), abs(y - cy) - (hh - r)
    outside = math.hypot(max(dx, 0), max(dy, 0))
    return outside + min(max(dx, dy), 0) - r


def cover(d):
    """Anti-aliased coverage from a signed distance."""
    return min(1.0, max(0.0, 0.5 - d))


def mix(a, b, t):
    return tuple(a[i] + (b[i] - a[i]) * t for i in range(3))


def pixel(x, y):
    c = N / 2
    outer = rrect_sdf(x, y, c, c, 470, 470, 210)
    a = cover(outer)
    if a == 0:
        return (0, 0, 0, 0)
    col = conic(x, y)
    # dark keycap inset
    cap = rrect_sdf(x, y, c, c + 14, 300, 300, 120)
    col = mix(col, BG, 0.0)
    shadow = cover(rrect_sdf(x, y, c, c + 40, 300, 300, 120) / 3)
    col = mix(col, (0, 0, 0), 0.35 * shadow)
    col = mix(col, CAP, cover(cap))
    # three glowing bars on the cap (RGB levels / equaliser)
    bars = min(
        rrect_sdf(x, y, c + dx, c + 14 + 150 - h, 38, h, 38)
        for dx, h in ((-120, 120), (0, 200), (120, 155))
    )
    glow = math.exp(-max(bars, 0) / 22) * cover(cap) * 0.55
    col = mix(col, linear(x, y), max(cover(bars), glow))
    return (*[int(round(v)) for v in col], int(round(a * 255)))


def main():
    out = sys.argv[1]
    rows = []
    for y in range(N):
        row = bytearray(b"\x00")
        for x in range(N):
            # 2x2 supersampling for smooth edges
            px = [pixel(x + dx, y + dy) for dx in (0.25, 0.75) for dy in (0.25, 0.75)]
            row += bytes(sum(p[i] for p in px) // 4 for i in range(4))
        rows.append(bytes(row))
    raw = zlib.compress(b"".join(rows), 9)

    def chunk(t, d):
        return struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)

    png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", N, N, 8, 6, 0, 0, 0)) + chunk(b"IDAT", raw) + chunk(b"IEND", b"")
    open(out, "wb").write(png)
    print("wrote", out)


if __name__ == "__main__":
    main()
