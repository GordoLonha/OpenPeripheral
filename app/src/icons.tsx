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
