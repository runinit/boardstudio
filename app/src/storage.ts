import type { ProjectDoc } from '@boardstudio/v2-contracts';
import { isErgogen, modelAssetIds } from '@boardstudio/v2-ergogen';
import { bundledModel, bundledModelBytes } from './bundledModels';
import type { ArchiveRequest, ArchiveReply } from './export.worker';

const DB_NAME = 'boardstudio-v2';
const PROJECT_STORE = 'projects';
const ASSET_STORE = 'assets';
const ACTIVE_PROJECT_KEY = 'boardstudio-v2-active-project';

export function activeProjectId(fallback: string): string {
  return localStorage.getItem(ACTIVE_PROJECT_KEY) ?? fallback;
}

function openDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, 1);

    request.onupgradeneeded = () => {
      request.result.createObjectStore(PROJECT_STORE, { keyPath: 'id' });
      request.result.createObjectStore(ASSET_STORE);
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

export async function loadProject(id: string): Promise<ProjectDoc | undefined> {
  const db = await openDb();

  return new Promise((resolve, reject) => {
    const request = db.transaction(PROJECT_STORE).objectStore(PROJECT_STORE).get(id);

    request.onsuccess = () => {
      db.close();
      resolve(request.result as ProjectDoc | undefined);
    };
    request.onerror = () => {
      db.close();
      reject(request.error);
    };
  });
}

export async function listProjects(): Promise<ProjectDoc[]> {
  const db = await openDb();

  return new Promise((resolve, reject) => {
    const request = db.transaction(PROJECT_STORE).objectStore(PROJECT_STORE).getAll();
    request.onsuccess = () => {
      db.close();
      resolve(request.result as ProjectDoc[]);
    };
    request.onerror = () => {
      db.close();
      reject(request.error);
    };
  });
}

export async function saveProject(doc: ProjectDoc): Promise<void> {
  const db = await openDb();

  return new Promise((resolve, reject) => {
    const transaction = db.transaction(PROJECT_STORE, 'readwrite');

    transaction.objectStore(PROJECT_STORE).put(doc);
    transaction.oncomplete = () => {
      localStorage.setItem(ACTIVE_PROJECT_KEY, doc.id);
      db.close();
      resolve();
    };
    transaction.onerror = () => {
      db.close();
      reject(transaction.error);
    };
  });
}

export async function deleteProject(id: string): Promise<void> {
  const db = await openDb();
  return new Promise((resolve, reject) => {
    const transaction = db.transaction(PROJECT_STORE, 'readwrite');
    transaction.objectStore(PROJECT_STORE).delete(id);
    transaction.oncomplete = () => {
      if (localStorage.getItem(ACTIVE_PROJECT_KEY) === id) localStorage.removeItem(ACTIVE_PROJECT_KEY);
      db.close();
      resolve();
    };
    transaction.onabort = transaction.onerror = () => {
      db.close();
      reject(transaction.error ?? new Error('The keyboard could not be deleted.'));
    };
  });
}

/** Replace project data atomically so a failed write cannot erase the old projects. */
export async function resetLocalProjects(document: ProjectDoc): Promise<void> {
  const db = await openDb();
  await new Promise<void>((resolve, reject) => {
    const transaction = db.transaction([PROJECT_STORE, ASSET_STORE], 'readwrite');
    transaction.oncomplete = () => { db.close(); resolve(); };
    transaction.onabort = () => { db.close(); reject(transaction.error ?? new Error('Local reset was cancelled')); };
    try {
      transaction.objectStore(PROJECT_STORE).clear();
      transaction.objectStore(ASSET_STORE).clear();
      transaction.objectStore(PROJECT_STORE).put(document);
    } catch (error) {
      transaction.abort();
      reject(error);
    }
  });
  localStorage.setItem(ACTIVE_PROJECT_KEY, document.id);
  for (const key of Object.keys(localStorage)) {
    if (key.startsWith('boardstudio:v2:setup-guide:')) localStorage.removeItem(key);
  }
}

export async function saveAsset(sha256: string, bytes: Uint8Array): Promise<void> {
  const db = await openDb();

  return new Promise((resolve, reject) => {
    const transaction = db.transaction(ASSET_STORE, 'readwrite');

    transaction.objectStore(ASSET_STORE).put(bytes, sha256);
    transaction.oncomplete = () => {
      db.close();
      resolve();
    };
    transaction.onerror = () => {
      db.close();
      reject(transaction.error);
    };
  });
}

export type ArchiveTransport = {
  archive(input: Omit<ArchiveRequest, 'id'>): Promise<Omit<Extract<ArchiveReply, { kind: 'archive' }>, 'id'>>;
};

export async function loadAsset(sha256: string): Promise<Uint8Array | undefined> {
  const db = await openDb();

  return new Promise((resolve, reject) => {
    const request = db.transaction(ASSET_STORE).objectStore(ASSET_STORE).get(sha256);

    request.onsuccess = () => {
      db.close();
      resolve(request.result as Uint8Array | undefined);
    };
    request.onerror = () => {
      db.close();
      reject(request.error);
    };
  });
}

type ProjectPackOptions = {
  /** Include referenced bundled Ergogen models. Defaults to true. */
  embedUsedModels?: boolean;
};

/**
 * Pack a project for download. Local assets always travel with the document;
 * the optional setting controls copies of models from the bundled catalogue.
 */
export async function packProject(doc: ProjectDoc, options: ProjectPackOptions = {}, archiveClient: ArchiveTransport): Promise<Uint8Array> {
  const embedAssets = options.embedUsedModels !== false;
  const embeddedAssets = [...doc.assets];
  const files = new Map<string, Uint8Array>();

  if (embedAssets) {
    const bundledIds = new Set(doc.definitions.flatMap((definition) => {
      if (!isErgogen(definition.generator?.source)) return [];
      return doc.parts.filter((part) => part.definitionId === definition.id).flatMap((part) => modelAssetIds(definition, part));
    }));

    for (const definition of doc.definitions) {
      for (const model of (definition.models ?? [])) if (bundledModel(model.assetId)) bundledIds.add(model.assetId);
    }
    for (const definition of doc.moduleDefinitions ?? []) {
      for (const model of definition.models) if (bundledModel(model.assetId)) bundledIds.add(model.assetId);
      for (const part of definition.circuit?.definitions ?? []) for (const model of part.models ?? []) if (bundledModel(model.assetId)) bundledIds.add(model.assetId);
    }
    for (const assembly of doc.assemblies ?? []) for (const member of assembly.members) {
      for (const model of member.models) if (bundledModel(model.assetId)) bundledIds.add(model.assetId);
      const definition = doc.definitions.find(d => d.id === member.definitionId);
      if (definition && isErgogen(definition.generator?.source)) for (const id of modelAssetIds({ ...definition, generator: { ...definition.generator!, parameters: { ...definition.generator!.parameters, ...member.parameters } } }, { id: member.id, definitionId: definition.id, reference: member.id, pose: member.pose, side: member.side })) bundledIds.add(id);
    }
    for (const reference of doc.boardReferences ?? []) for (const id of Object.values(reference.modelAssets)) if (bundledModel(id)) bundledIds.add(id);
    for (const id of bundledIds) {
      if (embeddedAssets.some((asset) => asset.id === id)) continue;
      const bundled = bundledModel(id);
      if (!bundled) throw new Error(`Bundled Ergogen model is unavailable: ${id}`);
      const bytes = await bundledModelBytes(id);
      const digest = await crypto.subtle.digest('SHA-256', new Uint8Array(bytes));
      const sha256 = Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, '0')).join('');
      embeddedAssets.push({ id, name: bundled.filename, mediaType: /\.wrl$/i.test(bundled.filename) ? 'model/vrml' : /\.stl$/i.test(bundled.filename) ? 'model/stl' : 'model/step', sha256, source: 'bundled Ergogen library' });
      files.set(`assets/${sha256}`, bytes);
    }
  }

  const packedDoc = embeddedAssets.length === doc.assets.length ? doc : { ...doc, assets: embeddedAssets };

  for (const asset of doc.assets) {
    const bytes = await loadAsset(asset.sha256);

    if (!bytes) {
      throw new Error(`Missing asset: ${asset.name}`);
    }

    files.set(`assets/${asset.sha256}`, bytes);
  }
  const entries = [...files].map(([path, bytes]) => ({ path, bytes }));
  const result = await archiveClient.archive({ kind: 'archive', request: { kind: 'pack-project', projectJson: JSON.stringify(packedDoc), archiveJson: JSON.stringify({ embedUsedModels: embedAssets }), assets: entries.map((entry, bufferIndex) => ({ path: entry.path, bufferIndex })) }, buffers: entries.map((entry) => entry.bytes) });
  if (result.kind !== 'archive' || result.reply.kind !== 'packed') throw new Error('Expected packed project archive');
  return result.reply.bytes;
}

export async function unpackProject(bytes: Uint8Array, archiveClient: ArchiveTransport): Promise<ProjectDoc> {
  const result = await archiveClient.archive({ kind: 'archive', request: { kind: 'unpack-project' }, buffers: [bytes] });
  if (result.kind !== 'archive' || result.reply.kind !== 'unpacked') throw new Error('Expected unpacked project archive');
  const doc = JSON.parse(result.reply.projectJson) as ProjectDoc;
  if (doc.format !== 'boardstudio/v2' || !Array.isArray(doc.parts) || !Array.isArray(doc.boards)) {
    throw new Error('Unsupported project format');
  }

  const verified = new Map(result.reply.assets.map((asset) => [asset.sha256, asset.bytes]));
  for (const asset of doc.assets) {
    if (!verified.has(asset.sha256)) throw new Error(`Archive has no asset: ${asset.name}`);
  }
  if (verified.size === 0) return doc;
  const db = await openDb();
  try {
    await new Promise<void>((resolve, reject) => {
      const transaction = db.transaction(ASSET_STORE, 'readwrite');
      transaction.oncomplete = () => resolve();
      transaction.onabort = () => reject(transaction.error ?? new Error('Asset import transaction was aborted'));
      transaction.onerror = () => reject(transaction.error ?? new Error('Asset import transaction failed'));
      try {
        for (const [hash, bytes] of verified) transaction.objectStore(ASSET_STORE).put(bytes, hash);
      } catch (error) {
        transaction.abort();
        reject(error);
      }
    });
  } finally {
    db.close();
  }

  return doc;
}
