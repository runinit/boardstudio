// Read-only reproduction using the exact published candidate core provider.
// No compiler, browser introspection, or product source mutation is involved.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { pathToFileURL } from 'node:url';

const root = process.cwd();
const evidence = path.join(root, '.scratch/dioxus-frontend-v1/evidence/case-handle-qualification-20261005');
const provider = path.join(root, 'web/target/builds/frontend-case-export-20261005/site-root/assets/core-worker');
const wasmPath = path.join(provider, 'm1_core_worker_bg.wasm');
const { default: init, CoreEngine } = await import(pathToFileURL(path.join(provider, 'm1_core_worker.js')));
const wasm = fs.readFileSync(wasmPath);
await init({ module_or_path: wasm });
const document = JSON.parse(execFileSync('python3', ['-c', 'import zipfile,sys; print(zipfile.ZipFile(sys.argv[1]).read("project.json").decode())', path.join(evidence, 'input.boardstudio')], { maxBuffer: 4 * 1024 * 1024 }).toString());
const engine = new CoreEngine();
const request = value => {
  const reply = JSON.parse(engine.request(JSON.stringify(value)));
  if (reply.kind === 'error') throw new Error(JSON.stringify(reply));
  return reply;
};
const opened = request({ kind: 'open', id: 'diagnosis-open', document });
const effective = structuredClone(opened.document);
const instance = effective.hardware.instances.find(value => value.id === 'right');
if (instance.flipped || instance.boardId !== 'right') throw new Error('fixture assumption changed');
effective.physicalInstanceId = instance.id;
const board = effective.boards.find(value => value.id === instance.boardId);
const shared = effective.hardware.sharedConstruction;
const configuration = structuredClone(instance.mechanical);
for (const field of ['method', 'mount', 'integratedPlateFrame', 'bottomStyle', 'middleFrame', 'plateThickness', 'plateFoamThickness', 'bottomFoamThickness', 'bottomThickness', 'plateToPcb', 'wallThickness', 'clearance', 'gasket', 'gasketTravel', 'openingAllowance', 'partProcesses', 'internalGasket', 'hardware', 'criticalFits', 'profiles']) {
  if (shared[field] !== undefined) configuration[field] = structuredClone(shared[field]);
}
if (shared.gasketLayout) throw new Error('fixture now requires gasket-layout support merge');
configuration.boardId = instance.boardId;
configuration.pcbThickness = board.thickness;
// Verify the remaining cad_jobs::mechanical_defaults branches are inapplicable.
if (effective.hardware.transport !== 'wired') throw new Error('battery-default branch needs mirroring');
const stabilizerKeys = effective.parts.filter(part => board.partIds.includes(part.id)).filter(part => {
  const definition = effective.definitions.find(value => value.id === part.definitionId);
  const size = part.keycap ?? definition?.keycap;
  return size && Math.max(size.x, size.y) >= 37 && definition?.generator?.source?.toLowerCase() === 'ceoloide/switch_mx';
});
if (stabilizerKeys.length) throw new Error('stabilizer-default branch needs mirroring');
configuration.stabilizers = [];
effective.mechanical = configuration;
const contours = opened.scene.boardContours.find(value => value.boardId === instance.boardId).contours;
const { assembly } = request({ kind: 'resolve-mechanical', id: 'diagnosis-resolve', document: effective, contours });
const { ir } = request({ kind: 'prepare-case', id: 'diagnosis-prepare', ir: assembly.case });
const bottom = ir.bodies.find(value => value.body.id === 'bottom');
const base = bottom.body.z ?? 0;
const top = base + bottom.body.thickness + (bottom.body.kind === 'plate' ? 0 : bottom.body.wallHeight ?? 0);
const constraints = {
  outer: bottom.regions.map(region => region.outer).filter(poly => poly.length >= 3),
  holes: [...bottom.regions.flatMap(region => region.holes), ...(bottom.body.openings ?? []).filter(opening => opening.z < top && opening.z + opening.height > base).map(opening => opening.points)],
};
const inside = (point, polygon) => {
  let hit = false;
  for (let i = 0, j = polygon.length - 1; i < polygon.length; j = i++) {
    const a = polygon[i], b = polygon[j];
    if ((a.y > point.y) !== (b.y > point.y) && point.x < (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x) hit = !hit;
  }
  return hit;
};
const edgeDistance = (point, polygon) => Math.min(...polygon.map((a, i) => {
  const b = polygon[(i + 1) % polygon.length];
  const dx = b.x - a.x, dy = b.y - a.y, length = dx * dx + dy * dy;
  const t = length ? Math.max(0, Math.min(1, ((point.x - a.x) * dx + (point.y - a.y) * dy) / length)) : 0;
  return Math.hypot(point.x - a.x - t * dx, point.y - a.y - t * dy);
}));
const selected = configuration.closureMounts.find(value => value.id.endsWith('mount-proposal-47'));
const margin = (selected.bossDiameter ?? selected.holeDiameter) / 2 + 0.5;
const evaluate = point => {
  const outer = constraints.outer.map(poly => ({ inside: inside(point, poly), distance: edgeDistance(point, poly) }));
  const holes = constraints.holes.map(poly => ({ inside: inside(point, poly), distance: edgeDistance(point, poly) }));
  const siblings = configuration.closureMounts.filter(value => value.id !== selected.id).map(value => ({ id: value.id, distance: Math.hypot(point.x - value.at.x, point.y - value.at.y), required: (selected.bossDiameter ?? selected.holeDiameter) / 2 + (value.bossDiameter ?? value.holeDiameter) / 2 + 0.5 }));
  return { point, accepted: outer.some(value => value.inside && value.distance >= margin) && !holes.some(value => value.inside || value.distance < margin) && siblings.every(value => value.distance >= value.required), outer, holes, siblings };
};
const reference = JSON.parse(execFileSync('python3', ['-c', 'import zipfile,sys; print(zipfile.ZipFile(sys.argv[1]).read("project.json").decode())', path.join(evidence, 'reference-after-move.boardstudio')], { maxBuffer: 4 * 1024 * 1024 }).toString());
const endpoint = reference.hardware.instances.find(value => value.id === 'right').mechanical.closureMounts.find(value => value.id === selected.id).at;
const result = {
  provider: path.relative(root, wasmPath), provider_sha256: crypto.createHash('sha256').update(wasm).digest('hex'),
  source_commit: 'eac47f42fec27816d09eec0530fb09e074c6685e', input_revision: opened.document.revision,
  selected_instance: instance.id, selected_mount: selected.id, constraint_body: bottom.body.id,
  z: assembly.stack.find(value => value.id === 'bottom').z + 0.8, margin,
  generation_blocked: assembly.generationBlocked, diagnostics: assembly.diagnostics,
  input_position: evaluate(selected.at), reference_endpoint: evaluate(endpoint),
  constraints, prepared_bottom: bottom,
};
fs.writeFileSync(path.join(evidence, 'prepared-constraint-analysis.json'), JSON.stringify(result, null, 2) + '\n');
console.log(JSON.stringify({ provider_sha256: result.provider_sha256, z: result.z, margin, outer_count: constraints.outer.length, hole_count: constraints.holes.length, input: result.input_position, reference: result.reference_endpoint }, null, 2));
engine.free();
