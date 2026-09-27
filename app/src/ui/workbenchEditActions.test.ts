import { describe, expect, test } from 'vitest';
import { buildMirrorConstraint, buildOffsetConstraint, assignNetPins } from './createWorkbenchEditActions';
import { documentWithPlacedPart } from './createWorkbenchPlacementActions';
import { defaultOutlineSettings, type PartDefinition, type ProjectDoc } from '@boardstudio/v2-contracts';

const definition: PartDefinition = { id: 'd', name: 'D', kind: 'custom', courtyard: [], pads: [{ id: '1', number: '1', at: { x: 0, y: 0 }, size: { x: 1, y: 1 }, shape: 'rect' }] };
const document = (): ProjectDoc => ({ format: 'boardstudio/v2', parameters: {}, id: 'p', name: 'P', revision: 1, boards: [{ id: 'b', name: 'B', outlineIds: ['o'], partIds: [], netIds: [], thickness: 1.6 }], outline: [{ id: 'o', kind: 'part-envelope', settings: defaultOutlineSettings, partIds: [], margin: 4, operation: 'add' }], parts: [], definitions: [], matrices: [], layouts: [{ id: 'l', name: 'L', boardId: 'b', matrixId: 'm', partIds: [], mirrorLink: undefined }], nets: [], constraints: [], scripts: [], assets: [], materials: [], caseBodies: [] });

describe('workbench edit helpers', () => {
  test('placed part joins board envelope and requested layout once', () => {
    const part = { id: 'part', reference: 'U1', definitionId: 'd', pose: { at: { x: 1, y: 2 }, rotation: 0 }, side: 'front' as const };
    const result = documentWithPlacedPart(document(), 'b', part, definition, 'l');
    expect(result.boards[0].partIds).toEqual(['part']);
    expect(result.outline[0].kind === 'part-envelope' && result.outline[0].partIds).toEqual(['part']);
    expect(result.layouts?.[0].partIds).toEqual(['part']);
  });
  test('assigning a terminal removes old pads before adding the replacement', () => {
    const result = assignNetPins([{ id: 'n', name: 'N', pins: [{ partId: 'p', padId: 'old' }, { partId: 'x', padId: 'old' }] }], 'p', ['old'], 'n');
    expect(result[0].pins).toEqual([{ partId: 'x', padId: 'old' }, { partId: 'p', padId: 'old' }]);
  });
  test('constraint builders reject blank or non-finite values', () => {
    expect(buildOffsetConstraint('c', 's', 't', '', '0', '0')).toBeUndefined();
    expect(buildMirrorConstraint('c', 's', 't', 'vertical', 'NaN')).toBeUndefined();
    expect(buildOffsetConstraint('c', 's', 't', '1', '2', '3')).toMatchObject({ kind: 'offset', offset: { x: 1, y: 2 } });
  });
});
