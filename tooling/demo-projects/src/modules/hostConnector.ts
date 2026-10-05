import type { PartDefinition } from '@boardstudio/v2-contracts';

const MODEL_ASSET = 'ergogen:model:vik/sadekbaroudi-vik/kicad/3dmodels/vik-connector-horizontal.stp';

/** Clone the source-derived host footprint already present in the VIK module snapshots. */
export async function loadVikHostConnectorDefinition(): Promise<PartDefinition> {
  const snapshot = await import('../../../../catalogue/modules/imported-modules.json');
  const definition = (snapshot.default.modules as Array<{ definition: { circuit?: { definitions?: PartDefinition[] } } }>)
    .flatMap(entry => entry.definition.circuit?.definitions ?? [])
    .find(candidate => candidate.hardwareProfile?.vikRole === 'host' && candidate.name.toLowerCase().includes('horizontal'));
  if (!definition) throw new Error('The source catalogue has no horizontal VIK host connector footprint');
  const clone = structuredClone(definition);
  clone.id = 'vik:source:horizontal-host-connector';
  clone.name = 'VIK horizontal host connector';
  clone.models = [{ assetId: MODEL_ASSET, offset: { x: -2.75, y: 2.3, z: 0 }, rotation: { x: 0, y: 0, z: 0 }, scale: { x: 1, y: 1, z: 1 } }];
  if (clone.kicadSource) clone.kicadSource.source = withoutEmbeddedModel(clone.kicadSource.source);
  return clone;
}

function withoutEmbeddedModel(source: string): string {
  const marker = '(model "../../kicad/3dmodels/vik-connector-horizontal.stp"';
  const start = source.indexOf(marker);
  if (start < 0) return source;
  let depth = 0;
  let quoted = false;
  let escaped = false;
  for (let index = start; index < source.length; index += 1) {
    const character = source[index];
    if (quoted) {
      if (escaped) escaped = false;
      else if (character === '\\') escaped = true;
      else if (character === '"') quoted = false;
      continue;
    }
    if (character === '"') quoted = true;
    else if (character === '(') depth += 1;
    else if (character === ')') {
      depth -= 1;
      if (depth === 0) return `${source.slice(0, start)}${source.slice(index + 1)}`;
    }
  }
  throw new Error('The source VIK host connector has a malformed embedded model form');
}
