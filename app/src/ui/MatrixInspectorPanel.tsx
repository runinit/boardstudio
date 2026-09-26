import type { Dispatch, SetStateAction } from 'react';
import type { EditCommand, Layout, Matrix, MatrixSplayChange, Part, PartDefinition, ProjectDoc, Vec2 } from '@boardstudio/v2-contracts';
import { CellInspector, MatrixEditor } from './MatrixInspector';
import { InspectorSection } from './InspectorSection';
import { DraftInput } from './InspectorControls';
import { KeySizeControls } from './KeySizeControls';
import { MirrorPairIcon } from './MirroredPairSetup';
import type { MatrixProjection } from './matrixGeometry';
import type { MatrixPresetId } from './assemblyCatalog';
import type { SwitchOrientation } from './assemblyPresets';
import type { SelectionScope } from './workbenchTypes';

type SelectedKeycap = { placement: { size: Vec2 }; pitch: Vec2; gap: Vec2 };

type MatrixInspectorPanelProps = {
  title: string;
  matrix: Matrix;
  scope: SelectionScope;
  layout?: Layout;
  partnerLayout?: Layout;
  linkedLayout?: Layout;
  selectedKeycaps: SelectedKeycap[];
  overlapReferences: string[];
  onKeycapResize: (units: Vec2, axis?: 'x' | 'y') => void;
  onEdit: (command: EditCommand) => void;
  onLayoutChange: (layout: Layout) => void;
  onUnlink: (layout: Layout) => void;
  onDuplicateDesign?: (matrixId: string, presetId: MatrixPresetId, orientation?: SwitchOrientation) => void;
  document: ProjectDoc;
  catalog: PartDefinition[];
  projection?: MatrixProjection;
  onSplay: (column: number, change: MatrixSplayChange) => void;
  splayAffect: 'column' | 'following';
  onSplayAffectChange: Dispatch<SetStateAction<'column' | 'following'>>;
  onPickOrigin: () => void;
  parts: Map<string, Part>;
  members: Map<string, string>;
  isMirrorTarget: boolean;
  onChange: (matrix: Matrix, definitions?: PartDefinition[]) => void;
  onDelete: () => void;
};

/** Matrix and key selection inspector. Kept independent of Workbench state ownership. */
export function MatrixInspectorPanel({
  title, matrix, scope, layout, partnerLayout, linkedLayout, selectedKeycaps, overlapReferences,
  onKeycapResize, onEdit, onLayoutChange, onUnlink, onDuplicateDesign, document, catalog, projection, onSplay,
  splayAffect, onSplayAffectChange, onPickOrigin, parts, members, isMirrorTarget, onChange, onDelete,
}: MatrixInspectorPanelProps) {
  return <>
    <div className="wb-inspect-head wb-matrix-detail-heading"><h2>{title}</h2></div>
    {scope.kind === 'matrix' && <div className="wb-object-actions"><label>{layout ? 'Layout name' : 'Matrix name'}<DraftInput ariaLabel={layout ? 'Layout name' : 'Matrix name'} value={layout?.name ?? matrix.name ?? title} onCommit={(name) => {
      if (!name.trim()) return;
      if (layout) onLayoutChange({ ...layout, name: name.trim() });
      else onChange({ ...matrix, name: name.trim() });
    }} /></label></div>}
    {layout && <div className="wb-layout-link"><strong><MirrorPairIcon />{partnerLayout ? `Linked to ${partnerLayout.name}` : 'Independent layout'}</strong><p>{partnerLayout ? 'Key assemblies, diodes and components mirror across both halves. Replace a component on one half to keep it local.' : 'Geometry and components can be edited independently.'}</p>{linkedLayout && <button className="wb-secondary" onClick={() => onUnlink(linkedLayout)}>Unlink halves</button>}</div>}
    {(['key', 'row', 'column', 'matrix'].includes(scope.kind)) && selectedKeycaps.length > 0 && <>
      <KeySizeControls key={JSON.stringify(scope)} items={selectedKeycaps.map(({ placement, pitch, gap }) => ({ size: placement.size, pitch, gap }))} onCommit={onKeycapResize} />
      {overlapReferences.length > 0 && <p className="wb-key-size-warning" role="status">Keycaps overlap: {overlapReferences.slice(0, 8).join(', ')}{overlapReferences.length > 8 ? ` and ${overlapReferences.length - 8} more` : ''}. Adjust their rows or columns to clear the overlap.</p>}
    </>}
    {scope.kind === 'matrix'
      ? <MatrixEditor catalog={catalog} document={document} onEdit={onEdit} scope={scope} onDuplicateDesign={onDuplicateDesign} />
      : <CellInspector projection={projection} onSplay={onSplay} splayAffect={splayAffect} onAffectChange={onSplayAffectChange} onPickOrigin={onPickOrigin} matrix={matrix} scope={scope} definitions={catalog} parts={parts} members={members} isMirrorTarget={isMirrorTarget} onChange={onChange} />}
    {scope.kind === 'matrix' && <InspectorSection title="Matrix actions"><button className="wb-secondary" onClick={onDelete}>Delete matrix</button></InspectorSection>}
  </>;
}
