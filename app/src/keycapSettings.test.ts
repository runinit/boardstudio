import { describe, expect, it } from 'vitest';
import { demoProject } from './demo';
import { keyBinding, withKeyBinding } from './keycapSettings';

describe('keymap document settings', () => {
  it('sets bindings before wiring is configured without changing source geometry', () => {
    const original = demoProject();
    const next = withKeyBinding(original, 'main-board', original.parts[0].id, '&kp A');
    expect(keyBinding(next, 'main-board', original.parts[0].id)).toBe('&kp A');
    expect(original.hardware).toBeUndefined();
    expect(next.parts).toBe(original.parts);
  });
  it('preserves protected wiring, other bindings, and other board configuration', () => {
    const original = withKeyBinding(demoProject(), 'main-board', 'key-a', '&kp A');
    const other = withKeyBinding(original, 'other', 'key-a', '&kp Z');
    other.hardware!.boards[0].protectedHandoff = { fingerprint: 'pins', revision: 5, assignments: { row: 'P1' } };
    const next = withKeyBinding(other, 'main-board', 'key-b', '&kp B');
    expect(next.hardware!.boards.find(board => board.boardId === 'main-board')!.protectedHandoff).toEqual(other.hardware!.boards[0].protectedHandoff);
    expect(keyBinding(next, 'main-board', 'key-a')).toBe('&kp A');
    expect(keyBinding(next, 'other', 'key-a')).toBe('&kp Z');
  });
});
