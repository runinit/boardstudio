import type {
  ArtifactReply,
  ArtifactRequest,
  Contour,
  ErgogenJobResult,
  ExportPlan,
  ExportTarget,
  ProjectDoc,
  ReservedNet,
  ArchiveRequest as RustArchiveRequest,
  ArchiveReply as RustArchiveReply,
} from '@boardstudio/v2-contracts';
import { modelBindings } from '@boardstudio/v2-ergogen';
import { exportErgogenForms } from '@boardstudio/v2-kicad';
import init, { artifact_request, archive_request } from '../../core/pkg/boardstudio_core.js';
const textBytes = (value: string): Uint8Array => new TextEncoder().encode(value);

export type ArchiveRequest = {
  id: string;
  kind: 'archive';
  request: RustArchiveRequest;
  buffers: Uint8Array[];
};
export type ArchiveReply =
  | { id: string; kind: 'archive'; reply: { kind: 'packed'; bytes: Uint8Array } | { kind: 'unpacked'; projectJson: string; assets: { sha256: string; bytes: Uint8Array }[] } }
  | { id: string; kind: 'error'; message: string };

export type ExportRequest = {
  id: string;
  kind: 'project' | 'footprints' | 'kicad' | 'outline' | 'pcb-preview';
  document: ProjectDoc;
  boardId?: string;
  contours?: Contour[];
  outlineFormat?: 'svg' | 'dxf';
  paths: [string, string][];
  files: Record<string, Uint8Array>;
};
export type ExportReply =
  | { id: string; kind: 'file'; filename: string; bytes: Uint8Array; mediaType: string }
  | { id: string; kind: 'error'; message: string };
export type ExportWorkerRequest = ExportRequest | ArchiveRequest | { id: string; kind: 'artifact'; request: ArtifactRequest };
export type ExportWorkerReply = ExportReply | ArchiveReply | ArtifactReply;

let ready: Promise<unknown> | undefined;
let queue = Promise.resolve();

function artifact(request: ArtifactRequest): ArtifactReply {
  const reply = JSON.parse(artifact_request(JSON.stringify(request))) as ArtifactReply;
  if (reply.kind === 'error') throw new Error(reply.error.message);
  return reply;
}

function archive(message: ArchiveRequest): Extract<ArchiveReply, { kind: 'archive' }> {
  const [replyJson, buffers] = archive_request(JSON.stringify(message.request), message.buffers);
  const reply = JSON.parse(replyJson) as RustArchiveReply;
  if (reply.kind === 'error') throw new Error(reply.message);
  const buffer = (index: number): Uint8Array => {
    const bytes = buffers[index];
    if (!(bytes instanceof Uint8Array)) throw new Error('Archive reply has a missing buffer');
    return bytes;
  };
  if (reply.kind === 'packed') return { id: message.id, kind: 'archive', reply: { kind: 'packed', bytes: buffer(0) } };
  return { id: message.id, kind: 'archive', reply: { kind: 'unpacked', projectJson: reply.projectJson, assets: reply.assets.map((asset) => ({ sha256: asset.sha256, bytes: buffer(asset.bufferIndex) })) } };
}

function netAllocator(initial: ReservedNet[], nextIndex: number): {
  lookup: (name: string) => number;
  snapshot: () => ReservedNet[];
} {
  const nets = initial.map((net) => ({ ...net }));
  return {
    lookup(name) {
      const existing = nets.find((net) => net.name === name);
      if (existing) return existing.index;
      const net = { name, index: nextIndex++ };
      nets.push(net);
      return net.index;
    },
    snapshot: () => nets.map((net) => ({ ...net })),
  };
}

function formsSource(forms: { footprints: string[]; objects: string[] }): string {
  return [...forms.footprints, ...forms.objects].join('\n');
}

function runErgogenJobs(plan: ExportPlan, paths: ReadonlyMap<string, string>): ErgogenJobResult[] {
  const allocator = netAllocator(plan.reservedNets, plan.nextNetIndex);
  return plan.jobs.map((job) => {
    const standalone = plan.target.kind === 'standalone-footprints';
    const forms = exportErgogenForms(job.definition, standalone ? undefined : job.part, paths, standalone ? () => 0 : allocator.lookup);
    return {
      snapshotToken: plan.snapshotToken,
      revision: plan.revision,
      jobId: job.jobId,
      source: formsSource(forms),
      nets: allocator.snapshot(),
    };
  });
}

function prepareRequest(request: ExportRequest, target: ExportTarget): ArtifactReply {
  return artifact({
    id: request.id,
    kind: request.kind === 'pcb-preview' ? 'prepare-preview' : 'prepare-export',
    request: {
      snapshotToken: crypto.randomUUID(),
      expectedRevision: request.document.revision,
      document: request.document,
      target,
      contours: request.contours ?? [],
      modelPaths: Object.fromEntries(request.paths),
    },
  });
}

function finishRequest(request: ExportRequest, plan: ExportPlan, paths: ReadonlyMap<string, string>): ArtifactReply {
  if (request.kind === 'pcb-preview') {
    const previewPaths = new Map(paths);
    for (const job of plan.jobs) for (const model of modelBindings(job.definition, job.part)) {
      if (model.assetId.startsWith('unresolved-model:')) {
        const original = decodeURIComponent(model.assetId.slice('unresolved-model:'.length));
        previewPaths.set(original, `models/unresolved/${encodeURIComponent(original)}`);
      }
    }
    paths = previewPaths;
  }
  return artifact({
    id: request.id,
    kind: request.kind === 'pcb-preview' ? 'finish-preview' : 'finish-export',
    request: { plan, results: runErgogenJobs(plan, paths) },
  });
}

function build(request: ExportRequest): ExportReply | ArtifactReply {
  const paths = new Map(request.paths);
  const files = request.files;

  if (request.kind === 'project') {
    const buffers = Object.values(files);
    const packed = archive({ id: request.id, kind: 'archive', request: { kind: 'pack-project', projectJson: JSON.stringify(request.document), archiveJson: files['archive.json'] ? new TextDecoder().decode(files['archive.json']) : undefined, assets: Object.keys(files).filter((path) => path.startsWith('assets/')).map((path) => ({ path, bufferIndex: Object.keys(files).indexOf(path) })) }, buffers });
    if (packed.kind !== 'archive' || packed.reply.kind !== 'packed') throw new Error('Expected Rust project archive');
    return { id: request.id, kind: 'file', filename: `${request.document.name}.boardstudio`, bytes: packed.reply.bytes, mediaType: 'application/zip' };
  }

  if (request.kind === 'outline') {
    const board = request.document.boards.find((entry) => entry.id === request.boardId);
    if (!board || !request.contours || !request.outlineFormat) throw new Error('Select a resolved board before outline export');
    const reply = artifact({
      id: request.id,
      kind: 'export-outline',
      request: {
        filename: `${request.document.name}.${request.outlineFormat}`,
        board,
        contours: request.contours,
        format: request.outlineFormat,
      },
    });
    if (reply.kind !== 'export-outline') throw new Error('Expected Rust outline export');
    const format = request.outlineFormat === 'svg' ? 'image/svg+xml' : 'application/dxf';
    return { id: request.id, kind: 'file', filename: reply.result.filename, bytes: textBytes(reply.result.content), mediaType: format };
  }

  if (request.kind === 'footprints') {
    const definitionIds = request.document.definitions.map((definition) => definition.id);
    const prepared = prepareRequest(request, { kind: 'standalone-footprints', definitionIds });
    if (prepared.kind !== 'prepare-export') throw new Error('Expected Rust export preparation');
    const exported = finishRequest(request, prepared.result, paths);
    if (exported.kind !== 'finish-export') throw new Error('Expected Rust footprint export');
    for (const file of exported.result.files) {
      const path = `BoardStudio.pretty/${file.filename}`;
      if (files[path]) throw new Error(`Footprint filename repeats: ${file.filename}`);
      files[path] = textBytes(file.content);
    }

    files['fp-lib-table'] = textBytes('(fp_lib_table (lib (name "BoardStudio") (type "KiCad") (uri "${KIPRJMOD}/BoardStudio.pretty") (options "") (descr "")))\n');
    const skippedUtilities = exported.result.skippedUtilities ?? [];
    const utilityNotice = [
      'Standalone footprint libraries contain footprints only.',
      'Generated board routing and graphics are included by placed board export.',
      ...(skippedUtilities.length ? ['', 'Generators skipped from this standalone footprint library:', ...skippedUtilities.map((name) => `- ${name}`)] : []),
      '',
    ].join('\n');
    files['BOARD-UTILITIES.txt'] = textBytes(utilityNotice);
    const packed = archive({ id: request.id, kind: 'archive', request: { kind: 'pack-files', entries: Object.keys(files).map((path, bufferIndex) => ({ path, bufferIndex })) }, buffers: Object.values(files) });
    if (packed.kind !== 'archive' || packed.reply.kind !== 'packed') throw new Error('Expected Rust export archive');
    return { id: request.id, kind: 'file', filename: `${request.document.name}-footprints.zip`, bytes: packed.reply.bytes, mediaType: 'application/zip' };
  }

  const board = request.document.boards.find((entry) => entry.id === request.boardId);
  if (!board || !request.contours) throw new Error('Select a resolved board before export');
  const prepared = prepareRequest(request, { kind: 'board', boardId: board.id });
  if (prepared.kind !== 'prepare-export' && prepared.kind !== 'prepare-preview') throw new Error('Expected Rust export preparation');
  const exported = finishRequest(request, prepared.result, paths);
  if (request.kind === 'pcb-preview') {
    if (exported.kind !== 'preview-board') throw new Error('Expected Rust board preview');
    return exported;
  }
  if (exported.kind !== 'finish-export') throw new Error('Expected Rust board export');
  const boardFile = exported.result.files[0];
  if (!boardFile) throw new Error('Rust returned no board file');
  if (paths.size === 0) return { id: request.id, kind: 'file', filename: boardFile.filename, bytes: textBytes(boardFile.content), mediaType: 'text/plain' };
  files[boardFile.filename] = textBytes(boardFile.content);
  const packed = archive({ id: request.id, kind: 'archive', request: { kind: 'pack-files', entries: Object.keys(files).map((path, bufferIndex) => ({ path, bufferIndex })) }, buffers: Object.values(files) });
  if (packed.kind !== 'archive' || packed.reply.kind !== 'packed') throw new Error('Expected Rust export archive');
  return { id: request.id, kind: 'file', filename: `${board.name}-kicad.zip`, bytes: packed.reply.bytes, mediaType: 'application/zip' };
}

async function handle(message: ExportWorkerRequest): Promise<ExportWorkerReply> {
  ready ??= init();
  try {
    await ready;
  } catch (cause) {
    ready = undefined;
    throw cause;
  }
  if (message.kind === 'artifact') return artifact(message.request);
  if (message.kind === 'archive') return archive(message);
  return build(message);
}

self.onmessage = (event: MessageEvent<ExportWorkerRequest>) => {
  const message = event.data;
  queue = queue.then(async () => {
    try {
      const reply = await handle(message);
      const transfers: ArrayBuffer[] = [];
      const transfer = (bytes: Uint8Array) => {
        if (!(bytes.buffer instanceof ArrayBuffer)) throw new Error('Archive output buffer is not transferable');
        transfers.push(bytes.buffer);
      };
      if (reply.kind === 'file') transfer(reply.bytes);
      if (reply.kind === 'archive') {
        if (reply.reply.kind === 'packed') transfer(reply.reply.bytes);
        else reply.reply.assets.forEach((asset) => transfer(asset.bytes));
      }
      self.postMessage(reply, transfers);
    } catch (cause) {
      self.postMessage({ id: message.id, kind: 'error', message: cause instanceof Error ? cause.message : String(cause) });
    }
  });
};
