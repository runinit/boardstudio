import type { Matrix, PartDefinition, Vec2 } from '@boardstudio/v2-contracts';
import type { PairPlacement } from './MirroredPairSetup';

export type PlacementState =
  | { kind: 'idle' }
  | { kind: 'matrix-setup' }
  | { kind: 'pair-setup' }
  | { kind: 'part'; definition: PartDefinition; layoutId: string; point: Vec2 }
  | { kind: 'matrix'; matrix: Matrix; definitions: PartDefinition[] }
  | { kind: 'mirrored-pair'; matrix: Matrix; definitions: PartDefinition[]; pair: PairPlacement };

export function movePlacement(state: PlacementState, point: Vec2): PlacementState {
  if (state.kind === 'part') return { ...state, point };
  if (state.kind === 'matrix' || state.kind === 'mirrored-pair') return { ...state, matrix: { ...state.matrix, origin: point } };
  return state;
}
