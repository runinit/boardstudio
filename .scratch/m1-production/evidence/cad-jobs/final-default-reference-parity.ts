import { strFromU8, unzipSync } from 'fflate';
import { unpackProject } from './storage';
import { CoreClient } from './CoreClient';
import { CaseClient } from './CaseClient';
import { ExportClient } from './ExportClient';
import { exportMechanical } from './exports/cases';
import { ExportContext, type ExportSnapshot } from './exports/context';
import type { ProjectDoc, SceneDelta } from '@boardstudio/v2-contracts';

const M1_STEP_SHA256 = 'f946d69c2ad39abd3f799ca996067d7f89dba7cc361a9516251c9e32307722f4';
const resultElement = document.querySelector<HTMLPreElement>('#result')!;
const archiveInput = document.querySelector<HTMLInputElement>('#archive')!;
const runButton = document.querySelector<HTMLButtonElement>('#run')!;

async function sha256(bytes: Uint8Array): Promise<string> {
  const digest = await crypto.subtle.digest('SHA-256', bytes);
  return [...new Uint8Array(digest)].map(byte => byte.toString(16).padStart(2, '0')).join('');
}
function stable(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(stable).join(',')}]`;
  if (value && typeof value === 'object') return `{${Object.entries(value).filter(([, item]) => item !== undefined).sort(([a], [b]) => a.localeCompare(b)).map(([key, item]) => `${JSON.stringify(key)}:${stable(item)}`).join(',')}}`;
  return JSON.stringify(value) ?? 'null';
}
function meshMetrics(positions: Float32Array) {
  let signedVolume = 0;
  const min = [Infinity, Infinity, Infinity];
  const max = [-Infinity, -Infinity, -Infinity];
  for (let index = 0; index < positions.length; index += 9) {
    const ax = positions[index], ay = positions[index + 1], az = positions[index + 2];
    const bx = positions[index + 3], by = positions[index + 4], bz = positions[index + 5];
    const cx = positions[index + 6], cy = positions[index + 7], cz = positions[index + 8];
    signedVolume += (ax * (by * cz - bz * cy) + ay * (bz * cx - bx * cz) + az * (bx * cy - by * cx)) / 6;
    for (const vertex of [[ax, ay, az], [bx, by, bz], [cx, cy, cz]]) for (let axis = 0; axis < 3; axis++) { min[axis] = Math.min(min[axis], vertex[axis]); max[axis] = Math.max(max[axis], vertex[axis]); }
  }
  return { vertexCount: positions.length / 3, absoluteSignedVolumeMm3: Math.abs(signedVolume), bounds: { min, max } };
}
function sceneDigestView(scene: SceneDelta) {
  return {
    boardContours: scene.boardContours,
    transforms: scene.transforms,
    boardReadiness: scene.boardReadiness,
    revision: scene.revision,
  };
}

runButton.addEventListener('click', async () => {
  runButton.disabled = true;
  let core: CoreClient | undefined;
  let cad: CaseClient | undefined;
  let exporter: ExportClient | undefined;
  const referenceCadRequests: unknown[] = [];
  let inputEvidence: Record<string, unknown> = {};
  try {
    const file = archiveInput.files?.[0];
    if (!file) throw new Error('Select the public M1 archive first.');
    const archiveBytes = new Uint8Array(await file.arrayBuffer());
    const archiveSha256 = await sha256(archiveBytes);
    const archiveFiles = unzipSync(archiveBytes);
    if (!archiveFiles['project.json']) throw new Error('Public archive has no project.json.');

    core = new CoreClient();
    cad = new CaseClient();
    exporter = new ExportClient();
    const document = await unpackProject(archiveBytes, exporter) as ProjectDoc;
    const opened = await core.request({ id: crypto.randomUUID(), kind: 'open', document });
    if (opened.kind !== 'scene') throw new Error(`Public CoreClient open failed: ${opened.kind}`);
    const scene = opened.scene;
    const instance = document.physicalInstanceId ? document.hardware?.instances.find(entry => entry.id === document.physicalInstanceId) : undefined;
    const boardId = instance?.boardId ?? document.mechanical?.boardId;
    if (!boardId || !document.mechanical) throw new Error('Archive lacks the active default mechanical configuration.');
    if (document.mechanical.boardId !== boardId) throw new Error(`Mechanical board ${document.mechanical.boardId} differs from selected instance board ${boardId}.`);
    const archiveProject = JSON.parse(strFromU8(archiveFiles['project.json'])) as ProjectDoc;
    if (stable(archiveProject) !== stable(document)) throw new Error('Public unpackProject changed the archived ProjectDoc value.');

    inputEvidence = { archive: { name: file.name, bytes: archiveBytes.byteLength, sha256: archiveSha256 }, input: { id: document.id, revision: document.revision, physicalInstanceId: document.physicalInstanceId, instanceId: instance?.id ?? null, flipped: instance?.flipped ?? null, instanceSelection: instance ? 'archive physicalInstanceId' : 'canonical default (no selected instance)', boardId, mechanicalMethod: document.mechanical.method, pcbThickness: document.mechanical.pcbThickness }, scene: { revision: scene.revision, outlineReady: scene.boardReadiness.find(item => item.boardId === boardId)?.outline ?? false, contours: scene.boardContours.find(item => item.boardId === boardId)?.contours.length ?? 0 } };
    const generation = { status: 'ready' as const, revision: document.revision };
    const previewContext = { documentId: document.id, boardId, revision: document.revision, scene, committedScene: scene, instanceId: instance?.id, session: 1 };
    const snapshot: ExportSnapshot = { document, scene, boardId, instance, embedUsedModels: true, generation, previewContext };
    const context = new ExportContext(snapshot, () => snapshot);
    const originalCadRequest = cad.request.bind(cad);
    cad.request = async ir => {
      const result = await originalCadRequest(ir);
      referenceCadRequests.push({ revision: ir.revision, bodyCount: ir.bodies.length, bodyIds: ir.bodies.map(body => body.body.id), stepBytes: result.step.byteLength, stepSha256: await sha256(result.step), mesh: meshMetrics(result.mesh.positions) });
      return result;
    };
    const artifact = await exportMechanical(context, {
      core: () => core!,
      cad: () => cad!,
      exporter: () => exporter!,
    });
    const handoff = unzipSync(artifact.bytes);
    const step = handoff['assembly.step'];
    if (!step) throw new Error('Existing exportMechanical service did not produce assembly.step.');
    globalThis.document.getElementById('reference-step-download')?.remove();
    const stepDownload = globalThis.document.createElement('a');
    stepDownload.id = 'reference-step-download';
    stepDownload.download = 'reference-reviung41-default.step';
    stepDownload.href = URL.createObjectURL(new Blob([step], { type: 'model/step' }));
    stepDownload.textContent = 'Download reference assembly STEP bytes';
    globalThis.document.body.append(stepDownload);
    const stepSha256 = await sha256(step);
    const assembly = handoff['assembly.json'] ? JSON.parse(strFromU8(handoff['assembly.json'])) : undefined;
    const report = {
      status: stepSha256 === M1_STEP_SHA256 ? 'same-step-bytes' : 'step-bytes-differ',
      appOrigin: location.origin,
      archive: { name: file.name, bytes: archiveBytes.byteLength, sha256: archiveSha256, projectJsonBytes: archiveFiles['project.json'].byteLength },
      input: { id: document.id, revision: document.revision, physicalInstanceId: document.physicalInstanceId, instanceId: instance?.id ?? null, flipped: instance?.flipped ?? null, instanceSelection: instance ? 'archive physicalInstanceId' : 'canonical default (no selected instance)', boardId, mechanicalMethod: document.mechanical.method, pcbThickness: document.mechanical.pcbThickness },
      publicCoreOpen: { kind: opened.kind, revision: scene.revision, outlineReady: scene.boardReadiness.find(item => item.boardId === boardId)?.outline ?? false, contours: scene.boardContours.find(item => item.boardId === boardId)?.contours.length ?? 0 },
      existingTypeScriptService: 'app/src/exports/cases.ts::exportMechanical -> app/src/hardwareInstances.ts effectiveCaseDocument/effectiveCaseScene -> app/src/exportMechanicalAssembly.ts -> public CoreClient/CaseClient/ExportClient workers',
      reference: { archiveHashRoundTrip: stable(archiveProject) === stable(document), publicCadRequests: referenceCadRequests, artifactName: artifact.filename, handoffBytes: artifact.bytes.byteLength, assemblyStepBytes: step.byteLength, assemblyStepSha256: stepSha256, m1AssemblyStepSha256: M1_STEP_SHA256, partStepCount: Object.keys(handoff).filter(name => name.startsWith('parts/') && name.endsWith('.step')).length, assemblySummary: assembly, expectedIncludesNominalPcbReference: true },
      publicSceneDigestSha256: await sha256(new TextEncoder().encode(stable(sceneDigestView(scene)))),
      limits: ['This uses existing TypeScript projection and export services and the public CoreClient/CaseClient/ExportClient workers. It does not reimplement the Rust projection mapper.', 'The archive import writes assets only on this isolated Vite origin; it does not touch the release origin IndexedDB.'],
      completedAt: new Date().toISOString(),
    };
    (window as typeof window & { __M1_DEFAULT_REFERENCE_PARITY__?: unknown }).__M1_DEFAULT_REFERENCE_PARITY__ = report;
    resultElement.textContent = JSON.stringify(report, null, 2);
  } catch (error) {
    const report = { status: 'failed', error: String(error), stack: error instanceof Error ? error.stack : undefined, input: inputEvidence, capturedPublicCadRequests: referenceCadRequests, sameInputStepHashMatch: referenceCadRequests.some((item: any) => item.stepSha256 === M1_STEP_SHA256), completedAt: new Date().toISOString() };
    (window as typeof window & { __M1_DEFAULT_REFERENCE_PARITY__?: unknown }).__M1_DEFAULT_REFERENCE_PARITY__ = report;
    resultElement.textContent = JSON.stringify(report, null, 2);
  } finally {
    core?.close();
    cad?.close();
    exporter?.close();
    runButton.disabled = false;
  }
});
