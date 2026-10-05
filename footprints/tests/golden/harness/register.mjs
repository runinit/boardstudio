import { registerHooks } from 'node:module';
import { readFileSync, existsSync } from 'node:fs';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = fileURLToPath(new URL('../../../../', import.meta.url));
const aliases = {
  '@boardstudio/v2-contracts': 'contracts/src/index.ts',
  '@boardstudio/v2-ergogen': 'ergogen/src/index.ts',
};

registerHooks({
  resolve(specifier, context, next) {
    if (aliases[specifier]) return { url: pathToFileURL(root + aliases[specifier]).href, shortCircuit: true };
    if (/^\.\.?\//u.test(specifier) && context.parentURL) {
      const base = fileURLToPath(new URL(specifier, context.parentURL));
      for (const suffix of ['', '.ts', '.mjs']) {
        if (/\.[a-z]+$/u.test(base + suffix) && existsSync(base + suffix) && !existsSync(base + suffix + '/')) {
          return { url: pathToFileURL(base + suffix).href, shortCircuit: true };
        }
      }
    }
    return next(specifier, context);
  },
  load(url, context, next) {
    if (url.endsWith('.json')) {
      return { format: 'json', source: readFileSync(fileURLToPath(url), 'utf8'), shortCircuit: true };
    }
    return next(url, context);
  },
});
