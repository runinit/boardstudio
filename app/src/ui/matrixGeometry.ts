import type { MatrixScene } from '@boardstudio/v2-contracts';
export type { MatrixScene } from '@boardstudio/v2-contracts';

export function matrixSceneAdapter(scene: MatrixScene | undefined) {
  const cells = new Map((scene?.cells ?? []).map((cell) => [`${cell.row}:${cell.column}`, cell]));
  const columns = new Map((scene?.columns ?? []).map((basis) => [basis.column, basis]));
  return {
    scene,
    members: new Map((scene?.cells ?? []).flatMap((cell) => cell.memberId ? [[`${cell.row}:${cell.column}`, cell.memberId] as const] : [])),
    cell: (row: number, column: number) => cells.get(`${row}:${column}`),
    pose: (row: number, column: number) => cells.get(`${row}:${column}`)?.pose,
    member: (row: number, column: number) => cells.get(`${row}:${column}`)?.memberId,
    basis: (column: number) => columns.get(column),
  };
}

export const matrixCellId = (matrixId: string, row: number, column: number): string => `matrix/${matrixId}/r${row}c${column}`;

export type MatrixProjection = ReturnType<typeof matrixSceneAdapter>;
