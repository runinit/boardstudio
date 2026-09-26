import type { Net, PartDefinition, ProjectDoc } from '@boardstudio/v2-contracts';

type RemapResult = { ok: true; nets: Net[] } | { ok: false; error: string };

/** Validate every placed instance before constructing replacement nets. */
export function remapDefinitionNets(document: ProjectDoc, original: PartDefinition, next: PartDefinition): RemapResult {
  const instances = document.parts.filter((part) => part.definitionId === next.id);
  const assignments: { partId: string; oldPads: string[]; newPads: string[]; netId: string }[] = [];

  for (const part of instances) {
    for (const [terminal, oldPads] of Object.entries(original.terminals ?? {})) {
      const netIds = [...new Set(document.nets.filter((net) => net.pins.some((pin) => pin.partId === part.id && oldPads.includes(pin.padId))).map((net) => net.id))];
      if (!netIds.length) continue;
      const newPads = next.terminals?.[terminal];
      if (netIds.length > 1 || !newPads?.length) {
        return { ok: false, error: `Cannot apply these generator settings: assigned ${terminal} terminal pads would be lost or are split across nets.` };
      }
      assignments.push({ partId: part.id, oldPads, newPads, netId: netIds[0] });
    }
  }

  const oldPadsByPart = new Map<string, Set<string>>();
  for (const { partId, oldPads } of assignments) {
    const pads = oldPadsByPart.get(partId) ?? new Set<string>();
    oldPads.forEach((pad) => pads.add(pad));
    oldPadsByPart.set(partId, pads);
  }
  const retained = (pin: Net['pins'][number]) => !oldPadsByPart.get(pin.partId)?.has(pin.padId);
  const pinKey = (partId: string, padId: string) => JSON.stringify([partId, padId]);
  const destinationNets = new Map<string, Set<string>>();
  for (const net of document.nets) {
    for (const pin of net.pins.filter(retained)) {
      const key = pinKey(pin.partId, pin.padId);
      const owners = destinationNets.get(key) ?? new Set<string>();
      owners.add(net.id);
      destinationNets.set(key, owners);
    }
  }
  // Old terminal pads are released together, so exchanging pads remains valid.
  for (const { partId, newPads, netId } of assignments) {
    for (const padId of newPads) {
      const key = pinKey(partId, padId);
      const owners = destinationNets.get(key);
      if (owners && [...owners].some((owner) => owner !== netId)) {
        return { ok: false, error: `Cannot apply these generator settings: destination pad ${padId} would be assigned to multiple nets.` };
      }
      destinationNets.set(key, new Set([netId]));
    }
  }
  const nets = document.nets.map((net) => ({
    ...net,
    pins: net.pins.filter(retained),
  }));
  const byId = new Map(nets.map((net) => [net.id, net]));
  for (const { partId, newPads, netId } of assignments) {
    const net = byId.get(netId)!;
    for (const padId of newPads) {
      if (!net.pins.some((pin) => pin.partId === partId && pin.padId === padId)) net.pins.push({ partId, padId });
    }
  }
  return { ok: true, nets };
}
