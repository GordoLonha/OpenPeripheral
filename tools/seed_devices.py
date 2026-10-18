#!/usr/bin/env python3
"""Print `untested` entries for devices/corsair.toml from other projects' device lists.

Usage: python3 -I tools/seed_devices.py OPENLINKHUB OPENRGB LIQUIDCTL > seed.toml

Takes facts only (PID, name, device type) from shallow clones of:
  OpenLinkHub  src/devices/devices.go     map `pid(decimal): {type, 0, "NAME", init, dongleInit}`
  OpenRGB      Controllers/Corsair*/      `#define CORSAIR_*_PID` + `REGISTER_*DETECTOR*(...)`
  liquidctl    liquidctl/driver/*.py      `_MATCHES = [(0x1b1c, 0xPID, 'Name', ...)]`
Never ckb-next (GPL-2.0 only). Nothing is executed; the files are only read as text.

Family is set only where the source's driver clearly speaks a protocol we have:
OpenRGB CorsairPeripheralController on usage page FFC2 -> nxp (K70 MK.2 style),
CorsairPeripheralV2Controller -> bragi (HS80 style). Everything else stays unknown.
The PIDs we already drive (VERIFIED) are skipped: their entries are hand-written.
"""

import glob
import os
import re
import sys

VERIFIED = {0x1B55, 0x0A6B, 0x0A69, 0x0A6A}


def openlinkhub(root):
    out = {}
    src = open(os.path.join(root, "src/devices/devices.go"), encoding="utf-8").read()
    body = src.split("var deviceRegisterMap", 1)[1].split("\n}\n", 1)[0]
    rx = re.compile(r'^\s*(\d+):\s*\{(\d+),\s*\d+,\s*"([^"]+)",\s*(\S+?),\s*(\S+?)\}')
    for line in body.splitlines():
        m = rx.match(line)
        if not m:
            continue  # commented out or not an entry
        pid, typ, name, init, dongle = int(m[1]), int(m[2]), m[3], m[4], m[5]
        if pid == 0 or "SCUF" in name.upper():
            continue  # memory pseudo-device; SCUF has its own vendor ID
        comment = line.split("//", 1)[1].strip() if "//" in line else ""
        out[pid] = {"name": name, "olh_type": typ, "dongle": dongle != "nil", "comment": comment}
    return out


def openrgb(root):
    out = {}
    defines = {}
    files = glob.glob(os.path.join(root, "Controllers/Corsair*/**/*.cpp"), recursive=True)
    files += glob.glob(os.path.join(root, "Controllers/Corsair*/**/*.h"), recursive=True)
    for f in files:
        for m in re.finditer(r"#define\s+(CORSAIR_\w+_PID)\s+(0x[0-9A-Fa-f]+)", open(f, encoding="utf-8", errors="replace").read()):
            defines[m[1]] = int(m[2], 16)
    rx = re.compile(r'REGISTER_\w*DETECTOR\w*\(\s*"([^"]+)"\s*,\s*(\w+)\s*,\s*CORSAIR_VID\s*,\s*(\w+)\s*(?:,\s*([^,)]+))?(?:,\s*([^,)]+))?')
    for f in files:
        text = open(f, encoding="utf-8", errors="replace").read()
        for line in text.splitlines():
            if line.lstrip().startswith("//"):
                continue
            m = rx.search(line)
            if not m or m[3] not in defines:
                continue
            pid = defines[m[3]]
            page = (m[5] or "").strip().upper()
            family = ""
            if "CorsairPeripheralV2Controller" in f:
                family = "bragi"
            elif m[2] == "DetectCorsairPeripheralControllers" and page == "0XFFC2":
                family = "nxp"
            out.setdefault(pid, {"name": m[1], "family": family})
    return out


def liquidctl(root):
    out = {}
    rx = re.compile(r"\(\s*0x1b1c\s*,\s*(0x[0-9a-f]+)\s*,\s*['\"]([^'\"]+)['\"]", re.I | re.S)
    for f in glob.glob(os.path.join(root, "liquidctl/driver/*.py")):
        for m in rx.finditer(open(f, encoding="utf-8").read()):
            out.setdefault(int(m[1], 16), {"name": m[2]})
    return out


KIND_RULES = [
    ("mousepad", r"\bMM\d|MOUSEPAD|MOUSE ?MAT|MOUSE BUNGEE"),
    ("psu", r"\b[HAR][XM]\d{3,4}i\b|\bAX\d+i|POWER SUPPLY|\bPSU\b"),
    ("headset", r"\bHS\d|VIRTUOSO|\bVOID|HEADSET"),
    ("keyboard", r"^K\d|STRAFE|VANGUARD|MAKR|CLIPPER|KEYBOARD"),
    ("mouse", r"^M\d\d|KATAR|IRONCLAW|DARK ?CORE|NIGHTSABRE|SCIMITAR|HARPOON|NIGHTSWORD|GLAIVE|SABRE|DARKSTAR|MOUSE"),
    ("cooler", r"HYDRO|\bH\d+i|\bELITE (LCD|CAPELLIX)|CAPELLIX|XC7|NAUTILUS|PLATINUM|PRO XT|TITAN|\bLCD\b"),
    ("controller", r"COMMANDER|LIGHTING NODE|LINK|NEXUS|CORSAIR ONE|1000D|\bLT100|\bST100|LED|TOWER|STAND|GPU|LS100|QL|SPEC.OMEGA|^ONE\b|MEMORY|DOMINATOR|VENGEANCE|XENEON"),
]


def kind_of(name, olh):
    up = name.upper()
    dongle = olh and olh["dongle"] or re.search(r"DONGLE|RECEIVER|SLIPSTREAM", up + " " + (olh or {}).get("comment", "").upper())
    for kind, rx in KIND_RULES:
        if re.search(rx, up.replace("CORSAIR ", ""), re.I):
            if dongle and kind == "headset":
                return "headset-dongle"
            if dongle and kind in ("keyboard", "mouse"):
                return "dongle"
            return kind
    if dongle:
        return "headset-dongle" if olh and olh["olh_type"] in (3, 4) else "dongle"
    return "other"


def clean(name):
    name = re.sub(r"^Corsair\s+", "", name, flags=re.I)
    return name.replace('"', "'").strip()


def main():
    olh, orgb, lqd = openlinkhub(sys.argv[1]), openrgb(sys.argv[2]), liquidctl(sys.argv[3])
    pids = sorted((set(olh) | set(orgb) | set(lqd)) - VERIFIED)
    print(f"# Seeded by tools/seed_devices.py: {len(pids)} PIDs, all untested (read-only).")
    for pid in pids:
        sources = [s for s, d in (("openlinkhub", olh), ("openrgb", orgb), ("liquidctl", lqd)) if pid in d]
        # OpenRGB / liquidctl names are mixed case and more specific; OpenLinkHub's are upper case.
        name = clean((orgb.get(pid) or lqd.get(pid) or olh[pid])["name"])
        family = orgb.get(pid, {}).get("family", "")
        print()
        print("[[device]]")
        print(f"pid = 0x{pid:04X}")
        print(f'name = "{name}"')
        print(f'kind = "{kind_of(name, olh.get(pid))}"')
        if family:
            print(f'family = "{family}"')
        print('status = "untested"')
        print("sources = [" + ", ".join(f'"{s}"' for s in sources) + "]")


if __name__ == "__main__":
    main()
