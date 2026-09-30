import { demoProject } from '../src/demo';

// Editor scenarios start with an existing keyboard rather than relying on
// first run to create one. First-run scenarios explicitly use empty storage.
export function savedProjectState(baseURL: string) {
  const document = demoProject();
  return {
    cookies: [],
    origins: [{
      origin: new URL(baseURL).origin,
      localStorage: [{ name: 'boardstudio-v2-active-project', value: document.id }],
      indexedDB: [{
        name: 'boardstudio-v2', version: 1,
        stores: [
          { name: 'projects', keyPath: 'id', autoIncrement: false, indexes: [], records: [{ value: document }] },
          { name: 'assets', autoIncrement: false, indexes: [], records: [] },
        ],
      }],
    }],
  };
}
