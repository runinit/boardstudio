// Preview experiment only. No STEP export or production readiness API is exposed.
export function manifoldBottom(wasm, input, includeMesh = false) {
  const { Manifold, CrossSection } = wasm;
  const owned = [];
  const own = object => { owned.push(object); return object; };
  const points = contour => contour.map(p => [p.x, p.y]);
  const { body, regions } = input;
  if (body.kind !== 'plate' || body.gasket || regions.some(r => r.cavities?.length || r.gaskets?.length
    || r.mounts?.some(m => m.kind !== 'hole'))) throw new Error('Unsupported profile experiment input');
  const z0 = body.z ?? 0;
  const z1 = z0 + body.thickness;
  const openings = body.openings ?? [];
  const levels = [...new Set([z0, z1, ...openings.flatMap(o => [o.z, o.z + o.height])
    .map(z => Math.max(z0, Math.min(z1, z)))])].sort((a, b) => a - b);
  let maxChordError = 0;
  let volumeErrorBound = 0;
  const start = performance.now();
  try {
    const parts = [];
    for (const region of regions) {
      const bands = [];
      for (let i = 1; i < levels.length; i++) {
        const middle = (levels[i - 1] + levels[i]) / 2;
        let section = own(new CrossSection([points(region.outer), ...(region.holes ?? []).map(points)], 'EvenOdd'));
        for (const opening of openings) {
          if (middle > opening.z && middle < opening.z + opening.height) {
            const cutter = own(new CrossSection([points(opening.points)], 'EvenOdd'));
            section = own(section.subtract(cutter));
          }
        }
        const height = levels[i] - levels[i - 1];
        // Use the same authored endpoint for both sides of every interface.
        // Adding height to the lower endpoint can leave a one-ULP open seam.
        bands.push(own(own(section.extrude(height)).warp(vertex => {
          vertex[2] = vertex[2] === 0 ? levels[i - 1] : levels[i];
        })));
      }
      const solid = own(Manifold.union(bands));
      const tools = (region.mounts ?? []).map(mount => {
        const radius = mount.holeDiameter / 2;
        // Satisfy both existing deflection bounds. Minimum 32 avoids using a much
        // coarser circular preview to manufacture a misleading speed advantage.
        const segments = 4 * Math.ceil(Math.max(32, 2 * Math.PI / 0.5,
          Math.PI / Math.acos(Math.max(-1, 1 - 0.1 / radius))) / 4);
        maxChordError = Math.max(maxChordError, radius * (1 - Math.cos(Math.PI / segments)));
        volumeErrorBound += (Math.PI * radius ** 2
          - segments * radius ** 2 * Math.sin(2 * Math.PI / segments) / 2) * body.thickness;
        return own(own(Manifold.cylinder(body.thickness, radius, radius, segments))
          .translate([mount.at.x, mount.at.y, z0]));
      });
      parts.push(own(Manifold.difference([solid, ...tools])));
    }
    const combined = own(Manifold.union(parts));
    const withNormals = own(combined.calculateNormals(0, 30));
    const treeBuilt = performance.now();
    // Forces all lazy CSG, tessellation, and normal work inside the timing.
    const mesh = withNormals.getMesh();
    const positions = new Float32Array(mesh.triVerts.length * 3);
    const normals = new Float32Array(positions.length);
    for (let i = 0; i < mesh.triVerts.length; i++) {
      const offset = mesh.triVerts[i] * mesh.numProp;
      for (let j = 0; j < 3; j++) {
        positions[i * 3 + j] = mesh.vertProperties[offset + j];
        normals[i * 3 + j] = mesh.vertProperties[offset + 3 + j];
      }
    }
    const meshed = performance.now();
    const status = combined.status();
    if (status !== 'NoError') throw new Error(`Manifold failed: ${status}`);
    const decomposed = combined.decompose();
    decomposed.forEach(own);
    const box = combined.boundingBox();
    const report = { variant: 'manifold', generationMs: meshed - start,
      treeConstructionMs: treeBuilt - start, forcedEvaluationAndMeshMs: meshed - treeBuilt,
      solids: decomposed.length, volume: combined.volume(), bounds: [box.min, box.max],
      triangles: mesh.triVerts.length / 3, meshBytes: positions.byteLength + normals.byteLength,
      maxChordError, volumeErrorBound, status };
    if (includeMesh) {
      report.mesh = { positions: [...positions], normals: [...normals] };
      report.components = decomposed.map(part => ({ volume: part.volume(), bounds: part.boundingBox() }));
    }
    return report;
  } finally {
    for (const object of owned.reverse()) object.delete();
  }
}
