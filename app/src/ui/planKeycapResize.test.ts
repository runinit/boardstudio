import { describe, expect, it } from 'vitest';
import type { Layout, Matrix, ProjectDoc, Vec2 } from '../../../contracts/src/index';
import { matrixSceneAdapter } from './matrixGeometry';
import type { KeycapPlacement } from './keycapReflow';
import { planKeycapResize } from './planKeycapResize';

const point = (x: number, y: number): Vec2 => ({ x, y });

const matrix = (id: string, partIds: string[], overrides: Partial<Matrix> = {}): Matrix => ({
  id,
  name: id,
  rows: 2,
  columns: 3,
  pitch: point(19, 19),
  origin: point(0, 0),
  definitionId: 'switch-mx',
  partIds,
  edgeGap: point(1, 1),
  ...overrides,
});

const docFor = (parts: ProjectDoc['parts'], matrices: Matrix[], layouts: Layout[] = []): ProjectDoc => ({
  format: 'boardstudio/v2', id: 'project', name: 'Test', revision: 4, parameters: {},
  definitions: [], parts, matrices, layouts, nets: [], outline: [], boards: [], caseBodies: [],
  materials: [], assets: [], scripts: [], constraints: [],
});

const part = (id: string, x: number, y: number, keycap: Vec2 = point(18, 18)) => ({
  id, definitionId: 'switch-mx', reference: id, pose: { at: point(x, y), rotation: 0 }, side: 'front' as const, keycap,
});

const placement = (id: string, matrixId: string, row: number, column: number, x: number, y: number, rotation = 0): KeycapPlacement => ({
  id, reference: id, matrixId, row, column, at: point(x, y), rotation, size: point(18, 18),
});

const projection = (matrixId: string, members: Array<[number, number, string]>) => matrixSceneAdapter({
  matrixId,
  cells: members.map(([row, column, memberId]) => ({ row, column, memberId, enabled: true, pose: { at: point(column * 19, row * 19), rotation: 0 } })),
  columns: [],
});

describe('planKeycapResize', () => {
  it('resizes a source key and reflows its row neighbours while preserving the group centre', () => {
    const parts = [part('k0', 0, 0), part('k1', 19, 0), part('k2', 38, 0)];
    const m = matrix('m', parts.map(({ id }) => id));
    const document = docFor(parts, [m]);
    const before = structuredClone(document);
    const result = planKeycapResize({
      document, layouts: [], matrices: new Map([['m', m]]),
      projections: new Map([['m', projection('m', [[0, 0, 'k0'], [0, 1, 'k1'], [0, 2, 'k2']])]]),
      placements: [placement('k0', 'm', 0, 0, 0, 0), placement('k1', 'm', 0, 1, 19, 0), placement('k2', 'm', 0, 2, 38, 0)],
      selectedIds: new Set(['k1']), units: point(2, 1),
    });

    expect(result?.document.parts.find(({ id }) => id === 'k1')?.keycap).toEqual(point(37, 18));
    expect(result?.document.parts.find(({ id }) => id === 'k0')?.pose.at.x).toBeCloseTo(-9.5);
    expect(result?.document.parts.find(({ id }) => id === 'k2')?.pose.at.x).toBeCloseTo(47.5);
    expect(result?.targetIds).toEqual(['k1', 'k0', 'k2', 'm']);
    expect(result?.document.matrices[0].cells).toEqual([
      { row: 0, column: 0, enabled: true, offset: point(-9.5, 0) },
      { row: 0, column: 2, enabled: true, offset: point(9.5, 0) },
    ]);
    expect(document).toEqual(before);
  });

  it('resolves a mirrored selection to the source matrix and updates both counterparts once', () => {
    const source = part('source', 0, 0);
    const mirrored = part('mirror', 0, 0);
    const sourceMatrix = matrix('source-matrix', ['source']);
    const mirrorMatrix = matrix('mirror-matrix', ['mirror']);
    const layouts: Layout[] = [
      { id: 'source-layout', name: 'Source', boardId: 'board', matrixId: sourceMatrix.id, partIds: ['source'] },
      { id: 'mirror-layout', name: 'Mirror', boardId: 'board', matrixId: mirrorMatrix.id, partIds: ['mirror'], mirrorLink: { sourceId: 'source-layout', axisX: 0 } },
    ];
    const result = planKeycapResize({
      document: docFor([source, mirrored], [sourceMatrix, mirrorMatrix], layouts), layouts,
      matrices: new Map([[sourceMatrix.id, sourceMatrix], [mirrorMatrix.id, mirrorMatrix]]),
      projections: new Map([
        [sourceMatrix.id, projection(sourceMatrix.id, [[0, 0, 'source']])],
        [mirrorMatrix.id, projection(mirrorMatrix.id, [[0, 0, 'mirror']])],
      ]),
      placements: [placement('source', sourceMatrix.id, 0, 0, 0, 0), placement('mirror', mirrorMatrix.id, 0, 0, 0, 0)],
      selectedIds: new Set(['mirror']), units: point(2, 1),
    });

    expect(result?.document.parts.map(({ keycap }) => keycap)).toEqual([point(37, 18), point(37, 18)]);
    expect(result?.targetIds.filter((id) => id === 'source' || id === 'mirror')).toHaveLength(2);
  });

  it('reflows a canonical cell once whether either or both linked counterparts are selected', () => {
    const parts = [part('source', 0, 0), part('neighbour', 19, 0), part('mirror', 100, 0), part('mirror-neighbour', 81, 0)];
    const sm = matrix('sm', ['source', 'neighbour']);
    const mm = matrix('mm', ['mirror', 'mirror-neighbour']);
    const layouts: Layout[] = [
      { id: 'sl', name: 'Source', boardId: 'b', matrixId: 'sm', partIds: [] },
      { id: 'ml', name: 'Mirror', boardId: 'b', matrixId: 'mm', partIds: [], mirrorLink: { sourceId: 'sl', axisX: 50 } },
    ];
    const args = {
      document: docFor(parts, [sm, mm], layouts), layouts,
      matrices: new Map([['sm', sm], ['mm', mm]]),
      projections: new Map([['sm', projection('sm', [[0, 0, 'source'], [0, 1, 'neighbour']])], ['mm', projection('mm', [[0, 0, 'mirror'], [0, 1, 'mirror-neighbour']])]]),
      placements: [placement('source', 'sm', 0, 0, 0, 0), placement('neighbour', 'sm', 0, 1, 19, 0), placement('mirror', 'mm', 0, 0, 100, 0), placement('mirror-neighbour', 'mm', 0, 1, 81, 0)],
      selectedIds: new Set(['source']), units: point(2, 1),
    };
    const sourceOnly = planKeycapResize(args)!;
    expect(planKeycapResize({ ...args, selectedIds: new Set(['mirror']) })).toEqual(sourceOnly);
    expect(planKeycapResize({ ...args, selectedIds: new Set(['source', 'mirror']) })).toEqual(sourceOnly);
    expect(sourceOnly.document.parts[1].pose.at).toEqual(point(28.5, 0));
    expect(sourceOnly.targetIds).toEqual(['source', 'mirror', 'neighbour', 'sm']);
    // The core linked-layout resolver propagates positions after this plan commits.
    expect(sourceOnly.document.parts[3]).toBe(parts[3]);
    expect(sourceOnly.document.matrices[1]).toBe(mm);
  });

  it('preserves disabled cells, saved rotations, and unrelated document data', () => {
    const selected = part('selected', 0, 0);
    const disabled = part('disabled', 19, 0);
    const m = matrix('m', ['selected', 'disabled'], { cells: [{ row: 0, column: 1, enabled: false }] });
    const document = docFor([selected, disabled], [m]);
    const placements = [placement('selected', 'm', 0, 0, 0, 0, 17), placement('disabled', 'm', 0, 1, 19, 0)];
    const result = planKeycapResize({
      document, layouts: [], matrices: new Map([['m', m]]),
      projections: new Map([['m', projection('m', [[0, 0, 'selected']])]]), placements: [placements[0]],
      selectedIds: new Set(['selected']), units: point(2, 2), axis: 'x',
    });
    expect(result?.document.parts.find(({ id }) => id === 'selected')?.keycap).toEqual(point(37, 18));
    expect(result?.document.parts.find(({ id }) => id === 'disabled')).toEqual(disabled);
    expect(result?.document.parts.find(({ id }) => id === 'selected')?.pose.rotation).toBe(0);
    expect(result?.document.nets).toBe(document.nets);
    expect(result?.document.boards).toBe(document.boards);
    expect(result?.document.definitions).toBe(document.definitions);
    expect(result?.document.matrices[0]).toBe(m);
  });

  it('converts resolved world movement to existing local offsets on rotated, mirrored, splayed columns', () => {
    const parts = [part('a', 999, 999), part('b', 999, 999), part('c', 999, 999)];
    const m = matrix('m', parts.map(({ id }) => id), {
      rotation: 90, mirror: 'x', columnSplays: [90, 0, 0],
      cells: [
        { row: 0, column: 0, enabled: true, offset: point(2, 3), rotation: 12 },
        { row: 0, column: 2, enabled: true, offset: point(-2, 5) },
      ],
    });
    const document = docFor(parts, [m]);
    const before = structuredClone(document);
    const scene = projection('m', [[0, 0, 'a'], [0, 1, 'b'], [0, 2, 'c']]).scene!;
    scene.columns = [0, 1, 2].map(column => ({ column, splayOrigin: point(0, 0), splayAngle: 90, customOrigin: false, axisX: point(-1, 0), axisY: point(0, 1) }));
    const result = planKeycapResize({
      document, layouts: [], matrices: new Map([['m', m]]),
      projections: new Map([['m', matrixSceneAdapter(scene)]]),
      placements: [placement('a', 'm', 0, 0, 100, 20), placement('b', 'm', 0, 1, 81, 20), placement('c', 'm', 0, 2, 62, 20)],
      selectedIds: new Set(['b']), units: point(2, 7), axis: 'x',
    })!;
    expect(result.document.parts[0].pose.at.x).toBeCloseTo(109.5);
    expect(result.document.parts[0].pose.at.y).toBeCloseTo(20);
    expect(result.document.parts[2].pose.at.x).toBeCloseTo(52.5);
    expect(result.document.parts[1].keycap).toEqual(point(37, 18));
    const first = result.document.matrices[0].cells!.find(cell => cell.column === 0)!;
    const last = result.document.matrices[0].cells!.find(cell => cell.column === 2)!;
    expect(first.offset!.x).toBeCloseTo(-7.5);
    expect(first.offset!.y).toBeCloseTo(3);
    expect(first.rotation).toBe(12);
    expect(last.offset!.x).toBeCloseTo(7.5);
    expect(last.offset!.y).toBeCloseTo(5);
    expect(document).toEqual(before);
  });

  it('resizes height independently and reflows column neighbours', () => {
    const parts = [part('a', 0, 0), part('b', 0, 19), part('c', 0, 38)];
    const m = matrix('m', parts.map(({ id }) => id), { rows: 3, columns: 1 });
    const result = planKeycapResize({
      document: docFor(parts, [m]), layouts: [], matrices: new Map([['m', m]]),
      projections: new Map([['m', projection('m', [[0, 0, 'a'], [1, 0, 'b'], [2, 0, 'c']])]]),
      placements: [placement('a', 'm', 0, 0, 0, 0), placement('b', 'm', 1, 0, 0, 19), placement('c', 'm', 2, 0, 0, 38)],
      selectedIds: new Set(['b']), units: point(7, 2), axis: 'y',
    })!;
    expect(result.document.parts[1].keycap).toEqual(point(18, 37));
    expect(result.document.parts.map(item => item.pose.at)).toEqual([point(0, -9.5), point(0, 19), point(0, 47.5)]);
    expect(result.document.matrices[0].cells).toEqual([
      { row: 0, column: 0, enabled: true, offset: point(0, -9.5) },
      { row: 2, column: 0, enabled: true, offset: point(0, 9.5) },
    ]);
  });

  it('returns undefined for an empty selection without mutating input', () => {
    const selected = part('selected', 0, 0);
    const m = matrix('m', ['selected']);
    const document = docFor([selected], [m]);
    const before = structuredClone(document);
    const args = {
      document, layouts: [], matrices: new Map([['m', m]]),
      projections: new Map([['m', projection('m', [[0, 0, 'selected']])]]),
      placements: [placement('selected', 'm', 0, 0, 0, 0)], selectedIds: new Set<string>(), units: point(1, 1),
    };
    expect(planKeycapResize(args)).toBeUndefined();
    expect(document).toEqual(before);
  });

  it('keeps a multi-key resize centred and treats unchanged nonempty input as a no-op', () => {
    const parts = [part('a', 0, 0), part('b', 19, 0), part('c', 38, 0)];
    const m = matrix('m', parts.map(({ id }) => id));
    const document = docFor(parts, [m]);
    const args = {
      document, layouts: [], matrices: new Map([['m', m]]),
      projections: new Map([['m', projection('m', [[0, 0, 'a'], [0, 1, 'b'], [0, 2, 'c']])]]),
      placements: [placement('a', 'm', 0, 0, 0, 0), placement('b', 'm', 0, 1, 19, 0), placement('c', 'm', 0, 2, 38, 0)],
      selectedIds: new Set(['a', 'b']), units: point(2, 1),
    };
    const result = planKeycapResize(args);
    const next = result?.document.parts.map(({ pose }) => pose.at.x) ?? [];
    expect(next[0]).toBeCloseTo(-9.5);
    expect(next[1]).toBeCloseTo(28.5);
    expect(next[2]).toBeCloseTo(57);
    expect(planKeycapResize({ ...args, selectedIds: new Set(['b']), units: point(1, 1) })).toBeUndefined();
  });
});
