import { useState } from 'react';
import type {
  EditCommand,
  Matrix,
  MatrixCell,
  MatrixSplayChange,
  Part,
  PartDefinition,
  ProjectDoc,
  Vec2
} from '../../../contracts/src/index';
import { assemblyName, matrixPresetDefinitions, type MatrixPresetId } from './assemblyCatalog';
import { type SwitchOrientation } from './assemblyPresets';
import { ToolIcon } from './CommandMenu';
import { CaseNumber, OrientationControl } from './InspectorControls';
import { InspectorSection } from './InspectorSection';
import { type MatrixProjection } from './matrixGeometry';
import { matrixWithPreset } from './matrixPresets';
import { resizeMatrix } from './matrixResize';
import { partCatalogLabel, partChoices } from './partsCatalog';
import { makeId, withCell } from './workbenchGeometry';
import { SelectionScope } from './workbenchTypes';

type NumberRule = Parameters<typeof CaseNumber>[0]['validation'];

export const CellInspector = ({ matrix, scope, definitions, onChange, projection, onSplay, splayAffect, onAffectChange, onPickOrigin, isMirrorTarget = false }: {
  projection?: MatrixProjection; onSplay: (column: number, change: MatrixSplayChange) => void;
  splayAffect: 'column' | 'following'; onAffectChange: (value: 'column' | 'following') => void; onPickOrigin: () => void;
  isMirrorTarget?: boolean;
  matrix: Matrix; scope: SelectionScope; definitions: PartDefinition[]; parts: Map<string, Part>; members: Map<string, string>;
  onChange: (matrix: Matrix, definitions?: PartDefinition[]) => void;
}) => {
  const field = (label: string, value: number, onCommit: (value: number) => void) => <CaseNumber key={label} label={label} value={value} unit="mm" validation="finite" onCommit={onCommit} />;
  if (scope.kind === 'row' || scope.kind === 'column') {
    const axis = scope.kind;
    const index = scope[axis] ?? 0;
    const basis = projection?.basis(index);
    const offsets = axis === 'row' ? matrix.rowOffsets ?? [] : matrix.columnOffsets ?? [];
    const offset = offsets[index] ?? { x: 0, y: 0 };
    const update = (value: Vec2) => {
      const next = [...offsets];
      while (next.length <= index) next.push({ x: 0, y: 0 });
      next[index] = value;
      onChange({ ...matrix, [axis === 'row' ? 'rowOffsets' : 'columnOffsets']: next });
    };
    return <section aria-label={`${axis} properties`}>
      {axis === 'column' && basis && <InspectorSection title="Splay & origin" defaultOpen>
        <div className="wb-splay-angle"><CaseNumber label="Splay" value={basis.splayAngle} unit="°" validation="finite" onCommit={(value) => onSplay(index, { kind: 'angle', angle: value, affect: splayAffect })} /></div>
        <label className="wb-script-select-label">Origin<select aria-label="Splay origin" value={basis.customOrigin ? 'custom' : 'base'} onChange={(event) => onSplay(index, { kind: 'origin', world: event.target.value === 'base' ? null : basis.splayOrigin })}><option value="base">Column base</option><option value="custom">Custom point</option></select></label>
        <button className="wb-secondary wb-pick-origin" onClick={onPickOrigin}><ToolIcon name="origin" />Pick origin</button>
        <div className="wb-matrix-number-grid">{field('Origin X', basis.splayOrigin.x, (x) => onSplay(index, { kind: 'origin', world: { ...basis.splayOrigin, x } }))}{field('Origin Y', basis.splayOrigin.y, (y) => onSplay(index, { kind: 'origin', world: { ...basis.splayOrigin, y } }))}</div>
        <label className="wb-script-select-label">Affect<select aria-label="Splay affects" value={splayAffect} onChange={(event) => onAffectChange(event.target.value as 'column' | 'following')}><option value="column">This column</option><option value="following">This and following</option></select></label>
        <p className="wb-empty-note">Move the origin without moving the keys.</p>

      </InspectorSection>}
      <InspectorSection title="Position & rotation" defaultOpen={axis === 'row'}>{axis === 'column' && <CaseNumber label="Stagger" value={matrix.columnStaggers?.[index] ?? 0} unit="mm" validation="finite" onCommit={(value) => { const next = Array.from({ length: matrix.columns }, (_, i) => matrix.columnStaggers?.[i] ?? 0); next[index] = value; onChange({ ...matrix, columnStaggers: next }); }} />}<div className="wb-matrix-number-grid">{field('Offset X', offset.x, (x) => update({ ...offset, x }))}{field('Offset Y', offset.y, (y) => update({ ...offset, y }))}</div><button className="wb-secondary" onClick={() => update({ x: 0, y: 0 })}>Reset offsets</button></InspectorSection>
      <InspectorSection title="Spacing"><p className="wb-empty-note">Pitch {matrix.pitch.x} × {matrix.pitch.y} mm. Edge gap {matrix.edgeGap?.x ?? 1} × {matrix.edgeGap?.y ?? 1} mm. Select the matrix to change shared spacing.</p></InspectorSection>
      {axis === 'column' && <InspectorSection title="Advanced"><button className="wb-secondary" onClick={() => onChange({ ...matrix, columnStaggers: (matrix.columnStaggers ?? []).map((value, i) => i === index ? 0 : value), columnSplays: (matrix.columnSplays ?? []).map((value, i) => i === index ? 0 : value) })}>Reset stagger and splay</button></InspectorSection>}
    </section>;
  }
  const row = scope.row ?? 0;
  const column = scope.column ?? 0;
  const cell = matrix.cells?.find((entry) => entry.row === row && entry.column === column) ?? { row, column, enabled: true };
  const update = (changes: Partial<MatrixCell>) => {
    const next = withCell(matrix, row, column, changes);
    const needed = new Set([changes.definitionId, ...(changes.assemblies ?? []).map(item => item.definitionId)]);
    const additions = definitions.filter(entry => needed.has(entry.id));
    onChange(next, additions.length ? additions : undefined);
  };
  return <section aria-label="Key properties">
    <label className="wb-matrix-diodes"><input type="checkbox" aria-label="Key enabled" checked={cell.enabled} onChange={(event) => update({ enabled: event.target.checked })} /> Enabled</label>
    <div className="wb-matrix-number-grid">
      {field('Local X', cell.offset?.x ?? 0, (x) => update({ offset: { x, y: cell.offset?.y ?? 0 } }))}
      {field('Local Y', cell.offset?.y ?? 0, (y) => update({ offset: { x: cell.offset?.x ?? 0, y } }))}
      <CaseNumber label="Key rotation" value={cell.rotation ?? 0} unit="°" validation="finite" onCommit={(rotation) => update({ rotation })} />
    </div>
    <button className="wb-secondary" onClick={() => update({ offset: { x: 0, y: 0 }, rotation: 0 })}>Reset local transform</button>
    <label className="wb-script-select-label">Key Assembly<select aria-label="Key Assembly" value={cell.definitionId ?? matrix.definitionId} onChange={(event) => update({ definitionId: event.target.value, variant: event.target.value })}>
      {partChoices(definitions, cell.definitionId ?? matrix.definitionId).filter((definition) => (definition.matrixTerminals && definition.terminals?.[definition.matrixTerminals.row]?.length && definition.terminals?.[definition.matrixTerminals.column]?.length)
        || (!definition.matrixTerminals && definition.pads.some((pad) => pad.id === 'one') && definition.pads.some((pad) => pad.id === 'two'))).map((definition) => <option key={definition.id} value={definition.id}>{partCatalogLabel(definition)}</option>)}
    </select></label>
    <h3 className="wb-subtitle">Attached components</h3>
    {isMirrorTarget && (cell.assembliesLocal
      ? <button className="wb-secondary" onClick={() => update({ assembliesLocal: false })}>Use mirrored components</button>
      : <p className="wb-empty-note">The key assembly and attached components follow the paired half. Replacing one here keeps this key local.</p>)}
    {(cell.assemblies ?? []).map((assembly) => <div className="wb-pad-row" key={assembly.id}>
      <label className="wb-script-select-label">{definitions.find((entry) => entry.id === assembly.definitionId)?.name ?? assembly.definitionId}<select aria-label={`Replace ${assembly.id}`} value={assembly.definitionId} onChange={(event) => update({ assemblies: cell.assemblies?.map((item) => item.id === assembly.id ? { ...item, definitionId: event.target.value } : item) })}>
        {partChoices(definitions, assembly.definitionId).map((definition) => <option key={definition.id} value={definition.id}>{partCatalogLabel(definition)}</option>)}
      </select></label>
      <button aria-label={`Remove ${assembly.id}`} onClick={() => update({ assemblies: cell.assemblies?.filter((entry) => entry.id !== assembly.id) })}>Remove</button>
    </div>)}
    {!cell.assemblies?.length && <p className="wb-empty-note">No attached components. Apply a component in Parts.</p>}
  </section>;
};

export const MatrixEditor = ({ document, catalog, onEdit, scope, onDuplicateDesign }: { document: ProjectDoc; catalog: PartDefinition[]; onEdit: (command: EditCommand) => void; scope: SelectionScope | null; onDuplicateDesign?: (matrixId: string, presetId: MatrixPresetId, orientation?: SwitchOrientation) => void }) => {
  const [presetError, setPresetError] = useState('');
  const [presetId, setPresetId] = useState<MatrixPresetId>('mx-solder');
  const [orientation, setOrientation] = useState<SwitchOrientation>('south');
  const matrix = document.matrices.find((item) => item.id === scope?.matrixId);
  const definitions = new Map(document.definitions.map((definition) => [definition.id, definition]));
  const commit = (next: Matrix, definitions?: PartDefinition[]) => onEdit({
    baseRevision: document.revision,
    transactionId: makeId(),
    phase: 'commit',
    targetIds: [next.id],
    operation: { kind: 'set-matrix', matrix: next, definitions },
  });

  const update = (changes: Partial<Matrix>) => {
    if (!matrix) return;
    commit({ ...matrix, ...changes });
  };

  const field = (label: string, value: number, unitLabel: string, validation: NumberRule, onCommit: (value: number) => void) => <CaseNumber key={label} label={label} value={value} unit={unitLabel} validation={validation} onCommit={onCommit} />;

  return <section className="wb-matrix-editor" aria-label="Key matrix editor">
    {!matrix ? <p className="wb-empty-state">Select a matrix to edit its layout.</p> : <>
      <InspectorSection title="Layout" detail={`${matrix.rows} × ${matrix.columns}`} defaultOpen>
        <div className="wb-matrix-number-grid">
          {field('Rows', matrix.rows, 'keys', 'positive-integer', (value) => commit(resizeMatrix(matrix, value, matrix.columns)))}
          {field('Columns', matrix.columns, 'keys', 'positive-integer', (value) => commit(resizeMatrix(matrix, matrix.rows, value)))}
          {field('Pitch X', matrix.pitch.x, 'mm', 'positive', (value) => update({ pitch: { ...matrix.pitch, x: value } }))}
          {field('Pitch Y', matrix.pitch.y, 'mm', 'positive', (value) => update({ pitch: { ...matrix.pitch, y: value } }))}
        </div>
      </InspectorSection>
      <InspectorSection title="Position & orientation" defaultOpen>
        <div className="wb-matrix-number-grid">
          {field('Origin X', matrix.origin.x, 'mm', 'finite', (value) => update({ origin: { ...matrix.origin, x: value } }))}
          {field('Origin Y', matrix.origin.y, 'mm', 'finite', (value) => update({ origin: { ...matrix.origin, y: value } }))}
          {field('Rotation', matrix.rotation ?? 0, '°', 'finite', (value) => update({ rotation: value }))}
        </div>
        <label className="wb-script-select-label">Mirror<select aria-label="Mirror matrix" value={matrix.mirror ?? 'none'} onChange={(event) => update({ mirror: event.target.value as Matrix['mirror'] })}>
          <option value="none">None</option><option value="x">X axis</option><option value="y" disabled={Boolean((document.layouts ?? []).find((layout) => layout.matrixId === matrix.id && (layout.mirrorLink || document.layouts?.some((other) => other.mirrorLink?.sourceId === layout.id))))}>Y axis</option>
        </select></label>
      </InspectorSection>
      <InspectorSection title="Key assembly" detail={definitions.get(matrix.definitionId)?.name}>
        <label className="wb-script-select-label">Assembly preset<select aria-label="Apply matrix preset" value={presetId} onChange={(event) => setPresetId(event.target.value as MatrixPresetId)}>
          {(Object.keys(matrixPresetDefinitions) as MatrixPresetId[]).map((id) => <option key={id} value={id}>{assemblyName(id)}</option>)}
        </select></label>
        <OrientationControl value={orientation} onChange={setOrientation} />
        <div className="wb-inspector-actions">
          <button className="wb-secondary" onClick={() => {
            try {
              const result = matrixWithPreset(matrix, presetId, orientation, document.definitions);
              setPresetError('');
              commit(result.matrix, result.definitions);
            } catch (error) { setPresetError(error instanceof Error ? error.message : String(error)); }
          }}>Update assembly preset</button>
          <button className="wb-inspector-link" disabled={!onDuplicateDesign} onClick={() => onDuplicateDesign?.(matrix.id, presetId, orientation)}>Duplicate design as variant</button>
        </div>
        {presetError && <p role="alert">{presetError}</p>}
        <label className="wb-script-select-label">Switch footprint<select aria-label="Matrix part definition" value={matrix.definitionId} onChange={(event) => { const definition = catalog.find(item => item.id === event.target.value); if (definition) commit({ ...matrix, definitionId: definition.id }, [definition]); }}>
          {partChoices(catalog, matrix.definitionId).filter((definition) => definition.kind === 'switch' || definition.id === matrix.definitionId).map((definition) => <option key={definition.id} value={definition.id}>{partCatalogLabel(definition)}</option>)}
        </select></label>
        <label className="wb-script-select-label">Diode direction<select aria-label="Diode direction" value={matrix.diodeDirection ?? 'row2col'} onChange={(event) => update({ diodeDirection: event.target.value as Matrix['diodeDirection'] })}>
          <option value="row2col">Rows to columns</option><option value="col2row">Columns to rows</option>
        </select></label>
      </InspectorSection>
      <InspectorSection title="Keycap spacing" detail="Preview only">
        <div className="wb-matrix-number-grid">
          {field('Edge gap X', matrix.edgeGap?.x ?? 1, 'mm', 'nonnegative', (value) => update({ edgeGap: { ...(matrix.edgeGap ?? { x: 1, y: 1 }), x: value } }))}
          {field('Edge gap Y', matrix.edgeGap?.y ?? 1, 'mm', 'nonnegative', (value) => update({ edgeGap: { ...(matrix.edgeGap ?? { x: 1, y: 1 }), y: value } }))}
        </div>
        <p className="wb-empty-note">Keycap preview {Math.max(0, matrix.pitch.x - (matrix.edgeGap?.x ?? 1)).toFixed(1)} × {Math.max(0, matrix.pitch.y - (matrix.edgeGap?.y ?? 1)).toFixed(1)} mm</p>
      </InspectorSection>
    </>}
  </section>;
};
