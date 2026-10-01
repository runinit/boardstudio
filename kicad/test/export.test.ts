import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { emptyProject } from '../../contracts/src/index.ts';
import type { Contour, ProjectDoc } from '../../contracts/src/index.ts';
import { compileNativeFootprint, exportNativeBoard, exportNativeFootprint, importNativeFootprint, nativeArtifact } from './nativeArtifact.ts';

const fixture = (): { doc: ProjectDoc; contours: Contour[] } => {
  const doc = emptyProject('project', 'Test');
  doc.revision = 2;
  doc.definitions.push({
    id: 'switch',
    name: 'Switch',
    kind: 'switch',
    courtyard: [
      { x: -7, y: -7 },
      { x: 7, y: -7 },
      { x: 7, y: 7 },
      { x: -7, y: 7 },
    ],
    pads: [
      { id: 'left', number: '1', at: { x: -3, y: 0 }, size: { x: 1.8, y: 1.8 }, shape: 'circle', drill: 0.9 },
      { id: 'right', number: '2', at: { x: 3, y: 0 }, size: { x: 1.8, y: 1.8 }, shape: 'circle', drill: 0.9 },
    ],
  });
  doc.parts.push({ id: 's1', definitionId: 'switch', reference: 'SW1', pose: { at: { x: 0, y: 0 }, rotation: 0 }, side: 'front' });
  doc.nets.push({ id: 'row', name: 'ROW0', pins: [{ partId: 's1', padId: 'left' }] });
  doc.boards.push({ id: 'main', name: 'main', outlineIds: [], partIds: ['s1'], netIds: ['row'], thickness: 1.6 });
  const contours = [{ hole: false, points: [
    { x: -12, y: -12 }, { x: 12, y: -12 }, { x: 12, y: 12 }, { x: -12, y: 12 },
  ] }];
  return { doc, contours };
};

const projectWith = (definition: ProjectDoc['definitions'][number]): ProjectDoc => {
  const doc = emptyProject(definition.id, definition.name);
  doc.definitions = [definition];
  return doc;
};

type DrcReport = {
  unconnected_items: { type: string }[];
  violations: { type: string }[];
};

const runDrc = (doc: ProjectDoc, contours: Contour[]): DrcReport => {
  const directory = mkdtempSync(join(tmpdir(), 'boardstudio-v2-drc-'));
  const path = join(directory, 'main.kicad_pcb');
  const report = join(directory, 'drc.json');
  writeFileSync(path, exportNativeBoard(doc, 'main', contours));
  execFileSync('kicad-cli', ['pcb', 'drc', '--format', 'json', '--output', report, path], { encoding: 'utf8' });
  return JSON.parse(readFileSync(report, 'utf8')) as DrcReport;
};

test('exports a deterministic board with flipped Y coordinates and net pads', () => {
  const { doc, contours } = fixture();
  const first = exportNativeBoard(doc, 'main', contours);
  assert.equal(first, exportNativeBoard(doc, 'main', contours));
  assert.match(first, /\(net 1 "ROW0"\)/u);
  assert.match(first, /\(start -12 12\) \(end 12 12\)/u);
  assert.match(first, /\(pad "1" thru_hole circle .*\(net 1 "ROW0"\)/u);
});

test('compiled geometry is immutable and ignores net labels', () => {
  const { doc } = fixture();
  const definition = doc.definitions[0];
  const first = compileNativeFootprint(definition);
  definition.pads[0].netId = 'row';
  const second = compileNativeFootprint(definition);

  assert.deepEqual(first.geometry, second.geometry);
  assert.equal('netId' in first.geometry.pads[0], false);
  assert.match(first.previewSvg ?? '', /<svg.*<rect/u);
  assert.notDeepEqual(compileNativeFootprint(definition, 'back').geometry, first.geometry);
});

test('exports front and back copper traces and vias', () => {
  const { doc, contours } = fixture();
  doc.boards[0].traces = [
    { id: 'front', start: { x: -3, y: 0 }, end: { x: 0, y: 0 }, width: 0.25, layer: 'front', netId: 'row' },
    { id: 'back', start: { x: 0, y: 0 }, end: { x: 3, y: 0 }, width: 0.25, layer: 'back', netId: 'row' },
  ];
  doc.boards[0].vias = [{ id: 'through', at: { x: 0, y: 0 }, size: 0.8, drill: 0.4, netId: 'row' }];

  const board = exportNativeBoard(doc, 'main', contours);
  assert.match(board, /\(segment \(start -3 0\) \(end 0 0\).*\(layer "F.Cu"\) \(net 1\)/u);
  assert.match(board, /\(segment \(start 0 0\) \(end 3 0\).*\(layer "B.Cu"\) \(net 1\)/u);
  assert.match(board, /\(via \(at 0 0\) \(size 0.8\) \(drill 0.4\).*\(net 1\)/u);
  assert.ok(Array.isArray(runDrc(doc, contours).violations));
});

test('rejects stale revisions and conflicting net assignments', () => {
  const { doc, contours } = fixture();
  assert.throws(() => nativeArtifact({ id: 'stale', kind: 'prepare-export', request: {
    snapshotToken: 'stale', expectedRevision: 1, document: doc,
    target: { kind: 'board', boardId: 'main' }, contours, modelPaths: {},
  } }), /committed current v2 revision/u);
  doc.definitions[0].pads[0].netId = 'another';
  assert.throws(() => exportNativeBoard(doc, 'main', contours), /Conflicting net/u);
});

test('requires relative model paths and emits standalone footprints', () => {
  const { doc } = fixture();
  const definition = doc.definitions[0];
  definition.models = [{
    assetId: 'switch-model',
    offset: { x: 0, y: 0, z: 0 },
    rotation: { x: 0, y: 0, z: 0 },
    scale: { x: 1, y: 1, z: 1 },
  }];
  const project = emptyProject('models', 'Models');
  project.definitions = [definition];
  assert.throws(() => exportNativeFootprint(project, definition.id, new Map([['switch-model', '../secret.step']])), /safe relative path/u);
  assert.match(exportNativeFootprint(project, definition.id, new Map([['switch-model', 'models/switch.step']])).content, /\$\{KIPRJMOD\}\/models\/switch.step/u);
});

test('footprint filename matches the serialized KiCad name', () => {
  const { doc } = fixture();
  const definition = doc.definitions[0];
  definition.name = '../Switch / Test';

  const project = emptyProject('roundtrip', 'Roundtrip');
  project.definitions = [definition];
  const exported = exportNativeFootprint(project, definition.id);
  const name = exported.content.match(/^\(footprint "([^"]+)"/u)?.[1];
  const imported = importNativeFootprint(exported.content, 'roundtrip');
  assert.equal(exported.filename, `${name}.kicad_mod`);
  assert.equal(imported.name, name);
});

test('standalone footprint rejects invalid courtyard and pad geometry', () => {
  const { doc } = fixture();
  const definition = doc.definitions[0];
  definition.courtyard[0].x = Number.NaN;
  const project = emptyProject('invalid', 'Invalid');
  project.definitions = [definition];
  assert.throws(() => exportNativeFootprint(project, definition.id), /courtyard|Invalid|invalid type/u);

  definition.courtyard[0].x = -7;
  definition.pads[0].size.x = Number.POSITIVE_INFINITY;
  assert.throws(() => exportNativeFootprint(projectWith(definition), definition.id), /Invalid pad|Invalid|invalid type/u);

  definition.pads[0].size.x = 1.8;
  definition.pads[0].drill = 0;
  assert.throws(() => exportNativeFootprint(projectWith(definition), definition.id), /Invalid pad|Invalid/u);
});

test('standalone footprints retain repeated logical numbers with unique physical pad identities', () => {
  const { doc } = fixture();
  const definition = doc.definitions[0];
  definition.pads[0].number = ' ';
  assert.throws(() => exportNativeFootprint(projectWith(definition), definition.id), /Empty electrical pad number/u);

  definition.pads[0].number = '2';
  const exported = exportNativeFootprint(projectWith(definition), definition.id);
  const imported = importNativeFootprint(exported.content, 'repeated-pads');
  assert.deepEqual(imported.pads.map(pad => pad.number), ['2', '2']);
  assert.notEqual(imported.pads[0].id, imported.pads[1].id);

  definition.pads[1].id = definition.pads[0].id;
  assert.throws(() => exportNativeFootprint(projectWith(definition), definition.id), /Duplicate .*pad/u);
});

test('imports supported external footprint pads and courtyard', () => {
  const { doc } = fixture();
  const source = exportNativeFootprint(doc, 'switch').content;
  const imported = importNativeFootprint(source, 'external-switch');
  assert.equal(imported.name, 'Switch');
  assert.equal(imported.pads.length, 2);
  assert.deepEqual(imported.pads[0].at, { x: -3, y: 0 });
  assert.equal(imported.courtyard.length, 4);
});

test('native source import preserves rotated pads and external model references', () => {
  const { doc } = fixture();
  const source = exportNativeFootprint(doc, 'switch').content;
  const rotated = source.replace('(at -3 0)', '(at -3 0 45)');
  assert.equal(importNativeFootprint(rotated, 'rotated').kicadSource?.source, rotated);

  const model = source.replace(/\n  \)\s*$/u, '\n    (model "${KIPRJMOD}/models/switch.step")\n  )');
  assert.equal(importNativeFootprint(model, 'model').kicadSource?.source, model);
});

test('imports shuffled courtyard lines by connected endpoints', () => {
  const { doc } = fixture();
  const source = exportNativeFootprint(doc, 'switch').content;
  const lines = [...source.matchAll(/\(fp_line \(start [^)]+\) \(end [^)]+\) \(stroke \(width [^)]+\) \(type solid\)\) \(layer "F\.CrtYd"\) \(uuid "[^"]+"\)\)/gu)].map((match) => match[0]);
  assert.equal(lines.length, 4);
  const markers = lines.map((_, index) => `__courtyard_${index}__`);
  let shuffled = source;
  lines.forEach((line, index) => { shuffled = shuffled.replace(line, markers[index]); });
  [2, 0, 3, 1].forEach((lineIndex, position) => {
    shuffled = shuffled.replace(markers[position], lines[lineIndex]);
  });
  const imported = importNativeFootprint(shuffled, 'shuffled');
  const original = importNativeFootprint(source, 'original').courtyard;
  const start = imported.courtyard.findIndex((point) => point.x === original[0].x && point.y === original[0].y);
  assert.deepEqual([...imported.courtyard.slice(start), ...imported.courtyard.slice(0, start)], original);
});

test('native import preserves disconnected and curved courtyard source with diagnostics', () => {
  const { doc } = fixture();
  const source = exportNativeFootprint(doc, 'switch').content;
  const broken = source.replace(/\(end 7 7\)/u, '(end 8 7)');
  assert.equal(importNativeFootprint(broken, 'broken').kicadSource?.source, broken);

  const arc = source.replace(/\n  \)\s*$/u, '\n    (fp_arc (start 0 0) (mid 1 1) (end 2 0) (layer "F.CrtYd"))\n  )');
  assert.equal(importNativeFootprint(arc, 'arc').kicadSource?.source, arc);
});

test('rejects a truncated footprint without hanging', () => {
  assert.throws(() => importNativeFootprint('(footprint "broken"', 'broken'));
});

test('native import preserves pad features beyond the static preview projection', () => {
  const { doc } = fixture();
  const source = exportNativeFootprint(doc, 'switch').content;
  for (const modified of [
    source.replace('(drill 0.9)', '(drill oval 0.9 1.1)'),
    source.replace('(layers "*.Cu" "*.Mask")', '(layers "B.Cu" "B.Mask")'),
    source.replace('(drill 0.9)', '(drill 0.9) (solder_mask_margin 0.2)'),
  ]) {
    assert.equal(importNativeFootprint(modified, 'preserved').kicadSource?.source, modified);
  }
});

test('KiCad 10 parses the exported board', () => {
  const { doc, contours } = fixture();
  const drc = runDrc(doc, contours);
  assert.deepEqual(drc.violations, []);
  assert.deepEqual(drc.unconnected_items, []);
  const directory = mkdtempSync(join(tmpdir(), 'boardstudio-v2-kicad-'));
  const path = join(directory, 'main.kicad_pcb');
  writeFileSync(path, exportNativeBoard(doc, 'main', contours));

  const stats = join(directory, 'stats.json');
  execFileSync('kicad-cli', ['pcb', 'export', 'stats', '--format', 'json', '--output', stats, path], { encoding: 'utf8' });
  const parsed = JSON.parse(readFileSync(stats, 'utf8'));
  assert.equal(parsed.board.has_outline, true);
  assert.equal(parsed.board.width, '24.0000 mm');
  assert.equal(parsed.pads.through_hole, 2);
  assert.equal(parsed.components.total.front, 1);

  const netlist = join(directory, 'netlist.d356');
  execFileSync('kicad-cli', ['pcb', 'export', 'ipcd356', '--output', netlist, path], { encoding: 'utf8' });
  const nets = readFileSync(netlist, 'utf8');
  assert.match(nets, /317ROW0\s+SW1\s+-1\s/u);
  assert.match(nets, /317N\/C\s+SW1\s+-2\s/u);
});

test('KiCad reports unrouted nets independently of serializer validity', () => {
  const { doc, contours } = fixture();
  doc.parts.push({ id: 's2', definitionId: 'switch', reference: 'SW2', pose: { at: { x: 0, y: 18 }, rotation: 0 }, side: 'front' });
  doc.boards[0].partIds.push('s2');
  doc.nets[0].pins.push({ partId: 's2', padId: 'left' });
  contours[0].points[2].y = 30;
  contours[0].points[3].y = 30;
  const drc = runDrc(doc, contours);
  assert.deepEqual(drc.unconnected_items.map((item) => item.type), ['unconnected_items']);
  assert.deepEqual(drc.violations, []);
});

test('KiCad reports copper clearance independently of unrouted nets', () => {
  const { doc, contours } = fixture();
  doc.definitions[0].pads[1].at.x = -1.1;
  doc.nets.push({ id: 'column', name: 'COL0', pins: [{ partId: 's1', padId: 'right' }] });
  doc.boards[0].netIds.push('column');

  const drc = runDrc(doc, contours);
  assert.deepEqual(drc.unconnected_items, []);
  assert.ok(drc.violations.some((item) => item.type === 'clearance'));
});

test('KiCad 10 plots the standalone footprint', () => {
  const { doc } = fixture();
  const directory = mkdtempSync(join(tmpdir(), 'boardstudio-v2-footprint-'));
  const library = join(directory, 'BoardStudio.pretty');
  const output = join(directory, 'svg');
  mkdirSync(library);
  mkdirSync(output);
  writeFileSync(join(library, 'Switch.kicad_mod'), exportNativeFootprint(doc, 'switch').content);
  execFileSync('kicad-cli', ['fp', 'export', 'svg', library, '--output', output], { encoding: 'utf8' });
  assert.ok(existsSync(join(output, 'Switch.svg')));
});
