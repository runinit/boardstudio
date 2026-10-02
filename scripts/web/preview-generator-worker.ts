import { modelBindings } from '../../ergogen/src/index.ts';
import { exportErgogenForms } from '../../kicad/src/ergogen.ts';
import type { ErgogenJob, ErgogenJobResult, ReservedNet } from '../../contracts/src/index.ts';

type OwnerEnvelope = {
  scope: Record<string, unknown>;
  token: string;
  viewer_instance: number;
  projection_generation: number;
};

type BatchEnvelope = {
  accepted_revision: number;
  batch_generation: number;
};

type PlanKey = {
  snapshot_token: string;
  revision: number;
  job_ids: string[];
};

export type PreviewGeneratorRequest = {
  kind: 'generate-preview-jobs';
  worker_generation: number;
  request_id: number;
  owner: OwnerEnvelope;
  batch: BatchEnvelope;
  plan_key: PlanKey;
  jobs: ErgogenJob[];
  reserved_nets: ReservedNet[];
  next_net_index: number;
  paths: [string, string][];
};

export type PreviewGeneratorReply = {
  kind: 'generated-preview-jobs';
  worker_generation: number;
  request_id: number;
  owner: OwnerEnvelope;
  batch: BatchEnvelope;
  plan_key: PlanKey;
  results: ErgogenJobResult[];
} | {
  kind: 'preview-generator-error';
  worker_generation: number;
  request_id: number;
  owner: OwnerEnvelope;
  batch: BatchEnvelope;
  plan_key: PlanKey;
  message: string;
};

const MAX_U32 = 0xffff_ffff;
const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null && !Array.isArray(value);

function safeInteger(value: unknown, label: string, minimum = 0, maximum = Number.MAX_SAFE_INTEGER): number {
  if (typeof value !== 'number' || !Number.isSafeInteger(value) || value < minimum || value > maximum) {
    throw new Error(`${label} must be a safe integer in range`);
  }
  return value;
}

function validateRequest(request: PreviewGeneratorRequest): void {
  if (request.kind !== 'generate-preview-jobs') throw new Error('Unsupported preview generator request kind');
  safeInteger(request.worker_generation, 'Worker generation', 1);
  safeInteger(request.request_id, 'Worker request ID', 1);
  if (!isRecord(request.owner) || !isRecord(request.owner.scope) || typeof request.owner.token !== 'string' || request.owner.token.length === 0) {
    throw new Error('Preview generator owner identity is invalid');
  }
  safeInteger(request.owner.viewer_instance, 'Viewer instance', 0);
  safeInteger(request.owner.projection_generation, 'Projection generation', 0);
  if (!isRecord(request.batch)) throw new Error('Preview generator batch identity is invalid');
  const acceptedRevision = safeInteger(request.batch.accepted_revision, 'Accepted revision');
  safeInteger(request.batch.batch_generation, 'Batch generation', 0);
  if (!isRecord(request.plan_key) || typeof request.plan_key.snapshot_token !== 'string' || request.plan_key.snapshot_token.length === 0) {
    throw new Error('Preview generator plan key is invalid');
  }
  const revision = safeInteger(request.plan_key.revision, 'Plan revision');
  if (revision !== acceptedRevision) throw new Error('Plan revision does not match accepted revision');
  if (!Array.isArray(request.plan_key.job_ids) || request.plan_key.job_ids.some((id) => typeof id !== 'string' || id.length === 0)) {
    throw new Error('Plan job IDs must be nonempty strings');
  }
  if (!Array.isArray(request.jobs) || !Array.isArray(request.reserved_nets) || !Array.isArray(request.paths)) {
    throw new Error('Preview generator inputs are invalid');
  }
  if (request.jobs.length !== request.plan_key.job_ids.length || request.jobs.some((job, index) => job.jobId !== request.plan_key.job_ids[index])) {
    throw new Error('Plan job order does not match the request key');
  }
  const jobIds = new Set(request.plan_key.job_ids);
  if (jobIds.size !== request.plan_key.job_ids.length) throw new Error('Plan job IDs must be unique');
  const indexes = new Set<number>();
  for (const net of request.reserved_nets) {
    if (!isRecord(net) || typeof net.name !== 'string') throw new Error('Reserved net name is invalid');
    safeInteger(net.index, 'Reserved net index', 1, MAX_U32);
    if (indexes.has(net.index)) throw new Error('Reserved net table contains duplicate indices');
    indexes.add(net.index);
  }
  const minimumNext = Math.max(0, ...indexes) + 1;
  const nextIndex = safeInteger(request.next_net_index, 'Next net index', 1, MAX_U32);
  if (nextIndex < minimumNext) throw new Error('Next net index would overlap reserved nets');
  const pathIds = new Set<string>();
  for (const pair of request.paths) {
    if (!Array.isArray(pair) || pair.length !== 2 || typeof pair[0] !== 'string' || typeof pair[1] !== 'string') {
      throw new Error('Model path entries must be string pairs');
    }
    if (pathIds.has(pair[0])) throw new Error('Model path IDs must be unique');
    pathIds.add(pair[0]);
  }
}

function netAllocator(initial: ReservedNet[], firstIndex: number): {
  lookup: (name: string) => number;
  snapshot: () => ReservedNet[];
} {
  const nets = initial.map((net) => ({ ...net }));
  let nextIndex = firstIndex;
  return {
    lookup(name) {
      const existing = nets.find((net) => net.name === name);
      if (existing) return existing.index;
      if (nextIndex > MAX_U32) throw new Error('Allocated net index exceeds the 32-bit range');
      const net = { name, index: nextIndex++ };
      nets.push(net);
      return net.index;
    },
    snapshot: () => nets.map((net) => ({ ...net })),
  };
}

function pathsForPreview(jobs: ErgogenJob[], source: [string, string][]): Map<string, string> {
  const paths = new Map(source);
  for (const job of jobs) {
    for (const model of modelBindings(job.definition, job.part)) {
      if (model.assetId.startsWith('unresolved-model:')) {
        let original: string;
        try {
          original = decodeURIComponent(model.assetId.slice('unresolved-model:'.length));
        } catch {
          throw new Error('Ergogen model path has invalid percent encoding');
        }
        paths.set(original, `models/unresolved/${encodeURIComponent(original)}`);
      }
    }
  }
  return paths;
}

export function generatePreviewJobs(request: PreviewGeneratorRequest): Extract<PreviewGeneratorReply, { kind: 'generated-preview-jobs' }> {
  validateRequest(request);
  if (request.plan_key.snapshot_token.length === 0) throw new Error('Plan snapshot token must not be empty');
  const paths = pathsForPreview(request.jobs, request.paths);
  const allocator = netAllocator(request.reserved_nets, request.next_net_index);
  const results = request.jobs.map((job) => {
    const forms = exportErgogenForms(job.definition, job.part, paths, allocator.lookup);
    return {
      snapshotToken: request.plan_key.snapshot_token,
      revision: request.plan_key.revision,
      jobId: job.jobId,
      source: [...forms.footprints, ...forms.objects].join('\n'),
      nets: allocator.snapshot(),
    };
  });
  return {
    kind: 'generated-preview-jobs',
    worker_generation: request.worker_generation,
    request_id: request.request_id,
    owner: request.owner,
    batch: request.batch,
    plan_key: request.plan_key,
    results,
  };
}

export function handlePreviewGeneratorMessage(request: PreviewGeneratorRequest): PreviewGeneratorReply {
  try {
    return generatePreviewJobs(request);
  } catch (cause) {
    return {
      kind: 'preview-generator-error',
      worker_generation: request.worker_generation,
      request_id: request.request_id,
      owner: request.owner,
      batch: request.batch,
      plan_key: request.plan_key,
      message: (cause instanceof Error ? cause.message : String(cause)).slice(0, 2048),
    };
  }
}

if (typeof self !== 'undefined' && typeof self.addEventListener === 'function') {
  self.addEventListener('message', (event: MessageEvent<PreviewGeneratorRequest>) => {
    self.postMessage(handlePreviewGeneratorMessage(event.data));
  });
}
