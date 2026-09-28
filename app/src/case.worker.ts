import type { CaseResult, PreparedCaseAssemblyIR } from '@boardstudio/v2-contracts';
import { buildAssembly, readStepModel } from '@boardstudio/v2-cad';
import type { StepModel, CadProgress } from '@boardstudio/v2-cad';
import { bodyKey, previewBodies, type BodyPreview } from '../../cad/src/preview';
import { beginCadMetrics, countCadMetric, endCadMetrics } from '../../cad/src/metrics';
import { cadTimestamp } from './cadPerformance';

type CaseMessage = ({ id: string; kind: 'case' | 'preview'; ir: PreparedCaseAssemblyIR } | { id: string; kind: 'model'; bytes: Uint8Array }) & { metricsSentAt?: number };
type CaseReply =
  | { id: string; kind: 'progress'; progress: CadProgress }
  | { id: string; kind: 'preview'; result: PreviewDelta }
  | { id: string; kind: 'case'; result: CaseResult }
  | { id: string; kind: 'model'; result: StepModel }
  | { id: string; kind: 'error'; message: string; revision: number };

let queue = Promise.resolve();
type PreviewDelta = { revision: number; bodyIds: string[]; bodies: NonNullable<BodyPreview['bodies']> };
let previewBodiesById = new Map<string, { key: string; name: string }>();
const previews = new Map<string, { cancelled: boolean }>();

async function handle(message: CaseMessage, receivedAt: number): Promise<void> {
  const { id } = message;
  const metrics = message.metricsSentAt === undefined ? undefined : beginCadMetrics();
  const startedAt = metrics ? cadTimestamp() : 0;
  const send = (reply: CaseReply, buffers: ArrayBufferLike[] = [], triangles = 0) => {
    const diagnostics = metrics ? { ...metrics, requestSentAt: message.metricsSentAt!, receivedAt, startedAt,
      requestTransferredBytes: message.kind === 'model' ? message.bytes.byteLength : 0,
      revision: message.kind === 'model' ? undefined : message.ir.revision,
      transferredBytes: [...new Set(buffers)].reduce((sum, buffer) => sum + buffer.byteLength, 0),
      triangles, replySentAt: cadTimestamp() } : undefined;
    self.postMessage({ ...reply, ...(diagnostics ? { metrics: diagnostics } : {}) }, [...new Set(buffers)] as Transferable[]);
  };

  try {
    if (message.kind === 'model') {
      const result = await readStepModel(message.bytes);
      const reply: CaseReply = { id, kind: 'model', result };

      send(reply, [result.mesh.positions.buffer, result.mesh.normals.buffer]);
      return;
    }

    if (message.kind === 'preview') {
      const cancelled = () => previews.get(id)?.cancelled === true;
      if (cancelled()) throw new Error('Preview superseded');
      const changed = message.ir.bodies.filter(body => {
        const previous = previewBodiesById.get(body.body.id);
        return !previous || previous.key !== bodyKey(body) || previous.name !== body.body.name;
      });
      let lastProgress = -Infinity;
      let buildingSent = false;
      const result = changed.length
        ? await previewBodies({ ...message.ir, bodies: changed }, progress => {
          countCadMetric('progressEvents', 1);
          const now = performance.now();
          const firstBuilding = progress.stage === 'building' && !buildingSent;
          if (!firstBuilding && progress.completed !== progress.total && now - lastProgress < 32) return;
          buildingSent ||= progress.stage === 'building';
          lastProgress = now;
          countCadMetric('progressMessages', 1);
          self.postMessage({ id, kind: 'progress', progress });
        }, cancelled)
        : { revision: message.ir.revision, bodies: [] };
      if (cancelled()) throw new Error('Preview superseded');
      const delta: PreviewDelta = { revision: message.ir.revision, bodyIds: message.ir.bodies.map(body => body.body.id), bodies: result.bodies };
      send({ id, kind: 'preview', result: delta }, result.bodies.flatMap(body => [body.positions.buffer, body.normals.buffer]), result.bodies.reduce((sum, body) => sum + body.positions.length / 9, 0));
      previewBodiesById = new Map(message.ir.bodies.map(body => [body.body.id, { key: bodyKey(body), name: body.body.name }]));
      return;
    }
    const result = await buildAssembly(message.ir);
    const reply: CaseReply = { id, kind: 'case', result };

    send(reply, [result.step.buffer, result.mesh.positions.buffer, result.mesh.normals.buffer, ...(result.bodies ?? []).flatMap(body => [body.positions.buffer, body.normals.buffer])], result.mesh.positions.length / 9);
  } catch (cause) {
    const reply: CaseReply = {
      id,
      kind: 'error',
      message: cause instanceof Error ? cause.message : String(cause),
      revision: message.kind !== 'model' ? message.ir.revision : 0,
    };

    send(reply);
  } finally {
    previews.delete(id);
    endCadMetrics();
  }
}

self.onmessage = (event: MessageEvent<CaseMessage | { id: string; kind: 'cancel-preview' }>) => {
  const message = event.data;
  // Control messages bypass the work queue and are observed at body yields.
  if (message.kind === 'cancel-preview') {
    const preview = previews.get(message.id);
    if (preview) preview.cancelled = true;
    return;
  }
  if (message.kind === 'preview') previews.set(message.id, { cancelled: false });
  const receivedAt = message.metricsSentAt === undefined ? 0 : cadTimestamp();
  queue = queue.then(() => handle(message, receivedAt));
};
