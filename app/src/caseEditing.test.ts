import { describe, expect, it, test } from 'vitest';
import { caseMountConstraints, moveCaseMount } from './caseEditing';
import type { Mount, PreparedCaseIR } from '@boardstudio/v2-contracts';

const mounts: Mount[] = [
  { id: 'a', kind: 'hole', at: { x: 0, y: 0 }, holeDiameter: 2 },
  { id: 'b', kind: 'hole', at: { x: 10, y: 0 }, holeDiameter: 2 },
];

describe('moveCaseMount', () => {
  it('moves only the authored mount and leaves other coordinates untouched', () => {
    expect(moveCaseMount({ x: 2, y: 3 }, 'a', mounts)?.map((mount) => mount.at)).toEqual([{ x: 2, y: 3 }, { x: 10, y: 0 }]);
  });

  it('rejects an overlapping mount without mutating the source', () => {
    expect(moveCaseMount({ x: 10, y: 0 }, 'a', mounts)).toBeUndefined();
    expect(mounts[0].at).toEqual({ x: 0, y: 0 });
  });

  it('uses each mount kind diameter and ignores unrelated existing pairs', () => {
    const value: Mount[] = [
      { id: 'boss', kind: 'boss', at: { x: 0, y: 0 }, holeDiameter: 1, bossDiameter: 10 },
      { id: 'small', kind: 'hole', at: { x: 30, y: 0 }, holeDiameter: 2 },
      { id: 'unrelated', kind: 'hole', at: { x: 31, y: 0 }, holeDiameter: 2 },
    ];
    expect(moveCaseMount({ x: 20, y: 0 }, 'small', value)).toBeDefined();
    expect(moveCaseMount({ x: 30, y: 4 }, 'small', value)).toBeDefined();
  });

  it('keeps mounts inside the case and out of case holes', () => {
    const outer = [{ x: -10, y: -10 }, { x: 10, y: -10 }, { x: 10, y: 10 }, { x: -10, y: 10 }];
    const hole = [[{ x: -2, y: -2 }, { x: 2, y: -2 }, { x: 2, y: 2 }, { x: -2, y: 2 }]];
    expect(moveCaseMount({ x: 12, y: 0 }, 'a', mounts, 0.5, { outer })).toBeUndefined();
    expect(moveCaseMount({ x: 0, y: 0 }, 'a', mounts, 0.5, { outer, holes: hole })).toBeUndefined();
    expect(moveCaseMount({ x: 5, y: 5 }, 'a', mounts, 0.5, { outer, holes: hole })).toBeDefined();
  });
});

test('rejects edge clearance violations even when the mount centre is inside', () => {
  const mounts = [{ id: 'a', at: { x: 5, y: 5 }, kind: 'hole' as const, holeDiameter: 2 }];
  const outer = [{ x: 0, y: 0 }, { x: 20, y: 0 }, { x: 20, y: 20 }, { x: 0, y: 20 }];
  expect(moveCaseMount({ x: 0.8, y: 5 }, 'a', mounts, 0.5, { outer })).toBeUndefined();
});

test('does not reject a moved mount because two unrelated mounts overlap', () => {
  const mounts = [
    { id: 'a', at: { x: 50, y: 50 }, kind: 'hole' as const, holeDiameter: 2 },
    { id: 'b', at: { x: 2, y: 2 }, kind: 'boss' as const, holeDiameter: 2, bossDiameter: 8 },
    { id: 'c', at: { x: 2, y: 2 }, kind: 'hole' as const, holeDiameter: 2 },
  ];
  expect(moveCaseMount({ x: 55, y: 50 }, 'a', mounts)).toBeDefined();
});


test('accepts a mount inside the second split contour but rejects the gap', () => {
  const rectangle = (x: number) => [{ x, y: 0 }, { x: x + 20, y: 0 }, { x: x + 20, y: 20 }, { x, y: 20 }];
  const mounts: Mount[] = [{ id: 'right', kind: 'hole', at: { x: 40, y: 10 }, holeDiameter: 2 }];
  const outer = [rectangle(0), rectangle(30)];
  expect(moveCaseMount({ x: 42, y: 10 }, 'right', mounts, 0.5, { outer })).toBeDefined();
  expect(moveCaseMount({ x: 25, y: 10 }, 'right', mounts, 0.5, { outer })).toBeUndefined();
});


test('prepared constraints keep split regions and ignore cavities and openings above the solid', () => {
  const rectangle = (x: number) => [{ x, y: 0 }, { x: x + 20, y: 0 }, { x: x + 20, y: 20 }, { x, y: 20 }];
  const region = (x: number) => ({ outer: rectangle(x), holes: [], cavities: [rectangle(x + 2)], gaskets: [], mounts: [] });
  const prepared: PreparedCaseIR = { revision: 1,
    body: { id: 'tray', name: 'Tray', boardId: 'board', kind: 'tray', thickness: 3, wallHeight: 4, clearance: 5, z: -10,
      openings: [{ points: rectangle(30), z: -3, height: 2 }] },
    regions: [region(0), region(30)],
  };
  const mount: Mount[] = [{ id: 'm', kind: 'hole', holeDiameter: 2, at: { x: 5, y: 5 } }];
  expect(moveCaseMount({ x: 40, y: 10 }, 'm', mount, 0.5, caseMountConstraints(prepared))).toBeDefined();
  expect(moveCaseMount({ x: 25, y: 10 }, 'm', mount, 0.5, caseMountConstraints(prepared))).toBeUndefined();
  prepared.body.openings![0].z = -4;
  expect(moveCaseMount({ x: 40, y: 10 }, 'm', mount, 0.5, caseMountConstraints(prepared))).toBeUndefined();
});
