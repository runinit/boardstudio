// Actual WASM export regression: wasm-pack build cad/wasm --target nodejs --out-dir pkg-node
// Then: node cad/wasm/tests/keycap-print-legends.cjs /absolute/path/to/package.js
const assert = require('node:assert/strict');
const path = require('node:path');
const packagePath = path.resolve(process.argv[2]);
const cad = require(packagePath);
console.log(`Actual WASM build_keycaps via Node ${process.version}: ${packagePath}`);

// Same authored DCS row 3 and tilted SA fixtures and tolerances as print_legend_tests.rs.
function cap(profile, legend = '', mount = 'mx') {
  return {
    id: 'cap', reference: 'SW1', profile, mount, row: 3,
    size: {x: 18.2, y: 18.2}, topSize: {x: 13.2, y: 13.2},
    height: profile === 'dcs' ? 8.5 : 14, tilt: profile === 'dcs' ? 0 : 8,
    dishDepth: profile === 'dcs' ? 0.6 : 0.7, spherical: profile !== 'dcs',
    wallThickness: 1.2, pose: {at: {x: 0, y: 0}, rotation: 0},
    side: 'front', z: 0, travel: 4, legend, color: '#eeeeee', legendColor: '#222222',
  };
}
function print(spec) {
  const result = cad.build_keycaps({revision: 3, printable: true, specs: [spec]});
  assert.equal(result.revision, 3);
  assert(result.step instanceof Uint8Array);
  assert.equal(result.step.length, 0);
  assert.equal(result.bodies.length, 1, 'cap, stem and legend are one print body');
  assert.equal(result.bodies[0].id, 'keycap:cap');
  assert.equal(result.bodies[0].name, 'SW1');
  return result.bodies[0];
}
const dot = (a, b) => a[0]*b[0] + a[1]*b[1] + a[2]*b[2];
const cross = (a, b) => [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]];
const sub = (a, b) => a.map((v, i) => v-b[i]);
function* triangles(mesh) {
  for (let i = 0; i < mesh.positions.length; i += 9) {
    yield [0, 3, 6].map(offset => Array.from(mesh.positions.subarray(i+offset, i+offset+3)));
  }
}
function contains(mesh, point) {
  let angle = 0;
  for (const triangle of triangles(mesh)) {
    const [a, b, c] = triangle.map(p => sub(p, point));
    const length = p => Math.sqrt(dot(p, p));
    const denominator = length(a)*length(b)*length(c) + dot(a,b)*length(c)
      + dot(b,c)*length(a) + dot(c,a)*length(b);
    angle += 2*Math.atan2(dot(a, cross(b,c)), denominator);
  }
  return Math.abs(angle) > 2*Math.PI;
}
function closedPositive(mesh) {
  const {positions, normals} = mesh;
  assert(positions instanceof Float32Array && normals instanceof Float32Array);
  assert(positions.length > 0 && positions.length % 9 === 0);
  assert.equal(positions.length, normals.length);
  assert(positions.every(Number.isFinite) && normals.every(Number.isFinite));
  const edges = new Set(), links = new Map();
  let volume = 0, index = 0;
  for (const [a,b,c] of triangles(mesh)) {
    const face = cross(sub(b,a), sub(c,a)), length = Math.sqrt(dot(face,face));
    assert(length > 0, 'zero-area triangle');
    for (const offset of [0,3,6]) {
      const n = Array.from(normals.subarray(index+offset, index+offset+3));
      assert(dot(n,face)/length > 0.99, 'normal disagrees with winding');
    }
    index += 9;
    volume += dot(a,cross(b,c))/6;
    const keys = [a,b,c].map(p => p.join(','));
    for (let e = 0; e < 3; e++) {
      const from = keys[e], to = keys[(e+1)%3], key = `${from}|${to}`;
      assert(!edges.has(key), 'duplicate directed edge');
      edges.add(key);
      if (!links.has(from)) links.set(from, new Set());
      if (!links.has(to)) links.set(to, new Set());
      links.get(from).add(to);
      links.get(to).add(from);
    }
  }
  for (const edge of edges) assert(edges.has(edge.split('|').reverse().join('|')), 'open or inverted edge');
  const seen = new Set(), pending = [links.keys().next().value];
  while (pending.length) {
    const vertex = pending.pop();
    if (!seen.has(vertex)) {
      seen.add(vertex);
      pending.push(...links.get(vertex));
    }
  }
  assert.equal(seen.size, links.size, 'disconnected material');
  assert(volume > 0, 'positive material volume');
  // Authored base extents, in mm, independent of the mesh constructor.
  const bounds = [[Infinity,Infinity,Infinity], [-Infinity,-Infinity,-Infinity]];
  positions.forEach((v,i) => {
    bounds[0][i%3] = Math.min(bounds[0][i%3], v);
    bounds[1][i%3] = Math.max(bounds[1][i%3], v);
  });
  for (const axis of [0,1]) {
    assert(Math.abs(bounds[0][axis]+9.1) < 1e-5);
    assert(Math.abs(bounds[1][axis]-9.1) < 1e-5);
  }
  assert.equal(bounds[0][2], 0);
  return volume;
}
// Analytic sagitta and tilt, independent of the production roof/lettering implementation.
function surface(spec, x, y) {
  const depth = spec.dishDepth, half = spec.topSize.y/2;
  const radius = (half*half+depth*depth)/(2*depth);
  const distance = spec.spherical ? Math.hypot(x,y) : Math.abs(y);
  const sag = Math.min(depth, Math.max(0, depth-(radius-Math.sqrt(Math.max(0,radius*radius-distance*distance)))));
  return spec.height+y*Math.tan(spec.tilt*Math.PI/180)-sag;
}
function roofAndCavity(spec, mesh) {
  const roof = surface(spec,5,-4);
  for (const [point,material,what] of [
    [[5,-4,roof-0.6],true,'roof'], [[5,-4,roof-1.5],false,'cavity under roof'],
    [[5,-4,roof+0.5],false,'air above dish'], [[0,8.5,0.2],true,'side wall'],
    [[0,6,1],false,'cavity beside stem'], [[2.5,2.5,1],true,'stem'],
    [[0.137,0.113,1],false,'blind socket'],
  ]) assert.equal(contains(mesh,point), material, what);
}

const bareVolumes = new Map();
for (const profile of ['dcs','sa']) {
  bareVolumes.set(profile, closedPositive(print(cap(profile))));
  for (const legend of ['A','O','T','1','4','Q','Enter','Backspace','Shift','F12','←','→','`']) {
    const spec = cap(profile,legend), mesh = print(spec), volume = closedPositive(mesh);
    assert(volume > bareVolumes.get(profile)+0.005, 'legend material retained');
    if (legend === 'A') {
      assert(volume > bareVolumes.get(profile)+0.05 && volume < bareVolumes.get(profile)+0.6);
      roofAndCavity(spec,mesh);
    }
    if (legend === 'O') {
      assert(!contains(mesh,[0,0,surface(spec,0,0)+0.02]), 'O counter remains open above the roof');
    }
    console.log(JSON.stringify({profile,legend,triangles:mesh.positions.length/9,volumeMm3:volume}));
  }
}
// The second mount from the DCS sliver regression retains the same material/void checks.
const alpsSpec = cap('dcs','A','alps'), alps = print(alpsSpec);
const alpsDelta = closedPositive(alps)-closedPositive(print(cap('dcs','','alps')));
assert(alpsDelta > 0.05 && alpsDelta < 0.6);
roofAndCavity(alpsSpec,alps);
const dashSpec = cap('sa','-'), dash = print(dashSpec);
closedPositive(dash);
for (const x of [-5,-3,-1,0,1,3,5]) {
  const roof = surface(dashSpec,x,0);
  assert(contains(dash,[x,0,roof+0.02]), `dash lost relief at ${x}`);
  assert(!contains(dash,[x,0,roof+0.06]), `dash stands proud at ${x}`);
}
// Same frozen rounded-loft/socket volume and tolerance as the native printable MX fixture.
const flat = {...cap('dcs'), profile:'g20', height:6.5, dishDepth:0};
assert(Math.abs(closedPositive(print(flat))-824.019207008451) < 0.05);
assert.throws(() => print({...cap('dcs','A'), wallThickness:0.1}), /Invalid keycap mesh parameters/);
assert.throws(() => print({...cap('dcs','A'), height:3}), /roof is too low for the socket/);
assert.throws(() => cad.build_keycaps({revision:3, printable:true, specs:[]}), /at least one keycap/);
console.log('PASS: 26 ordinary legend bodies, DCS Alps A, long dash, analytic flat volume, roof/material/void/normals/dimensions, and 3 public refusal controls');
console.log('LIMIT: floating/open internal operands are tested natively; no public API accepts those operands. No browser/UI/ZIP check.');
