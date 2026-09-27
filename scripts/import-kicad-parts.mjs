// Reviewed KiCad sources -> reusable Board Studio definitions via the Rust importer.
import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2).filter(arg => arg !== '--check');
const manifestPath = resolve(args[0] ?? `${root}/app/src/parts/import-manifest.json`);
const outputPath = resolve(args[1] ?? `${dirname(manifestPath)}/imported-parts.json`);
const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
if (manifest.formatVersion !== 1 || !Array.isArray(manifest.entries)) throw new Error('Expected a version 1 import manifest');
const driver = process.env.BOARDSTUDIO_ARTIFACT_DRIVER ?? `${root}/core/target/debug/examples/artifact_request`;
function artifact(input) {
  const result = spawnSync(driver, { input: JSON.stringify({ id: 'catalogue-import', ...input }) + '\n', encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(result.stderr || 'Rust importer failed');
  const reply = JSON.parse(result.stdout.trim());
  if (reply.kind === 'error') throw new Error(reply.error.message);
  if (reply.kind !== input.kind) throw new Error('Unexpected importer response');
  return reply.result;
}
const ids = new Set();
const parts = manifest.entries.map(entry => {
  for (const field of ['id', 'name', 'kind', 'file', 'sha256', 'repository', 'revision', 'sourcePath', 'license', 'licenseFile']) {
    if (typeof entry[field] !== 'string' || !entry[field].trim()) throw new Error(`Missing ${field}`);
  }
  if (ids.has(entry.id)) throw new Error(`Duplicate definition ID: ${entry.id}`);
  ids.add(entry.id);
  readFileSync(resolve(dirname(manifestPath), entry.licenseFile));
  const source = readFileSync(resolve(dirname(manifestPath), entry.file), 'utf8');
  if (createHash('sha256').update(source).digest('hex') !== entry.sha256) throw new Error(`Source hash mismatch: ${entry.file}`);
  const imported = artifact({ kind: 'import-footprint', definitionId: entry.id, source });
  const definition = { ...imported.definition, name: entry.name, kind: entry.kind };
  if (!['switch', 'controller', 'connector', 'encoder', 'passive', 'custom', 'utility'].includes(entry.kind)) throw new Error(`Invalid kind: ${entry.kind}`);
  // Roles are explicit review decisions; never guess a circuit from pad numbers.
  if (entry.terminals) {
    definition.terminals = Object.fromEntries(Object.entries(entry.terminals).map(([role, numbers]) => {
      if (!Array.isArray(numbers) || !numbers.length || numbers.some(number => typeof number !== 'string' || !number)) throw new Error(`Invalid terminal mapping: ${role}`);
      const pads = numbers.flatMap(number => {
        const matches = definition.pads.filter(pad => pad.number === number && !(pad.drill && pad.plated === false));
        if (!matches.length) throw new Error(`Missing conductive pad ${number} for ${role}`);
        return matches.map(pad => pad.id);
      });
      return [role, [...new Set(pads)]];
    }));
  }
  if (entry.matrixTerminals) {
    for (const role of [entry.matrixTerminals.row, entry.matrixTerminals.column]) {
      if (!definition.terminals?.[role]?.length) throw new Error(`Missing matrix terminal: ${role}`);
    }
    definition.matrixTerminals = entry.matrixTerminals;
  }
  if (entry.mechanical) {
    const extracted = artifact({ kind: 'extract-mechanical', source, mappings: entry.mechanical.mappings, maxDeviationMm: 0.02 });
    definition.mechanicalProfile = { definitionId: entry.id, source: `${entry.repository}/blob/${entry.revision}/${entry.sourcePath}`, cutouts: extracted.plateCutouts, pcbHoles: extracted.pcbHoles, clearances: extracted.clearanceEnvelopes, sourceGeometry: extracted.sourceGeometry, plateToPcb: entry.mechanical.plateToPcb };
  }
  return { definition, diagnostics: imported.diagnostics, provenance: { repository: entry.repository, revision: entry.revision, sourcePath: entry.sourcePath, sha256: entry.sha256, license: entry.license, licenseFile: entry.licenseFile } };
});
const output = JSON.stringify({ formatVersion: 1, parts }, null, 2) + '\n';
if (process.argv.includes('--check')) {
  if (readFileSync(outputPath, 'utf8') !== output) throw new Error('Imported catalogue drift: regenerate and review the changed definitions');
  console.log(`Verified ${parts.length} imported parts`);
} else {
  // All sources and conversions must succeed before publishing a catalogue.
  writeFileSync(outputPath, output);
  console.log(`Imported ${parts.length} parts into ${outputPath}`);
}
