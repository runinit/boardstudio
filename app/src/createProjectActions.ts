import type { CoreClient } from './CoreClient';
import type { CoreReply, CoreRequest, EditCommand, ProjectDoc } from '@boardstudio/v2-contracts';
import { defaultOutlineSettings, emptyProject } from '@boardstudio/v2-contracts';
import { catalogue as ergogenCatalogue } from '@boardstudio/v2-ergogen';
import type { MutableRefObject } from 'react';
import { ExportClient } from './ExportClient';
import { updateInstanceMechanical } from './hardwareInstances';
import { saveAsset, unpackProject } from './storage';
import { matrixWithPreset } from './ui/matrixPresets';
import { openKeyboardDemo, type DemoId } from './demos/keyboards';

const ERGOGEN_DEFINITIONS = ergogenCatalogue();
const MAX_PROJECT_BYTES = 128 * 1024 * 1024;
const MAX_MODEL_BYTES = 32 * 1024 * 1024;

type Inputs = {
  projectRef: MutableRefObject<ProjectDoc>;
  client: MutableRefObject<CoreClient | null>;
  exportClient: MutableRefObject<ExportClient | null>;
  selectedInstance: NonNullable<ProjectDoc['hardware']>['instances'][number] | undefined;
  schedule: (work: () => Promise<void>) => void;
  accept: (reply: CoreReply, mode: 'open' | 'commit' | 'preview') => Promise<void>;
  ensureExportClient: () => ExportClient;
  onProjectCreated?: (projectId: string) => void;
};

export function createProjectActions({ projectRef, client, exportClient, selectedInstance, schedule, accept, ensureExportClient, onProjectCreated }: Inputs) {
  function edit(command: EditCommand): void {
    schedule(async () => {
      if (!client.current) {
        return;
      }

      const current = projectRef.current;
      const operation = command.operation.kind === 'set-mechanical' && selectedInstance
        ? { kind: 'replace-document' as const, document: updateInstanceMechanical(current, selectedInstance.id, command.operation.configuration) }
        : command.operation;
      const baseRevision = operation.kind === 'replace-document'
        ? command.baseRevision
        : current.revision;
      const request: CoreRequest = {
        id: crypto.randomUUID(),
        kind: 'edit',
        command: { ...command, operation, baseRevision },
      };
      const reply = await client.current.request(request);

      await accept(reply, command.phase);
    });
  }

  function history(kind: 'undo' | 'redo'): void {
    schedule(async () => {
      if (!client.current) {
        return;
      }

      const reply = await client.current.request({ id: crypto.randomUUID(), kind });

      await accept(reply, 'commit');
    });
  }

  function importProject(file: File): void {
    schedule(async () => {
      if (!client.current) {
        return;
      }

      // Check metadata before allocating; Rust independently validates the archive.
      if (!file.size || file.size > MAX_PROJECT_BYTES) {
        throw new Error('Choose a nonempty project file no larger than 128 MiB');
      }

      const document = await unpackProject(new Uint8Array(await file.arrayBuffer()), ensureExportClient());
      const reply = await client.current.request({ id: crypto.randomUUID(), kind: 'open', document });

      await accept(reply, 'open');
    });
  }

  function newProject(): void {
    schedule(async () => {
      if (!client.current) {
        return;
      }

      const document = emptyProject(crypto.randomUUID(), 'Untitled keyboard');
      const boardId = crypto.randomUUID();
      const outlineId = crypto.randomUUID();

      document.outline = [{ id: outlineId, kind: 'part-envelope', settings: defaultOutlineSettings, partIds: [], margin: 4, operation: 'add' }];
      document.boards = [{ id: boardId, name: 'Main board', outlineIds: [outlineId], partIds: [], netIds: [], thickness: 1.6 }];
      document.materials = [{ id: 'pla', name: 'PLA', thickness: 3 }];

      const reply = await client.current.request({ id: crypto.randomUUID(), kind: 'open', document });

      await accept(reply, 'open');
      if (reply.kind === 'scene') onProjectCreated?.(reply.document.id);
    });
  }

  function openDemo(variant: DemoId): void {
    schedule(async () => {
      const core = client.current;
      if (!core) return;
      const previous = projectRef.current;
      try {
        const reply = await openKeyboardDemo(variant, request => core.request(request));
        await accept(reply, 'open');
      } catch (error) {
        // Demo construction uses the core; restore the working project on failure.
        await core.request({ id: crypto.randomUUID(), kind: 'open', document: previous });
        throw error;
      }
    });
  }

  function duplicateDesign(matrixId: string, presetId: Parameters<typeof matrixWithPreset>[1], orientation?: Parameters<typeof matrixWithPreset>[2]): void {
    schedule(async () => {
      if (!client.current) {
        return;
      }

      const original = projectRef.current;
      const matrix = original.matrices.find((item) => item.id === matrixId);

      if (!matrix) {
        throw new Error('Select a matrix to duplicate the design');
      }

      const variant = matrixWithPreset(matrix, presetId, orientation, original.definitions);
      const document: ProjectDoc = {
        ...structuredClone(original),
        id: crypto.randomUUID(),
        name: `${original.name} — ${presetId}`,
      };

      const opened = await client.current.request({ id: crypto.randomUUID(), kind: 'open', document });

      if (opened.kind !== 'scene') {
        throw new Error(opened.kind === 'error' ? opened.message : 'Expected project snapshot');
      }

      const changed = await client.current.request({
        id: crypto.randomUUID(),
        kind: 'edit',
        command: {
          baseRevision: opened.document.revision,
          transactionId: crypto.randomUUID(),
          phase: 'commit',
          targetIds: [matrixId],
          operation: { kind: 'set-matrix', matrix: variant.matrix, definitions: variant.definitions },
        },
      });

      if (changed.kind !== 'scene') {
        await client.current.request({ id: crypto.randomUUID(), kind: 'open', document: original });
        throw new Error(changed.kind === 'error' ? changed.message : 'Expected variant snapshot');
      }

      await accept(changed, 'open');
    });
  }

  function importPart(file: File): void {
    schedule(async () => {
      if (!file.name.endsWith('.kicad_mod')) {
        throw new Error('Select a .kicad_mod footprint');
      }

      exportClient.current ??= new ExportClient();
      const imported = await exportClient.current.artifact({ kind: 'import-footprint', definitionId: crypto.randomUUID(), source: await file.text() });
      if (imported.kind !== 'import-footprint') throw new Error('Expected Rust footprint import');
      const definition = { ...imported.result.definition, pads: imported.result.geometry.pads, courtyard: imported.result.geometry.courtyard };
      const current = projectRef.current;
      const document: ProjectDoc = {
        ...current,
        definitions: [...current.definitions, definition],
      };

      if (!client.current) {
        return;
      }

      const reply = await client.current.request({
        id: crypto.randomUUID(),
        kind: 'edit',
        command: {
          baseRevision: current.revision,
          transactionId: crypto.randomUUID(),
          phase: 'commit',
          targetIds: [definition.id],
          operation: { kind: 'replace-document', document },
        },
      });

      await accept(reply, 'commit');
    });
  }

  function importModel(file: File, definitionId: string, parameter?: string): void {
    schedule(async () => {
      const extension = file.name.match(/\.(step|stp|stl|wrl)$/i)?.[1]?.toLowerCase();

      if (!extension) {
        throw new Error('Select a STEP, STL, or WRL model');
      }

      if (!file.size || file.size > MAX_MODEL_BYTES) {
        throw new Error('Choose a nonempty model file no larger than 32 MiB');
      }

      const current = projectRef.current;
      const definition = current.definitions.find((item) => item.id === definitionId)
        ?? ERGOGEN_DEFINITIONS.find((item) => item.id === definitionId);

      if (!definition) {
        throw new Error('Part definition is missing');
      }

      const bytes = new Uint8Array(await file.arrayBuffer());
      const digest = await crypto.subtle.digest('SHA-256', new Uint8Array(bytes));
      const sha256 = Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, '0')).join('');
      const assetId = crypto.randomUUID();
      const asset = {
        id: assetId,
        name: file.name,
        mediaType: extension === 'wrl' ? 'model/vrml' : extension === 'stl' ? 'model/stl' : 'model/step',
        sha256,
        source: 'local file',
      };
      const updatedDefinition = parameter && definition.generator ? {
        ...definition,
        generator: { ...definition.generator, parameters: { ...definition.generator.parameters, [parameter]: `boardstudio-asset:${assetId}` } },
      } : {
        ...definition,
        models: [{ assetId, offset: { x: 0, y: 0, z: 0 }, rotation: { x: 0, y: 0, z: 0 }, scale: { x: 1, y: 1, z: 1 } }, ...(definition.models?.slice(1) ?? [])],
      };
      const nextDefinitions = current.definitions.some((item) => item.id === definitionId)
        ? current.definitions.map((item) => item.id === definitionId ? updatedDefinition : item)
        : [...current.definitions, updatedDefinition];
      const document: ProjectDoc = {
        ...current,
        assets: [...current.assets, asset],
        definitions: nextDefinitions,
      };

      await saveAsset(sha256, bytes);

      if (!client.current) {
        return;
      }

      const reply = await client.current.request({
        id: crypto.randomUUID(),
        kind: 'edit',
        command: {
          baseRevision: current.revision,
          transactionId: crypto.randomUUID(),
          phase: 'commit',
          targetIds: [definitionId, assetId],
          operation: { kind: 'replace-document', document },
        },
      });

      await accept(reply, 'commit');
    });
  }

  return { edit, history, importProject, newProject, openDemo, duplicateDesign, importPart, importModel };
}
