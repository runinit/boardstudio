import { describe, expect, it } from 'vitest';
import { demos, keyboardProject } from './keyboards';
import { sofleProject } from './sofle';
import { keyboardPreview } from './keyboardPreviews';

describe('demo layout previews', () => {
  it.each(demos)('$name matches the keyboard that opens', ({ id }) => {
    const document = id === 'v2' || id === 'rgb' || id === 'choc' ? sofleProject(id) : keyboardProject(id);
    const definitions = new Map(document.definitions.map(definition => [definition.id, definition]));
    const actual = document.parts.filter(part => definitions.get(part.definitionId)?.kind === 'switch').map(part => {
      const size = part.keycap ?? definitions.get(part.definitionId)!.keycap!;
      return { x: part.pose.at.x, y: -part.pose.at.y, angle: -part.pose.rotation, width: size.x, height: size.y };
    }).sort((a, b) => a.x - b.x || a.y - b.y);
    const preview = keyboardPreview(id);
    expect(preview.boardCount).toBe(document.boards.length);
    const expected = preview.keys.map(({ x, y, angle, width, height }) => ({ x, y, angle, width, height })).sort((a, b) => a.x - b.x || a.y - b.y);
    expect(expected).toHaveLength(actual.length);
    expected.forEach((key, index) => {
      for (const field of ['x', 'y', 'angle', 'width', 'height'] as const) expect(key[field]).toBeCloseTo(actual[index][field], 6);
    });
  });
});
