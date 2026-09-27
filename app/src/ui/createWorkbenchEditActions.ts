import type { Constraint, Net, Part, PartDefinition, ProjectDoc } from '@boardstudio/v2-contracts';

export function assignNetPins(nets: Net[], partId: string, padIds: string[], netId: string): Net[] {
  const padSet = new Set(padIds);
  return nets.map((net) => {
    const next = { ...net, pins: net.pins.filter((pin) => pin.partId !== partId || !padSet.has(pin.padId)) };
    return next.id === netId ? { ...next, pins: [...next.pins, ...padIds.map((padId) => ({ partId, padId }))] } : next;
  });
}

export function replacePartDefinition(document: ProjectDoc, part: Part, definition: PartDefinition): ProjectDoc {
  const validPads = new Set([...definition.pads.map((pad) => pad.id), ...Object.values(definition.terminals ?? {}).flat()]);
  const nets = document.nets.map((net) => ({ ...net, pins: net.pins.filter((pin) => pin.partId !== part.id || validPads.has(pin.padId)) }));
  return { ...document, parts: document.parts.map((item) => item.id === part.id ? { ...item, definitionId: definition.id } : item), nets };
}

export function buildOffsetConstraint(id: string, sourcePartId: string, targetPartId: string, x: string, y: string, rotation: string): Constraint | undefined {
  const values = [x, y, rotation].map(Number);
  if ([x, y, rotation].some((value) => !value.trim()) || !values.every(Number.isFinite)) return undefined;
  return { id, kind: 'offset', sourcePartId, targetPartId, offset: { x: values[0], y: values[1] }, rotation: values[2] };
}

export function buildMirrorConstraint(id: string, sourcePartId: string, targetPartId: string, axis: 'vertical' | 'horizontal', coordinate: string): Constraint | undefined {
  const value = Number(coordinate);
  if (!coordinate.trim() || !Number.isFinite(value)) return undefined;
  return { id, kind: 'mirror', sourcePartId, targetPartId, axis, coordinate: value };
}
