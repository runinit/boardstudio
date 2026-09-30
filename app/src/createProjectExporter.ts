import type { SceneDelta } from '@boardstudio/v2-contracts';
import type { CoreClient } from './CoreClient';
import type { CoreReply, ElectricalPlan, Part, PartDefinition, ProjectDoc } from '@boardstudio/v2-contracts';
import { isErgogen, modelBindings } from '@boardstudio/v2-ergogen';
import type { MutableRefObject } from 'react';
import { CaseClient } from './CaseClient';
import { ExportClient } from './ExportClient';
import { bundledModel, bundledModelBytes } from './bundledModels';
import { caseAssembly } from './caseAssembly';
import type { CasePreviewContext } from './casePreviewContext';
import { casePreviewContextMatches } from './casePreviewContext';
import { pcbAssemblyFiles } from './electricalHandoff';
import { exportMechanicalAssembly } from './exportMechanicalAssembly';
import { firmwareRequest } from './firmwareHandoff';
import type { GenerationState } from './generationState';
import { effectiveCaseDocument, effectiveCaseScene } from './hardwareInstances';
import { prepareCase } from './prepareCase';
import { loadAsset, packProject } from './storage';

function download(name: string, bytes: string | Uint8Array, type: string): void {
  const content = typeof bytes === 'string' ? bytes : new Uint8Array(bytes);
  const url = URL.createObjectURL(new Blob([content], { type }));
  const anchor = document.createElement('a');

  anchor.href = url;
  anchor.download = name;
  anchor.click();

  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
function bindings(definition: PartDefinition, part?: Part): NonNullable<PartDefinition['models']> {
  let generated: NonNullable<PartDefinition['models']> = [];
  if (isErgogen(definition.generator?.source)) {
    try {
      generated = modelBindings(definition, part);
    } catch (cause) {
      generated = [{ assetId: `invalid-generator-model:${encodeURIComponent(String(cause))}`, offset: { x: 0, y: 0, z: 0 }, rotation: { x: 0, y: 0, z: 0 }, scale: { x: 1, y: 1, z: 1 } }];
    }
  }
  return [
    ...(definition.models ?? []),
    ...generated,
  ];
}
async function modelFiles(document: ProjectDoc, definitions: PartDefinition[], parts: Part[] = []): Promise<{ paths: Map<string, string>; files: Record<string, Uint8Array> }> {
  const paths = new Map<string, string>();
  const files: Record<string, Uint8Array> = {};

  const modelIds = definitions.flatMap((definition) => {
    const instances = parts.filter((part) => part.definitionId === definition.id);
    return (instances.length ? instances : [undefined]).flatMap((part) => bindings(definition, part).map((model) => model.assetId));
  });

  for (const id of new Set(modelIds)) {
    const asset = document.assets.find((item) => item.id === id);
    const bundled = !asset ? bundledModel(id) : undefined;
    const name = asset?.name ?? bundled?.filename;
    const extension = name?.match(/\.(step|stp|stl|wrl)$/i)?.[1]?.toLowerCase();

    if (!asset || !extension) {
      if (!bundled || !extension) throw new Error(`Model asset ${id} needs a STEP, STP, or WRL filename`);
    }

    const bytes = asset ? await loadAsset(asset.sha256) : await bundledModelBytes(id);

    if (!bytes) {
      throw new Error(`Model asset ${name ?? id} is missing`);
    }

    const digest = asset?.sha256 ?? Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', new Uint8Array(bytes))),
      (byte) => byte.toString(16).padStart(2, '0')).join('');
    const path = `models/${digest}.${extension}`;

    paths.set(id, path);
    files[path] = bytes;
  }

  return { paths, files };
}

type Inputs = {
  projectRef: MutableRefObject<ProjectDoc>;
  committedScene: MutableRefObject<SceneDelta>;
  client: MutableRefObject<CoreClient | null>;
  caseClient: MutableRefObject<CaseClient | null>;
  exportClient: MutableRefObject<ExportClient | null>;
  selectedBoardId: string;
  selectedInstance: NonNullable<ProjectDoc['hardware']>['instances'][number] | undefined;
  embedUsedModels: boolean;
  generation: GenerationState;
  currentPreviewContext: MutableRefObject<CasePreviewContext>;
  schedule: (work: () => Promise<void>) => void;
  accept: (reply: CoreReply, mode: 'open' | 'commit' | 'preview') => Promise<void>;
  ensureExportClient: () => ExportClient;
  resolveWiring: (document?: ProjectDoc, boardId?: string, instanceId?: string | null) => Promise<ElectricalPlan>;
  applyExportWiring: (document: ProjectDoc, boardId: string, draft: boolean) => Promise<{ document: ProjectDoc; plan: ElectricalPlan }>;
};

export function createProjectExporter({ projectRef, committedScene, client, caseClient, exportClient, selectedBoardId, selectedInstance, embedUsedModels, generation, currentPreviewContext, schedule, accept, ensureExportClient, resolveWiring, applyExportWiring }: Inputs) {
  function exportFile(kind: 'project' | 'kicad' | 'kicad-draft' | 'firmware' | 'footprints' | 'case-step' | 'keycaps-step' | 'svg' | 'dxf', boardId?: string): void {
    schedule(async () => {
      let document = projectRef.current;
      let resolved = committedScene.current;

      if (resolved.revision !== document.revision) {
        throw new Error('The committed scene is still resolving');
      }

      if (kind === 'project') {
        const bytes = await packProject(document, { embedUsedModels }, ensureExportClient());
        download(`${document.name}.boardstudio`, bytes, 'application/zip');
        return;
      }

      if (kind === 'footprints') {
        const { paths, files } = await modelFiles(document, document.definitions);
        exportClient.current ??= new ExportClient();
        const result = await exportClient.current.request({ kind: 'footprints', document, paths: [...paths], files });
        download(result.filename, result.bytes, result.mediaType);
        return;
      }

      const board = document.boards.find((item) => item.id === (boardId ?? selectedBoardId));

      if (!board) {
        throw new Error('Select a board before export');
      }

      let wiringPlan: ElectricalPlan | undefined;
      if (kind === 'kicad' || kind === 'kicad-draft') {
        const prepared = await applyExportWiring(document, board.id, kind === 'kicad-draft');
        document = prepared.document;
        wiringPlan = prepared.plan;
        resolved = committedScene.current;
      }

      if (kind === 'keycaps-step') {
        const reply = await client.current!.request({ id: crypto.randomUUID(), kind: 'resolve-keycaps', document, boardId: board.id, cases: null });
        if (reply.kind !== 'keycaps-resolved') throw new Error(reply.kind === 'error' ? reply.message : 'Expected resolved keycaps');
        if (reply.result.revision !== document.revision || projectRef.current.revision !== document.revision) throw new Error('Keycaps changed during export; export again');
        const errors = reply.result.findings.filter(finding => finding.severity === 'error');
        if (errors.length) throw new Error(errors.map(finding => finding.message).join('\n'));
        if (!reply.result.specs.length) throw new Error('Choose a keycap profile in Keymap before export');
        caseClient.current ??= new CaseClient();
        const result = await caseClient.current.keycaps(document.revision, reply.result.specs, true);
        if (projectRef.current.revision !== document.revision) throw new Error('Keycaps changed during export; export again');
        download(`${document.name}-keycaps.step`, result.step, 'model/step');
        return;
      }

      if (kind === 'firmware') {
        const instances = document.hardware?.instances ?? [];
        const split = document.hardware?.topology === 'split';
        const central = instances.find(instance => instance.role === 'central');
        const peripheral = instances.find(instance => instance.role === 'peripheral');
        if (split && (instances.length !== 2 || !central || !peripheral || central.half !== 'left' || peripheral.half !== 'right')) throw new Error('Split firmware requires a left central and a right peripheral assembly');
        const primary = await resolveWiring(document, central?.boardId ?? board.id, central?.id ?? null);
        const secondary = peripheral ? await resolveWiring(document, peripheral.boardId, peripheral.id) : undefined;
        const handoff = firmwareRequest(document, primary, secondary);
        const reply = await client.current!.request({ id: crypto.randomUUID(), kind: 'generate-firmware', request: handoff.request });
        if (reply.kind !== 'firmware-generated') throw new Error(reply.kind === 'error' ? reply.message : 'Expected generated firmware');
        const files = { ...reply.package.files, 'electrical-plan.json': JSON.stringify({ central: primary, peripheral: secondary }, null, 2) };
        const packed = await packHandoff(files);
        if (projectRef.current.revision !== document.revision) throw new Error('The wiring changed during firmware generation; export again');
        download(`${document.name}-zmk.zip`, packed, 'application/zip');
        return;
      }

      const boardReady = resolved.boardReadiness.find((item) => item.boardId === board.id);

      if (kind === 'svg' || kind === 'dxf') {
        if (!boardReady?.outline) {
          throw new Error('Resolve outline findings before export');
        }

        const contours = resolved.boardContours.find((entry) => entry.boardId === board.id)?.contours ?? [];
        exportClient.current ??= new ExportClient();
        const result = await exportClient.current.request({ kind: 'outline', document, boardId: board.id, contours, outlineFormat: kind, paths: [], files: {} });
        download(result.filename, result.bytes, result.mediaType);
        return;
      }

      if (kind === 'case-step') {
        if (!boardReady?.case) {
          throw new Error('Resolve case findings before export');
        }

        caseClient.current ??= new CaseClient();
        const prepared = await prepareCase(client.current!, caseAssembly(document, resolved, board.id));
        const result = await caseClient.current.request(prepared);
        const boardSuffix = document.boards.length === 1 ? '' : `-${board.name.replace(/[^a-zA-Z0-9_.-]+/g, '_')}`;

        download(`${document.name}${boardSuffix}-case.step`, result.step, 'model/step');
        return;
      }

      if (!boardReady?.pcb) {
        throw new Error('Resolve PCB findings before export');
      }

      const usedDefinitions = document.definitions
        .filter((definition) => board.partIds.some((id) => document.parts.find((part) => part.id === id)?.definitionId === definition.id))
      const usedParts = board.partIds.map((id) => document.parts.find((part) => part.id === id)).filter((part): part is Part => Boolean(part));
      const { paths, files } = await modelFiles(document, usedDefinitions, usedParts);

      const contours = resolved.boardContours.find((entry) => entry.boardId === board.id)?.contours ?? [];
      exportClient.current ??= new ExportClient();
      const result = await exportClient.current.request({ kind: 'kicad', document, boardId: board.id, contours, paths: [...paths], files });
      const handoffFiles: Record<string, string | Uint8Array> = { [result.filename]: result.bytes };
      if (wiringPlan) {
        const populations = [];
        for (const instance of document.hardware?.instances.filter(instance => instance.boardId === board.id) ?? []) {
          const plan = await resolveWiring(document, board.id, instance.id);
          if (kind !== 'kicad-draft' && plan.diagnostics.some(finding => finding.severity === 'error')) {
            throw new Error(`Resolve wiring findings for ${instance.name} before export`);
          }
          populations.push({ name: instance.name, plan });
        }
        Object.assign(handoffFiles, pcbAssemblyFiles(wiringPlan, kind === 'kicad-draft', populations));
      }
      const handoffBytes = await packHandoff(handoffFiles);
      if (wiringPlan) {
        // The exporter has produced the artifact. Persist protection before
        // initiating the browser download; failures above never record a handoff.
        await accept(await client.current!.request({ id: crypto.randomUUID(), kind: 'protect-electrical-handoff', baseRevision: document.revision, boardId: board.id, plan: wiringPlan }), 'commit');
      }
      download(`${document.name}-${kind === 'kicad-draft' ? 'draft-' : ''}pcb-handoff.zip`, handoffBytes, 'application/zip');
    });
  }

  async function packHandoff(files: Record<string, string | Uint8Array>): Promise<Uint8Array> {
    const entries = Object.keys(files).map((path, bufferIndex) => ({ path, bufferIndex }));
    const buffers = Object.values(files).map(value => typeof value === 'string' ? new TextEncoder().encode(value) : value);
    const packed = await ensureExportClient().archive({ kind: 'archive', request: { kind: 'pack-files', entries }, buffers });
    if (packed.reply.kind !== 'packed') throw new Error('Could not package the handoff');
    return packed.reply.bytes;
  }

  function exportMechanical(): void {
    schedule(async () => {
      const document = effectiveCaseDocument(projectRef.current, selectedInstance);
      const resolved = effectiveCaseScene(projectRef.current, committedScene.current, selectedInstance);
      const configuration = document.mechanical;
      if (generation.draft || generation.status !== 'ready' || generation.revision !== document.revision) throw new Error('Update the current preview before export');
      if (!configuration || configuration.boardId !== selectedBoardId) {
        throw new Error('Enable a mechanical assembly for the selected board before export');
      }
      if (resolved.revision !== document.revision) throw new Error('The committed scene is still resolving');
      const contours = resolved.boardContours.find((entry) => entry.boardId === configuration.boardId)?.contours ?? [];
      const captured = currentPreviewContext.current;
      const isCurrent = () => casePreviewContextMatches(captured, currentPreviewContext.current);
      caseClient.current ??= new CaseClient();
      const exporter = ensureExportClient();
      const files = await exportMechanicalAssembly({ document, contours, core: client.current!, cad: caseClient.current, exporter, isCurrent });
      const entries = Object.keys(files).map((path, bufferIndex) => ({ path, bufferIndex }));
      const packed = await exporter.archive({ kind: 'archive', request: { kind: 'pack-files', entries }, buffers: Object.values(files) });
      if (!isCurrent()) throw new Error('The assembly changed during export; export the current revision again');
      if (packed.reply.kind !== 'packed') throw new Error('Expected a mechanical assembly archive');
      download(`${document.name}${selectedInstance ? `-${selectedInstance.name.replace(/[^a-zA-Z0-9_-]/g, '-')}` : ''}-mechanical.zip`, packed.reply.bytes, 'application/zip');
    });
  }

  return { exportFile, exportMechanical };
}
