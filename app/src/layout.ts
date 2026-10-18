// K70 RGB MK.2 Low Profile, ABNT2. Own geometry in key units (1u = one key).
// Key ids match layout/k70_mk2_lp_BR.json (key name -> key index; LED slots via get_ledmap).

export type KeyDef = { id: string; x: number; y: number; w: number; h: number; label: string; small?: boolean };

const k = (id: string, x: number, y: number, label: string, w = 1, h = 1, small = false): KeyDef => ({
  id, x, y, w, h, label, small,
});

const row = (ids: [string, string][], x0: number, y: number): KeyDef[] =>
  ids.map(([id, label], i) => k(id, x0 + i, y, label));

const LETTERS = (s: string) => [...s].map((c) => [c, c] as [string, string]);

export const KEYS: KeyDef[] = [
  // Top strip: function buttons and mute
  k('Key_Profile', 0.2, 0, 'Perfil', 1.1, 0.6, true),
  k('Brightness', 1.4, 0, '☀', 1.1, 0.6, true),
  k('WinLock', 2.6, 0, '🔒', 1.1, 0.6, true),
  k('Mute', 21.5, 0, '🔇', 1, 0.6, true),

  // Function row
  k('Escape', 0, 1, 'Esc'),
  ...row([['F1', 'F1'], ['F2', 'F2'], ['F3', 'F3'], ['F4', 'F4']], 2, 1),
  ...row([['F5', 'F5'], ['F6', 'F6'], ['F7', 'F7'], ['F8', 'F8']], 6.5, 1),
  ...row([['F9', 'F9'], ['F10', 'F10'], ['F11', 'F11'], ['F12', 'F12']], 11, 1),
  ...row([['PrintScreen', 'PrtSc'], ['ScrollLock', 'ScrLk'], ['PauseBreak', 'Pause']], 15.25, 1),
  ...row([['Stop', '■'], ['ScanPreviousTrack', '⏮'], ['PlayPause', '⏯'], ['ScanNextTrack', '⏭']], 18.5, 1),

  // Number row
  k('GraveAccentAndTilde', 0, 2.25, "'"),
  ...row([['1', '1'], ['2', '2'], ['3', '3'], ['4', '4'], ['5', '5'], ['6', '6'], ['7', '7'], ['8', '8'], ['9', '9'], ['0', '0'],
    ['MinusAndUnderscore', '-'], ['EqualsAndPlus', '=']], 1, 2.25),
  k('Backspace', 13, 2.25, '⌫', 2),
  ...row([['Insert', 'Ins'], ['Home', 'Home'], ['PageUp', 'PgUp']], 15.25, 2.25),
  ...row([['NumLock', 'Num'], ['KeypadSlash', '/'], ['KeypadAsterisk', '*'], ['KeypadMinus', '-']], 18.5, 2.25),

  // QWERTY row
  k('Tab', 0, 3.25, 'Tab', 1.5),
  ...row([...LETTERS('QWERTYUIOP'), ['BracketLeft', '´'], ['BracketRight', '[']], 1.5, 3.25),
  k('Enter', 13.75, 3.25, 'Enter', 1.25, 2),
  ...row([['Delete', 'Del'], ['End', 'End'], ['PageDown', 'PgDn']], 15.25, 3.25),
  ...row([['Keypad7', '7'], ['Keypad8', '8'], ['Keypad9', '9']], 18.5, 3.25),
  k('KeypadPlus', 21.5, 3.25, '+', 1, 2),

  // Home row
  k('CapsLock', 0, 4.25, 'Caps', 1.75),
  ...row([...LETTERS('ASDFGHJKL'), ['SemicolonAndColon', 'Ç'], ['ApostropheAndDoubleQuote', '~'], ['NonUsTilde', ']']], 1.75, 4.25),
  ...row([['Keypad4', '4'], ['Keypad5', '5'], ['Keypad6', '6']], 18.5, 4.25),

  // Shift row
  k('LeftShift', 0, 5.25, 'Shift', 1.25),
  ...row([['NonUsBackslash', '\\'], ...LETTERS('ZXCVBNM'), ['CommaAndLessThan', ','], ['PeriodAndBiggerThan', '.'],
    ['SlashAndQuestionMark', ';'], ['International1', '/']], 1.25, 5.25),
  k('RightShift', 13.25, 5.25, 'Shift', 1.75),
  k('UpArrow', 16.25, 5.25, '↑'),
  ...row([['Keypad1', '1'], ['Keypad2', '2'], ['Keypad3', '3']], 18.5, 5.25),
  k('KeypadEnter', 21.5, 5.25, '⏎', 1, 2),

  // Bottom row
  k('LeftCtrl', 0, 6.25, 'Ctrl', 1.25),
  k('LeftGui', 1.25, 6.25, 'Win', 1.25),
  k('LeftAlt', 2.5, 6.25, 'Alt', 1.25),
  k('Space', 3.75, 6.25, '', 6.25),
  k('RightAlt', 10, 6.25, 'AltGr', 1.25),
  k('RightGui', 11.25, 6.25, 'Win', 1.25),
  k('Application', 12.5, 6.25, '☰', 1.25),
  k('RightCtrl', 13.75, 6.25, 'Ctrl', 1.25),
  ...row([['LeftArrow', '←'], ['DownArrow', '↓'], ['RightArrow', '→']], 15.25, 6.25),
  k('Keypad0', 18.5, 6.25, '0', 2),
  k('KeypadPeriodAndDelete', 20.5, 6.25, ','),
];

export const BOARD_W = 22.5;
export const BOARD_H = 7.25;
