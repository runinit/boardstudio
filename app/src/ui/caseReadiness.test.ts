import { describe, expect, it } from 'vitest';
import { emptyProject, type MechanicalAssembly } from '@boardstudio/v2-contracts';
import { caseReadiness, mechanicalFindings } from './caseReadiness';

const assembly: MechanicalAssembly = { revision: 3, case: { revision: 3, bodies: [] }, stack: [], diagnostics: [],
  generationBlocked: false, gasketSupports: [], gasketTracks: [], generatedHardware: [], suggestedMounts: [], nominalPlateContours: [], plateContours: [] };
const current = { revision: 3, sceneRevision: 3, previewRevision: 3, boardId: 'board', configuredBoardId: 'board',
  generation: { status: 'ready' as const, revision: 3 }, assembly };

describe('case readiness', () => {
  it('permits current geometry with warnings but blocks errors', () => {
    const finding = { id: 'fit', scope: 'case' as const, targetIds: ['plate'], severity: 'warning' as const, message: 'Review the fit' };
    expect(caseReadiness(current).canExport).toBe(true);
    expect(caseReadiness({ ...current, assembly: { ...assembly, diagnostics: [finding] } }).canExport).toBe(true);
    const blocked = caseReadiness({ ...current, assembly: { ...assembly, diagnostics: [{ ...finding, severity: 'error' }] } });
    expect(blocked.canExport).toBe(false);
    expect(blocked.reviewRequired).toBe(true);
    expect(blocked.message).toContain('errors');
  });

  it.each(['sceneRevision', 'previewRevision'] as const)('rejects stale %s', key => {
    expect(caseReadiness({ ...current, [key]: 2 }).canExport).toBe(false);
  });

  it('never exports draft geometry even when its revision matches', () => {
    expect(caseReadiness({ ...current, generation: { ...current.generation, draft: true } }).canExport).toBe(false);
  });

  it('rejects stale generations, assemblies, and another board', () => {
    expect(caseReadiness({ ...current, generation: { status: 'ready', revision: 2 } }).canExport).toBe(false);
    expect(caseReadiness({ ...current, assembly: { ...assembly, revision: 2 } }).canExport).toBe(false);
    const other = caseReadiness({ ...current, boardId: 'other' });
    expect(other.canExport).toBe(false);
    expect(other.message).toContain('Configure');
  });

  it.each(['required', 'preparing', 'running', 'cancelled', 'failed', 'blocked'] as const)('does not export in %s state and identifies retained geometry', status => {
    const readiness = caseReadiness({ ...current, generation: { status, revision: 3 }, hasGeometry: true });
    expect(readiness.canExport).toBe(false);
    expect(readiness.message).toContain('Previous geometry');
  });

  it('groups duplicate mechanical diagnostics once for both count and list', () => {
    const finding = { id: 'one', scope: 'case' as const, targetIds: ['plate'], severity: 'error' as const, message: 'Unsupported material' };
    const result = mechanicalFindings({ ...assembly, diagnostics: [finding, { ...finding, id: 'two', targetIds: ['bottom'] }] }, emptyProject('test', 'Test'));
    expect(result).toHaveLength(1);
    expect(result[0].targetIds).toEqual(['plate', 'bottom']);
    expect(result[0].message).toContain('2 affected parts');
  });
});
