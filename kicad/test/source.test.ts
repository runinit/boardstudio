import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { emptyProject } from '../../contracts/src/index.ts';
import type { Contour, ProjectDoc } from '../../contracts/src/index.ts';
import { nativeArtifact } from './nativeArtifact.ts';

const source = `(footprint "Rich Imported Ω" (version 20240108) (generator "pcbnew")
  (layer "F.Cu") (uuid 00000000-0000-4000-8000-000000000001)
  (at 0 0)
  (property "Reference" "REF**" (at 0 0 0) (layer "F.SilkS") (effects (font (size 1 1) (thickness 0.15))))
  (property "Value" "Rich Imported Ω" (at 0 1 0) (layer "F.Fab") (effects (font (size 1 1) (thickness 0.15))))
  (property "Vendor note" "café 東京 & Ω" (at 0 2 0) (layer "F.Fab") (effects (font (size 1 1) (thickness 0.15))))
  (attr through_hole)
  (fp_arc (start -3 -2) (mid -2 -3) (end -1 -2) (stroke (width 0.12) (type default)) (layer "F.CrtYd") (uuid 00000000-0000-4000-8000-000000000002))
  (fp_line (start -3 -2) (end 3 -2) (stroke (width 0.12) (type default)) (layer "F.Fab") (uuid 00000000-0000-4000-8000-000000000003))
  (pad "1" thru_hole oval (at -1.5 0 37) (size 2 1) (drill oval 1.2 0.6 (offset 0.2 0)) (layers "*.Cu" "*.Mask") (uuid 00000000-0000-4000-8000-000000000004))
  (pad "1" thru_hole circle (at 2 3 30) (size 1.8 1.8) (drill 0.8) (layers "*.Cu" "*.Mask") (uuid 00000000-0000-4000-8000-000000000005))
  (model "\${KIPRJMOD}/models/rich.step" (offset (xyz 0 0 0)) (scale (xyz 1 1 1)) (rotate (xyz 0 0 0)))
  (group "vendor geometry" (uuid 00000000-0000-4000-8000-000000000006) (members 00000000-0000-4000-8000-000000000003))
)`;

type Reply<T> = { id: string; kind: string; result: T };
type Imported = { definition: ProjectDoc['definitions'][number]; geometry: { pads: unknown[]; courtyard: unknown[] }; diagnostics: { kind: string; message: string }[] };
type File = { filename: string; content: string };

const importSource = (id: string, sourceText = source): Imported => {
  const reply = nativeArtifact<Reply<Imported>>({ id: `import-${id}`, kind: 'import-footprint', definitionId: id, source: sourceText });
  assert.equal(reply.kind, 'import-footprint');
  assert.equal(reply.id, `import-${id}`);
  return reply.result;
};

function boardDocument(sourceDefinition: ProjectDoc['definitions'][number], conflicting = false): ProjectDoc {
  const doc = emptyProject('source-test', 'Imported source fixture');
  doc.revision = 4;
  doc.definitions.push(sourceDefinition);
  doc.parts.push(
    { id: 'front-part', definitionId: sourceDefinition.id, reference: 'J1', pose: { at: { x: 11, y: 13 }, rotation: 37 }, side: 'front' },
    { id: 'back-part', definitionId: sourceDefinition.id, reference: 'J2', pose: { at: { x: 11, y: 13 }, rotation: 37 }, side: 'back' },
  );
  doc.nets.push(
    { id: 'shared', name: 'SHARED', pins: [
      { partId: 'front-part', padId: 'pad-0' }, { partId: 'front-part', padId: 'pad-1' },
      { partId: 'back-part', padId: 'pad-0' }, ...(conflicting ? [] : [{ partId: 'back-part', padId: 'pad-1' }]),
    ] },
  );
  if (conflicting) {
    doc.nets.push({ id: 'conflict', name: 'CONFLICT', pins: [{ partId: 'back-part', padId: 'pad-1' }] });
  }
  doc.boards.push({ id: 'board', name: 'source-board', outlineIds: [], partIds: ['front-part', 'back-part'], netIds: conflicting ? ['shared', 'conflict'] : ['shared'], thickness: 1.6 });
  return doc;
}

const contours: Contour[] = [{ hole: false, points: [
  { x: -30, y: -24 }, { x: 30, y: -24 }, { x: 30, y: 24 }, { x: -30, y: 24 },
] }];

function exportBoard(doc: ProjectDoc): string {
  const finished = nativeArtifact<Reply<{ files: File[] }>>({
    id: 'export-source-board', kind: 'export-pcb', request: {
      snapshotToken: 'source-fixture', expectedRevision: doc.revision, document: doc,
      target: { kind: 'board', boardId: 'board' }, contours, modelPaths: {},
    },
  });
  const board = finished.result.files.find((file) => file.filename === 'source-board.kicad_pcb');
  assert.ok(board, 'native exporter should return the requested board');
  return board.content;
}

function footprintForReference(board: string, reference: string): string {
  let start = -1;
  while (true) {
    const footprint = board.indexOf('(footprint ', start + 1);
    const module = board.indexOf('(module ', start + 1);
    start = footprint < 0 ? module : module < 0 ? footprint : Math.min(footprint, module);
    if (start < 0) break;
    let depth = 0;
    let quoted = false;
    let escaped = false;
    for (let index = start; index < board.length; index += 1) {
      const character = board[index];
      if (quoted) {
        if (escaped) escaped = false;
        else if (character === '\\') escaped = true;
        else if (character === '"') quoted = false;
      } else if (character === '"') quoted = true;
      else if (character === '(') depth += 1;
      else if (character === ')') {
        depth -= 1;
        if (depth === 0) {
          const form = board.slice(start, index + 1);
          if (form.includes(`(property "Reference" "${reference}"`)) return form;
          break;
        }
      }
    }
  }
  throw new Error(`Footprint with reference ${reference} is missing`);
}

function globalKiCad(root: { x: number; y: number; rotation: number }, local: { x: number; y: number }) {
  const theta = (root.rotation * Math.PI) / 180;
  return {
    x: root.x + Math.cos(theta) * local.x + Math.sin(theta) * local.y,
    y: root.y - Math.sin(theta) * local.x + Math.cos(theta) * local.y,
  };
}

test('native source import projects asymmetric pads and retains exact KiCad source', () => {
  const imported = importSource('rich-source');
  assert.equal(imported.definition.name, 'Rich Imported Ω');
  assert.equal(imported.definition.kicadSource?.source, source);
  assert.equal(imported.geometry.pads.length, 2);
  assert.equal(imported.geometry.courtyard.length, 4);
  assert.ok(imported.diagnostics.some(({ message }) => message.includes('models/rich.step')));
  assert.ok(source.includes('(drill oval 1.2 0.6 (offset 0.2 0))'));
  assert.ok(source.includes('café 東京 & Ω'));
});

test('native board export preserves untouched source spans and KiCad 10 parses and plots both sides', () => {
  const imported = importSource('board-source');
  const board = exportBoard(boardDocument(imported.definition));
  const front = footprintForReference(board, 'J1');
  const back = footprintForReference(board, 'J2');
  assert.match(front, /\(at 11 -13 37\)/u);
  assert.match(front, /\(pad "1" thru_hole circle \(at 2 3 67\)/u);
  assert.match(back, /\(at 11 -13 -143\)/u);
  assert.match(back, /\(pad "1" thru_hole circle \(at 2 -3 187\)/u);
  const frontGlobal = globalKiCad({ x: 11, y: -13, rotation: 37 }, { x: 2, y: 3 });
  const backGlobal = globalKiCad({ x: 11, y: -13, rotation: -143 }, { x: 2, y: -3 });
  assert.ok(Math.abs(frontGlobal.x - 14.402716) < 0.000001);
  assert.ok(Math.abs(frontGlobal.y - -11.807724) < 0.000001);
  assert.ok(Math.abs(backGlobal.x - 11.208174) < 0.000001);
  assert.ok(Math.abs(backGlobal.y - -9.400463) < 0.000001);
  assert.match(board, /\(drill oval 1\.2 0\.6 \(offset 0\.2 0\)\)/u);
  assert.match(board, /\(property "Vendor note" "café 東京 & Ω"/u);
  assert.match(board, /\(model "\$\{KIPRJMOD\}\/models\/rich\.step"/u);
  assert.match(board, /\(fp_arc \(start -3 -2\) \(mid -2 -3\) \(end -1 -2\)/u);
  const uuids = [...board.matchAll(/\(uuid "([0-9a-f-]{36})"\)/gu)].map((match) => match[1]);
  const groupMember = board.match(/\(group "vendor geometry" \(uuid "([0-9a-f-]{36})"\) \(members "([0-9a-f-]{36})"\)\)/u);
  assert.ok(groupMember, 'group and member UUIDs should be retained and rewritten consistently');
  assert.ok(uuids.includes(groupMember[1]));
  assert.ok(uuids.includes(groupMember[2]));
  assert.equal([...board.matchAll(/\(net 1 "SHARED"\)/gu)].length, 5);

  const directory = mkdtempSync(join(tmpdir(), 'boardstudio-v2-source-'));
  const path = join(directory, 'source-board.kicad_pcb');
  writeFileSync(path, board);
  execFileSync('kicad-cli', [
    'pcb', 'export', 'svg', '--layers', 'F.Cu,B.Cu,F.SilkS,B.SilkS,F.CrtYd,B.CrtYd', '--output', join(directory, 'source-board.svg'), path,
  ], { encoding: 'utf8' });
  const plotted = readFileSync(join(directory, 'source-board.svg'), 'utf8');
  assert.match(plotted, /<svg/u);

  const hasPcbnew = spawnSync('python3', ['-c', 'import pcbnew'], { encoding: 'utf8' }).status === 0;
  if (hasPcbnew) {
    const oracle = spawnSync('python3', ['-c', [
      'import json, pcbnew, sys',
      'board = pcbnew.LoadBoard(sys.argv[1])',
      'rows = [{"ref": fp.GetReference(), "number": pad.GetNumber(), "x": pcbnew.ToMM(pad.GetPosition().x), "y": pcbnew.ToMM(pad.GetPosition().y), "angle": pad.GetOrientationDegrees()} for fp in board.GetFootprints() for pad in fp.Pads() if pad.GetNumber() == "1"]',
      'print(json.dumps(rows))',
    ].join('\n'), path], { encoding: 'utf8' });
    assert.equal(oracle.status, 0, oracle.stderr);
    const rows = JSON.parse(oracle.stdout) as { ref: string; number: string; x: number; y: number; angle: number }[];
    for (const [reference, x, y, angle] of [
      ['J1', 14.402716, -11.807724, 67],
      ['J2', 11.208174, -9.400463, 187],
    ] as const) {
      const actual = rows.find((row) => row.ref === reference && row.number === '1' && Math.abs(row.angle - angle) < 0.001);
      assert.ok(actual, `pcbnew should report ${reference} pad angle ${angle}`);
      assert.ok(Math.abs(actual.x - x) < 0.000001);
      assert.ok(Math.abs(actual.y - y) < 0.000001);
    }
  }
});

test('imported source without a pad angle preserves the absolute angle on both board sides', () => {
  const angledSource = `(footprint "Angled" (layer "F.Cu") (at 20 30 37)
    (fp_line (start -4 -4) (end 4 -4) (layer "F.CrtYd") (width 0.05))
    (fp_line (start 4 -4) (end 4 4) (layer "F.CrtYd") (width 0.05))
    (fp_line (start 4 4) (end -4 4) (layer "F.CrtYd") (width 0.05))
    (fp_line (start -4 4) (end -4 -4) (layer "F.CrtYd") (width 0.05))
    (pad 1 smd rect (at 2 3) (size 2 1) (layers F.Cu F.Paste F.Mask))
    (pad 2 smd rect (at 3 3) (size 1 1) (layers F.Cu F.Paste F.Mask)))`;
  const imported = importSource('angled-source', angledSource);
  assert.equal(imported.geometry.pads.length, 2);
  assert.equal(imported.definition.kicadSource?.source, angledSource);
  const pad = imported.definition.pads.find(({ id }) => id === 'pad-0');
  assert.equal(pad?.rotation, -37, 'root angle 37 and omitted pad angle (0) normalize to a local offset of -37 degrees');

  const board = exportBoard(boardDocument(imported.definition));
  const front = footprintForReference(board, 'J1');
  const back = footprintForReference(board, 'J2');
  assert.match(front, /\(pad 1 smd rect \(at 2 3 0\)/u);
  assert.match(back, /\(pad 1 smd rect \(at 2 -3 254\)/u);

  const directory = mkdtempSync(join(tmpdir(), 'boardstudio-v2-angle-'));
  const path = join(directory, 'angle-board.kicad_pcb');
  writeFileSync(path, board);
  execFileSync('kicad-cli', [
    'pcb', 'export', 'svg', '--layers', 'F.Cu,B.Cu,F.CrtYd,B.CrtYd', '--output', join(directory, 'angle-board.svg'), path,
  ], { encoding: 'utf8' });
  assert.match(readFileSync(join(directory, 'angle-board.svg'), 'utf8'), /<svg/u);

  const hasPcbnew = spawnSync('python3', ['-c', 'import pcbnew'], { encoding: 'utf8' }).status === 0;
  if (hasPcbnew) {
    const oracle = spawnSync('python3', ['-c', [
      'import json, pcbnew, sys',
      'board = pcbnew.LoadBoard(sys.argv[1])',
      'rows = [{"ref": fp.GetReference(), "angle": pad.GetOrientationDegrees()} for fp in board.GetFootprints() for pad in fp.Pads()]',
      'print(json.dumps(rows))',
    ].join('\n'), path], { encoding: 'utf8' });
    assert.equal(oracle.status, 0, oracle.stderr);
    const rows = JSON.parse(oracle.stdout) as { ref: string; angle: number }[];
    assert.ok(Math.abs(rows.find(({ ref }) => ref === 'J1')!.angle) < 0.001);
    assert.ok(Math.abs(rows.find(({ ref }) => ref === 'J2')!.angle - 254) < 0.001);
  }
});

test('legacy KiCad arc preserves sweep geometry in a KiCad 10 board export', () => {
  const legacySource = `(module LegacyArc (layer F.Cu)
    (fp_line (start -4 -4) (end 4 -4) (layer F.CrtYd) (width 0.05))
    (fp_line (start 4 -4) (end 4 4) (layer F.CrtYd) (width 0.05))
    (fp_line (start 4 4) (end -4 4) (layer F.CrtYd) (width 0.05))
    (fp_line (start -4 4) (end -4 -4) (layer F.CrtYd) (width 0.05))
    (fp_arc (start 0 0) (end 2 0) (angle 90) (layer F.SilkS) (width 0.15))
    (pad 1 smd rect (at 0 0) (size 1 1) (layers F.Cu F.Paste F.Mask))
    (pad 2 smd rect (at 3 0) (size 1 1) (layers F.Cu F.Paste F.Mask)))`;
  const imported = importSource('legacy-arc', legacySource);
  assert.equal(imported.definition.kicadSource?.source, legacySource);
  assert.equal(imported.geometry.courtyard.length, 4);
  assert.equal(imported.geometry.pads.length, 2);

  const board = exportBoard(boardDocument(imported.definition));
  const front = footprintForReference(board, 'J1');
  const back = footprintForReference(board, 'J2');
  assert.match(front, /\(fp_arc \(start 2 0\) \(mid 1\.414214 1\.414214\) \(end 0 2\)/u);
  assert.match(back, /\(fp_arc \(start 2 0\) \(mid 1\.414214 -1\.414214\) \(end 0 -2\)/u);
  const directory = mkdtempSync(join(tmpdir(), 'boardstudio-v2-legacy-arc-'));
  const path = join(directory, 'legacy-arc-board.kicad_pcb');
  writeFileSync(path, board);
  execFileSync('kicad-cli', [
    'pcb', 'export', 'svg', '--layers', 'F.Cu,B.Cu,F.SilkS,B.SilkS,F.CrtYd,B.CrtYd', '--output', join(directory, 'legacy-arc.svg'), path,
  ], { encoding: 'utf8' });
  assert.match(readFileSync(join(directory, 'legacy-arc.svg'), 'utf8'), /<svg/u);
});

test('native source export rejects repeated pad numbers assigned to different nets', () => {
  const imported = importSource('conflict-source');
  assert.throws(() => exportBoard(boardDocument(imported.definition, true)), /Repeated logical pad number 1 has conflicting net assignments/u);
});
