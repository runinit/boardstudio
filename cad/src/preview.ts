import type { CaseResult, PreparedCaseAssemblyIR, PreparedCaseIR } from '@boardstudio/v2-contracts';
import type { CadProgress } from './index.ts';
import { getKernel } from './kernel.ts';
import { cadBody, cadStage, countCadMetric } from './metrics.ts';

type BodyPreview = { revision: number; bodies: NonNullable<CaseResult['bodies']> };

export function bodyKey(ir: PreparedCaseIR): string {
  const { id: _id, name: _name, ...geometry } = ir.body;
  return `final-mesh:v1:0.1:0.5:${JSON.stringify({ body: geometry, regions: ir.regions })}`;
}

export async function previewBodies(ir: PreparedCaseAssemblyIR, progress: (value: CadProgress) => void, cancelled: () => boolean = () => false): Promise<BodyPreview> {
  const checkpoint = () => { if (cancelled()) throw new Error('Preview superseded'); };
  checkpoint();
  if (!ir.bodies.length) throw new Error('Case assembly requires at least one body');
  const base = { revision: ir.revision, total: ir.bodies.length };
  progress({ ...base, stage: 'loading', completed: 0 });
  const cadrum = await getKernel();
  const bodies: NonNullable<CaseResult['bodies']> = [];
  let sliceStarted = performance.now();
  for (const body of ir.bodies) {
    checkpoint();
    if (body.revision !== ir.revision) throw new Error('Case assembly contains a stale body revision');
    const notify = (stage: 'building' | 'tessellating', region = 0) => progress({ ...base, stage, body: body.body.name, region, completed: bodies.length });
    notify('building');
    const finishBody = cadBody(body.body.id, body.body.name);
    const mesh = cadrum.preview_body(body, bodyKey(body), notify) as CaseResult['mesh'];
    finishBody();
    bodies.push({ id: body.body.id, name: body.body.name, ...mesh });
    progress({ ...base, stage: 'tessellating', body: body.body.name, completed: bodies.length });
    // Cache hits and tiny pads share a slice. A synchronous kernel operation
    // can exceed this budget, so always yield after an expensive body.
    if (performance.now() - sliceStarted >= 12) {
      const finishYield = cadStage('bodyYield');
      await new Promise(resolve => setTimeout(resolve, 0));
      finishYield();
      sliceStarted = performance.now();
    }
  }
  checkpoint();
  return { revision: ir.revision, bodies };
}

export function combinedPreview(result: BodyPreview): BodyPreview & Pick<CaseResult, 'mesh'> {
  let mesh: CaseResult['mesh'] | undefined;
  return { revision: result.revision, bodies: result.bodies, get mesh() {
    if (mesh) return mesh;
    const bodies = result.bodies;
    const count = bodies.reduce((sum, body) => sum + body.positions.length, 0);
    const finishCopy = cadStage('combinedMeshCopy');
    const positions = new Float32Array(count);
    const normals = new Float32Array(count);
    let offset = 0;
    for (const body of bodies) { positions.set(body.positions, offset); normals.set(body.normals, offset); offset += body.positions.length; }
    countCadMetric('copiedMeshBytes', positions.byteLength + normals.byteLength);
    finishCopy();
    mesh = { positions, normals };
    return mesh;
  } };
}
