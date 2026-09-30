import { useEffect, useState } from 'react';
import type { Board, Constraint, EditCommand, Part, ProjectDoc } from '@boardstudio/v2-contracts';
import { buildMirrorConstraint, buildOffsetConstraint } from './createWorkbenchEditActions';
import { ConstraintNumber } from './InspectorControls';
import { InspectorSection } from './InspectorSection';
import { makeId } from './workbenchGeometry';

export function ConstraintEditor({ document, part: activePart, board: selectedBoard, parts: visibleParts, emit }: {
  document: ProjectDoc; part: Part; board?: Board; parts: Part[];
  emit: (operation: EditCommand['operation'], ids: string[]) => unknown;
}) {
  const activeConstraint = document.constraints.find(constraint => constraint.targetPartId === activePart.id);
  const defaultConstraintSourceId = visibleParts.find(part => part.id !== activePart.id)?.id ?? '';
  const boardPartIds = new Set(selectedBoard?.partIds ?? document.parts.map(part => part.id));
  const selectedBoardId = selectedBoard?.id;
  const parts = new Map(visibleParts.map(part => [part.id, part]));
  const [constraintKind, setConstraintKind] = useState<Constraint['kind']>('offset');
  const [constraintSourceId, setConstraintSourceId] = useState('');
  const [constraintX, setConstraintX] = useState('0');
  const [constraintY, setConstraintY] = useState('0');
  const [constraintRotation, setConstraintRotation] = useState('0');
  const [constraintAxis, setConstraintAxis] = useState<'vertical' | 'horizontal'>('vertical');
  const [constraintCoordinate, setConstraintCoordinate] = useState('0');
  useEffect(() => {
    const sourceId = activeConstraint?.sourcePartId ?? defaultConstraintSourceId;
    setConstraintKind(activeConstraint?.kind ?? 'offset');
    setConstraintSourceId(sourceId);
    setConstraintX(activeConstraint?.kind === 'offset' ? String(activeConstraint.offset.x) : '0');
    setConstraintY(activeConstraint?.kind === 'offset' ? String(activeConstraint.offset.y) : '0');
    setConstraintRotation(activeConstraint?.kind === 'offset' ? String(activeConstraint.rotation) : '0');
    setConstraintAxis(activeConstraint?.kind === 'mirror' ? activeConstraint.axis : 'vertical');
    setConstraintCoordinate(activeConstraint?.kind === 'mirror' ? String(activeConstraint.coordinate) : '0');
  }, [activePart?.id, selectedBoardId, activeConstraint?.id, defaultConstraintSourceId]);
  const saveConstraint = () => {
    if (!activePart || !selectedBoard || !boardPartIds.has(activePart.id) || !boardPartIds.has(constraintSourceId) || constraintSourceId === activePart.id) return;
    const id = activeConstraint?.id ?? makeId();
    let constraint: Constraint | undefined;
    if (constraintKind === 'offset') {
      constraint = buildOffsetConstraint(id, constraintSourceId, activePart.id, constraintX, constraintY, constraintRotation);
    } else {
      constraint = buildMirrorConstraint(id, constraintSourceId, activePart.id, constraintAxis, constraintCoordinate);
    }
    if (!constraint) return;
    emit({ kind: 'set-constraint', constraint }, [constraint.id, constraint.sourcePartId, constraint.targetPartId]);
  };

  const removeConstraint = () => {
    if (!activeConstraint) return;
    emit({ kind: 'remove-constraint', id: activeConstraint.id }, [activeConstraint.id, activeConstraint.targetPartId]);
  };

  return (
        <InspectorSection title="Layout constraint" detail={activeConstraint ? 'Active' : 'Optional'} defaultOpen={Boolean(activeConstraint)}>
        {visibleParts.length > 1 ? <div className="wb-constraint-form">
          <label>Relationship
            <select aria-label="Constraint type" value={constraintKind} onChange={(event) => setConstraintKind(event.target.value as Constraint['kind'])}>
              <option value="offset">Offset from part</option>
              <option value="mirror">Mirror placement across axis</option>
            </select>
          </label>
          <label>Source part
            <select aria-label="Constraint source part" value={constraintSourceId} onChange={(event) => setConstraintSourceId(event.target.value)}>
              {visibleParts.filter((part) => part.id !== activePart.id).map((part) => <option key={part.id} value={part.id}>{part.reference}</option>)}
            </select>
          </label>
          {constraintKind === 'offset' ? <>
            <div className="wb-constraint-number-grid">
              <ConstraintNumber label="Offset X (mm)" value={constraintX} onChange={setConstraintX} />
              <ConstraintNumber label="Offset Y (mm)" value={constraintY} onChange={setConstraintY} />
            </div>
            <ConstraintNumber label="Rotation (degrees)" value={constraintRotation} onChange={setConstraintRotation} />
          </> : <>
            <label>Axis
              <select aria-label="Mirror axis" value={constraintAxis} onChange={(event) => setConstraintAxis(event.target.value as 'vertical' | 'horizontal')}>
                <option value="vertical">Vertical</option>
                <option value="horizontal">Horizontal</option>
              </select>
            </label>
            <ConstraintNumber label="Axis coordinate (mm)" value={constraintCoordinate} onChange={setConstraintCoordinate} />
            <p className="wb-constraint-note">Footprint geometry stays unchanged. Use a handed definition where needed.</p>
          </>}
          {activeConstraint && <p className="wb-constraint-note">{parts.get(activeConstraint.sourcePartId)?.reference ?? 'A part'} drives {activePart.reference}.</p>}
          <div className="wb-constraint-actions">
            <button type="button" onClick={saveConstraint} disabled={!constraintSourceId}>{activeConstraint ? 'Save constraint' : 'Add constraint'}</button>
            {activeConstraint && <button type="button" className="is-remove" onClick={removeConstraint}>Remove</button>}
          </div>
        </div> : <p className="wb-empty-note">Add another part on this board to create a layout constraint.</p>}
        </InspectorSection>
  );
}
