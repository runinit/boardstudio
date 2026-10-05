import type { PartDefinition } from '@boardstudio/v2-contracts';
import imported from '../../../../catalogue/parts/imported-parts.json';

/** Return independent definitions so project edits never mutate the library. */
export function importedPartDefinitions(): PartDefinition[] {
  return structuredClone(imported.parts.map(part => part.definition)) as PartDefinition[];
}
