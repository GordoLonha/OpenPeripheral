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

export function Switch({ on, disabled, onClick }: { on: boolean; disabled?: boolean; onClick: () => void }) {
  return (
    <button role="switch" aria-checked={on} className={'switch' + (on ? ' on' : '')} disabled={disabled} onClick={onClick}>
      <span />
    </button>
  );
}
