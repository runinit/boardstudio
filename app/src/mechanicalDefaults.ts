import type { MechanicalConfiguration, MechanicalStabilizerOverride, ProjectDoc } from '@boardstudio/v2-contracts';
import { inferSwitchFamily } from './mechanicalPresets';

export function stabilizerCandidates(document: ProjectDoc, boardId: string) {
  const board = document.boards.find(board => board.id === boardId);
  const existingStabilizers = document.parts.filter(part => board?.partIds.includes(part.id) &&
    document.definitions.find(definition => definition.id === part.definitionId)?.kicadSource?.source.includes('(footprint "STAB_MX_'));
  return document.parts.flatMap(part => {
    if (!board?.partIds.includes(part.id)) return [];
    const definition = document.definitions.find(definition => definition.id === part.definitionId);
    const size = part.keycap ?? definition?.keycap;
    if (!size || Math.max(size.x, size.y) < 37 || inferSwitchFamily(definition, part.generatorParameters) !== 'mx') return [];
    // Imported PCB stabilizers already contribute their own plate and foam cutouts.
    if (existingStabilizers.some(stabilizer => Math.hypot(stabilizer.pose.at.x - part.pose.at.x, stabilizer.pose.at.y - part.pose.at.y) < 0.01)) return [];
    const keyUnits = Math.round((Math.max(size.x, size.y) + 1) / 19.05 * 4) / 4;
    const units = keyUnits <= 2.75 ? 2 : keyUnits;
    const stabilizer: MechanicalStabilizerOverride = { partId: part.id, kind: 'pcb-mount', units, rotation: size.y > size.x ? 90 : 0 };
    return [{ part, keyUnits, stabilizer }];
  });
}

export function mechanicalDefaults(document: ProjectDoc, config: MechanicalConfiguration, flipped = false): MechanicalConfiguration {
  const parts = document.parts.filter(part => document.boards.find(board => board.id === config.boardId)?.partIds.includes(part.id));
  const xs = parts.map(part => part.pose.at.x), ys = parts.map(part => part.pose.at.y);
  const at = { x: xs.length ? (Math.min(...xs) + Math.max(...xs)) / 2 * (flipped ? -1 : 1) : 0, y: ys.length ? (Math.min(...ys) + Math.max(...ys)) / 2 : 0 };
  const battery = config.battery ?? (document.hardware?.transport === 'wireless' ? { size: { x: 30, y: 20, z: 6 }, at, cableExit: { ...at, x: at.x + 17 }, cableWidth: 2 } : undefined);
  const explicit = new Map(config.stabilizers?.map(stabilizer => [stabilizer.partId, stabilizer]));
  const stabilizers = stabilizerCandidates(document, config.boardId).map(({ stabilizer }) => {
    const saved = explicit.get(stabilizer.partId);
    return saved ? { ...saved, units: stabilizer.units, rotation: stabilizer.rotation } : stabilizer;
  });
  return { ...config, battery, batteryHeight: battery?.size.z ?? config.batteryHeight, stabilizers };
}
