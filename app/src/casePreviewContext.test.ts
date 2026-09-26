import { expect, test, vi } from 'vitest';
import type { CaseResult, SceneDelta } from '@boardstudio/v2-contracts';
import { casePreviewContextMatches, currentCaseResult, reusableCaseResult } from './casePreviewContext';
import type { CasePreviewContext } from './casePreviewContext';
import { generateCasePreview } from './useCaseGeneration';

function context(): CasePreviewContext {
  const scene = { revision: 7 } as SceneDelta;
  return { documentId: 'project', boardId: 'left', revision: 7, scene, committedScene: scene };
}
const result = (): CaseResult => ({ revision: 7, step: new Uint8Array(), mesh: { positions: new Float32Array(), normals: new Float32Array() } });

test('completed results disappear during a draft scene at the same revision', () => {
  const captured = context();
  const draft = { ...captured, scene: { ...captured.scene } };
  expect(currentCaseResult({ context: captured, result: result() }, draft)).toBeUndefined();
});

test('completed results cannot follow selection to another board at the same revision', () => {
  const captured = context();
  expect(currentCaseResult({ context: captured, result: result() }, { ...captured, boardId: 'right' })).toBeUndefined();
});

test('reopened document and replacement committed scene invalidate prior results', () => {
  const captured = context();
  expect(currentCaseResult({ context: captured, result: result() }, { ...captured, documentId: 'other-project' })).toBeUndefined();
  const replacement = { ...captured.scene };
  expect(currentCaseResult({ context: captured, result: result() }, { ...captured, scene: replacement, committedScene: replacement })).toBeUndefined();
});

test('matching committed context exposes mesh and mechanical resolution results', () => {
  const captured = context();
  const value = result();
  expect(currentCaseResult({ context: captured, result: value }, captured)).toBe(value);
  expect(currentCaseResult({ context: captured, result: { revision: 7, diagnostics: [] } }, captured)).toEqual({ revision: 7, diagnostics: [] });
  expect(currentCaseResult({ context: captured, result: { revision: 6 } }, captured)).toBeUndefined();
});

test('an in-flight CAD reply is rejected after a board switch without a revision change', async () => {
  const captured = context();
  let current = captured;
  const core = { request: vi.fn(async () => ({ kind: 'case-prepared', ir: { revision: 7, bodies: [] } })) };
  let finish!: (value: CaseResult) => void;
  const cad = { preview: vi.fn(() => new Promise<CaseResult>(resolve => { finish = resolve; })) };
  const pending = generateCasePreview(core as never, () => cad as never, { revision: 7, bodies: [] }, () => casePreviewContextMatches(captured, current), vi.fn());
  await vi.waitFor(() => expect(cad.preview).toHaveBeenCalledOnce());
  current = { ...captured, boardId: 'right' };
  finish(result());
  await expect(pending).resolves.toBeUndefined();
});


test('a draft scene arriving during preparation prevents starting CAD', async () => {
  const captured = context();
  let current = captured;
  let finish!: (value: unknown) => void;
  const core = { request: vi.fn(() => new Promise(resolve => { finish = resolve; })) };
  const cad = { preview: vi.fn() };
  const pending = generateCasePreview(core as never, cad as never, { revision: 7, bodies: [] }, () => casePreviewContextMatches(captured, current), vi.fn());
  current = { ...captured, scene: { ...captured.scene } };
  finish({ kind: 'case-prepared', ir: { revision: 7, bodies: [] } });
  await expect(pending).resolves.toBeUndefined();
  expect(cad.preview).not.toHaveBeenCalled();
});


test('a view-only rerender preserves completed results for the same committed context', () => {
  const captured = context();
  const completed = { context: captured, result: result() };
  const exportViewContext = { ...captured };
  expect(currentCaseResult(completed, exportViewContext)).toBe(completed.result);
});

test('completed geometry can follow an electrical-only revision within the same physical instance', () => {
  const captured = { ...context(), instanceId: 'left', session: 1, mechanicalFingerprint: 'geometry-a' };
  const scene = { revision: 8 } as SceneDelta;
  const current = { ...captured, revision: 8, scene, committedScene: scene };
  expect(reusableCaseResult({ context: captured, result: result() }, current)?.revision).toBe(8);
  expect(casePreviewContextMatches(captured, current)).toBe(false);
  expect(reusableCaseResult({ context: captured, result: result() }, { ...current, instanceId: 'right' })).toBeUndefined();
  expect(reusableCaseResult({ context: captured, result: result() }, { ...current, session: 2 })).toBeUndefined();
  expect(reusableCaseResult({ context: captured, result: result() }, { ...current, mechanicalFingerprint: 'geometry-b' })).toBeUndefined();
  expect(reusableCaseResult({ context: captured, result: result() }, { ...current, scene: { ...scene } })).toBeUndefined();
});
