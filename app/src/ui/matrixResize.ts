import type { Matrix } from '@boardstudio/v2-contracts';

export function resizeMatrix(matrix: Matrix, rows: number, columns: number): Matrix {
  const template = matrix.cells?.find(cell => cell.definitionId === matrix.definitionId && cell.variant?.startsWith('preset/'));
  if (!template) return { ...matrix, rows, columns };

  const cells = (matrix.cells ?? []).filter(cell => cell.row < rows && cell.column < columns);
  for (let row = 0; row < rows; row++) {
    for (let column = 0; column < columns; column++) {
      if (row < matrix.rows && column < matrix.columns) continue;
      // Inherit the saved recipe, not the template key's enabled state or local pose.
      cells.push({
        row, column, enabled: true, definitionId: template.definitionId,
        variant: template.variant, rotation: template.variant?.endsWith('/north') ? 180 : 0,
        assembliesLocal: template.assembliesLocal,
        assemblies: structuredClone(template.assemblies?.filter(member => ['diode', 'led'].includes(member.id))),
      });
    }
  }
  return { ...matrix, rows, columns, cells };
}
