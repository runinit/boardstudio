// Independently authored inputs for two models; expected upstream poses come only from points.parse.
const COLS = ['a', 'b', 'c'], ROWS = ['r0', 'r1', 'r2'];
const matrix = (id = 'main', patch = {}) => ({ id, rows: 3, columns: 3,
  pitch: { x: 19, y: 19 }, origin: { x: 0, y: 0 }, ...patch });
const cell = (row, column, patch) => ({ row, column, enabled: true, ...patch });
const mapping = (id = 'main', zone = 'm', cols = COLS, rows = ROWS, prefix = '') =>
  cols.flatMap((col, column) => rows.map((name, row) => ({
    ergogenName: `${prefix}${zone}_${col}_${name}`, matrix: id, row, column,
  })));
const zone = (columns = {}, rows = {}, extra = {}) => ({
  columns: Object.fromEntries(COLS.map(name => [name, columns[name] || {}])),
  rows: Object.fromEntries(ROWS.map(name => [name, rows[name] || {}])), ...extra,
});
const grid = (id, columns = {}, rows = {}, extra = {}, patch = {}) => ({ id,
  comparison: 'match', ergogenConfig: { points: { zones: { m: zone(columns, rows, extra) } } },
  boardStudioMatrices: [matrix('main', patch)], mapping: mapping(),
});
function mirrorInput(source, id, axis, anchorMatrix) {
  const result = structuredClone(source);
  result.id = id;
  result.origin.x = source.anchor ? -source.origin.x : 2 * axis - source.origin.x;
  result.rotation = -(source.rotation || 0);
  result.mirror = source.mirror === 'x' ? 'none' : 'x';
  if (result.anchor) result.anchor.matrixId = anchorMatrix;
  return result;
}

const cases = [{
  id: 'translated-and-turned',
  comparison: 'match',
  ergogenConfig: { points: { zones: { m: {
    anchor: { shift: [10, 20], rotate: 30 }, columns: { a: {} }, rows: { r0: {} },
  } } } },
  boardStudioMatrices: [{ id: 'main', rows: 1, columns: 1,
    pitch: { x: 19, y: 19 }, origin: { x: 10, y: 20 }, rotation: 30 }],
  mapping: [{ ergogenName: 'm_a_r0', matrix: 'main', row: 0, column: 0 }],
},
grid('S1-grid'),
grid('S2-stagger', { b: { key: { stagger: 5 } }, c: { key: { stagger: 3 } } }, {}, {},
  { columnLayout: [{}, { stagger: 5 }, { stagger: 3 }] }),
grid('S3-splay', { a: { key: { stagger: 2, splay: 4 } }, b: { key: { stagger: 5, splay: -10 } },
  c: { key: { stagger: 3, splay: -5 } } }, {}, {},
  { columnLayout: [{ stagger: 2, splay: 4 }, { stagger: 5, splay: -10 }, { stagger: 3, splay: -5 }] }),
grid('S4-relative-origin', { b: { key: { stagger: 5, splay: -10, origin: [-9.5, -9.5] } },
  c: { key: { splay: -5 } } }, {}, {},
  { columnLayout: [{}, { stagger: 5, splay: -10, splayOrigin: { x: -9.5, y: -9.5 } }, { splay: -5 }] }),
grid('S4b-origin-follows-stagger', { b: { key: { stagger: 8, splay: -10, origin: [-9.5, -9.5] } },
  c: { key: { splay: -5 } } }, {}, {},
  { columnLayout: [{}, { stagger: 8, splay: -10, splayOrigin: { x: -9.5, y: -9.5 } }, { splay: -5 }] }),
grid('S5-spread', { b: { key: { spread: 23 } } }, {}, {}, { columnLayout: [{}, { spread: 23 }] }),
grid('S5b-spread-splay', { b: { key: { spread: 23, splay: -10 } } }, {}, {},
  { columnLayout: [{}, { spread: 23, splay: -10 }] }),
grid('S5d-column-offset', { b: { key: { stagger: 5, splay: -10, adjust: { shift: [4, 2] } } },
  c: { key: { splay: -5 } } }, {}, {},
  { columnLayout: [{}, { stagger: 5, splay: -10, offset: { x: 4, y: 2 } }, { splay: -5 }] }),
grid('S6-row-shift', {}, { r1: { shift: [0, 3] } }, {}, { rowLayout: [{}, { shift: { x: 0, y: 3 } }] }),
grid('S6b-row-offset', { b: { key: { splay: -10 } } }, { r1: { adjust: { shift: [2, 3] } } }, {},
  { columnLayout: [{}, { splay: -10 }], rowLayout: [{}, { offset: { x: 2, y: 3 } }] }),
// Ergogen padding belongs to the outgoing key; BoardStudio stores the step on the incoming row.
grid('S7-padding', {}, { r1: { padding: 22 } }, {}, { rowLayout: [{}, {}, { padding: 22 }] }),
grid('S8-key-bend', { b: { rows: { r1: { rotate: 10 } } } }, {}, {},
  { cells: [cell(1, 1, { bend: 10 })] }),
grid('S9-key-offset-rotation', { b: { key: { splay: -10 }, rows: { r1: { adjust: { shift: [3, 1], rotate: 15 } } } } },
  {}, {}, { columnLayout: [{}, { splay: -10 }], cells: [cell(1, 1, { offset: { x: 3, y: 1 }, rotation: 15 })] }),
grid('S10-key-shift', { b: { rows: { r1: { shift: [2, 0] } } } }, {}, {},
  { cells: [cell(1, 1, { shift: { x: 2, y: 0 } })] }),
grid('S11-preoriented-adjust-equivalent', { b: { rows: { r1: { adjust: { orient: 30, shift: [3, 0] } } } } },
  {}, {}, { cells: [cell(1, 1, { offset: { x: 2.598076211353316, y: 1.5 }, rotation: 30 })] }),
grid('S12-world-turn-equivalent', {}, {}, { anchor: { shift: [50, 20] }, rotate: 15 },
  { origin: { x: 43.119910412403, y: 32.259468780907405 }, rotation: 15 }),
grid('S13-matrix-frame', { b: { key: { stagger: 5, splay: -10 } } }, {},
  { anchor: { shift: [50, 20], rotate: 15 } },
  { origin: { x: 50, y: 20 }, rotation: 15, columnLayout: [{}, { stagger: 5, splay: -10 }] }),
grid('S15-additive-offset-divergence', { b: { key: { adjust: { shift: [4, 0] } } } },
  { r1: { adjust: { shift: [0, 3] } } }, {},
  { columnLayout: [{}, { offset: { x: 4, y: 0 } }], rowLayout: [{}, { offset: { x: 0, y: 3 } }] }),
grid('S15a-one-offset-level', { b: { key: { adjust: { shift: [4, 0] } } } }, {}, {},
  { columnLayout: [{}, { offset: { x: 4, y: 0 } }] }),
grid('X1-bend-then-offset', { b: { key: { splay: -8 }, rows: {
  r0: { rotate: 12 }, r1: { adjust: { shift: [2, 1], rotate: 5 } },
} } }, {}, {}, { columnLayout: [{}, { splay: -8 }],
  cells: [cell(0, 1, { bend: 12 }), cell(1, 1, { offset: { x: 2, y: 1 }, rotation: 5 })] }),
grid('X2-curved-column', { b: { key: { spread: 21, stagger: 4, splay: -6, origin: [2, -3] } } },
  { r0: { shift: [1, 0], rotate: -4 }, r1: { padding: 20 } }, {},
  { columnLayout: [{}, { spread: 21, stagger: 4, splay: -6, splayOrigin: { x: 2, y: -3 } }],
    rowLayout: [{ shift: { x: 1, y: 0 }, bend: -4 }, {}, { padding: 20 }] }),
grid('X3-row-key-sum', { b: { rows: { r1: { shift: [1, 2], rotate: 5 } } } },
  { r1: { shift: [0.25, 0.5], rotate: 2 } }, {},
  { rowLayout: [{}, { shift: { x: 0.25, y: 0.5 }, bend: 2 }],
    cells: [cell(1, 1, { shift: { x: 0.75, y: 1.5 }, bend: 3 })] }),
grid('zero-negative-steps', { b: { key: { spread: 0 } }, c: { key: { spread: -5 } } },
  { r0: { padding: 0 }, r1: { padding: -3 } }, {},
  { columnLayout: [{}, { spread: 0 }, { spread: -5 }], rowLayout: [{}, { padding: 0 }, { padding: -3 }] }),
];

const divergence = cases.find(item => item.id === 'S15-additive-offset-divergence');
divergence.comparison = 'intentional-divergence';
divergence.differences = [{ matrix: 'main', row: 1, column: 1, pose: { x: 23, y: 22, rotation: 0 } }];
divergence.note = 'Ergogen specific adjust replaces the column value: (19,22). BoardStudio adds offsets: (23,22). Compare other coordinates normally.';
cases.find(item => item.id === 'S11-preoriented-adjust-equivalent').note =
  'This fixed input pre-rotates (3,0) into its offset; it does not add an orient parameter.';
cases.find(item => item.id === 'S12-world-turn-equivalent').note =
  'The authored Origin is R(15)*(50,20); no world-origin rotation feature is implied.';

const defaultPitch = grid('default-19.05-pitch', {}, {}, {}, { pitch: { x: 19.05, y: 19.05 } });
defaultPitch.ergogenConfig.points.key = { spread: 19.05, padding: 19.05 };
cases.push(defaultPitch);

const mirrored = grid('S14-mirrored-heading', { b: { key: { stagger: 5, splay: -10 }, rows: {
  r1: { shift: [1, 2], rotate: 6, adjust: { rotate: 7 } },
} } }, {}, { anchor: { shift: [50, 20], rotate: 15 }, mirror: 120 },
{ origin: { x: 50, y: 20 }, rotation: 15, columnLayout: [{}, { stagger: 5, splay: -10 }],
  cells: [cell(1, 1, { shift: { x: 1, y: 2 }, bend: 6, rotation: 7 })] });
mirrored.boardStudioMatrices.push(mirrorInput(mirrored.boardStudioMatrices[0], 'right', 120));
mirrored.mapping.push(...mapping('right', 'm', COLS, ROWS, 'mirror_'));
cases.push(mirrored);

const fullHeading = grid('full-heading-not-modulo-180', { a: { rows: { r0: { rotate: 270, adjust: { rotate: 45 } } } } },
  {}, { mirror: 0 }, { cells: [cell(0, 0, { bend: 270, rotation: 45 })] });
fullHeading.boardStudioMatrices.push(mirrorInput(fullHeading.boardStudioMatrices[0], 'right', 0));
fullHeading.mapping.push(...mapping('right', 'm', COLS, ROWS, 'mirror_'));
cases.push(fullHeading);

const skipped = grid('skipped-key-carries-and-anchors', { b: { rows: { r1: { skip: true, shift: [2, 0], rotate: 10 } } } },
  {}, {}, { cells: [cell(1, 1, { enabled: false, shift: { x: 2, y: 0 }, bend: 10 })] });
skipped.mapping.find(item => item.row === 1 && item.column === 1).absent = true;
skipped.ergogenConfig.points.zones.probe = {
  anchor: { ref: 'm_b_r1' }, columns: { a: {} }, rows: { r0: {} },
};
skipped.boardStudioMatrices.push(matrix('probe', { rows: 1, columns: 1,
  anchor: { matrixId: 'main', row: 1, column: 1, follow: ['x', 'y', 'rotation'] } }));
skipped.mapping.push(...mapping('probe', 'probe', ['a'], ['r0']));
cases.push(skipped);

for (const asym of ['source', 'clone']) {
  const item = grid(`asym-${asym}`, { b: { rows: { r1: { asym, rotate: 10 } } } }, {}, { mirror: 50 },
    { cells: [cell(1, 1, { bend: 10, enabled: asym !== 'clone' })] });
  const right = mirrorInput(item.boardStudioMatrices[0], 'right', 50);
  right.cells = [cell(1, 1, { bend: 10, enabled: asym !== 'source' })];
  item.boardStudioMatrices.push(right);
  item.mapping.push(...mapping('right', 'm', COLS, ROWS, 'mirror_'));
  item.mapping.find(entry => entry.matrix === (asym === 'source' ? 'right' : 'main') &&
    entry.row === 1 && entry.column === 1).absent = true;
  item.note = 'Enabled flags model output filtering only; actual Local/Omitted ownership is production lifecycle acceptance, not this oracle.';
  cases.push(item);
}

for (const affect of [[], ['x'], ['y'], ['r'], ['x', 'y'], ['x', 'r'], ['y', 'r'], ['x', 'y', 'r']]) {
  const follow = affect.map(value => value === 'r' ? 'rotation' : value);
  cases.push({ id: `anchor-follow-${affect.join('') || 'none'}`, comparison: 'match',
    ergogenConfig: { points: { zones: {
      m: { anchor: { shift: [10, 20], rotate: 30 }, columns: { a: {} }, rows: { r0: {} } },
      thumb: { anchor: { ref: 'm_a_r0', shift: [3, 4], rotate: 10, affect }, columns: { a: {} }, rows: { r0: {} } },
    } } },
    boardStudioMatrices: [matrix('main', { rows: 1, columns: 1, origin: { x: 10, y: 20 }, rotation: 30 }),
      matrix('thumb', { rows: 1, columns: 1, origin: { x: 3, y: 4 }, rotation: 10,
        anchor: { matrixId: 'main', row: 0, column: 0, follow } })],
    mapping: [...mapping('main', 'm', ['a'], ['r0']), ...mapping('thumb', 'thumb', ['a'], ['r0'])],
  });
}

const anchored = grid('anchor-final-key-and-mirror', { b: { key: { splay: -10 }, rows: {
  r1: { adjust: { shift: [3, 1], rotate: 15 } },
} } }, {}, { anchor: { shift: [50, 20], rotate: 15 }, mirror: 120 },
{ origin: { x: 50, y: 20 }, rotation: 15, columnLayout: [{}, { splay: -10 }],
  cells: [cell(1, 1, { offset: { x: 3, y: 1 }, rotation: 15 })] });
anchored.boardStudioMatrices.push(mirrorInput(anchored.boardStudioMatrices[0], 'right', 120));
anchored.mapping.push(...mapping('right', 'm', COLS, ROWS, 'mirror_'));
anchored.ergogenConfig.points.zones.thumb = { anchor: { ref: 'm_b_r1', shift: [4, -8], rotate: 12 },
  columns: { a: {}, b: {}, c: {} }, rows: { r0: {} }, mirror: 120 };
const thumb = matrix('thumb', { rows: 1, origin: { x: 4, y: -8 }, rotation: 12,
  anchor: { matrixId: 'main', row: 1, column: 1, follow: ['x', 'y', 'rotation'] } });
anchored.boardStudioMatrices.push(thumb, mirrorInput(thumb, 'right-thumb', 120, 'right'));
anchored.mapping.push(...mapping('thumb', 'thumb', COLS, ['r0']), ...mapping('right-thumb', 'thumb', COLS, ['r0'], 'mirror_'));
anchored.links = [{ source: 'main', mirror: 'right', axisX: 120 }, { source: 'thumb', mirror: 'right-thumb', axisX: 120 }];
cases.push(anchored);

const partial = {
  id: 'linked-partial-follow-reflects-zero', comparison: 'match',
  ergogenConfig: { points: { zones: {
    m: { anchor: { shift: [20, 30], rotate: 30 }, columns: { a: {} }, rows: { r0: {} }, mirror: 50 },
    thumb: { anchor: { ref: 'm_a_r0', affect: ['y'] }, columns: { a: {} }, rows: { r0: {} }, mirror: 50 },
  } } },
  boardStudioMatrices: [matrix('main', { rows: 1, columns: 1, origin: { x: 20, y: 30 }, rotation: 30 }),
    matrix('thumb', { rows: 1, columns: 1, anchor: { matrixId: 'main', row: 0, column: 0, follow: ['y'] } })],
  mapping: [...mapping('main', 'm', ['a'], ['r0']), ...mapping('thumb', 'thumb', ['a'], ['r0'])],
  links: [{ source: 'main', mirror: 'right', axisX: 50 }, { source: 'thumb', mirror: 'right-thumb', axisX: 50 }],
};
partial.boardStudioMatrices.push(mirrorInput(partial.boardStudioMatrices[0], 'right', 50),
  mirrorInput(partial.boardStudioMatrices[1], 'right-thumb', 50, 'right'));
partial.mapping.push(...mapping('right', 'm', ['a'], ['r0'], 'mirror_'),
  ...mapping('right-thumb', 'thumb', ['a'], ['r0'], 'mirror_'));
cases.push(partial);

const average = grid('aggregate-average');
average.ergogenConfig.points.zones.probe = {
  anchor: { aggregate: { parts: [{ ref: 'm_a_r0' }, { ref: 'm_c_r0' }] } },
  columns: { a: {} }, rows: { r0: {} },
};
average.boardStudioMatrices.push({ id: 'probe', rows: 1, columns: 1,
  pitch: { x: 19, y: 19 }, origin: { x: 0, y: 0 },
  anchor: { follow: ['x', 'y', 'rotation'],
    aggregate: { method: 'average', parts: [
      { matrixId: 'main', row: 0, column: 0 },
      { matrixId: 'main', row: 0, column: 2 }] } } });
average.mapping.push(...mapping('probe', 'probe', ['a'], ['r0']));
cases.push(average);

const intersect = grid('aggregate-intersect', { c: { rows: { r2: { rotate: 90 } } } }, {}, {},
  { cells: [cell(2, 2, { bend: 90 })] });
intersect.ergogenConfig.points.zones.probe = {
  anchor: { aggregate: { method: 'intersect', parts: [{ ref: 'm_a_r0' }, { ref: 'm_c_r2' }] } },
  columns: { a: {} }, rows: { r0: {} },
};
intersect.boardStudioMatrices.push({ id: 'probe', rows: 1, columns: 1,
  pitch: { x: 19, y: 19 }, origin: { x: 0, y: 0 },
  anchor: { follow: ['x', 'y', 'rotation'],
    aggregate: { method: 'intersect', parts: [
      { matrixId: 'main', row: 0, column: 0 },
      { matrixId: 'main', row: 2, column: 2 }] } } });
intersect.mapping.push(...mapping('probe', 'probe', ['a'], ['r0']));
cases.push(intersect);

const pose = (matrix, row, column, x, y, rotation = 0) => ({ matrix, row, column, pose: { x, y, rotation } });
const quarter = matrix('main', { rows: 3, pitch: { x: 10, y: 10 }, columnLayout: [{}, { spread: 30, rotation: 90 }] });
const held = [pose('main', 0, 0, 0, 0), pose('main', 1, 0, 0, 10),
  pose('main', 0, 1, 10, 0, 180), pose('main', 1, 1, 10, -10, 180),
  pose('main', 0, 2, 10, 10, 90), pose('main', 1, 2, 0, 10, 90)];
const zeroHeld = [pose('left', 0, 0, 0, 0, 180), pose('left', 1, 0, 0, -10, 180),
  pose('left', 0, 1, 0, 10, 90), pose('left', 1, 1, -10, 10, 90),
  pose('right', 0, 0, 100, 0, -180), pose('right', 1, 0, 100, -10, -180),
  pose('right', 0, 1, 100, 10, -90), pose('right', 1, 1, 110, 10, -90)];
const zeroBefore = matrix('left', { rows: 2, columns: 2, pitch: { x: 10, y: 10 }, columnLayout: [{ splay: 90, rotation: 90 }] });
const zeroAfter = { ...structuredClone(zeroBefore), origin: { x: -2, y: -4 },
  columnLayout: [{ splay: 90, rotation: 90, splayOrigin: { x: 1, y: 2 } }, { spread: 13, stagger: 1 }] };
const boardStudioOnly = [
  { id: 'column-rotation-does-not-carry', basis: 'A quarter-turn about column 1 base (30,0); column 2 remains unturned.',
    boardStudioMatrices: [quarter], expected: [
      pose('main', 0, 0, 0, 0), pose('main', 1, 0, 0, 10), pose('main', 2, 0, 0, 20),
      pose('main', 0, 1, 30, 0, 90), pose('main', 1, 1, 20, 0, 90), pose('main', 2, 1, 10, 0, 90),
      pose('main', 0, 2, 40, 0), pose('main', 1, 2, 40, 10), pose('main', 2, 2, 40, 20),
    ] },
  { id: 'local-y-mirror-full-heading', basis: 'Reflect local Y; negate Splay plus independent Key rotation together.',
    boardStudioMatrices: [matrix('main', { rows: 2, columns: 2, pitch: { x: 10, y: 10 }, origin: { x: 5, y: 7 },
      mirror: 'y', columnLayout: [{}, { splay: 90 }], cells: [cell(0, 1, { rotation: 30 })] })],
    expected: [pose('main', 0, 0, 5, 7), pose('main', 1, 0, 5, -3),
      pose('main', 0, 1, 15, 7, -120), pose('main', 1, 1, 5, 7, -90)] },
  { id: 'hold-origin-with-column-rotation', basis: 'Quarter/half-turn worked example: delta(0,2), this step +(0,-4), next +(2,2).',
    beforeMatrices: [matrix('main', { rows: 2, pitch: { x: 10, y: 10 }, columnLayout: [{}, { splay: 90, rotation: 90 }] })],
    edit: { matrix: 'main', column: 1, storedOriginDelta: { x: 0, y: 2 }, mode: 'hold' },
    afterMatrices: [matrix('main', { rows: 2, pitch: { x: 10, y: 10 }, columnLayout: [{},
      { spread: 10, stagger: -4, splay: 90, rotation: 90, splayOrigin: { x: 0, y: 2 } }, { spread: 12, stagger: 2 }] })],
    expectedBefore: held, expectedAfter: held },
  { id: 'hold-column-zero-linked-origin', basis: 'delta(1,2), matrix Origin +(-2,-4), next step +(3,1); reflected Origin becomes (102,-4).',
    beforeMatrices: [zeroBefore, mirrorInput(zeroBefore, 'right', 50)],
    edit: { matrix: 'left', column: 0, storedOriginDelta: { x: 1, y: 2 }, mode: 'hold', requestedWorldOrigin: { x: -1, y: -2 } },
    afterMatrices: [zeroAfter, mirrorInput(zeroAfter, 'right', 50)],
    links: [{ source: 'left', mirror: 'right', axisX: 50 }],
    expectedBefore: zeroHeld, expectedAfter: zeroHeld },
];

module.exports = { cases, boardStudioOnly };
