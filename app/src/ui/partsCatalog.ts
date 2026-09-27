import type { PartDefinition } from '@boardstudio/v2-contracts';

const preferredNames: Record<string, string> = {
  'ergogen:ceoloide/switch_mx': 'MX switch',
  'ergogen:ceoloide/switch_choc_v1_v2': 'Choc V1 / V2 switch',
  'ergogen:ceoloide/switch_gateron_ks27_ks33': 'Gateron KS27 / KS33 switch',
  'ergogen:ceoloide/diode_tht_sod123': 'Matrix diode (SOD-123 / THT)',
  'ergogen:ceoloide/led_sk6812mini-e': 'SK6812 MINI-E',
};

export function partCatalogLabel(definition: PartDefinition): string {
  return preferredNames[definition.id] ?? definition.name;
}

export function partCatalogSearchText(definition: PartDefinition): string {
  const aliases = definition.id === 'ergogen:ceoloide/led_sk6812mini-e' ? 'RGB LED reverse mount' : definition.kind === 'switch' ? 'solder hotswap' : '';
  return `${partCatalogLabel(definition)} ${definition.name} ${definition.kind} ${definition.generator?.source ?? ''} ${aliases}`;
}

// These are placement snapshots, not additional library products. Keep them
// selectable where already assigned to the current project.
const isAssemblySnapshot = (definition: PartDefinition) => !definition.kicadSource && /^(?:assembly-.+|[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})\/definition\//i.test(definition.id);

export function partChoices(definitions: PartDefinition[], currentId?: string): PartDefinition[] {
  return definitions.filter(definition => definition.generator?.source !== 'infused-kim/nice_nano_pretty' && (definition.id === currentId || !isAssemblySnapshot(definition)));
}
