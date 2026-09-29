import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { emptyProject } from '../../contracts/src/index.ts';
import type { Contour, Part, PartDefinition, ProjectDoc } from '../../contracts/src/index.ts';
import { catalogue, child, modelAssetIds, parseForms, value } from '../../ergogen/src/index.ts';
import type { Expression } from '../../ergogen/src/index.ts';
import { exportNativeBoard, finishNativeExport, prepareNativeExport } from './nativeArtifact.ts';

const definitions = catalogue();
const contour: Contour[] = [{ hole: false, points: [
  { x: -30, y: -30 }, { x: 90, y: -30 }, { x: 90, y: 30 }, { x: -30, y: 30 },
] }];
const kiCad10 = spawnSync('kicad-cli', ['version'], { encoding: 'utf8' });
const hasKiCad10 = kiCad10.status === 0 && /^10\./u.test(kiCad10.stdout.trim());

function definition(source: string): PartDefinition {
  const found = definitions.find((entry) => entry.generator?.source === source);
  assert.ok(found, `Missing bundled Ergogen generator ${source}`);
  return structuredClone(found);
}

function part(item: PartDefinition, id: string, x: number, side: 'front' | 'back', parameters: Part['generatorParameters']): Part {
  return {
    id,
    definitionId: item.id,
    reference: id.toUpperCase(),
    pose: { at: { x, y: 0 }, rotation: 0 },
    side,
    generatorParameters: parameters,
  };
}

function project(items: PartDefinition[], parts: Part[]): ProjectDoc {
  const doc = emptyProject('ergogen-export', 'Ergogen export');
  doc.definitions = items;
  doc.parts = parts;
  doc.boards = [{ id: 'main', name: 'main', outlineIds: [], partIds: parts.map((entry) => entry.id), netIds: [], thickness: 1.6 }];
  return doc;
}

function parseWithKiCad(content: string): Record<string, unknown> {
  const directory = mkdtempSync(join(tmpdir(), 'boardstudio-ergogen-kicad-'));
  const board = join(directory, 'main.kicad_pcb');
  const report = join(directory, 'stats.json');
  writeFileSync(board, content);
  execFileSync('kicad-cli', ['pcb', 'export', 'stats', '--format', 'json', '--output', report, board], { encoding: 'utf8' });
  return JSON.parse(readFileSync(report, 'utf8')) as Record<string, unknown>;
}

function children(node: Expression[], name: string): Expression[][] {
  return node.filter((entry): entry is Expression[] => Array.isArray(entry) && entry[0] === name);
}

test('exports placed front and back Ergogen switches with instance nets and multiple models', { skip: !hasKiCad10 }, () => {
  const mx = definition('ceoloide/switch_mx');
  const model = '${KIPRJMOD}/models/boardstudio/';
  const parts = [
    part(mx, 'sw1', 0, 'front', {
      from: 'ROW0', to: 'COL0', side: 'F', hotswap: true,
      switch_3dmodel_filename: `${model}switch.step`,
      hotswap_3dmodel_filename: `${model}socket.step`,
      keycap_3dmodel_filename: `${model}keycap.step`,
    }),
    part(mx, 'sw2', 40, 'back', {
      from: 'ROW1', to: 'COL1', side: 'B', hotswap: true,
      switch_3dmodel_filename: `${model}switch.step`,
      hotswap_3dmodel_filename: `${model}socket.step`,
      keycap_3dmodel_filename: `${model}keycap.step`,
    }),
  ];
  const nets = ['ROW0', 'COL0', 'ROW1', 'COL1'].map((name) => ({ id: `net-${name}`, name, pins: [] }));
  const doc = project([mx], parts);
  doc.nets = nets;
  doc.boards[0].netIds = nets.map(({ id }) => id);
  const paths = new Map([
    ['ergogen:model:switch.step', 'models/switch.step'],
    ['ergogen:model:socket.step', 'models/socket.step'],
    ['ergogen:model:keycap.step', 'models/keycap.step'],
  ]);
  const board = exportNativeBoard(doc, 'main', contour, paths);

  for (const net of ['ROW0', 'COL0', 'ROW1', 'COL1']) {
    assert.match(board, new RegExp(`\\(net \\d+ "${net}"\\)`, 'u'));
  }
  const forms = parseForms(board);
  const footprints = children(forms[0], 'footprint');
  for (const [index, expected] of [[0, ['ROW0', 'COL0']], [1, ['ROW1', 'COL1']]] as const) {
    const pads = children(footprints[index], 'pad').filter((pad) => ['1', '2'].includes(value(pad[1])));
    assert.deepEqual(pads.map((pad) => value(child(pad, 'net')?.[2])), expected);
    assert.equal(children(footprints[index], 'model').length, 3);
  }
  assert.equal((board.match(/\(model "\$\{KIPRJMOD\}\/models\//gu) ?? []).length, 6);
  assert.match(board, /\(layer "F\.Cu"\)/u);
  assert.match(board, /\(layer "B\.Cu"\)/u);
  const stats = parseWithKiCad(board) as { components: { total: { front: number; back: number } }; pads: { through_hole: number; smd: number } };
  assert.equal(stats.components.total.front, 1);
  assert.equal(stats.components.total.back, 1);
  assert.ok(stats.pads.through_hole + stats.pads.smd >= 4);
});

test('exports document pad-net assignments for Ergogen terminal groups', () => {
  const connector = definition('ceoloide/battery_connector_jst_ph_2');
  const placed = part(connector, 'j1', 0, 'front', {});
  const doc = project([connector], [placed]);
  const pad = connector.pads[0];
  doc.nets = [{ id: 'gnd', name: 'GND', pins: [{ partId: placed.id, padId: pad.id }] }];
  doc.boards[0].netIds = ['gnd'];
  const paths = new Map(modelAssetIds(connector).map((id) => [id, `models/${id.replaceAll(':', '_').replaceAll('/', '_')}.step`]));

  const board = exportNativeBoard(doc, 'main', contour, paths);
  assert.match(board, /\(net 1 "GND"\)/u);
  assert.match(board, new RegExp(`\\(pad "${pad.number}"[^\\n]*\\(net 1 "GND"\\)` , 'u'));
});

test('exports placed Ergogen zone, reversible text, and front/back route as KiCad board objects', { skip: !hasKiCad10 }, () => {
  const zone = definition('ceoloide/utility_filled_zone');
  const text = definition('ceoloide/utility_text');
  const router = definition('ceoloide/utility_router');
  const parts = [
    part(zone, 'zone', 0, 'front', { net: 'GND', side: 'F', points: [[-20, -20], [80, -20], [80, 20], [-20, 20]] }),
    part(text, 'label', 0, 'front', { text: 'BOARD', reversible: true }),
    part(router, 'route', 0, 'front', { net: 'GND', route: 'f(0,0)(2,0)v(2,2)' }),
  ];
  const board = exportNativeBoard(project([zone, text, router], parts), 'main', contour);

  assert.match(board, /\(net \d+ "GND"\)/u);
  assert.match(board, /\(zone /u);
  assert.equal((board.match(/\(gr_text "BOARD"/gu) ?? []).length, 2);
  assert.equal((board.match(/\(segment /gu) ?? []).length, 2);
  assert.match(board, /\(via /u);
  assert.doesNotThrow(() => parseWithKiCad(board));
});

test('upgrades legacy vendor footprint arcs for KiCad 10', { skip: !hasKiCad10 }, () => {
  for (const source of ['infused-kim/nice_view']) {
    const item = definition(source);
    const paths = new Map(modelAssetIds(item).map((id) => [id, `models/${id.slice(14).replaceAll('/', '_')}`]));
    const board = exportNativeBoard(project([item], [part(item, 'part', 0, 'front', {})]), 'main', contour, paths);
    assert.match(board, /\(fp_arc \(start [^)]+\) \(mid [^)]+\) \(end [^)]+\)/u);
    assert.doesNotThrow(() => parseWithKiCad(board));
  }
});

test('KiCad 10 parses default board output from every bundled generator', { skip: !hasKiCad10 }, () => {
  for (const item of definitions) {
    const paths = new Map(modelAssetIds(item).map((id) => [id, `models/${id.slice(14).replaceAll('/', '_')}`]));
    const board = exportNativeBoard(project([item], [part(item, 'part', 0, 'front', {})]), 'main', contour, paths);
    assert.doesNotThrow(() => parseWithKiCad(board), item.generator?.source);
  }
});

test('standalone export batches all Ergogen generators and reports skipped board utilities', () => {
  assert.equal(definitions.length, 36);
  const doc = emptyProject('ergogen-library', 'Ergogen library');
  doc.definitions = definitions;
  const paths = new Map(definitions.flatMap((item) => modelAssetIds(item).map((id) => [
    id,
    `models/${id.replaceAll(':', '_').replaceAll('/', '_')}.step`,
  ] as const)));
  const plan = prepareNativeExport(
    doc,
    { kind: 'standalone-footprints', definitionIds: definitions.map((item) => item.id) },
    [],
    paths,
    'ergogen-library',
  );
  const artifact = finishNativeExport(plan, paths, 'ergogen-library');

  assert.ok(artifact.skippedUtilities.includes('utility text'));
  assert.ok(artifact.skippedUtilities.includes('utility router'));
  assert.equal(artifact.files.length + artifact.skippedUtilities.length, definitions.length);
  assert.ok(artifact.files.some((file) => file.content.includes('${KIPRJMOD}/models/')));
});
