import { matrixPresetDefinitions, type MatrixPresetId } from './assemblyCatalog';
import type {
  AssemblyDefinition,
  AssemblyMember,
  PartDefinition,
} from '@boardstudio/v2-contracts';
export type AssemblyConstruction = 'single-sided' | 'reversible';
export type SwitchOrientation = 'south' | 'north';
export function assemblyPreset(
  id: string,
  definitions: PartDefinition[],
  orientation: SwitchOrientation = 'south',
  construction: AssemblyConstruction = 'single-sided',
): AssemblyDefinition {
  const preset = matrixPresetDefinitions[id as MatrixPresetId];
  if (!preset) throw new Error(`Unknown assembly preset: ${id}`);
  const source = preset.definitionId.slice('ergogen:'.length);
  const main = definitions.find((d) => d.generator?.source === source);
  if (!main) throw new Error(`Missing switch footprint: ${source}`);
  const diode = definitions.find(
    (d) => d.generator?.source === 'ceoloide/diode_tht_sod123',
  )!;
  const members: AssemblyMember[] = [
    {
      id: 'switch',
      definitionId: main.id,
      pose: { at: { x: 0, y: 0 }, rotation: 0 },
      side: 'front',
      models: [],
      parameters: {
        hotswap: preset.hotswap,
        solder: !preset.hotswap,
        reversible: construction === 'reversible',
        // Generator side names the socket, opposite the switch housing.
        side: 'B',
        include_keycap: true,
        ...(preset.family === 'choc' ? { choc_v1_support: true, choc_v2_support: false, include_choc_v1_led_cutout_marks: true } : {}),
      },
    },
  ];
  if (diode)
    members.push({
      id: 'diode',
      definitionId: diode.id,
      pose: { at: { x: 7.4, y: -1.5 }, rotation: 90 },
      side: 'back',
      models: [],
      parameters: { side: 'B', reversible: construction === 'reversible', include_tht: false },
    });
  if (preset.led) {
    const led =
      definitions.find(
        (d) => d.generator?.source === 'ceoloide/led_sk6812mini-e',
      );
    if (!led) throw new Error('Missing SK6812 MINI-E footprint');
    if (led)
      members.push({
        id: 'led',
        definitionId: led.id,
        // Ceoloide's switch sockets/pins are north; the LED cavity is south.
        // Choc PG1350 (also Infused Kim's choc.js): 4.7 mm; MX: 4.75 mm.
        pose: { at: { x: 0, y: preset.family === 'choc' ? -4.7 : -4.75 }, rotation: 180 },
        side: 'back',
        models: [],
        parameters: { side: 'B', reverse_mount: true, reversible: construction === 'reversible' },
      });
  }
  if (orientation === 'north') {
    for (const member of members) {
      member.pose = { at: { x: -member.pose.at.x || 0, y: -member.pose.at.y || 0 }, rotation: (member.pose.rotation + 180) % 360 };
    }
  }
  return {
    id: crypto.randomUUID(),
    name: id.split('-').join(' ').toUpperCase(),
    members,
  };
}
