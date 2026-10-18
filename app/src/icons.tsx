// Small line icons drawn for OpenPeripheral (24x24, currentColor).
const P = {
  key: 'M3 7h18v10H3zM6 10h1M9 10h1M12 10h1M15 10h1M18 10h1M7 14h10',
  combo: 'M2 8h8v8H2zM14 8h8v8h-8zM11 12h2M12 11v2',
  mouse: 'M12 3a6 6 0 0 1 6 6v6a6 6 0 0 1-12 0V9a6 6 0 0 1 6-6zM12 3v6M6 9h12',
  text: 'M5 6h10M10 6v12M17 8v10M15 8h4M15 18h4',
  media: 'M5 6l7 6-7 6zM15 6v12M19 6v12',
  launch: 'M4 5h16v14H4zM4 9h16M7 7h1M10 7h1',
  disabled: 'M12 3a9 9 0 1 1 0 18 9 9 0 0 1 0-18zM8 8l8 8M16 8l-8 8',
  macro: 'M4 6h2M9 6h11M4 12h2M9 12h11M4 18h2M9 18h11',
  plus: 'M12 5v14M5 12h14',
  profile: 'M4 7h12M13 4l3 3-3 3M20 17H8M11 14l-3 3 3 3',
  close: 'M6 6l12 12M18 6L6 18',
  record: 'M12 7a5 5 0 1 1 0 10 5 5 0 0 1 0-10z',
  home: 'M4 11l8-7 8 7M6 9.5V20h12V9.5M10 20v-6h4v6',
  keyboard: 'M2 7h20v11H2zM5 10h1M8 10h1M11 10h1M14 10h1M17 10h1M5 13h1M18 13h1M8 13h8M7 16h10',
  headset: 'M4 14v-2a8 8 0 0 1 16 0v2M4 14h3v6H5a1 1 0 0 1-1-1zM20 14h-3v6h2a1 1 0 0 0 1-1z',
  layers: 'M12 4l9 5-9 5-9-5zM3 14l9 5 9-5',
  settings:
    'M12 9a3 3 0 1 1 0 6 3 3 0 0 1 0-6zM12 2v3M12 19v3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M2 12h3M19 12h3M4.9 19.1L7 17M17 7l2.1-2.1',
};

export type IconName = keyof typeof P;

export function Icon({ name, size = 18 }: { name: IconName; size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.8}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
    >
      <path d={P[name]} />
    </svg>
  );
}
