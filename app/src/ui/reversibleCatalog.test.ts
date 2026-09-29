import { it, expect } from 'vitest';
import { catalogue } from '@boardstudio/v2-ergogen';
import { constructionDefinition } from '../projectConstruction';
import { generatorParameters } from './generatorSettings';
it('every reversible catalog part supports its default reversible configuration', () => {
  for (const definition of catalogue()) {
    if (!generatorParameters(definition).reversible) continue;
    expect(() => constructionDefinition(definition, true), definition.id).not.toThrow();
  }
});
