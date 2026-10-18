"""Print the K70 key list (name, x, y) in physical order for a layout, read from
the user's local iCUE install. Output feeds `opctl k70 map --keys`.

usage: python -I tools/k70_keylist.py [--layout BR] > dumps/k70_keys_BR.tsv

Only key names and positions are read (for interoperability); the generated file
stays local (dumps/ is gitignored). Requires tools/rcc_list.py's extraction:
  python -I tools/rcc_list.py "<iCUE>/devices/k70_rgb_mk_2_lp.rcc" --match manifest --extract dumps/rcc
"""
import argparse
import glob
import json
import sys


# Prompt labels as printed on an ABNT2 keycap, with a location hint.
LABELS_BR = {
    "Key_Profile": "botão de PERFIL (fileira de cima, à esquerda)",
    "Brightness": "botão de BRILHO (fileira de cima, ícone de sol)",
    "WinLock": "botão TRAVA DO WINDOWS (fileira de cima, ícone de cadeado)",
    "Mute": "MUDO (canto superior direito, ao lado da rodinha de volume)",
    "Stop": "mídia: PARAR (quadrado)",
    "ScanPreviousTrack": "mídia: FAIXA ANTERIOR (|<<)",
    "PlayPause": "mídia: PLAY/PAUSE",
    "ScanNextTrack": "mídia: PRÓXIMA FAIXA (>>|)",
    "Escape": "Esc",
    "PrintScreen": "PrtSc / Print Screen",
    "ScrollLock": "Scroll Lock",
    "PauseBreak": "Pause / Break",
    "GraveAccentAndTilde": "' \"  (aspas, à esquerda do 1)",
    "MinusAndUnderscore": "- _  (hífen)",
    "EqualsAndPlus": "= +",
    "Backspace": "Backspace (apagar, seta para trás)",
    "Insert": "Insert",
    "Home": "Home",
    "PageUp": "Page Up",
    "NumLock": "Num Lock (teclado numérico)",
    "KeypadSlash": "/ do teclado numérico",
    "KeypadAsterisk": "* do teclado numérico",
    "KeypadMinus": "- do teclado numérico",
    "KeypadPlus": "+ do teclado numérico",
    "KeypadEnter": "Enter do teclado numérico",
    "KeypadPeriodAndDelete": ", (vírgula/Del) do teclado numérico",
    "Tab": "Tab",
    "BracketLeft": "´ `  (acento agudo, à direita do P)",
    "BracketRight": "[ {  (à direita do acento agudo)",
    "Delete": "Delete",
    "End": "End",
    "PageDown": "Page Down",
    "Enter": "Enter (o grande, principal)",
    "CapsLock": "Caps Lock",
    "SemicolonAndColon": "Ç",
    "ApostropheAndDoubleQuote": "~ ^  (til, à direita do Ç)",
    "NonUsTilde": "] }  (à esquerda do Enter, abaixo do [ )",
    "LeftShift": "Shift ESQUERDO",
    "NonUsBackslash": "\\ |  (entre o Shift esquerdo e o Z)",
    "CommaAndLessThan": ", <  (vírgula)",
    "PeriodAndBiggerThan": ". >  (ponto)",
    "SlashAndQuestionMark": "; :  (ponto e vírgula)",
    "International1": "/ ?  (barra, à esquerda do Shift direito)",
    "RightShift": "Shift DIREITO",
    "UpArrow": "seta para CIMA",
    "LeftCtrl": "Ctrl ESQUERDO",
    "LeftGui": "Windows ESQUERDO",
    "LeftAlt": "Alt (esquerdo)",
    "Space": "Barra de espaço",
    "RightAlt": "Alt Gr (direito)",
    "RightGui": "Windows DIREITO (se não existir, espere)",
    "Application": "Menu (ícone de lista, ao lado do Ctrl direito; ou Fn — se não existir, espere)",
    "RightCtrl": "Ctrl DIREITO",
    "LeftArrow": "seta para ESQUERDA",
    "DownArrow": "seta para BAIXO",
    "RightArrow": "seta para DIREITA",
    "Led_KeyboardTopLogo1": "(LED do logo — nada para apertar)",
    "Led_KeyboardTopLogo2": "(LED do logo — nada para apertar)",
}


def label(name):
    if name in LABELS_BR:
        return LABELS_BR[name]
    if name.startswith("Keypad"):
        return f"{name[6:]} do teclado numérico"
    return name  # letters, digits, F1..F12 are printed as-is


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--layout", default="BR")
    ap.add_argument("--manifest", default=None)
    a = ap.parse_args()
    sys.stdout.reconfigure(encoding="utf-8")  # -I ignores PYTHONIOENCODING
    path = a.manifest or glob.glob("dumps/rcc/**/*manifest*", recursive=True)[0]
    common = json.load(open(path, encoding="utf-8"))["common"]
    geo = common["StaticGeometryMap"]["RegularStatic"]
    keys = {}
    for item in common["Layouts"][a.layout]["keyItems"]:
        if item["type"] != "Led":
            continue
        pts = geo.get(str(item["deviceLinkGeometryIndex"]))
        if not pts:
            continue
        keys[item["id"]] = (pts[0]["x"], pts[0]["y"])
    # Rows: cluster by y (keys in a row share y within ~60 units), then sort by x.
    ordered = sorted(keys.items(), key=lambda kv: (round(kv[1][1] / 100), kv[1][0]))
    for name, (x, y) in ordered:
        print(f"{name}\t{x}\t{y}\t{label(name)}")


if __name__ == "__main__":
    main()
