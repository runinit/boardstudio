import type { SceneDelta } from '@boardstudio/v2-contracts';

export type CasePreviewContext = {
  documentId: string;
  boardId: string;
  revision: number;
  scene: SceneDelta;
  committedScene: SceneDelta;
  instanceId?: string;
  session?: number;
  mechanicalFingerprint?: string;
};

export type ContextualCaseResult<T> = { context: CasePreviewContext; result: T };

export function casePreviewContextMatches(captured: CasePreviewContext, current: CasePreviewContext): boolean {
  return captured.documentId === current.documentId
    && captured.boardId === current.boardId
    && captured.instanceId === current.instanceId
    && captured.session === current.session
    && captured.revision === current.revision
    && captured.scene.revision === captured.revision
    && current.scene.revision === current.revision
    && captured.scene === captured.committedScene
    && current.scene === current.committedScene
    && captured.scene === current.scene;
}

/** Completed meshes can be reused only after comparing committed physical inputs. */
export function reusableCaseResult<T extends { revision: number }>(
  value: ContextualCaseResult<T> | undefined,
  context: CasePreviewContext,
): T | undefined {
  const previous = value?.context;
  if (!value || !previous) return undefined;
  if (!previous.mechanicalFingerprint || !context.mechanicalFingerprint) return currentCaseResult(value, context);
  if (previous.documentId !== context.documentId || previous.boardId !== context.boardId
    || previous.instanceId !== context.instanceId || previous.session !== context.session
    || previous.mechanicalFingerprint !== context.mechanicalFingerprint
    || value.result.revision !== previous.revision
    || previous.scene !== previous.committedScene || context.scene !== context.committedScene
    || previous.scene.revision !== previous.revision || context.scene.revision !== context.revision) return undefined;
  // Preserve lazy mesh accessors when only the revision changes.
  return value.result.revision === context.revision ? value.result : Object.defineProperties({}, {
    ...Object.getOwnPropertyDescriptors(value.result),
    revision: { value: context.revision, enumerable: true, configurable: true, writable: true },
  }) as T;
}

export function currentCaseResult<T extends { revision: number }>(
  value: ContextualCaseResult<T> | undefined,
  context: CasePreviewContext,
): T | undefined {
  return value && value.result.revision === context.revision && casePreviewContextMatches(value.context, context)
    ? value.result : undefined;
}
