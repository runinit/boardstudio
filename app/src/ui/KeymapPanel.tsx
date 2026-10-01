import { useState } from 'react';
import type { KeyBinding, KeymapChange, KeymapConfiguration, KeymapLayer, KeymapMacro, MacroStep, MacroChange } from '@boardstudio/v2-contracts';
import type { KeymapView } from './createKeymapWorkspace';
import { KeyBindingEditor, bindingTitle } from './KeyBindingEditor';
import { InspectorSection } from './InspectorSection';
import './keymap.css';

export function KeymapPanel({ view, map, layer, selectedKeyId, onLayer, onSelect, bindingFor, onChange, onExport, encoders }: {
  view: KeymapView; map: KeymapConfiguration; layer: KeymapLayer; selectedKeyId?: string;
  onLayer: (id: string) => void; onSelect: (id: string) => void; bindingFor: (id: string) => KeyBinding;
  onChange: (change: KeymapChange) => void; onExport: () => void; encoders: { id: string; name: string; pushKeyId?: string | null }[];
}) {
  const [query, setQuery] = useState('');
  const [section, setSection] = useState<'keys' | 'macros' | 'encoders'>('keys');
  const selected = view.keys.find(key => key.part.id === selectedKeyId);
  const addLayer = () => { const id = crypto.randomUUID(); onChange({ kind: 'add-layer', id, name: `Layer ${map.layers.length}` }); onLayer(id); };
  return <div className="wb-keymap-panel">
    <div className="wb-inspect-head"><h2>Keymap</h2><span>{map.layers.length} layers · {view.keys.length} keys</span></div>
    <InspectorSection title="Layers" defaultOpen>
      <div className="wb-layer-list" role="group" aria-label="Keymap layers">{map.layers.map((entry, index) => <button key={entry.id} aria-pressed={entry.id === layer.id} onClick={() => onLayer(entry.id)}><span>{index}</span>{entry.name}</button>)}</div>
      <button disabled={map.layers.length >= 32} onClick={addLayer}>Add layer</button>
      <label>Layer name<input aria-label="Layer name" maxLength={32} key={`${layer.id}:${layer.name}`} defaultValue={layer.name} onBlur={event => { if (event.target.value !== layer.name) onChange({ kind: 'rename-layer', id: layer.id, name: event.target.value }); }} /></label>
      {layer.id !== map.layers[0].id && <button onClick={() => onChange({ kind: 'remove-layer', id: layer.id })}>Remove layer</button>}
      <p className="wb-empty-note">Higher layers take precedence. Transparent keys fall through to the layer below.</p>
    </InspectorSection>
    <div className="wb-keymap-actions" role="group" aria-label="Keymap editors">{(['keys', 'macros', 'encoders'] as const).map(tab => <button key={tab} aria-pressed={section === tab} onClick={() => setSection(tab)}>{tab[0].toUpperCase() + tab.slice(1)}</button>)}</div>
    {section === 'keys' && <InspectorSection title={selected ? `${selected.part.reference} · ${layer.name}` : 'Select a key'} defaultOpen>
      <label>Find a key<input type="search" aria-label="Find a key" value={query} onChange={event => setQuery(event.target.value)} /></label>
      <label>Selected key<select aria-label="Selected key" value={selected?.part.id ?? ''} onChange={event => onSelect(event.target.value)}><option value="">Choose on the layout…</option>{view.keys.filter(key => `${key.part.reference} ${bindingTitle(bindingFor(key.part.id), map)}`.toLowerCase().includes(query.toLowerCase())).map(key => <option key={key.part.id} value={key.part.id}>{key.part.reference} · {bindingTitle(bindingFor(key.part.id), map)}</option>)}</select></label>
      {selected ? <KeyBindingEditor label={selected.part.reference} value={bindingFor(selected.part.id)} map={map} onChange={binding => onChange({ kind: 'binding', layerId: layer.id, keyId: selected.part.id, binding })} /> : <p className="wb-empty-note">Select a switch on the layout to assign its behavior.</p>}
    </InspectorSection>}
    {section === 'macros' && <InspectorSection title="Macros" defaultOpen>
      {map.macros.map(macro => <MacroEditor key={macro.id} macro={macro} onChange={change => onChange({ kind: 'edit-macro', macroId: macro.id, change })} onRemove={() => onChange({ kind: 'remove-macro', id: macro.id })} />)}
      {!map.macros.length && <p className="wb-empty-note">Build a sequence of key taps, presses, releases and delays, then assign it to a key.</p>}
      <button onClick={() => onChange({ kind: 'save-macro', value: { id: crypto.randomUUID(), name: `Macro ${map.macros.length + 1}`, tapMs: 30, waitMs: 0, steps: [{ kind: 'tap', binding: { kind: 'key-press', keycode: 'A' } }] } })}>Add macro</button>
    </InspectorSection>}
    {section === 'encoders' && <InspectorSection title={`Encoders · ${layer.name}`} defaultOpen>
      {!encoders.length && <p className="wb-empty-note">Add a rotary encoder in Layout. Configure its GPIOs in PCB, then assign rotation here.</p>}
      {encoders.map(encoder => {
        const value = layer.sensors[encoder.id] ?? { clockwise: { kind: 'none' as const }, counterclockwise: { kind: 'none' as const } };
        const pushKeyId = encoder.pushKeyId ?? `${encoder.id}/push`;
        return <div key={encoder.id}><h3>{encoder.name}</h3>{(['clockwise', 'counterclockwise'] as const).map(direction => <details key={direction} open><summary>{direction === 'clockwise' ? 'Clockwise' : 'Counterclockwise'}</summary><KeyBindingEditor label={`${encoder.name} ${direction}`} value={value[direction]} map={map} onChange={binding => onChange({ kind: 'encoder', layerId: layer.id, encoderId: encoder.id, direction, binding })} /></details>)}{encoder.pushKeyId !== null && <details><summary>Push button</summary><KeyBindingEditor label={`${encoder.name} push`} value={bindingFor(pushKeyId)} map={map} onChange={binding => onChange({ kind: 'binding', layerId: layer.id, keyId: pushKeyId, binding })} /></details>}</div>;
      })}
    </InspectorSection>}
    <button className="wb-primary" disabled={!view.keys.length && !encoders.length} onClick={onExport}>Export ZMK source</button>
    <p className="wb-empty-note">Local source export. Configure controller and wiring in PCB before building firmware.</p>
  </div>;
}
function MacroEditor({ macro, onChange, onRemove }: { macro: KeymapMacro; onChange: (change: MacroChange) => void; onRemove: () => void }) {
  const update = (index: number, step: MacroStep) => onChange({ kind: 'step', index, value: step });
  return <fieldset className="wb-keycap-matrix"><legend>{macro.name}</legend>
    <label>Name<input aria-label={`Macro name ${macro.name}`} maxLength={32} key={macro.name} defaultValue={macro.name} onBlur={event => { if (event.target.value !== macro.name) onChange({ kind: 'name', value: event.target.value }); }} /></label>
    {(['tapMs', 'waitMs'] as const).map(field => <label key={field}>{field === 'tapMs' ? 'Tap duration (ms)' : 'Between actions (ms)'}<input aria-label={`${macro.name} ${field}`} type="number" min={0} max={10000} defaultValue={macro[field]} key={macro[field]} onBlur={event => { const value = Number(event.target.value); if (value !== macro[field]) onChange({ kind: field === 'tapMs' ? 'tap-ms' : 'wait-ms', value }); }} /></label>)}
    {macro.steps.map((step, index) => <div className="wb-macro-step" key={index}>
      <label>Step {index + 1}<select aria-label={`${macro.name} step ${index + 1}`} value={step.kind} onChange={event => update(index, event.target.value === 'wait' ? { kind: 'wait', ms: 100 } : { kind: event.target.value as 'tap' | 'press' | 'release', binding: { kind: 'key-press', keycode: 'A' } })}>{['tap', 'press', 'release', 'wait'].map(kind => <option key={kind}>{kind}</option>)}</select></label>
      {step.kind === 'wait' ? <label>Delay (ms)<input type="number" min={0} max={10000} defaultValue={step.ms} key={step.ms} onBlur={event => { const ms = Number(event.target.value); if (ms !== step.ms) update(index, { ...step, ms }); }} /></label> : <label>Keycode<input defaultValue={'keycode' in step.binding ? step.binding.keycode : ''} key={JSON.stringify(step.binding)} onBlur={event => { const keycode = event.target.value.trim(); if (!('keycode' in step.binding) || keycode !== step.binding.keycode) update(index, { ...step, binding: { kind: 'key-press', keycode } }); }} /></label>}
      <button disabled={macro.steps.length === 1} onClick={() => onChange({ kind: 'remove-step', index })}>Remove step {index + 1}</button>
    </div>)}
    <button disabled={macro.steps.length >= 128} onClick={() => onChange({ kind: 'add-step', value: { kind: 'tap', binding: { kind: 'key-press', keycode: 'A' } } })}>Add step</button>
    <button onClick={onRemove}>Remove macro</button>
  </fieldset>;
}
