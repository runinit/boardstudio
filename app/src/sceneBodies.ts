import type { ModelMesh } from './modelMesh';

export type SceneBody = { id: string; name?: string; color?: string; mesh: ModelMesh };

function sameBuffer(a: Float32Array | undefined, b: Float32Array | undefined): boolean {
  if (a === b) return true;
  if (!a || !b || a.length !== b.length) return false;
  for (let index = 0; index < a.length; index += 1) if (a[index] !== b[index]) return false;
  return true;
}

export function changedSceneBodies(previous: SceneBody[], next: SceneBody[]) {
  const byId = new Map(previous.map(body => [body.id, body]));
  const bodies = next.filter(body => {
    const old = byId.get(body.id);
    byId.delete(body.id);
    return !old || old.name !== body.name || old.color !== body.color || !sameBuffer(old.mesh.positions, body.mesh.positions)
      || !sameBuffer(old.mesh.normals, body.mesh.normals) || !sameBuffer(old.mesh.colors, body.mesh.colors);
  });
  return { bodies, removed: [...byId.keys()] };
}
