import type { ProjectDoc } from '@boardstudio/v2-contracts';

import { keycapDefaults } from '@boardstudio/v2-contracts';

export const defaultKeycapBoard = keycapDefaults.board;
export const defaultKeycapMatrix = keycapDefaults.matrix;
export const defaultKeycapKey = keycapDefaults.key;
export function keyBinding(document: ProjectDoc, boardId: string, keyId: string): string {
  const typed = document.keymap?.layers[0]?.bindings[keyId];
  if (typed) {
    if ('keycode' in typed) return `&kp ${typed.keycode}`;
    if ('tap' in typed) return `&kp ${typed.tap}`;
    return typed.kind === 'transparent' ? '&trans' : '&none';
  }
  return document.hardware?.boards.find(board => board.boardId === boardId)?.keyBindings[keyId] ?? '&none';
}
