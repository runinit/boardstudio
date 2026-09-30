import type { CoreReply, ElectricalPlan, ProjectDoc, SceneDelta } from '@boardstudio/v2-contracts';
import type { CoreClient } from '../CoreClient';
import type { CaseClient } from '../CaseClient';
import type { ExportClient } from '../ExportClient';
import { casePreviewContextMatches, type CasePreviewContext } from '../casePreviewContext';
import type { GenerationState } from '../generationState';

export type ExportKind = 'project' | 'kicad' | 'kicad-draft' | 'firmware' | 'footprints' | 'case-step' | 'keycaps-step' | 'svg' | 'dxf';
export type ExportArtifact = { filename: string; bytes: string | Uint8Array; mediaType: string };
export type ExportSnapshot = {
  document: ProjectDoc;
  scene: SceneDelta;
  boardId: string;
  instance?: NonNullable<ProjectDoc['hardware']>['instances'][number];
  embedUsedModels: boolean;
  generation: GenerationState;
  previewContext: CasePreviewContext;
};
export type ExportServices = {
  core: () => CoreClient;
  cad: () => CaseClient;
  exporter: () => ExportClient;
  resolveWiring: (document: ProjectDoc, boardId: string, instanceId?: string | null) => Promise<ElectricalPlan>;
  accept: (reply: CoreReply, mode: 'commit') => Promise<void>;
};

function sameScope(a: CasePreviewContext, b: CasePreviewContext): boolean {
  return a.documentId === b.documentId && a.session === b.session && a.boardId === b.boardId && a.instanceId === b.instanceId;
}

/** A committed snapshot can advance only through the export's own accepted edits. */
export class ExportContext {
  private expected: { document: ProjectDoc; scene: SceneDelta };
  readonly boardId: string;

  constructor(readonly snapshot: ExportSnapshot, private readonly read: () => ExportSnapshot, boardId = snapshot.boardId, private readonly scope = snapshot.previewContext) {
    this.expected = { document: snapshot.document, scene: snapshot.scene };
    this.boardId = boardId;
    if (snapshot.scene.revision !== snapshot.document.revision) throw new Error('The committed scene is still resolving');
    this.assertCurrent();
  }

  get document(): ProjectDoc { return this.expected.document; }
  get scene(): SceneDelta { return this.expected.scene; }

  isCurrent = (): boolean => {
    const current = this.read();
    return sameScope(this.scope, current.previewContext)
      && current.document === this.document && current.scene === this.scene
      && this.scene.revision === this.document.revision;
  };

  isPreviewCurrent = (): boolean => this.isCurrent() && casePreviewContextMatches(this.snapshot.previewContext, this.read().previewContext);

  assertCurrent(revision = this.document.revision): void {
    if (!this.isCurrent() || revision !== this.document.revision) {
      throw new Error('The project or assembly changed during export; export the current revision again');
    }
  }

  adopt(document: ProjectDoc): void {
    const current = this.read();
    if (!sameScope(this.scope, current.previewContext) || current.document !== document) {
      throw new Error('The project or assembly changed during export; export the current revision again');
    }
    this.expected = { document, scene: current.scene };
    this.assertCurrent();
  }
}
