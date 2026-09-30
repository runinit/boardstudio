import { useEffect, useState } from 'react';
import { OutlineFeatureEditor } from './OutlineFeatureEditor';
import { InspectorSection } from './InspectorSection';
import { OutlineToolIcon } from './OutlineTools';
import type { OutlineSelection, OutlineDrawMode } from './outlineEditing';
import { defaultOutlineSettings } from '../../../contracts/src/index';
import type { Board, OutlineFeature, OutlineSettings, Part, PartDefinition, ProjectDoc } from '../../../contracts/src/index';
import type { BoardOutlineScene, EditOperation, OutlineBridge, OutlineGap, OutlineRepairSettings } from '@boardstudio/v2-contracts';
import { editableOutlineFeatures, nextOutlineName, outlineVersion, updateOutlineFeature } from './boardOutlines';

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

export function OutlineInspector({ document, board, onChange, onDraw, selection, onSelect, selectedPoint, onSelectPoint, grid, gridSelection, onGrid, outlineScene, onVersion, onFeatureEdit, onFocusGap, bridge, onCloseBridge }: {
  document: ProjectDoc; board?: Board; onChange: (document: ProjectDoc, ids: string[]) => void;
  onDraw: (operation: OutlineDrawMode) => void; selection: OutlineSelection | null;
  onSelect: (selection: OutlineSelection | null) => void; selectedPoint: number; onSelectPoint: (index: number) => void;
  grid: number; gridSelection?:number; onGrid: (grid: number) => void;
  outlineScene?: BoardOutlineScene; onVersion: (operation: EditOperation) => void;
  onFeatureEdit: (feature: OutlineFeature) => void; onFocusGap: (gap: OutlineGap) => void;
  bridge?: OutlineBridge; onCloseBridge: () => void;
}) {
  const version = board ? outlineVersion(document, board.id) : undefined;
  const [name, setName] = useState(version?.name ?? '');
  useEffect(() => setName(version?.name ?? ''), [version?.id, version?.name]);
  if (!board) return <p className="wb-empty-note">Add a board to generate its outline.</p>;
  const state = document.boardOutlines?.find(state => state.boardId === board.id);
  const generated = board.outlineIds.map(id => document.outline.find(feature => feature.id === id)).filter((feature): feature is OutlineFeature => !!feature);
  const automatic = generated.find((feature): feature is Automatic => feature.kind === 'part-envelope');
  const features = editableOutlineFeatures(document, board, outlineScene);
  const authored = features.filter(feature => feature.kind !== 'part-envelope');
  const settings: OutlineSettings = version?.geometry.settings ?? automatic?.settings ?? defaultOutlineSettings;
  const repair: OutlineRepairSettings = { enabled: true, maximumGapSpan: 20, minimumConnectionWidth: 2, edgeClearance: 0, ...settings.repair };
  const selected = selection?.contour !== undefined ? features[selection.contour] : features.find(feature => feature.id === selection?.featureId)
    ?? generated.find(feature => feature.id === selection?.featureId && feature.kind === 'part-envelope' && feature.connections?.some(connection => connection.id === selection.connectionId));
  const edit = (feature: OutlineFeature) => onFeatureEdit(feature);
  if (selected) {
    const title = selected.kind === 'part-envelope'
      ? `Connection ${(selected.connections?.findIndex(connection => connection.id === selection?.connectionId) ?? 0) + 1}`
      : selected.operation === 'subtract' ? 'Cutout' : selected.id.includes(':contour:') || selected.id.startsWith('outline-source:') ? 'Perimeter' : 'Addition';
    return <OutlineFeatureEditor feature={selected} connectionId={selection?.connectionId} title={title} parts={document.parts.filter(part => board.partIds.includes(part.id))} selectedPoint={selectedPoint} onSelectPoint={onSelectPoint} grid={grid} gridSelection={gridSelection} onGrid={onGrid} onChange={edit} onClose={() => onSelect(null)} fixed={!!version} createsCopy={!version && selected.kind !== 'part-envelope'} />;
  }
  if (bridge) return <div className="wb-outline-panel">
    <div className="wb-inspect-head"><h2>Outline bridge</h2><button className="wb-secondary" onClick={onCloseBridge}>Done</button></div>
    <dl className="wb-measure-list"><div><dt>Width</dt><dd>{bridge.width} mm</dd></div><div><dt>Source</dt><dd>{bridge.authored ? 'Authored connection' : 'Automatic connection'}</dd></div></dl>
    <p className="wb-inspector-description">{bridge.partIds.map(id => document.parts.find(part => part.id === id)?.reference ?? id).join(' to ')}</p>
    <p className="wb-empty-note">{version ? 'This bridge belongs to the fixed copy. Edit its surrounding perimeter to change the connection.' : 'The highlighted connection joins these component groups. Its source follows the layout.'}</p>
  </div>;
  const update = (patch: Partial<Automatic>) => {
    const next: Automatic = automatic ? { ...automatic, ...patch } : { id: crypto.randomUUID(), kind: 'part-envelope', partIds: board.partIds, margin: 4, operation: 'add', settings: defaultOutlineSettings, ...patch };
    onChange({ ...document, outline: automatic ? document.outline.map(feature => feature.id === next.id ? next : feature) : [next, ...document.outline], boards: document.boards.map(item => item.id === board.id ? { ...item, outlineIds: automatic ? item.outlineIds : [next.id, ...item.outlineIds] } : item) }, [board.id, next.id]);
  };
  const updateSettings = (patch: Partial<OutlineSettings>) => {
    if (!version) { update({ settings: { ...settings, ...patch } }); return; }
    onChange({ ...document, boardOutlines: document.boardOutlines?.map(state => state.boardId !== board.id ? state : {
      ...state, versions: state.versions.map(item => item.id !== version.id ? item : { ...item, geometry: { ...item.geometry, settings: { ...settings, ...patch } } }),
    }) }, [board.id, version.id]);
  };
  const updateRepair = (patch: Partial<OutlineRepairSettings>) => updateSettings({ repair: { ...repair, ...patch } });
  return <div className="wb-outline-panel">
    <div className="wb-inspect-head"><h2>Board outline</h2><span className="wb-mini-tag">{board.name}</span></div>
    <p className="wb-inspector-description">{version ? 'A fixed outline; component placement is shared with every version.' : 'Generated follows your keycaps and included components. Narrow recesses are repaired automatically.'}</p>
    <InspectorSection title="Outline versions" detail={version?.name ?? 'Generated'} defaultOpen>
      <label className="wb-outline-select"><span>Active outline</span><select aria-label="Active outline" value={version?.id ?? ''} onChange={event => onVersion({ kind: 'select-outline', boardId: board.id, versionId: event.target.value || null })}>
        <option value="">Generated</option>{state?.versions.map(item => <option key={item.id} value={item.id}>{item.name}</option>)}
      </select></label>
      {version && <label className="wb-outline-version-name"><span>Version name</span><input aria-label="Outline version name" value={name} maxLength={120} aria-invalid={!name.trim()}
        onChange={event => setName(event.target.value)} onBlur={() => { if (name.trim() && name.trim() !== version.name) onVersion({ kind: 'rename-outline', boardId: board.id, versionId: version.id, name: name.trim() }); }}
        onKeyDown={event => { if (event.key === 'Enter') event.currentTarget.blur(); if (event.key === 'Escape') setName(version.name); }} /></label>}
      <div className="wb-outline-version-actions"><button className="wb-secondary" onClick={() => onVersion({ kind: 'copy-outline', boardId: board.id, versionId: crypto.randomUUID(), name: nextOutlineName(document, board.id) })}>Copy outline</button>
        <button className="wb-secondary" disabled={!version} onClick={() => version && onVersion({ kind: 'remove-outline', boardId: board.id, versionId: version.id })}>Delete outline</button></div>
      <p className="wb-empty-note">Selecting a version activates it for every view and export. Copy, rename and delete support Undo.</p>
    </InspectorSection>
    <button className="wb-primary wb-outline-edit-perimeter" disabled={!features.length} onClick={() => onSelect({ featureId: features[0].id, contour: 0 })}>Edit perimeter points</button>
    {(!automatic && !version) ? <button className="wb-primary" onClick={() => update({})}>Generate automatic outline</button> : <InspectorSection title={version ? 'Outline settings' : 'Automatic outline'} detail={version ? 'Fixed' : `${automatic?.margin} mm margin`}>
      <div className="wb-outline-settings">
        {!version && automatic && <Dimension label="Outline margin" value={automatic.margin} onCommit={margin => update({ margin })} />}
        <label className="wb-outline-select"><span>Corners</span><select aria-label="Outline corners" value={settings.corners} onChange={event => updateSettings({ corners: event.target.value as OutlineSettings['corners'] })}>
          <option value="sharp">Sharp</option><option value="fillet">Fillet</option><option value="chamfer">Chamfer</option>
        </select></label>
        {settings.corners !== 'sharp' && <Dimension label={settings.corners === 'fillet' ? 'Fillet radius' : 'Chamfer size'} value={settings.size} onCommit={size => updateSettings({ size })} />}
        {!version && <Dimension label="Bridge width" positive value={settings.bridgeWidth} onCommit={bridgeWidth => updateSettings({ bridgeWidth })} />}
      </div>
      <p className="wb-empty-note">Separate groups connect automatically. Tight corners use a smaller radius or chamfer when needed; review adjustments in Findings.</p>
    </InspectorSection>}
    {!version && !!outlineScene?.gaps.length && <InspectorSection title="Gap repair" detail={`${outlineScene.gaps.length} recesses`} defaultOpen>
      <p className="wb-empty-note">Keep gap preserves an intentional recess and follows its source components.</p>
      <div className="wb-outline-gap-list">{outlineScene.gaps.map((gap, index) => <div className="wb-outline-gap-row" key={gap.id}>
        <button className="wb-outline-gap-focus" aria-label={`Show gap ${index + 1}`} onClick={() => onFocusGap(gap)}>Gap {index + 1}<small>{gap.span} mm span</small></button>
        <label><input type="checkbox" aria-label={`Keep gap ${index + 1}`} checked={gap.protected} onChange={event => {
          const owner = generated.find((feature): feature is Automatic => feature.id === gap.featureId && feature.kind === 'part-envelope');
          if (!owner) return;
          const settings: OutlineSettings = owner.settings ?? defaultOutlineSettings;
          const current = { ...repair, ...settings.repair };
          const matches = new Set([gap.id, ...(gap.protectedIds ?? [])]);
          const keepGaps = event.target.checked ? [...(current.keepGaps ?? []).filter(item => !matches.has(item.id)), { id: gap.id, points: gap.points }] : (current.keepGaps ?? []).filter(item => !matches.has(item.id));
          onChange(updateOutlineFeature(document, board, { ...owner, settings: { ...settings, repair: { ...current, keepGaps } } }), [board.id, owner.id]);
        }} />Keep gap</label>
      </div>)}</div>
    </InspectorSection>}
    <InspectorSection title="Advanced cleanup and clearance">
      {!version && <label className="wb-outline-checkbox"><input type="checkbox" aria-label="Automatic gap cleanup" checked={repair.enabled} onChange={event => updateRepair({ enabled: event.target.checked })} />Automatic cleanup</label>}
      {!version && <Dimension label="Maximum gap span" value={repair.maximumGapSpan} onCommit={maximumGapSpan => updateRepair({ maximumGapSpan })} />}
      <Dimension label="Minimum connection width" value={repair.minimumConnectionWidth} onCommit={minimumConnectionWidth => updateRepair({ minimumConnectionWidth })} />
      <Dimension label="PCB edge clearance" value={repair.edgeClearance} onCommit={edgeClearance => updateRepair({ edgeClearance })} />
      <p className="wb-empty-note">Support and clearance findings block affected fabrication exports. Editing and project saving stay available.</p>
      {!version && repair.keepGaps?.map((gap, index) => <button className="wb-secondary" key={gap.id} aria-label={`Remove protected gap ${index + 1}`} onClick={() => updateRepair({ keepGaps: repair.keepGaps?.filter(item => item.id !== gap.id) })}>Remove protected gap {index + 1}</button>)}
    </InspectorSection>
    <div className="wb-outline-manual">
      <h3 className="wb-subtitle">Manual geometry</h3>
      <div className="wb-outline-tools">{([
        ['add', 'Draw addition', 'Extend the board edge'],
        ['subtract', 'Draw cutout', 'Create an opening'],
        ['connect', 'Connect points', 'Join groups along a path'],
      ] as const).map(([operation, label, description]) => <button key={operation} aria-label={label} disabled={operation === 'connect' && (!automatic || !!version)} title={operation === 'connect' ? version ? 'Select Generated to add a linked connection.' : !automatic ? 'Generate an automatic outline first.' : undefined : undefined} onClick={() => onDraw(operation)}>
        <OutlineToolIcon kind={operation} /><span><strong>{label}</strong><small>{description}</small></span><span className="wb-outline-tool-arrow" aria-hidden="true"><OutlineToolIcon kind="back" /></span>
      </button>)}</div>
    </div>
    {(authored.length > 0 || !!automatic?.connections?.length) && <div className="wb-outline-feature-list" role="group" aria-label="Saved outline features">
      <h3 className="wb-subtitle">Saved geometry</h3>
      {authored.map((feature, index) => <div className="wb-outline-feature-row" key={feature.id}>
        <button aria-label={index === 0 && feature.operation === 'add' ? 'Edit perimeter' : `Edit ${feature.operation === 'add' ? 'addition' : 'cutout'} ${index}`} onClick={() => onSelect({ featureId: feature.id, contour: index })}><OutlineToolIcon kind={feature.operation} /><span><strong>{feature.operation === 'add' ? (index === 0 ? 'Perimeter' : 'Addition') : 'Cutout'} {index || 1}</strong><small>{feature.kind === 'polygon' ? `${feature.points.length} points` : 'Rectangle'} · {version ? 'Fixed' : 'Creates a fixed copy'}</small></span></button>
        {version && index > 0 && <button className="wb-outline-remove" aria-label={`Remove ${feature.operation === 'add' ? 'addition' : 'cutout'} ${index}`} onClick={() => {
          onChange({ ...document, boardOutlines: document.boardOutlines?.map(state => state.boardId !== board.id ? state : { ...state, versions: state.versions.map(item => item.id !== version.id ? item : { ...item, geometry: { ...item.geometry, features: item.geometry.features.filter(item => item.id !== feature.id) } }) }) }, [board.id, feature.id]);
        }}><OutlineToolIcon kind="remove" /></button>}
      </div>)}
      {!version && automatic?.connections?.map((connection, index) => <div className="wb-outline-feature-row" key={connection.id}>
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
    <label><input type="checkbox" aria-label="Allow component body overhang" checked={override?.allowBodyOverhang ?? false} onChange={event => onChange({ ...part, outline: { ...override, allowBodyOverhang: event.target.checked } })} />Allow body overhang</label>
    {definition.kind === 'switch' && <>
      {part.keycap && <button onClick={() => onChange({ ...part, keycap: undefined })}>Use definition keycap</button>}
      {!keycap && <p className="wb-constraint-note">Keycap dimensions missing: using the courtyard.</p>}
      <Dimension label="Keycap width" positive value={(keycap ?? fallback).x} onCommit={(x) => onChange({ ...part, keycap: { ...(keycap ?? fallback), x } })} />
      <Dimension label="Keycap depth" positive value={(keycap ?? fallback).y} onCommit={(y) => onChange({ ...part, keycap: { ...(keycap ?? fallback), y } })} />
    </>}
    <small>Pad and drill support remains required when a part is excluded or body overhang is allowed.</small>
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
