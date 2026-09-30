import { useId } from 'react';
import type { KeyBinding, KeymapConfiguration } from '@boardstudio/v2-contracts';
import { keyBindingChoices } from './keyBindingChoices';

export function bindingTitle(binding: KeyBinding, map: KeymapConfiguration): string {
  if ('keycode' in binding) return binding.keycode;
  if (binding.kind === 'mod-tap') return `${binding.tap} / ${binding.hold}`;
  if ('layerId' in binding) return `${'tap' in binding ? binding.tap + ' / ' : ''}${map.layers.find(layer => layer.id === binding.layerId)?.name ?? '?'}`;
  if (binding.kind === 'macro') return map.macros.find(macro => macro.id === binding.macroId)?.name ?? 'Macro';
  return binding.kind === 'transparent' ? 'Transparent' : 'Unassigned';
}
const behaviors: [KeyBinding['kind'], string][] = [
  ['key-press', 'Key press'], ['mod-tap', 'Mod tap'], ['layer-tap', 'Layer tap'],
  ['momentary-layer', 'Momentary layer'], ['toggle-layer', 'Toggle layer'], ['to-layer', 'Go to layer'],
  ['sticky-layer', 'Sticky layer'], ['sticky-key', 'Sticky key'], ['macro', 'Macro'], ['transparent', 'Transparent'], ['none', 'Unassigned'],
];
export function KeyBindingEditor({ value, map, label, onChange }: { value: KeyBinding; map: KeymapConfiguration; label: string; onChange: (binding: KeyBinding) => void }) {
  const keycodesId = useId();
  const choose = (kind: KeyBinding['kind']): KeyBinding => {
    if (kind === 'key-press' || kind === 'sticky-key') return { kind, keycode: 'A' };
    if (kind === 'mod-tap') return { kind, hold: 'LSHIFT', tap: 'A' };
    if (kind === 'layer-tap') return { kind, layerId: map.layers[1]?.id ?? map.layers[0].id, tap: 'SPACE' };
    if (kind === 'momentary-layer' || kind === 'toggle-layer' || kind === 'to-layer' || kind === 'sticky-layer') return { kind, layerId: map.layers[1]?.id ?? map.layers[0].id };
    if (kind === 'macro') return { kind, macroId: map.macros[0]?.id ?? '' };
    return { kind };
  };
  const codeInput = (field: 'keycode' | 'tap', code: string) => <label>{field === 'tap' ? 'Tap keycode' : 'Keycode'}<input key={`${label}:${field}:${code}`} aria-label={`${label} ${field}`} list={keycodesId} defaultValue={code} placeholder="Search or enter a ZMK keycode" onBlur={event => { const next = event.target.value.trim(); if (next !== code) onChange({ ...value, [field]: next } as KeyBinding); }} /></label>;
  return <div className="wb-binding-editor">
    <label>Behavior<select aria-label={`${label} behavior`} value={value.kind} onChange={event => onChange(choose(event.target.value as KeyBinding['kind']))}>{behaviors.map(([kind, title]) => <option key={kind} value={kind} disabled={kind === 'macro' && !map.macros.length}>{title}</option>)}</select></label>
    {'keycode' in value && codeInput('keycode', value.keycode)}
    {'tap' in value && codeInput('tap', value.tap)}
    {value.kind === 'mod-tap' && <label>Hold modifier<select aria-label={`${label} hold modifier`} value={value.hold} onChange={event => onChange({ ...value, hold: event.target.value })}>{['LSHIFT', 'RSHIFT', 'LCTRL', 'RCTRL', 'LALT', 'RALT', 'LGUI', 'RGUI'].map(code => <option key={code}>{code}</option>)}</select></label>}
    {'layerId' in value && <label>Layer<select aria-label={`${label} layer`} value={value.layerId} onChange={event => onChange({ ...value, layerId: event.target.value })}>{map.layers.map(layer => <option key={layer.id} value={layer.id}>{layer.name}</option>)}</select></label>}
    {value.kind === 'macro' && <label>Macro<select aria-label={`${label} macro`} value={value.macroId} onChange={event => onChange({ ...value, macroId: event.target.value })}>{map.macros.map(macro => <option key={macro.id} value={macro.id}>{macro.name}</option>)}</select></label>}
    <datalist id={keycodesId}>{keyBindingChoices.filter(([code]) => code.startsWith('&kp ')).map(([code, title]) => <option key={code} value={code.slice(4)}>{title}</option>)}</datalist>
  </div>;
}
