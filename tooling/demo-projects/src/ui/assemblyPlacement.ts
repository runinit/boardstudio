import { emptyProject, type AssemblyDefinition, type PartDefinition, type ProjectDoc, type Vec2 } from '@boardstudio/v2-contracts';
import { normalizeDefinition } from '@boardstudio/v2-ergogen';

/** Explicit placement snapshots definitions so later preset edits do not move placed hardware. */
function placeAssembly(
  document: ProjectDoc,
  assembly: AssemblyDefinition,
  definitions: PartDefinition[],
  boardId: string,
  origin: Vec2,
  id: string,
): ProjectDoc {
  const next = structuredClone(document);
  const board = next.boards.find((b) => b.id === boardId);
  if (!board) throw new Error('Select a board before placing an assembly');
  for (const member of assembly.members) {
    const source = definitions.find((d) => d.id === member.definitionId);
    if (member.definitionId && !source)
      throw new Error(`Missing component definition: ${member.definitionId}`);
    const definition: PartDefinition = source
      ? structuredClone(source)
      : {
          id: '',
          name: 'Visual model',
          kind: 'custom',
          pads: [],
          courtyard: [],
        };
    definition.id = `${id}/definition/${member.id}`;
    if (definition.generator && member.parameters)
      definition.generator.parameters = {
        ...definition.generator.parameters,
        ...member.parameters,
      };
    if (member.models.length || member.modelMode === 'custom') {
      definition.models = structuredClone(member.models);
    }
    next.definitions.push(normalizeDefinition(definition));
    const part = {
      id: `${id}/${member.id}`,
      definitionId: definition.id,
      reference: `${assembly.name} ${member.id}`,
      pose: {
        at: { x: origin.x + member.pose.at.x, y: origin.y + member.pose.at.y },
        rotation: member.pose.rotation,
      },
      side: member.side,
      properties: { assemblyId: id, visualOnly: !member.definitionId },
      outline: !member.definitionId ? { excluded: true } : undefined,
    };
    next.parts.push(part);
    board.partIds.push(part.id);
  }
  return next;
}

/** Uses the existing atomic matrix operation, retaining key identities and layout recipes. */
export function matrixWithAssembly(
  matrix: import('@boardstudio/v2-contracts').Matrix,
  assembly: AssemblyDefinition,
  definitions: PartDefinition[],
  revision: number,
) {
  const primary = assembly.members[0];
  if (!primary?.definitionId)
    throw new Error('A matrix assembly needs a footprint as its first member');
  if (
    primary.pose.at.x ||
    primary.pose.at.y ||
    primary.side !== 'front'
  )
    throw new Error(
      'For matrix placement, keep the first member at the origin on the front',
    );
  const seed = emptyProject('assembly', 'Assembly');
  seed.boards = [
    {
      id: 'board',
      name: 'Board',
      thickness: 1.6,
      partIds: [],
      outlineIds: [],
      netIds: [],
    },
  ];
  const placed = placeAssembly(
    seed,
    assembly,
    definitions,
    'board',
    { x: 0, y: 0 },
    `assembly-${assembly.id}-${matrix.id}-${revision}`,
  );
  const previous = new Map(
    matrix.cells?.map((cell) => [`${cell.row}:${cell.column}`, cell]),
  );
  const cells = Array.from(
    { length: matrix.rows * matrix.columns },
    (_, index) => {
      const row = Math.floor(index / matrix.columns),
        column = index % matrix.columns;
      const old = previous.get(`${row}:${column}`);
      return {
        ...old,
        row,
        column,
        enabled: old?.enabled ?? true,
        definitionId: placed.definitions[0].id,
        rotation: (old?.rotation ?? 0) + primary.pose.rotation,

        assembliesLocal: true,
        assemblies: assembly.members
          .slice(1)
          .map((member, i) => ({
            id: member.id,
            definitionId: placed.definitions[i + 1].id,
            offset: (() => {
              const angle = -primary.pose.rotation * Math.PI / 180;
              return { x: member.pose.at.x * Math.cos(angle) - member.pose.at.y * Math.sin(angle), y: member.pose.at.x * Math.sin(angle) + member.pose.at.y * Math.cos(angle) };
            })(),
            rotation: member.pose.rotation - primary.pose.rotation,
            side: member.side,
          })),
      };
    },
  );
  return {
    matrix: {
      ...matrix,
      definitionId: placed.definitions[0].id,

      cells,
    },
    definitions: placed.definitions,
  };
}
