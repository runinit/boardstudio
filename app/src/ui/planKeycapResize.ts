import type { Layout, Matrix, ProjectDoc, Vec2 } from '../../../contracts/src/index';
import { keycapReflowDeltas, type KeycapPlacement, type KeycapResize } from './keycapReflow';
import type { MatrixProjection } from './matrixGeometry';
import { localMatrixDelta, withCell } from './workbenchGeometry';

type Inputs = {
  document: ProjectDoc;
  layouts: readonly Layout[];
  matrices: ReadonlyMap<string, Matrix>;
  projections: ReadonlyMap<string, MatrixProjection>;
  placements: readonly KeycapPlacement[];
  selectedIds: ReadonlySet<string>;
  units: Vec2;
  axis?: 'x' | 'y';
};

export function planKeycapResize({ document, layouts, matrices, projections, placements, selectedIds, units, axis }: Inputs): { document: ProjectDoc; targetIds: string[] } | undefined {
  const placementById = new Map(placements.map((placement) => [placement.id, placement]));
  const resizeByCell = new Map<string, KeycapResize>();
  const updatedSizes = new Map<string, Vec2>();

  for (const placement of placements) {
    if (!selectedIds.has(placement.id) || !matrices.has(placement.matrixId)) continue;
    // Reflow the source once even when both linked halves are selected. Core
    // layout resolution propagates its offsets to the mirrored half on commit.
    const owner = layouts.find((layout) => layout.matrixId === placement.matrixId);
    const canonicalLayout = owner?.mirrorLink
      ? layouts.find((layout) => layout.id === owner.mirrorLink?.sourceId)
      : owner;
    const matrixId = canonicalLayout?.matrixId ?? placement.matrixId;
    const matrix = matrices.get(matrixId);
    const cell = `${placement.row}:${placement.column}`;
    const id = projections.get(matrixId)?.member(placement.row, placement.column) ?? placement.id;
    const sourcePlacement = placementById.get(id) ?? placement;
    if (!matrix) continue;

    const oldSize = sourcePlacement.size;
    const gap = matrix.edgeGap ?? { x: 1, y: 1 };
    const nextSize = {
      x: axis === 'y' ? oldSize.x : Math.max(1, units.x * matrix.pitch.x - gap.x),
      y: axis === 'x' ? oldSize.y : Math.max(1, units.y * matrix.pitch.y - gap.y),
    };
    if (nextSize.x === oldSize.x && nextSize.y === oldSize.y) continue;

    const basis = projections.get(matrixId)?.basis(placement.column);
    const rotation = sourcePlacement.rotation * Math.PI / 180;
    const alignAxis = (vector: Vec2, reference: Vec2 | undefined): Vec2 => reference
      && vector.x * reference.x + vector.y * reference.y < 0
      ? { x: -vector.x, y: -vector.y }
      : vector;
    const axisX = alignAxis({ x: Math.cos(rotation), y: Math.sin(rotation) }, basis?.axisX);
    const axisY = alignAxis({ x: -Math.sin(rotation), y: Math.cos(rotation) }, basis?.axisY);
    const resize: KeycapResize = { ...sourcePlacement, matrixId, size: oldSize, nextSize, axisX, axisY };
    resizeByCell.set(`${matrixId}:${cell}`, resize);
    updatedSizes.set(id, nextSize);

    const pairedLayout = canonicalLayout?.mirrorLink
      ? layouts.find((layout) => layout.id === canonicalLayout.mirrorLink?.sourceId)
      : layouts.find((layout) => layout.mirrorLink?.sourceId === canonicalLayout?.id);
    const pairedId = pairedLayout ? projections.get(pairedLayout.matrixId)?.member(placement.row, placement.column) : undefined;
    if (pairedId) updatedSizes.set(pairedId, nextSize);
  }

  if (updatedSizes.size === 0) return;

  const reflow = keycapReflowDeltas([...resizeByCell.values()], placements);
  const updatedMatrices = new Map<string, Matrix>();
  for (const [id, delta] of reflow) {
    const cell = placementById.get(id);
    const matrix = cell ? updatedMatrices.get(cell.matrixId) ?? matrices.get(cell.matrixId) : undefined;
    if (!cell || !matrix) continue;
    const existingOffset = matrix.cells?.find((item) => item.row === cell.row && item.column === cell.column)?.offset ?? { x: 0, y: 0 };
    const local = localMatrixDelta(matrix, delta, cell.column);
    updatedMatrices.set(cell.matrixId, withCell(matrix, cell.row, cell.column, {
      offset: { x: existingOffset.x + local.x, y: existingOffset.y + local.y },
    }));
  }

  const nextDocument: ProjectDoc = {
    ...document,
    parts: document.parts.map((part) => {
      const size = updatedSizes.get(part.id);
      const delta = reflow.get(part.id);
      if (!size && !delta) return part;
      const position = placementById.get(part.id)?.at ?? part.pose.at;
      return {
        ...part,
        ...(size ? { keycap: size } : {}),
        ...(delta ? { pose: { ...part.pose, at: { x: position.x + delta.x, y: position.y + delta.y } } } : {}),
      };
    }),
    matrices: document.matrices.map((matrix) => updatedMatrices.get(matrix.id) ?? matrix),
  };
  return {
    document: nextDocument,
    targetIds: [...new Set([...updatedSizes.keys(), ...reflow.keys(), ...updatedMatrices.keys()])],
  };
}
