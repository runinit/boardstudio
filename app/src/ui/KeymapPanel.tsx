import { useState } from 'react';
import type { EditOperation, KeycapBoardChange, KeycapKeyChange, KeycapMatrixChange, KeycapKeySettings, KeycapMatrixSettings, KeycapProfile } from '@boardstudio/v2-contracts';
import { defaultKeycapKey } from '../keycapSettings';
import type { KeymapView } from './createKeymapWorkspace';
import { bindingLabel, keyBindingChoices } from './keyBindingChoices';
import { InspectorSection } from './InspectorSection';
import './keymap.css';

const profiles: readonly (readonly [KeycapProfile, string])[] = [['cherry', 'Cherry'], ['oem', 'OEM'], ['dcs', 'DCS'], ['dsa', 'DSA'], ['sa', 'SA'], ['hi-pro', 'Hi-Pro'], ['g20', 'G20'], ['choc', 'Choc']];
const profileOptions = <><option value="">No generated keycap</option>{profiles.map(([id, label]) => <option key={id} value={id}>{label}</option>)}</>;
export function KeymapPanel({ view, selectedKeyId, onEdit, onSelect, onExport, onExportKeycaps, findings, firmwareControls }: {
  view: KeymapView; selectedKeyId?: string;
  onEdit: (operation: EditOperation) => void; onSelect: (id: string) => void; onExport: () => void; onExportKeycaps: () => void;
  findings: string[]; firmwareControls?: React.ReactNode;
}) {
  const { boardId, keys, colors, matrices } = view;
  const selectedKey = keys.find(key => key.part.id === selectedKeyId);
  const part = selectedKey?.part;
  const override = selectedKey?.settings ?? defaultKeycapKey;
  const binding = selectedKey?.binding ?? '&none';
  const [query, setQuery] = useState('');
  const setKey = (change: KeycapKeyChange) => { if (part) onEdit({ kind: 'set-keycap-key', keyId: part.id, change }); };
  const setMatrix = (matrixId: string, change: KeycapMatrixChange) => onEdit({ kind: 'set-matrix-keycaps', matrixId, change });
  const setBoard = (change: KeycapBoardChange) => onEdit({ kind: 'set-keycap-board', boardId, change });
  return <div className="wb-keymap-panel">
    <div className="wb-inspect-head"><h2>Keymap & keycaps</h2><span>{keys.filter(key => key.binding !== '&none').length}/{keys.length} assigned</span></div>
    {!keys.length && <p className="wb-empty-note">Add switches in Layout to create a keymap.</p>}
    <InspectorSection title="Board colors" defaultOpen>
      <label>Keycap color<input aria-label="Board keycap color" type="color" value={colors.color} onChange={event => setBoard({ kind: 'color', value: event.target.value })} /></label>
      <label>Legend color<input aria-label="Board legend color" type="color" value={colors.legendColor} onChange={event => setBoard({ kind: 'legend-color', value: event.target.value })} /></label>
      <label>Minimum clearance (mm)<input aria-label="Keycap clearance" type="number" min="0" max="5" step="0.1" value={colors.clearance} onChange={event => { if (event.target.value !== '') setBoard({ kind: 'clearance', value: Number(event.target.value) }); }} /></label>
    </InspectorSection>
    <InspectorSection title="Matrix profiles" defaultOpen>
      {matrices.map(matrix => {
        const setting = matrix.settings;
        return <fieldset className="wb-keycap-matrix" key={matrix.id}><legend>{matrix.name}</legend>
          <label>Profile<select aria-label={`Keycap profile for ${matrix.name}`} value={setting.profile ?? ''} onChange={event => setMatrix(matrix.id, { kind: 'profile', value: event.target.value as KeycapProfile || null })}>{profileOptions}</select></label>
          <details><summary>Profile dimensions & socket</summary>
            <label>First profile row<input aria-label={`First profile row for ${matrix.name}`} type="number" min="1" max="5" step="1" value={setting.firstRow} onChange={event => { if (event.target.value) setMatrix(matrix.id, { kind: 'first-row', value: Number(event.target.value) }); }} /></label>
            <label>Wall thickness (mm)<input type="number" min="0.8" max="2" step="0.1" value={setting.wallThickness} onChange={event => { if (event.target.value) setMatrix(matrix.id, { kind: 'wall-thickness', value: Number(event.target.value) }); }} /></label>
            <label>Socket<select aria-label={`Keycap socket for ${matrix.name}`} value={setting.mount ?? ''} onChange={event => setMatrix(matrix.id, { kind: 'mount', value: event.target.value as KeycapMatrixSettings['mount'] || null })}><option value="">From switch profile</option><option value="mx">MX cross</option><option value="choc-v1">Choc v1</option><option value="choc-v2">Choc v2 cross</option><option value="alps">Alps</option></select></label>
          </details>
        </fieldset>;
      })}
      {!matrices.length && <p className="wb-empty-note">Standalone switches use their individual profile override.</p>}
    </InspectorSection>
    <InspectorSection title={part ? `${part.reference} · key` : 'Select a key'} defaultOpen>
      <label>Find a key<input type="search" aria-label="Find a key" value={query} onChange={event => setQuery(event.target.value)} /></label>
      <label>Selected key<select aria-label="Selected key" value={part?.id ?? ''} onChange={event => onSelect(event.target.value)}><option value="">Choose on the layout…</option>{keys.filter(key => `${key.part.reference} ${bindingLabel(key.binding)}`.toLowerCase().includes(query.toLowerCase())).map(key => <option key={key.part.id} value={key.part.id}>{key.part.reference} · {bindingLabel(key.binding) || 'Unassigned'}</option>)}</select></label>
      {part && <div key={part.id}>
        <label>ZMK binding<select aria-label={`Binding for ${part.reference}`} value={binding} onChange={event => onEdit({ kind: 'set-key-binding', boardId, keyId: part.id, binding: event.target.value })}>{!keyBindingChoices.some(([value]) => value === binding) && <option value={binding}>{binding}</option>}{keyBindingChoices.map(([value, label]) => <option key={value} value={value}>{label}</option>)}</select></label>
        <label>Legend<input aria-label={`Legend for ${part.reference}`} maxLength={12} defaultValue={override.legend ?? ''} key={`${part.id}:${override.legend ?? 'auto'}`} placeholder={bindingLabel(binding) || 'From binding'} onBlur={event => { if (event.target.value !== (override.legend ?? '')) setKey({ kind: 'legend', value: event.target.value }); }} /></label>
        <div className="wb-keymap-actions"><button onClick={() => setKey({ kind: 'legend', value: null })}>Use binding legend</button><button onClick={() => setKey({ kind: 'legend', value: '' })}>Blank keycap</button></div>
        <label>Keycap color<input aria-label={`Keycap color for ${part.reference}`} type="color" value={override.color ?? colors.color} onChange={event => setKey({ kind: 'color', value: event.target.value })} /></label>
        {override.color && <button onClick={() => setKey({ kind: 'color', value: null })}>Use board color</button>}
        <details><summary>Keycap overrides</summary>
          <label>Profile<select aria-label={`Profile override for ${part.reference}`} value={override.profile ?? ''} onChange={event => setKey({ kind: 'profile', value: event.target.value as KeycapProfile || null })}><option value="">From matrix</option>{profiles.map(([id, label]) => <option key={id} value={id}>{label}</option>)}</select></label>
          <label>Socket<select aria-label={`Socket override for ${part.reference}`} value={override.mount ?? ''} onChange={event => setKey({ kind: 'mount', value: event.target.value as KeycapKeySettings['mount'] || null })}><option value="">From matrix / switch</option><option value="mx">MX cross</option><option value="choc-v1">Choc v1</option><option value="choc-v2">Choc v2 cross</option><option value="alps">Alps</option></select></label>
          <label>Profile row<input type="number" aria-label={`Profile row for ${part.reference}`} min="1" max="5" value={override.row ?? ''} placeholder="From matrix" onChange={event => setKey({ kind: 'row', value: event.target.value ? Number(event.target.value) : null })} /></label>
          <label>Width (u)<input aria-label={`Keycap width for ${part.reference}`} type="number" min="0.75" max="7" step="0.25" value={override.units?.x ?? ''} placeholder="From envelope" onChange={event => setKey({ kind: 'units', value: event.target.value ? { x: Number(event.target.value), y: override.units?.y ?? 1 } : null })} /></label>
          <label>Depth (u)<input aria-label={`Keycap depth for ${part.reference}`} type="number" min="0.75" max="7" step="0.25" value={override.units?.y ?? ''} placeholder="From envelope" onChange={event => setKey({ kind: 'units', value: event.target.value ? { x: override.units?.x ?? 1, y: Number(event.target.value) } : null })} /></label>
        </details>
      </div>}
    </InspectorSection>
    {firmwareControls && <InspectorSection title="All firmware positions">{firmwareControls}</InspectorSection>}
    <InspectorSection title="Clearance findings" detail={`${findings.length}`} defaultOpen={findings.length > 0}>
      {findings.map((finding, index) => <p key={index} className="wb-empty-note">{finding}</p>)}
      <p className="wb-empty-note">Checks use conservative keycap envelopes through full switch travel. Generate the Case assembly to check its walls and solids.</p>
    </InspectorSection>
    <button disabled={!keys.length} onClick={onExportKeycaps}>Export keycap STEP</button>
    <button className="wb-primary" disabled={!keys.length} onClick={onExport}>Export ZMK source</button>
    <p className="wb-empty-note">Includes the keymap, resolved scan pins, and a local build script. Configure the controller and wiring in PCB before export.</p>
  </div>;
}
