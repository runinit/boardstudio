// TEMPORARY recorder for the framework's numeric and render-context vectors
// (step 2 of docs/investigations/footprint-generators-rust.md). Like record.mjs
// it needs the JavaScript provider and is deleted with it in step 6.
//
//   node --no-warnings --import ./footprints/tests/golden/harness/register.mjs \
//     footprints/tests/golden/harness/record_context.mjs
//
// numeric_vectors.json: JavaScript number formatting, Number() parsing,
//   Math.sin/cos/hypot results, encodeURIComponent.
// context_vectors.json: the real render() driven through a synthetic generator
//   that echoes every field and helper of the `p` render context.
import { writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../../../', import.meta.url));
const out = `${root}footprints/tests/golden/`;
const provider = await import(`${root}ergogen/src/index.ts`);
const { default: modules } = await import(`${root}ergogen/generated/catalogue.mjs`);
const { render, serialize, parameters } = provider;

// ----- Numeric vectors -----------------------------------------------------

const formatted = [0, 1, -1, 0.5, -0.5, 1.5, 100, 7.109999999999999, 0.1 + 0.2, 1 / 3, 2 / 3, 1e21, 1e-7, 1.5e-7,
  123456789012345680000, 1e20, 0.000001, 0.0000001, 5e-324, 1.7976931348623157e308, 1e300, 123456.789, -0.000123,
  4.35, 4.349999999999999, 17.5, 5.08, 2.54, 19.05, 9.525, 12345678901234567890, 0.30000000000000004, 1e6, 1e-6,
  255, 1e15, 1e16, 1e17, 123e-20, 4.1, 7.2, 3.0000000000000004, Math.PI, Math.E, -1e21, -1e-7, 8.41, 6.58, 0.01];
const parsed = ['0', '1', ' 12 ', '', ' ', '1e3', '1E3', '.5', '5.', '+5', '-5', '0x10', '0X1f', '0b101', '0o17', 'Infinity', '-Infinity',
  'infinity', 'inf', 'nan', 'NaN', '1_000', '1,5', '0.1.2', 'abc', '12abc', ' 12 ', '﻿7', '1e', 'e1', '--1', '1e+3',
  '1e-3', '00012', '-0', '9007199254740993', '1e400', '-1e400', '0x', '0xg', '\u0000abc'];
const degrees = [0, 90, 180, 270, 360, 37, -37, 23, 45, 60, 30, 15, 12.5, 359.9, 0.1, 89.999, 135, 225, 315, 720, 1, 2, 3, 5, 10, 20,
  40, 50, 70, 80, 100, 110, 120, 150, 170, 190, 200, 250, 300, 330, 17.5, 22.5, 67.5, 123.456, -90, -180, -45, 1e-9];
const fixedValues = [0, -0, 0.5, 1, 2.501231, -2.50123, 0.0078125, -0.0078125, 2.5e-7, 5e-7, 1.0000005, 1.0000015, 0.0000005, -0.0000005,
  1e-10, -1e-10, 123456.1234565, 1.005, 2.675, 4.35, 17.5, 9.5, 0.1 + 0.2, 1 / 3, -1 / 3, 99999.9999995, 999999.9999995, 1e20, 1e21, 1.5e21,
  -1e21, 0.9999995, 0.99999949999, 12.3456785, 7.109999999999999, 1.7e-6, 123456789.123456789, Math.PI, -Math.PI, 5.5, 6.5, 0.045];
const fixedDigits = [6, 2, 0, 3];
const hypots = [[0.01, 0.02], [0.03, 0], [0.02, 0.0224], [1, 1], [3, 4], [1e-9, 1e-9], [0.0299999, 0], [0.3, 0.4]];
const encoded = ['a b', 'é', '', 'part-1', 'matrix/left-keys/r0c0', "!~*'()", '${KIPRJMOD}/models/x.step', 'custom://x.step', '日本', '😀', '%', 'a&b=c'];
writeFileSync(`${out}numeric_vectors.json`, `${JSON.stringify({
  format: formatted.map((value) => [value, String(value)]),
  toNumber: parsed.map((text) => [text, Number(text)]).map(([text, value]) => ({ text, value: Number.isNaN(value) ? 'NaN' : Object.is(value, -0) ? '-0' : value === Infinity ? 'Infinity' : value === -Infinity ? '-Infinity' : value })),
  trig: degrees.map((degree) => {
    const angle = degree * Math.PI / 180;
    return { degrees: degree, cos: Math.cos(angle), sin: Math.sin(angle), cosText: String(Math.cos(angle)), sinText: String(Math.sin(angle)) };
  }),
  toFixed: fixedDigits.flatMap((digits) => fixedValues.map((value) => [value, digits, value.toFixed(digits)])),
  hypot: hypots.map(([x, y]) => [x, y, Math.hypot(x, y)]),
  encodeURIComponent: encoded.map((text) => [text, encodeURIComponent(text)]),
}, null, 1)}\n`);

// ----- Render-context vectors ----------------------------------------------

const SOURCE = 'test/context';
modules[SOURCE] = {
  params: {
    flag: false,
    count: 3,
    ratio: 1.5,
    label: 'hello',
    items: [1, 2, 3],
    grid: [[0, 0], [1, 2.5]],
    in: { type: 'net', value: 'DEFAULT_NET' },
    out: { type: 'net', value: '' },
    open: { type: 'net', value: undefined },
    big: 1e21,
    tiny: 1e-7,
  },
  body: (p) => `
(footprint "test:context" (layer "${p.side === 'B' ? 'B.Cu' : 'F.Cu'}") ${p.at}
  (x ${p.x}) (y ${p.y}) (r ${p.r}) (rot ${p.rot}) (xy ${p.xy}) (side ${p.side})
  (ref "${p.ref}") (hide "${p.ref_hide}")
  (point ${p.point.x} ${p.point.y} ${p.point.r} ${p.point.meta.mirrored})
  (isxy ${p.isxy(1.5, -2.25)}) (isxy0 ${p.isxy(0, 0)}) (iaxy ${p.iaxy(1.5, -2.25)})
  (esxy ${p.esxy(1.5, -2.25)}) (esxy0 ${p.esxy(0, 0)}) (eaxy ${p.eaxy(3, 4)}) (eaxy2 ${p.eaxy(-7.5, 0.125)})
  (flag ${p.flag}) (count ${p.count}) (ratio ${p.ratio}) (label ${JSON.stringify(p.label)})
  (items ${p.items.join(' ')}) (grid ${p.grid.map((point) => point.join(' ')).join(' / ')})
  (big ${p.big}) (tiny ${p.tiny})
  ${p.in} ${p.out} ${p.open}
  (in ${p.in.index} ${JSON.stringify(p.in.name)}) (out ${p.out.index} ${JSON.stringify(p.out.name)}) (open ${p.open.index} ${JSON.stringify(p.open.name)})
  ${p.local_net('a b')} ${p.local_net('é')} ${p.local_net('a b')}
)`,
};

const definition = (parametersValue = {}, id = 'ergogen:test/context') => ({
  id, name: 'context', kind: 'custom', pads: [], courtyard: [],
  generator: { source: SOURCE, version: 'bundled-1', parameters: parametersValue },
});

function allocator(reserved, next) {
  const nets = reserved.map((net) => ({ ...net }));
  let nextIndex = next;
  return {
    lookup(name) {
      const known = nets.find((net) => net.name === name);
      if (known) return known.index;
      const net = { name, index: nextIndex++ };
      nets.push(net);
      return net.index;
    },
    snapshot: () => nets.map((net) => ({ ...net })),
  };
}

const cases = [];
const addCase = (id, definitionParameters, part, { reserved = [{ name: 'GND', index: 1 }], next = 2, standalone = false, definitionId } = {}) => {
  const source = definition(definitionParameters, definitionId);
  const a = standalone ? null : allocator(reserved, next);
  let output;
  try {
    output = { forms: render(source, { part, netIndex: a?.lookup }).map(serialize), nets: a?.snapshot() };
  } catch (error) {
    output = { error: error instanceof Error ? error.message : String(error) };
  }
  cases.push({ id, input: { definitionId: source.id, definitionParameters, part: part ?? null, reservedNets: standalone ? null : reserved, nextNetIndex: standalone ? null : next }, output });
};
const part = (pose, side = 'front', generatorParameters, extra = {}) => ({
  id: 'part-1', definitionId: 'ergogen:test/context', reference: 'U1', pose, side,
  ...(generatorParameters ? { generatorParameters } : {}), ...extra,
});

addCase('standalone-defaults', {}, undefined, { standalone: true });
addCase('standalone-with-allocator', {}, undefined);
addCase('standalone-other-id', {}, undefined, { standalone: true, definitionId: 'definition/other id' });
const poses = [[0, 0, 0], [12.5, -7.25, 0], [12.5, -7.25, 90], [12.5, -7.25, 37], [-33.3, 41.7, -37], [0, 0, 180],
  [100, 200, 270], [1e-7, -1e-7, 12.5], [5, 5, 359.9], [0.1, 0.2, 720], [3, -3, 45], [3, -3, 123.456]];
for (const side of ['front', 'back']) {
  for (const [x, y, rotation] of poses) {
    addCase(`pose:${side}:${x},${y},${rotation}`, {}, part({ at: { x, y }, rotation }, side));
  }
}
addCase('side:param-B-front', {}, part({ at: { x: 1, y: 2 }, rotation: 10 }, 'front', { side: 'B' }));
addCase('side:param-F-back', {}, part({ at: { x: 1, y: 2 }, rotation: 10 }, 'back', { side: 'F' }));
addCase('side:param-weird', {}, part({ at: { x: 1, y: 2 }, rotation: 10 }, 'front', { side: 'X' }));
addCase('reference:custom', {}, part({ at: { x: 0, y: 0 }, rotation: 0 }, 'front', undefined, { reference: 'SW99' }));
addCase('params:definition-values', { flag: true, count: 7, ratio: 0.25, label: 'a"b\\c', items: [9, 8], grid: [[3, 4]], in: 'N1', out: 'N2' }, undefined);
addCase('params:part-overrides', { flag: true, count: 7 }, part({ at: { x: 1, y: 1 }, rotation: 0 }, 'front', { flag: false, count: 8, in: 'GND', out: 'N3', open: 'N4' }));
addCase('params:null-falls-back', {}, part({ at: { x: 1, y: 1 }, rotation: 0 }, 'front', { count: null, in: null, label: null }));
addCase('params:undeclared-ignored', {}, part({ at: { x: 1, y: 1 }, rotation: 0 }, 'front', { unknown: 'x', 'side-extra': 1 }));
addCase('params:numbers', { big: 1e21, tiny: 1e-7, ratio: 0.1 + 0.2, count: 123456789012345680000 }, undefined);
addCase('nets:reserved-names', {}, part({ at: { x: 0, y: 0 }, rotation: 0 }, 'front', { in: 'GND', out: 'GND', open: 'GND' }));
addCase('nets:fresh-names-order', {}, part({ at: { x: 0, y: 0 }, rotation: 0 }, 'front', { open: 'Z', out: 'A', in: 'M' }));
addCase('nets:repeated-reserved', {}, part({ at: { x: 0, y: 0 }, rotation: 0 }, 'front', { in: 'SHARED', out: 'SHARED' }),
  { reserved: [{ name: 'SHARED', index: 1 }, { name: 'SHARED', index: 2 }], next: 3 });
addCase('nets:local-follow-part-id', {}, { ...part({ at: { x: 0, y: 0 }, rotation: 0 }), id: 'matrix/left keys/r0c0' });
addCase('nets:index-limit', {}, part({ at: { x: 0, y: 0 }, rotation: 0 }, 'front', { in: 'A', out: 'B' }), { reserved: [], next: 0xffff_ffff });
addCase('nets:numeric-name', {}, part({ at: { x: 0, y: 0 }, rotation: 0 }, 'front', { in: 5 }));

addCase('error:unknown-generator', {}, undefined, { standalone: true });
cases.at(-1).input.definitionParameters = null;
const unknown = (() => {
  const d = definition({}); d.generator.source = 'test/missing';
  try { render(d); return { error: 'none' }; } catch (error) { return { error: error.message }; }
})();
const version = (() => {
  const d = definition({}); d.generator.version = 'future';
  try { render(d); return { error: 'none' }; } catch (error) { return { error: error.message }; }
})();
cases.pop();

writeFileSync(`${out}context_vectors.json`, `${JSON.stringify({
  generator: { source: SOURCE, parameters: parameters(SOURCE) },
  unknownGenerator: unknown, unsupportedVersion: version,
  cases: cases.map((entry) => entry),
}, null, 0).replaceAll('},{"id"', '},\n{"id"')}\n`);
console.log(`numeric vectors, ${cases.length} context cases`);
