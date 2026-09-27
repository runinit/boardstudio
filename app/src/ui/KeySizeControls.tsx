import { useEffect, useRef, useState } from 'react';
import type { Vec2 } from '../../../contracts/src/index';

type KeySizeItem = { size: Vec2; pitch: Vec2; gap: Vec2 };

export function KeySizeControls({ items, onCommit }: { items: KeySizeItem[]; onCommit: (units: Vec2, axis?: 'x' | 'y') => void }) {
  const units = ({ size, pitch, gap }: KeySizeItem) => ({ x: Math.round((size.x + gap.x) / pitch.x * 4) / 4, y: Math.round((size.y + gap.y) / pitch.y * 4) / 4 });
  const value = items.length ? units(items[0]) : { x: 1, y: 1 };
  const mixed = items.some((item) => { const itemUnits = units(item); return itemUnits.x !== value.x || itemUnits.y !== value.y; });
  const [draft, setDraft] = useState(value);
  useEffect(() => setDraft(value), [value.x, value.y, mixed]);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const sent = useRef<string | undefined>(undefined);
  useEffect(() => { sent.current = undefined; }, [value.x, value.y, mixed]);
  useEffect(() => () => clearTimeout(timer.current), []);
  const commit = (next = draft, axis?: 'x' | 'y') => {
    clearTimeout(timer.current);
    const signature = JSON.stringify({ next, axis });
    if (sent.current === signature) return;
    if (axis ? items.some((item) => units(item)[axis] !== next[axis]) : mixed || next.x !== value.x || next.y !== value.y) {
      sent.current = signature;
      onCommit(next, axis);
    }
  };
  const keyboardCommit = (axis: 'x' | 'y') => {
    clearTimeout(timer.current);
    timer.current = setTimeout(() => commit(draft, axis), 150);
  };
  return <section className="wb-key-size" aria-label="Key size">
    <div className="wb-key-size-heading"><strong>Key size</strong><output>{mixed ? 'Mixed' : `${draft.x}u × ${draft.y}u`}</output></div>
    {(['x', 'y'] as const).map((axis) => <label key={axis}>{axis === 'x' ? 'Width' : 'Height'}<input type="range" aria-label={axis === 'x' ? 'Key width' : 'Key height'} min="1" max="7" step=".25" value={draft[axis]} aria-valuetext={`${draft[axis]}u`} onChange={(event) => setDraft({ ...draft, [axis]: Number(event.target.value) })} onPointerUp={() => commit(draft, axis)} onKeyUp={() => keyboardCommit(axis)} onBlur={() => commit(draft, axis)} /><span>{draft[axis]}u</span></label>)}
    <div className="wb-key-orientation" role="group" aria-label="Key orientation">{(['Wide', 'Tall'] as const).map((label) => <button key={label} aria-pressed={!mixed && (label === 'Wide' ? draft.x >= draft.y : draft.y > draft.x)} onClick={() => { const long = Math.max(draft.x, draft.y); const short = Math.min(draft.x, draft.y); const next = label === 'Wide' ? { x: long, y: short } : { x: short, y: long }; setDraft(next); commit(next); }}>{label}</button>)}</div>
  </section>;
}
