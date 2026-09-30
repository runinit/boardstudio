import type { ElectricalBoardConfiguration, KeycapBoardSettings, KeycapConfiguration, KeycapKeySettings, KeycapMatrixSettings, ProjectDoc } from '@boardstudio/v2-contracts';

export const defaultKeycapBoard: KeycapBoardSettings = { color: '#e8e4dc', legendColor: '#202630', clearance: 0.5 };
export const defaultKeycapMatrix: KeycapMatrixSettings = { profile: null, mount: null, firstRow: 1, wallThickness: 1.2 };
export const defaultKeycapKey: KeycapKeySettings = { mount: null, legend: null, color: null, row: null, units: null, profile: null };
export function keycapConfiguration(document: ProjectDoc): KeycapConfiguration {
  return document.keycaps ?? { boards: {}, matrices: {}, keys: {} };
}
export function keyBinding(document: ProjectDoc, boardId: string, keyId: string): string {
  return document.hardware?.boards.find(board => board.boardId === boardId)?.keyBindings[keyId] ?? '&none';
}
export function withKeyBinding(document: ProjectDoc, boardId: string, keyId: string, binding: string): ProjectDoc {
  const hardware = document.hardware ?? { topology: 'unibody', transport: 'none', instances: [], boards: [], sharedConstruction: null };
  const current: ElectricalBoardConfiguration = hardware.boards.find(board => board.boardId === boardId) ?? {
    boardId, controllerPartId: null, mode: 'matrix', locks: {}, assignments: {}, keyBindings: {}, jumperStates: {}, protectedHandoff: null,
  };
  const board = { ...current, keyBindings: { ...current.keyBindings, [keyId]: binding } };
  return { ...document, hardware: { ...hardware, boards: [...hardware.boards.filter(board => board.boardId !== boardId), board] } };
}
