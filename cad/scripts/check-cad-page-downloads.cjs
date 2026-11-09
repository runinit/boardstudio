// Independent binary-STL delivery and material checks for the frozen U1 product workflow.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

function stl(data) {
  assert(data.length >= 84, 'truncated binary STL');
  const count = data.readUInt32LE(80);
  assert.equal(data.length, 84 + 50*count, 'complete binary STL triangle payload');
  const positions = new Float32Array(count*9), normals = new Float32Array(count*9);
  for (let t = 0; t < count; t++) {
    const offset = 84 + 50*t;
    assert.equal(data.readUInt16LE(offset+48), 0, 'no unrecorded attribute data');
    for (let i = 0; i < 9; i++) {
      positions[t*9+i] = data.readFloatLE(offset+12+4*i);
      normals[t*9+i] = data.readFloatLE(offset+4*(i%3));
    }
  }
  assert(positions.every(Number.isFinite) && normals.every(Number.isFinite));
  return { positions, normals };
}

function oracle() {
  // Reuse the existing independent topology/normals/material oracle without executing its provider matrix.
  const filename = path.join(__dirname, '../wasm/tests/keycap-print-legends.cjs');
  const source = fs.readFileSync(filename, 'utf8');
  const start = 'const dot =', end = 'const bareVolumes =';
  assert.equal(source.split(start).length, 2);
  assert.equal(source.split(end).length, 2);
  let code = source.slice(source.indexOf(start), source.indexOf(end));
  // U1 requests 18mm caps. Parameterize only the old fixture's hardcoded 18.2mm bounds;
  // retain the same 1e-5mm coordinate gate and every topology/material assertion.
  for (const [from, to] of [
    ['function closedPositive(mesh)', 'function closedPositive(mesh, spec)'],
    ['bounds[0][axis]+9.1', 'bounds[0][axis]+spec.size[axis === 0 ? "x" : "y"]/2'],
    ['bounds[1][axis]-9.1', 'bounds[1][axis]-spec.size[axis === 0 ? "x" : "y"]/2'],
  ]) {
    assert.equal(code.split(from).length, 2);
    code = code.replace(from, to);
  }
  const context = vm.createContext({ assert, Float32Array });
  vm.runInContext(code + '; globalThis.checks = {closedPositive, roofAndCavity, contains, surface};', context);
  return context.checks;
}

function validate(directory) {
  const workers = JSON.parse(fs.readFileSync(path.join(directory, 'page-workers.log')));
  const root = path.join(directory, 'downloads/keycaps');
  const manifest = JSON.parse(fs.readFileSync(path.join(root, 'keycaps.json')));
  const request = workers.requests.find(r => r.operation === 'print-keycaps');
  assert(request);
  assert.equal(manifest.revision, request.identity.revision);
  assert.equal(manifest.units, 'mm');
  const specs = request.keycaps.specs;
  assert.equal(manifest.keycaps.length, specs.length);
  assert.equal(specs.length, 4, 'complete frozen U1 four-cap export');
  const checks = oracle(), rows = [];
  for (const [i, spec] of specs.entries()) {
    const entry = manifest.keycaps[i], worker = workers.printBodies[i];
    assert.equal(entry.id, `keycap:${spec.id}`);
    assert.equal(entry.name, spec.reference);
    assert.equal(worker.id, entry.id);
    const mesh = stl(fs.readFileSync(path.join(root, entry.file)));
    assert.equal(mesh.positions.length, worker.positions.length);
    for (let index = 0; index < mesh.positions.length; index++) {
      // JSON observation normalizes signed zero; all other finite f32 values must match exactly.
      assert(mesh.positions[index] === worker.positions[index], 'STL preserves actual worker vertex values/order');
      const first = Math.floor(index/9)*9 + index%3;
      assert(mesh.normals[index] === worker.normals[first], 'STL preserves actual worker triangle normal');
    }
    assert.equal(spec.side, 'front');
    assert.equal(spec.pose.rotation, 0);
    const local = { positions: Float32Array.from(mesh.positions, (v, index) => v -
      [spec.pose.at.x, spec.pose.at.y, spec.z][index%3]), normals: mesh.normals };
    const volume = checks.closedPositive(local, spec);
    checks.roofAndCavity(spec, local);
    rows.push({ id: entry.id, triangles: mesh.positions.length/9, measuredVolumeMm3: volume,
      quality: 'closed/connected/outward, positive material, authored size, analytic roof/cavity/stem/socket probes',
      values: 'diagnostic-not-benchmark' });
  }
  // The decoder rejects the actual failure pattern of an incomplete download.
  const first = fs.readFileSync(path.join(root, manifest.keycaps[0].file));
  assert.throws(() => stl(first.subarray(0, first.length-1)), /complete binary STL/);
  console.log(JSON.stringify({ passed: true, validationOnly: true, rows }, null, 2));
}

module.exports = { stl, oracle };
if (require.main === module) validate(path.resolve(process.argv[2]));
