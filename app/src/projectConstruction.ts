import type { PartDefinition, ProjectDoc } from '@boardstudio/v2-contracts';
import { normalizeDefinition, parameters, isErgogen } from '@boardstudio/v2-ergogen';

export function reversibleLayout(document: ProjectDoc): boolean {
  return typeof document.parameters.reversibleLayout === 'boolean' ? document.parameters.reversibleLayout
    : document.hardware?.instances.some(instance => instance.flipped) ?? false;
}

/** Gateron low-profile sockets overlap when mirrored; its reversible variant uses solder pads. */
export function constructionDefinition(definition: PartDefinition, reversible: boolean): PartDefinition {
  if (!definition.generator || !isErgogen(definition.generator.source) || !parameters(definition.generator.source).reversible) return definition;
  const solderOnly = reversible && definition.generator.source === 'ceoloide/switch_gateron_ks27_ks33';
  return normalizeDefinition({ ...definition, generator: { ...definition.generator, parameters: { ...definition.generator.parameters, reversible, ...(solderOnly ? { hotswap: false, solder: true } : {}) } } });
}

/** Keep physical halves and the existing generated footprints in one construction mode. */
export function withReversibleLayout(document: ProjectDoc, reversible: boolean): ProjectDoc {
  const definitions = document.definitions.map(definition => constructionDefinition(definition, reversible));
  const supportsReversible = new Set(definitions.filter(definition => definition.generator && isErgogen(definition.generator.source) && parameters(definition.generator.source).reversible).map(definition => definition.id));
  return { ...document, parameters: { ...document.parameters, reversibleLayout: reversible }, definitions,
    parts: document.parts.map(part => supportsReversible.has(part.definitionId) && part.generatorParameters?.reversible !== undefined
      ? { ...part, generatorParameters: { ...part.generatorParameters, reversible } } : part),
    hardware: document.hardware && { ...document.hardware, instances: document.hardware.instances.map(instance => ({ ...instance, flipped: reversible && instance.half === 'right' })) },
  };
}
