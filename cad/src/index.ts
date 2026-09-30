import type { KeycapSpec, CaseResult, PreparedCaseAssemblyIR, PreparedCaseIR } from '@boardstudio/v2-contracts';
import { getKernel } from './kernel.ts';
import { bodyKey, previewBodies, combinedPreview } from './preview.ts';

const MAX_STEP_BYTES = 32 * 1024 * 1024;

/** Builds one revisioned case body; call from a dedicated CAD worker. */
export async function buildCase(ir: PreparedCaseIR): Promise<CaseResult> {
  if (ir.regions.length === 0) throw new Error('Case requires at least one prepared region');
  const cadrum = await getKernel();
  return cadrum.build_case(ir) as CaseResult;
}

/** Exports all bodies as one STEP compound and one combined mesh. */
export async function buildAssembly(ir: PreparedCaseAssemblyIR): Promise<CaseResult> {
  if (ir.bodies.length === 0) throw new Error('Case assembly requires at least one body');
  for (const body of ir.bodies) {
    if (body.revision !== ir.revision) throw new Error('Case assembly contains a stale body revision');
    if (body.regions.length === 0) throw new Error('Case requires at least one prepared region');
  }

  const cadrum = await getKernel();
  return cadrum.export_cached_assembly({ revision: ir.revision, bodies: ir.bodies }, ir.bodies.map(bodyKey)) as CaseResult;
}

export interface StepModel {
  mesh: CaseResult['mesh'];
  bounds: { min: [number, number, number]; max: [number, number, number] };
}

/** Imports a bounded STEP file into the CAD kernel and tessellates its shape. */
export async function readStepModel(bytes: Uint8Array): Promise<StepModel> {
  if (!(bytes instanceof Uint8Array) || bytes.byteLength === 0 || bytes.byteLength > MAX_STEP_BYTES) {
    throw new Error('STEP import failed: invalid file size');
  }

  const cadrum = await getKernel();
  return cadrum.read_step_model(bytes) as StepModel;
}

export type CadProgress = { revision: number; stage: 'loading' | 'building' | 'tessellating'; completed: number; total: number; body?: string; region?: number };
export type CasePreviewResult = Omit<CaseResult, 'step'>;

export async function previewAssembly(ir: PreparedCaseAssemblyIR, progress: (value: CadProgress) => void): Promise<CasePreviewResult> {
  const result = combinedPreview(await previewBodies(ir, progress));
  // Preserve eager materialization at the public legacy boundary.
  return { ...result };
}

/** Generates revisioned keycap CAD in the dedicated worker, with optional STEP. */
export async function buildKeycaps(revision: number, specs: KeycapSpec[], exportStep = false, cancelled: () => boolean = () => false): Promise<CaseResult> {
  const kernel = await getKernel();
  if (cancelled()) throw new Error('Keycap preview superseded');
  if (exportStep) return kernel.build_keycaps({ revision, specs, export: true }) as CaseResult;
  const bodies: NonNullable<CaseResult['bodies']> = [];
  for (let index = 0; index < specs.length; index += 8) {
    if (cancelled()) throw new Error('Keycap preview superseded');
    const result = kernel.build_keycaps({ revision, specs: specs.slice(index, index + 8), export: false }) as CaseResult;
    bodies.push(...result.bodies ?? []);
    await new Promise(resolve => setTimeout(resolve, 0));
  }
  if (cancelled()) throw new Error('Keycap preview superseded');
  return { revision, step: new Uint8Array(), mesh: { positions: new Float32Array(), normals: new Float32Array() }, bodies };
}
