import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import type { CompiledFootprint, Contour, PartDefinition, Part, ProjectDoc, Side } from '../../contracts/src/index.ts';

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const driver = process.env.BOARDSTUDIO_ARTIFACT_DRIVER
  ?? resolve(packageRoot, '../core/target/debug/examples/artifact_request');

export type NativeArtifactReply = { id: string; kind: string; result?: unknown; error?: { message: string } };

export function nativeArtifact<T extends NativeArtifactReply = NativeArtifactReply>(request: object): T {
  const process = spawnSync(driver, { input: `${JSON.stringify(request)}\n`, encoding: 'utf8' });
  if (process.error) throw process.error;
  if (process.status !== 0) throw new Error(process.stderr || `Native artifact driver exited with ${process.status}`);
  const output = process.stdout.trim().split('\n').at(-1);
  if (!output) throw new Error('Native artifact driver returned no reply');
  const reply = JSON.parse(output) as T;
  if (reply.kind === 'error') throw new Error(reply.error?.message ?? 'Rust artifact request failed');
  return reply;
}

export type NativeExportArtifact = { files: { filename: string; content: string }[]; skippedUtilities: string[] };
function exportNative(document: ProjectDoc, target: object, contours: Contour[], modelPaths: ReadonlyMap<string, string>, id: string): NativeExportArtifact {
  return nativeArtifact<{ id: string; kind: string; result: NativeExportArtifact }>({
    id, kind: 'export-pcb', request: {
      snapshotToken: `${id}:snapshot`, expectedRevision: document.revision, document, target, contours,
      modelPaths: Object.fromEntries(modelPaths),
    },
  }).result;
}

export function exportNativeBoard(
  document: ProjectDoc,
  boardId: string,
  contours: Contour[],
  modelPaths: ReadonlyMap<string, string> = new Map(),
  id = 'native-board',
): string {
  const artifact = exportNative(document, { kind: 'board', boardId }, contours, modelPaths, id);
  const file = artifact.files.find((entry) => entry.filename === `${document.boards.find((board) => board.id === boardId)?.name}.kicad_pcb`);
  if (!file) throw new Error(`Native artifact omitted board ${boardId}`);
  return file.content;
}

export function exportNativeFootprint(
  document: ProjectDoc,
  definitionId: string,
  modelPaths: ReadonlyMap<string, string> = new Map(),
  id = 'native-footprint',
): { filename: string; content: string } {
  const artifact = exportNative(document, { kind: 'standalone-footprints', definitionIds: [definitionId] }, [], modelPaths, id);
  const file = artifact.files[0];
  if (!file) throw new Error(`Native artifact omitted footprint ${definitionId}`);
  return file;
}

export function importNativeFootprint(source: string, definitionId: string): PartDefinition {
  const reply = nativeArtifact<{ id: string; kind: string; result: { definition: PartDefinition } }>({
    id: `import:${definitionId}`, kind: 'import-footprint', definitionId, source,
  });
  return reply.result.definition;
}

export function compileNativeFootprint(
  definition: PartDefinition,
  side: Side = 'front',
): CompiledFootprint {
  const reply = nativeArtifact<{ id: string; kind: string; result: CompiledFootprint[] }>({
    id: `compile:${definition.id}:${side}`, kind: 'compile-footprints', jobs: [{
      id: definition.id, definition, side,
    }],
  });
  const compiled = reply.result[0];
  if (!compiled) throw new Error(`Native compiler omitted footprint ${definition.id}`);
  return compiled;
}
