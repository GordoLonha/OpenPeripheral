import { fromHex, toHex, type RGB } from './api';

/** Small form controls shared by the device pages. */

export function ColorField({ label, value, onChange }: { label: string; value: RGB; onChange: (c: RGB) => void }) {
  return (
    <label className="color-field">
      <input type="color" value={toHex(value)} onChange={(e) => onChange(fromHex(e.target.value))} />
      <span>
        {label}
        <code>{toHex(value).toUpperCase()}</code>
      </span>
    </label>
  );
}

export function Slider(p: { label: string; value: number; min: number; max: number; step: number; unit: string; onChange: (v: number) => void }) {
  return (
    <label className="slider">
      <span>
        {p.label}
        <output>{p.unit === '%' ? `${p.value}%` : `${p.value.toFixed(1)}×`}</output>
      </span>
      <input type="range" min={p.min} max={p.max} step={p.step} value={p.value} onChange={(e) => p.onChange(Number(e.target.value))} />
    </label>
  );
}

export function Switch({ on, disabled, label, onClick }: { on: boolean; disabled?: boolean; label?: string; onClick: () => void }) {
  return (
    <button role="switch" aria-checked={on} aria-label={label} className={'switch' + (on ? ' on' : '')} disabled={disabled} onClick={onClick}>
      <span />
    </button>
  );
}

/** Underlined tabs for sections inside a page (not for picking a value: use .seg). */
export function Tabs<T extends string>(p: { value: T; options: [T, string][]; onChange: (v: T) => void; label: string }) {
  return (
    <div className="tabs" role="tablist" aria-label={p.label}>
      {p.options.map(([id, name]) => (
        <button key={id} role="tab" aria-selected={p.value === id} className={p.value === id ? 'on' : ''} onClick={() => p.onChange(id)}>
          {name}
        </button>
      ))}
    </div>
  );
}
