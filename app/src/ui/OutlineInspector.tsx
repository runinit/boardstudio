import { useEffect, useState } from 'react';
import { OutlineFeatureEditor } from './OutlineFeatureEditor';
import { InspectorSection } from './InspectorSection';
import { OutlineToolIcon } from './OutlineTools';
import type { OutlineSelection, OutlineDrawMode } from './outlineEditing';
import { defaultOutlineSettings } from '../../../contracts/src/index';
import type { Board, OutlineFeature, OutlineSettings, Part, PartDefinition, ProjectDoc } from '../../../contracts/src/index';

type Automatic = Extract<OutlineFeature, { kind: 'part-envelope' }>;

function keycapFallback(definition: PartDefinition) {
  if (definition.courtyard.length < 3) return { x: 18, y: 18 };
  return { x: Math.max(...definition.courtyard.map((p) => p.x)) - Math.min(...definition.courtyard.map((p) => p.x)), y: Math.max(...definition.courtyard.map((p) => p.y)) - Math.min(...definition.courtyard.map((p) => p.y)) };
}

function Dimension({ label, value, positive = false, onCommit }: { label: string; value: number; positive?: boolean; onCommit: (value: number) => void }) {
  const [draft, setDraft] = useState(String(value));
  useEffect(() => setDraft(String(value)), [value]);
  const valid = draft.trim() !== '' && Number.isFinite(Number(draft)) && (positive ? Number(draft) > 0 : Number(draft) >= 0);
  return <label className="wb-coordinate"><span>{label}</span><span className="wb-coordinate-input">
    <input aria-label={label} type="number" step="0.1" min={positive ? 0.001 : 0} value={draft} aria-invalid={!valid}
      onChange={(event) => setDraft(event.target.value)} onBlur={() => { if (valid && Number(draft) !== value) onCommit(Number(draft)); }}
      onKeyDown={(event) => { if (event.key === 'Enter') event.currentTarget.blur(); if (event.key === 'Escape') setDraft(String(value)); }} /><small>mm</small>
  </span>{!valid && <small role="alert">Enter a {positive ? 'positive' : 'nonnegative'} dimension.</small>}</label>;
}

export function OutlineInspector({ document, board, onChange, onDraw, selection, onSelect, selectedPoint, onSelectPoint, grid, onGrid }: {
  document: ProjectDoc; board?: Board; onChange: (document: ProjectDoc, ids: string[]) => void;
  onDraw: (operation: OutlineDrawMode) => void; selection: OutlineSelection | null;
  onSelect: (selection: OutlineSelection | null) => void; selectedPoint: number; onSelectPoint: (index: number) => void;
  grid: number; onGrid: (grid: number) => void;
}) {
  if (!board) return <p className="wb-empty-note">Add a board to generate its outline.</p>;
  const features = board.outlineIds.map(id => document.outline.find(feature => feature.id === id)).filter((feature): feature is OutlineFeature => !!feature);
  const automatic = features.find((feature): feature is Automatic => feature.kind === 'part-envelope');
  const authored = features.filter(feature => feature.kind !== 'part-envelope');
  const settings = automatic?.settings ?? defaultOutlineSettings;
  const selected = features.find(feature => feature.id === selection?.featureId && (feature.kind !== 'part-envelope' || feature.connections?.some(connection => connection.id === selection.connectionId)));
  const edit = (feature: OutlineFeature) => onChange({ ...document, outline: document.outline.map(item => item.id === feature.id ? feature : item) }, [board.id, feature.id]);
  if (selected) {
    const title = selected.kind === 'part-envelope'
      ? `Connection ${(selected.connections?.findIndex(connection => connection.id === selection?.connectionId) ?? 0) + 1}`
      : `${selected.operation === 'add' ? 'Addition' : 'Cutout'} ${authored.indexOf(selected) + 1}`;
    return <OutlineFeatureEditor feature={selected} connectionId={selection?.connectionId} title={title} parts={document.parts.filter(part => board.partIds.includes(part.id))} selectedPoint={selectedPoint} onSelectPoint={onSelectPoint} grid={grid} onGrid={onGrid} onChange={edit} onClose={() => onSelect(null)} />;
  }
  const update = (patch: Partial<Automatic>) => {
    const next: Automatic = automatic ? { ...automatic, ...patch } : { id: crypto.randomUUID(), kind: 'part-envelope', partIds: board.partIds, margin: 4, operation: 'add', settings: defaultOutlineSettings, ...patch };
    onChange({ ...document, outline: automatic ? document.outline.map(feature => feature.id === next.id ? next : feature) : [next, ...document.outline], boards: document.boards.map(item => item.id === board.id ? { ...item, outlineIds: automatic ? item.outlineIds : [next.id, ...item.outlineIds] } : item) }, [board.id, next.id]);
  };
  return <div className="wb-outline-panel">
    <div className="wb-inspect-head"><h2>Board outline</h2><span className="wb-mini-tag">{board.name}</span></div>
    <p className="wb-inspector-description">Follows your keycaps and included components as the layout changes.</p>
    {!automatic ? <button className="wb-primary" onClick={() => update({})}>Generate automatic outline</button> : <InspectorSection title="Automatic outline" detail={`${automatic.margin} mm margin`} defaultOpen>
      <div className="wb-outline-settings">
        <Dimension label="Outline margin" value={automatic.margin} onCommit={margin => update({ margin })} />
        <label className="wb-outline-select"><span>Corners</span><select aria-label="Outline corners" value={settings.corners} onChange={event => update({ settings: { ...settings, corners: event.target.value as OutlineSettings['corners'] } })}>
          <option value="sharp">Sharp</option><option value="fillet">Fillet</option><option value="chamfer">Chamfer</option>
        </select></label>
        {settings.corners !== 'sharp' && <Dimension label={settings.corners === 'fillet' ? 'Fillet radius' : 'Chamfer size'} value={settings.size} onCommit={size => update({ settings: { ...settings, size } })} />}
        <Dimension label="Bridge width" positive value={settings.bridgeWidth} onCommit={bridgeWidth => update({ settings: { ...settings, bridgeWidth } })} />
      </div>
      <p className="wb-empty-note">Separate groups connect automatically. Tight corners use a smaller radius or chamfer when needed; review adjustments in Findings.</p>
    </InspectorSection>}
    <div className="wb-outline-manual">
      <h3 className="wb-subtitle">Manual geometry</h3>
      <div className="wb-outline-tools">{([
        ['add', 'Draw addition', 'Extend the board edge'],
        ['subtract', 'Draw cutout', 'Create an opening'],
        ['connect', 'Connect points', 'Join groups along a path'],
      ] as const).map(([operation, label, description]) => <button key={operation} aria-label={label} disabled={operation === 'connect' && !automatic} title={operation === 'connect' && !automatic ? 'Generate an automatic outline first.' : undefined} onClick={() => onDraw(operation)}>
        <OutlineToolIcon kind={operation} /><span><strong>{label}</strong><small>{description}</small></span><span className="wb-outline-tool-arrow" aria-hidden="true"><OutlineToolIcon kind="back" /></span>
      </button>)}</div>
    </div>
    {(authored.length > 0 || !!automatic?.connections?.length) && <div className="wb-outline-feature-list" role="group" aria-label="Saved outline features">
      <h3 className="wb-subtitle">Saved geometry</h3>
      {authored.map((feature, index) => <div className="wb-outline-feature-row" key={feature.id}>
        <button aria-label={`Edit ${feature.operation === 'add' ? 'addition' : 'cutout'} ${index + 1}`} onClick={() => onSelect({ featureId: feature.id })}><OutlineToolIcon kind={feature.operation} /><span><strong>{feature.operation === 'add' ? 'Addition' : 'Cutout'} {index + 1}</strong><small>{feature.kind === 'polygon' ? `${feature.points.length} points` : 'Rectangle'} · {feature.anchorPartId ? 'Attached' : 'Fixed'}</small></span></button>
        <button className="wb-outline-remove" aria-label={`Remove ${feature.operation === 'add' ? 'addition' : 'cutout'} ${index + 1}`} onClick={() => {
          const boards = document.boards.map(item => item.id === board.id ? { ...item, outlineIds: item.outlineIds.filter(id => id !== feature.id) } : item);
          const retained = boards.some(item => item.outlineIds.includes(feature.id));
          onChange({ ...document, boards, outline: retained ? document.outline : document.outline.filter(item => item.id !== feature.id) }, [board.id, feature.id]);
        }}><OutlineToolIcon kind="remove" /></button>
      </div>)}
      {automatic?.connections?.map((connection, index) => <div className="wb-outline-feature-row" key={connection.id}>
        <button aria-label={`Edit connection ${index + 1}`} onClick={() => onSelect({ featureId: automatic.id, connectionId: connection.id })}><OutlineToolIcon kind="connect" /><span><strong>Connection {index + 1}</strong><small>{connection.points.length} points · {connection.width} mm wide</small></span></button>
        <button className="wb-outline-remove" aria-label={`Remove connection ${index + 1}`} onClick={() => update({ connections: automatic.connections?.filter(item => item.id !== connection.id) })}><OutlineToolIcon kind="remove" /></button>
      </div>)}
    </div>}
  </div>;
}

export function PartOutlineControls({ part, definition, onChange }: { part: Part; definition: PartDefinition; onChange: (part: Part) => void }) {
  const override = part.outline;
  const keycap = part.keycap ?? definition.keycap;
  const fallback = keycapFallback(definition);
  return <fieldset disabled={part.locked} className="wb-constraint-form">
    <legend>Board outline contribution</legend>
    <label><input type="checkbox" aria-label="Include in outline" checked={!override?.excluded} onChange={(event) => onChange({ ...part, outline: { ...override, excluded: !event.target.checked } })} /> Include in outline</label>
    {!override?.excluded && <>
      <label><input type="checkbox" aria-label="Use board margin" checked={override?.margin === undefined || override.margin === null} onChange={(event) => onChange({ ...part, outline: { ...override, margin: event.target.checked ? undefined : 0 } })} /> Use board margin</label>
      {override?.margin !== undefined && override.margin !== null && <Dimension label="Part edge margin" value={override.margin} onCommit={(margin) => onChange({ ...part, outline: { ...override, margin } })} />}
    </>}
    {definition.kind === 'switch' && <>
      {part.keycap && <button onClick={() => onChange({ ...part, keycap: undefined })}>Use definition keycap</button>}
      {!keycap && <p className="wb-constraint-note">Keycap dimensions missing: using the courtyard.</p>}
      <Dimension label="Keycap width" positive value={(keycap ?? fallback).x} onCommit={(x) => onChange({ ...part, keycap: { ...(keycap ?? fallback), x } })} />
      <Dimension label="Keycap depth" positive value={(keycap ?? fallback).y} onCommit={(y) => onChange({ ...part, keycap: { ...(keycap ?? fallback), y } })} />
    </>}
    <small>Zero margin allows edge placement. Excluding a part does not check pad support.</small>
  </fieldset>;
}

export function DefinitionKeycapControls({ definition, onChange }: { definition: PartDefinition; onChange: (keycap: PartDefinition['keycap']) => void }) {
  if (definition.kind !== 'switch') return null;
  const size = definition.keycap ?? keycapFallback(definition);
  return <div className="wb-constraint-form">
    <strong>Keycap envelope</strong>
    {!definition.keycap && <small>Using the courtyard until keycap dimensions are set.</small>}
    <Dimension label="Definition keycap width" positive value={size.x} onCommit={(x) => onChange({ ...size, x })} />
    <Dimension label="Definition keycap depth" positive value={size.y} onCommit={(y) => onChange({ ...size, y })} />
  </div>;
}
