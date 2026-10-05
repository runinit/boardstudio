import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { readFile } from 'node:fs/promises';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { readStepModel } from '../src/index.ts';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const catalogue = JSON.parse(readFileSync(resolve(root, 'catalogue/modules/imported-modules.json'), 'utf8'));
const definition = catalogue.modules.find(entry => entry.row === 'vik-display-adapter'
  && entry.definition.variant === 'pcb/1.47inch/pcb')?.definition;
const ledger = JSON.parse(readFileSync(resolve(root, 'catalogue/modules/asset-ledger.json'), 'utf8'));
const asset = ledger.models.find(model => model.path === 'pcb/1.47inch/1.47inch.step');

test('1.47-inch display STEP transform aligns PCB bounds and asymmetric H1/H2 centers', async () => {
  assert.ok(definition && asset?.boardToModelTransform, 'the source-backed aligned profile is imported');
  const transform = asset.boardToModelTransform;
  assert.deepEqual(transform.rotation, { x: 0, y: 0, z: 0 });
  assert.deepEqual(transform.scale, { x: 1, y: 1, z: 1 });
  assert.match(transform.evidence.asymmetricFeatures, /H1\/H2/);

  const bytes = await readFile(resolve(root, asset.bundledFile));
  const imported = await readStepModel(new Uint8Array(bytes));
  const positions = imported.mesh.positions;
  const transformed = (index) => [
    positions[index] * transform.scale.x + transform.offset.x,
    positions[index + 1] * transform.scale.y + transform.offset.y,
    positions[index + 2] * transform.scale.z + transform.offset.z,
  ];
  const boardPoints = definition.board.contours.flatMap(contour => contour.points);
  const boardBounds = {
    minX: Math.min(...boardPoints.map(point => point.x)), maxX: Math.max(...boardPoints.map(point => point.x)),
    minY: Math.min(...boardPoints.map(point => point.y)), maxY: Math.max(...boardPoints.map(point => point.y)),
  };
  const source = asset.boardNativeBoundsMm;
  assert.equal(source.XMin + transform.offset.x, boardBounds.minX);
  assert.equal(source.XMax + transform.offset.x, boardBounds.maxX);
  assert.equal(source.YMin + transform.offset.y, boardBounds.minY);
  assert.equal(source.YMax + transform.offset.y, boardBounds.maxY);
  assert.equal(source.ZMin + transform.offset.z, -0.8);
  assert.equal(source.ZMax + transform.offset.z, 0.8);

  // KiCad source places H1/H2 at X ±7.5, Y +16 mm. The actual STEP mesh
  // carries their 2.65 mm through-hole rims at native X ±7.5, Y -2.5.
  // Matching both the board and these asymmetric features checks placement,
  // not just a coincident overall bounding box.
  for (const mount of definition.mounts) {
    let rimVertices = 0;
    for (let index = 0; index < positions.length; index += 3) {
      const [x, y, z] = transformed(index);
      const radialError = Math.abs(Math.hypot(x - mount.at.x, y - mount.at.y) - mount.diameter / 2);
      if (radialError < 0.035 && Math.abs(Math.abs(z) - 0.8) < 0.035) rimVertices++;
    }
    assert.ok(rimVertices > 20, `${mount.sourceId} source mount maps onto STEP hole rim (${rimVertices} mesh vertices)`);
  }
}, { timeout: 60_000 });
