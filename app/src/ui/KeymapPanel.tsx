import { useState } from 'react';
import type { KeycapKeySettings, KeycapMatrixSettings, KeycapProfile, ProjectDoc } from '@boardstudio/v2-contracts';
import { defaultKeycapBoard, defaultKeycapKey, defaultKeycapMatrix, keyBinding, keycapConfiguration, withKeyBinding } from '../keycapSettings';
import { bindingLabel, keyBindingChoices } from './keyBindingChoices';
import { InspectorSection } from './InspectorSection';
import './keymap.css';

const profiles: readonly (readonly [KeycapProfile, string])[] = [['cherry', 'Cherry'], ['oem', 'OEM'], ['dcs', 'DCS'], ['dsa', 'DSA'], ['sa', 'SA'], ['hi-pro', 'Hi-Pro'], ['g20', 'G20'], ['choc', 'Choc']];
const profileOptions = <><option value="">No generated keycap</option>{profiles.map(([id, label]) => <option key={id} value={id}>{label}</option>)}</>;
export function KeymapPanel({ document, boardId, selectedKeyId, onChange, onSelect, onExport, onExportKeycaps, findings, firmwareControls }: {
  document: ProjectDoc; boardId: string; selectedKeyId?: string;
  onChange: (document: ProjectDoc) => void; onSelect: (id: string) => void; onExport: () => void; onExportKeycaps: () => void;
  findings: string[]; firmwareControls?: React.ReactNode;
}) {
  const board = document.boards.find(board => board.id === boardId);
  const keys = document.parts.filter(part => board?.partIds.includes(part.id) && document.definitions.find(definition => definition.id === part.definitionId)?.kind === 'switch');
  const part = keys.find(key => key.id === selectedKeyId);
  const config = keycapConfiguration(document);
  const colors = config.boards[boardId] ?? defaultKeycapBoard;
  const override = part ? config.keys[part.id] ?? defaultKeycapKey : defaultKeycapKey;
  const binding = part ? keyBinding(document, boardId, part.id) : '&none';
  const matrices = document.matrices.filter(matrix => matrix.boardId === boardId || matrix.partIds.some(id => board?.partIds.includes(id)));
  const [query, setQuery] = useState('');
  const setKey = (change: Partial<KeycapKeySettings>) => { if (part) onChange({ ...document, keycaps: { ...config, keys: { ...config.keys, [part.id]: { ...override, ...change } } } }); };
  const setMatrix = (id: string, change: Partial<KeycapMatrixSettings>) => onChange({ ...document, keycaps: { ...config, matrices: { ...config.matrices, [id]: { ...(config.matrices[id] ?? defaultKeycapMatrix), ...change } } } });
  return <div className="wb-keymap-panel">
    <div className="wb-inspect-head"><h2>Keymap & keycaps</h2><span>{keys.filter(key => keyBinding(document, boardId, key.id) !== '&none').length}/{keys.length} assigned</span></div>
    {!keys.length && <p className="wb-empty-note">Add switches in Layout to create a keymap.</p>}
    <InspectorSection title="Board colors" defaultOpen>
      <label>Keycap color<input aria-label="Board keycap color" type="color" value={colors.color} onChange={event => onChange({ ...document, keycaps: { ...config, boards: { ...config.boards, [boardId]: { ...colors, color: event.target.value } } } })} /></label>
      <label>Legend color<input aria-label="Board legend color" type="color" value={colors.legendColor} onChange={event => onChange({ ...document, keycaps: { ...config, boards: { ...config.boards, [boardId]: { ...colors, legendColor: event.target.value } } } })} /></label>
      <label>Minimum clearance (mm)<input aria-label="Keycap clearance" type="number" min="0" max="5" step="0.1" value={colors.clearance} onChange={event => { if (event.target.value !== '') onChange({ ...document, keycaps: { ...config, boards: { ...config.boards, [boardId]: { ...colors, clearance: Number(event.target.value) } } } }); }} /></label>
    </InspectorSection>
    <InspectorSection title="Matrix profiles" defaultOpen>
      {matrices.map(matrix => {
        const setting = config.matrices[matrix.id] ?? defaultKeycapMatrix;
        return <fieldset className="wb-keycap-matrix" key={matrix.id}><legend>{matrix.name ?? matrix.id}</legend>
          <label>Profile<select aria-label={`Keycap profile for ${matrix.name ?? matrix.id}`} value={setting.profile ?? ''} onChange={event => setMatrix(matrix.id, { profile: event.target.value as KeycapProfile || null })}>{profileOptions}</select></label>
          <details><summary>Profile dimensions & socket</summary>
            <label>First profile row<input aria-label={`First profile row for ${matrix.name ?? matrix.id}`} type="number" min="1" max="5" step="1" value={setting.firstRow} onChange={event => { if (event.target.value) setMatrix(matrix.id, { firstRow: Number(event.target.value) }); }} /></label>
            <label>Wall thickness (mm)<input type="number" min="0.8" max="2" step="0.1" value={setting.wallThickness} onChange={event => { if (event.target.value) setMatrix(matrix.id, { wallThickness: Number(event.target.value) }); }} /></label>
            <label>Socket<select aria-label={`Keycap socket for ${matrix.name ?? matrix.id}`} value={setting.mount ?? ''} onChange={event => setMatrix(matrix.id, { mount: event.target.value as KeycapMatrixSettings['mount'] || null })}><option value="">From switch profile</option><option value="mx">MX cross</option><option value="choc-v1">Choc v1</option><option value="choc-v2">Choc v2 cross</option><option value="alps">Alps</option></select></label>
          </details>
        </fieldset>;
      })}
      {!matrices.length && <p className="wb-empty-note">Standalone switches use their individual profile override.</p>}
    </InspectorSection>
    <InspectorSection title={part ? `${part.reference} · key` : 'Select a key'} defaultOpen>
      <label>Find a key<input type="search" aria-label="Find a key" value={query} onChange={event => setQuery(event.target.value)} /></label>
      <label>Selected key<select aria-label="Selected key" value={part?.id ?? ''} onChange={event => onSelect(event.target.value)}><option value="">Choose on the layout…</option>{keys.filter(key => `${key.reference} ${bindingLabel(keyBinding(document, boardId, key.id))}`.toLowerCase().includes(query.toLowerCase())).map(key => <option key={key.id} value={key.id}>{key.reference} · {bindingLabel(keyBinding(document, boardId, key.id)) || 'Unassigned'}</option>)}</select></label>
      {part && <div key={part.id}>
        <label>ZMK binding<select aria-label={`Binding for ${part.reference}`} value={binding} onChange={event => onChange(withKeyBinding(document, boardId, part.id, event.target.value))}>{!keyBindingChoices.some(([value]) => value === binding) && <option value={binding}>{binding}</option>}{keyBindingChoices.map(([value, label]) => <option key={value} value={value}>{label}</option>)}</select></label>
        <label>Legend<input aria-label={`Legend for ${part.reference}`} maxLength={12} defaultValue={override.legend ?? ''} key={`${part.id}:${override.legend ?? 'auto'}`} placeholder={bindingLabel(binding) || 'From binding'} onBlur={event => { if (event.target.value !== (override.legend ?? '')) setKey({ legend: event.target.value }); }} /></label>
        <div className="wb-keymap-actions"><button onClick={() => setKey({ legend: null })}>Use binding legend</button><button onClick={() => setKey({ legend: '' })}>Blank keycap</button></div>
        <label>Keycap color<input aria-label={`Keycap color for ${part.reference}`} type="color" value={override.color ?? colors.color} onChange={event => setKey({ color: event.target.value })} /></label>
        {override.color && <button onClick={() => setKey({ color: null })}>Use board color</button>}
        <details><summary>Keycap overrides</summary>
          <label>Profile<select aria-label={`Profile override for ${part.reference}`} value={override.profile ?? ''} onChange={event => setKey({ profile: event.target.value as KeycapProfile || null })}><option value="">From matrix</option>{profiles.map(([id, label]) => <option key={id} value={id}>{label}</option>)}</select></label>
          <label>Socket<select aria-label={`Socket override for ${part.reference}`} value={override.mount ?? ''} onChange={event => setKey({ mount: event.target.value as KeycapKeySettings['mount'] || null })}><option value="">From matrix / switch</option><option value="mx">MX cross</option><option value="choc-v1">Choc v1</option><option value="choc-v2">Choc v2 cross</option><option value="alps">Alps</option></select></label>
          <label>Profile row<input type="number" aria-label={`Profile row for ${part.reference}`} min="1" max="5" value={override.row ?? ''} placeholder="From matrix" onChange={event => setKey({ row: event.target.value ? Number(event.target.value) : null })} /></label>
          <label>Width (u)<input aria-label={`Keycap width for ${part.reference}`} type="number" min="0.75" max="7" step="0.25" value={override.units?.x ?? ''} placeholder="From envelope" onChange={event => setKey({ units: event.target.value ? { x: Number(event.target.value), y: override.units?.y ?? 1 } : null })} /></label>
          <label>Depth (u)<input aria-label={`Keycap depth for ${part.reference}`} type="number" min="0.75" max="7" step="0.25" value={override.units?.y ?? ''} placeholder="From envelope" onChange={event => setKey({ units: event.target.value ? { x: override.units?.x ?? 1, y: Number(event.target.value) } : null })} /></label>
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
