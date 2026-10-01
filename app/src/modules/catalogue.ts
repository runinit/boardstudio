import { useEffect, useMemo, useState } from 'react';
import type { ModuleDefinition } from '../../../contracts/src/index';

type CatalogueEntry = { row: string; definition: ModuleDefinition };
let pending: Promise<CatalogueEntry[]> | undefined;
function loadCatalogue() {
  return pending ??= import('./imported-modules.json').then(module => module.default.modules as CatalogueEntry[])
    .catch(error => { pending = undefined; throw error; });
}

/** Source snapshots are loaded when browsing Parts; project snapshots take precedence. */
export function useModuleCatalogue(active: boolean, snapshots: ModuleDefinition[] = []) {
  const [entries, setEntries] = useState<CatalogueEntry[]>([]);
  const [error, setError] = useState('');
  useEffect(() => {
    if (!active || entries.length) return;
    let cancelled = false;
    void loadCatalogue().then(value => { if (!cancelled) { setEntries(value); setError(''); } }, failure => {
      if (!cancelled) setError(String(failure instanceof Error ? failure.message : failure));
    });
    return () => { cancelled = true; };
  }, [active, entries.length]);
  const definitions = useMemo(() => {
    const merged = new Map(entries.map(entry => [entry.definition.id, entry.definition]));
    for (const snapshot of snapshots) merged.set(snapshot.id, snapshot);
    return [...merged.values()];
  }, [entries, snapshots]);
  return { definitions, pending: active && !entries.length && !error, error };
}
