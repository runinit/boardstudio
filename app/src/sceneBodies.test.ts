import { expect, test } from 'vitest';
import { changedSceneBodies, type SceneBody } from './sceneBodies';
const body = (id: string, values = [0, 1, 2]): SceneBody => ({ id, mesh: { positions: new Float32Array(values), normals: new Float32Array([0, 0, 1]) } });
test('compares interior vertices and normals, not just the endpoints and size', () => {
  const previous = body('tray');
  expect(changedSceneBodies([previous], [body('tray', [0, 9, 2])]).bodies).toHaveLength(1);
  const changedNormal = body('tray'); changedNormal.mesh.normals[1] = 1;
  expect(changedSceneBodies([previous], [changedNormal]).bodies).toHaveLength(1);
  expect(changedSceneBodies([previous], [body('tray')])).toEqual({ bodies: [], removed: [] });
});
test('tracks membership changes by stable id without changing caller-owned arrays', () => {
  const a = body('a'), b = body('b'), c = body('c');
  expect(changedSceneBodies([a, b], [body('b'), c])).toEqual({ bodies: [c], removed: ['a'] });
  expect(a.mesh.positions).toEqual(new Float32Array([0, 1, 2]));
});
