import type { PartDefinition } from '@boardstudio/v2-contracts';

/** Library filtering is advisory; Rust validates the selected input at commit. */
export function matrixInputAvailable(definition: PartDefinition): boolean {
  const press = definition.inputProfile ? definition.inputProfile.press : definition.matrixTerminals
    ? { ...definition.matrixTerminals, independent: true }
    : definition.generator?.source === 'ceoloide/rotary_encoder_ec11_ec12'
      ? { row: 'S1', column: 'S2', independent: true } : undefined;
  if (!press) return !definition.inputProfile && definition.pads.some(pad => pad.id === 'one') && definition.pads.some(pad => pad.id === 'two');
  const pads = (terminal: string) => definition.terminals?.[terminal] ?? definition.pads.filter(pad => pad.id === terminal || pad.number === terminal).map(pad => pad.id);
  const row = pads(press.row), column = pads(press.column);
  const rotary = definition.inputProfile?.rotary;
  const rotation = rotary ? [rotary.a, rotary.b, rotary.common].flatMap(pads) : [];
  return press.independent && row.length > 0 && column.length > 0 && row.every(id => !column.includes(id))
    && [...row, ...column].every(id => definition.pads.some(pad => pad.id === id) && !rotation.includes(id));
}
