import type { Part, ProjectDoc } from '@boardstudio/v2-contracts';
import { catalogue, normalizeDefinition } from '@boardstudio/v2-ergogen';

const mountingHole = catalogue().find(definition => definition.generator?.source === 'ceoloide/mounting_hole_npth')!;

/** Closure bosses crossing the PCB need real clearance holes in the exported board. */
export function withClosureClearance(document: ProjectDoc): ProjectDoc {
  const prefix = 'case-closure/';
  const old = new Set(document.parts.filter(part => part.id.startsWith(prefix)).map(part => part.id));
  const definitionPrefix = 'assembly-closure/definition/';
  const configurations = [
    ...(document.mechanical ? [{ configuration: document.mechanical, flipped: false }] : []),
    ...(document.hardware?.instances.flatMap(instance => instance.mechanical ? [{ configuration: instance.mechanical, flipped: instance.flipped }] : []) ?? []),
  ];
  const holes = new Map<string, { boardId: string; at: { x: number; y: number }; diameter: number }>();
  for (const { configuration, flipped } of configurations) {
    for (const mount of configuration.closureMounts ?? []) {
      const diameter = mount.kind === 'boss' ? (mount.bossDiameter ?? mount.holeDiameter) + 2 * configuration.clearance : mount.holeDiameter;
      const at = { x: mount.at.x * (flipped ? -1 : 1), y: mount.at.y };
      const key = `${configuration.boardId}/${at.x.toFixed(5)}/${at.y.toFixed(5)}`;
      const previous = holes.get(key);
      holes.set(key, { boardId: configuration.boardId, at, diameter: Math.max(diameter, previous?.diameter ?? 0) });
    }
  }
  const entries = [...holes.entries()];
  const definitions = entries.map(([key, hole]) => normalizeDefinition({ ...mountingHole, id: `${definitionPrefix}${key}`, generator: { ...mountingHole.generator!, parameters: { hole_drill: String(hole.diameter), hole_size: String(hole.diameter) } } }));
  const nextReference = document.parts.filter(part => !old.has(part.id)).reduce((max, part) => Math.max(max, Number(/^MH(\d+)$/.exec(part.reference)?.[1] ?? 0)), 0) + 1;
  const parts: Part[] = entries.map(([key, hole], index) => ({ id: `${prefix}${key}`, definitionId: definitions[index].id, reference: `MH${nextReference + index}`,
    pose: { at: hole.at, rotation: 0 }, side: 'front', outline: { excluded: true },
  }));
  return { ...document, definitions: [...document.definitions.filter(definition => !definition.id.startsWith(definitionPrefix)), ...definitions],
    parts: [...document.parts.filter(part => !old.has(part.id)), ...parts],
    layouts: document.layouts?.map(layout => ({ ...layout, partIds: layout.partIds.filter(id => !old.has(id)) })),
    boards: document.boards.map(board => ({ ...board, partIds: [...board.partIds.filter(id => !old.has(id)), ...parts.filter((_, index) => entries[index][1].boardId === board.id).map(part => part.id)] })),
  };
}
