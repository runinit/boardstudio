export type MatrixPresetId = 'mx-solder' | 'mx-hotswap' | 'choc-solder' | 'choc-hotswap' | 'mx-rgb' | 'choc-rgb' | 'mx-hotswap-rgb' | 'choc-hotswap-rgb';
export const matrixPresetDefinitions: Record<MatrixPresetId, { definitionId: string; led: boolean; hotswap: boolean; family: 'mx' | 'choc' }> = {
  'mx-solder': { definitionId: 'ergogen:ceoloide/switch_mx', led: false, hotswap: false, family: 'mx' },
  'mx-hotswap': { definitionId: 'ergogen:ceoloide/switch_mx', led: false, hotswap: true, family: 'mx' },
  'choc-solder': { definitionId: 'ergogen:ceoloide/switch_choc_v1_v2', led: false, hotswap: false, family: 'choc' },
  'choc-hotswap': { definitionId: 'ergogen:ceoloide/switch_choc_v1_v2', led: false, hotswap: true, family: 'choc' },
  'mx-rgb': { definitionId: 'ergogen:ceoloide/switch_mx', led: true, hotswap: false, family: 'mx' },
  'choc-rgb': { definitionId: 'ergogen:ceoloide/switch_choc_v1_v2', led: true, hotswap: false, family: 'choc' },
  'mx-hotswap-rgb': { definitionId: 'ergogen:ceoloide/switch_mx', led: true, hotswap: true, family: 'mx' },
  'choc-hotswap-rgb': { definitionId: 'ergogen:ceoloide/switch_choc_v1_v2', led: true, hotswap: true, family: 'choc' },
};
