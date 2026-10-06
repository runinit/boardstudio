/** All document coordinates are millimetres in a right-handed, Y-up frame. */
export type * from './generated/index';
export { keycapDefaults } from './generated/keycapDefaults.ts';

import type { OutlineSettings, ProjectDoc } from './generated/index';
import type { CoreRequest as RustCoreRequest } from './generated/CoreRequest';
import type { CoreReply as RustCoreReplyType } from './generated/CoreReply';

export type Id = string;

export type CoreRequest = RustCoreRequest & { diagnostics?: true };
export type CoreTiming = { wasmMs: number; parseMs: number };
type WithTiming<T> = T extends unknown ? T & { timing?: CoreTiming } : never;
export type CoreReply = WithTiming<RustCoreReplyType>;

export type CaseResult = {
  revision: number;
  step: Uint8Array;
  bodies?: import('./generated/CaseBodyMesh').CaseBodyMesh[];
  mesh: { positions: Float32Array; normals: Float32Array };
};

export const defaultOutlineSettings = {
  corners: 'fillet',
  size: 2,
  bridgeWidth: 10,
} satisfies OutlineSettings;

export const emptyProject = (id: Id, name: string): ProjectDoc => ({
  format: 'boardstudio/v2',
  formatVersion: 2,
  id,
  name,
  revision: 0,
  parameters: {},
  definitions: [],
  parts: [],
  matrices: [],
  constraints: [],
  nets: [],
  outline: [],
  boards: [],
  caseBodies: [],
  materials: [],
  assets: [],
  scripts: [],
});
