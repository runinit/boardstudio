import { describe, expect, it } from 'vitest';
import { deriveSetupGuide } from './setupGuide';
import { readSetupGuidePrefs } from './useSetupGuide';
import { emptyProject } from '../../../contracts/src/index';
import type { ProjectDoc } from '../../../contracts/src/index';

const document = (boards: ProjectDoc['boards'], matrices: ProjectDoc['matrices'], parts: ProjectDoc['parts']) => ({ ...emptyProject('test', 'Test'), boards, matrices, parts, definitions: [{ id: 'switch', name: 'Switch', kind: 'switch' as const, pads: [], courtyard: [] }] });
const board = { id: 'b', name: 'Main', partIds: ['k'], netIds: [], outlineIds: [], thickness: 1.6 };
const matrix = { id: 'm', rows: 1, columns: 1, pitch: { x: 19, y: 19 }, origin: { x: 0, y: 0 }, definitionId: 'switch', partIds: ['k'], boardId: 'b' };
const part = { id: 'k', definitionId: 'switch', reference: 'SW1', pose: { at: { x: 0, y: 0 }, rotation: 0 }, side: 'front' as const };

describe('deriveSetupGuide', () => {
  it('starts empty projects incomplete and does not infer wiring from part counts', () => {
    const stages = deriveSetupGuide({ document: document([], [], []), boardId: undefined, wiring: { current: true, ready: true, applied: true }, caseReadiness: { configured: false, canExport: false, message: '' }, layoutErrorCount: 0 });
    expect(stages.project.ready).toBe(false);
    expect(stages.wiring.ready).toBe(false);
  });
  it('requires applied current wiring and exposes optional case readiness', () => {
    const base = { ...document([board], [matrix], [part]), hardware: { topology: 'unibody', transport: 'none', sharedConstruction: null, boards: [{ boardId: 'b', controllerPartId: null, mode: 'matrix', locks: {}, assignments: {}, keyBindings: {}, jumperStates: {}, protectedHandoff: null }], instances: [] } } as ProjectDoc;
    const stages = deriveSetupGuide({ document: base, boardId: 'b', wiring: { current: true, ready: true, applied: false }, caseReadiness: { configured: false, canExport: false, message: '' }, layoutErrorCount: 0 });
    expect(stages.project.ready).toBe(true);
    expect(stages.layout.ready).toBe(true);
    expect(stages.wiring.ready).toBe(false);
    expect(stages.case.optional).toBe(true);
  });
  it('scopes readiness to the selected board and reports layout errors', () => {
    const other = { ...board, id: 'other', partIds: [] };
    const stages = deriveSetupGuide({ document: document([board, other], [matrix], [part]), boardId: 'other', wiring: { current: false, ready: false, applied: false }, caseReadiness: { configured: true, canExport: false, message: 'Stale geometry' }, layoutErrorCount: 2 });
    expect(stages.project.ready).toBe(false);
    expect(stages.layout.detail).toContain('2 layout findings');
    expect(stages.case.detail).toBe('Stale geometry');
  });
  it('does not report wiring or review ready without a selected board', () => {
    const stages = deriveSetupGuide({ document: document([], [], []), boardId: undefined, wiring: { current: true, ready: true, applied: true }, caseReadiness: { configured: false, canExport: true, message: '' }, layoutErrorCount: 0 });
    expect(stages.wiring.ready).toBe(false);
    expect(stages.review.ready).toBe(false);
  });
  it('counts sparse matrix cells and ignores disabled cells', () => {
    const sparse = { ...matrix, columns: 3, cells: [{ row: 0, column: 1, enabled: false }] };
    const base = { ...document([board], [sparse], [part]), hardware: { topology: 'unibody', transport: 'none', sharedConstruction: null, boards: [{ boardId: 'b', controllerPartId: null, mode: 'matrix', locks: {}, assignments: {}, keyBindings: {}, jumperStates: {}, protectedHandoff: null }], instances: [] } } as ProjectDoc;
    expect(deriveSetupGuide({ document: base, boardId: 'b', wiring: { current: false, ready: false, applied: false }, caseReadiness: { configured: true, canExport: false, message: 'Stale' }, layoutErrorCount: 0 }).layout.detail).toContain('2 enabled key positions');
  });
  it('distinguishes current warnings from stale or error cases', () => {
    const base = { ...document([board], [matrix], [part]), hardware: { topology: 'unibody', transport: 'none', sharedConstruction: null, boards: [{ boardId: 'b', controllerPartId: null, mode: 'matrix', locks: {}, assignments: {}, keyBindings: {}, jumperStates: {}, protectedHandoff: null }], instances: [] } } as ProjectDoc;
    const wiring = { current: true, ready: true, applied: true };
    expect(deriveSetupGuide({ document: base, boardId: 'b', wiring, caseReadiness: { configured: true, canExport: true, message: 'Warnings to review' }, layoutErrorCount: 0 }).case.ready).toBe(true);
    expect(deriveSetupGuide({ document: base, boardId: 'b', wiring, caseReadiness: { configured: true, canExport: false, message: 'Mechanical error' }, layoutErrorCount: 0 }).review.ready).toBe(false);
  });
  it('sanitizes missing, corrupt, and invalid browser preferences', () => {
    expect(readSetupGuidePrefs(null)).toEqual({ open: false, currentStep: 'project' });
    expect(readSetupGuidePrefs('null')).toEqual({ open: false, currentStep: 'project' });
    expect(readSetupGuidePrefs('{"open":true,"currentStep":"wiring"}')).toEqual({ open: true, currentStep: 'wiring' });
    expect(readSetupGuidePrefs('{"open":true,"currentStep":"unknown"}')).toEqual({ open: true, currentStep: 'project' });
    expect(readSetupGuidePrefs('{')).toEqual({ open: false, currentStep: 'project' });
  });
});
