import { useCallback, useMemo, useState } from 'react';

export type CaseDisplay = { hidden: string[]; colors: Record<string, string> };
const empty: CaseDisplay = { hidden: [], colors: {} };
export const displayIds = (id: string) => id === 'gaskets' ? ['Gaskets'] : id === 'pcb' ? ['PCB', 'Models', 'Keycaps', 'Copper', 'Mask', 'Silkscreen']
  : id.startsWith('gasket:') ? [id.replace(/:(upper|lower)$/, ':lower'), id.replace(/:(upper|lower)$/, ':upper')] : [id];

export function useCaseDisplay(projectId: string, instanceId: string) {
  const [saved, setSaved] = useState<Record<string, CaseDisplay>>({});
  const read = useCallback((id: string): CaseDisplay => {
    const key = `${projectId}:${id}`;
    if (saved[key]) return saved[key];
    try {
      const value = JSON.parse(localStorage.getItem(`boardstudio:case-display:${key}`) ?? 'null');
      if (Array.isArray(value?.hidden) && value.colors && typeof value.colors === 'object') return value;
    } catch { /* Display preferences are optional. */ }
    return empty;
  }, [saved, projectId]);
  const update = (next: CaseDisplay, id = instanceId) => {
    const key = `${projectId}:${id}`;
    setSaved(current => ({ ...current, [key]: next }));
    try { localStorage.setItem(`boardstudio:case-display:${key}`, JSON.stringify(next)); } catch { /* Display preferences are optional. */ }
  };
  const toggle = (partId: string, id = instanceId) => {
    const current = read(id), ids = displayIds(partId);
    const hidden = ids.every(part => current.hidden.includes(part));
    update({ ...current, hidden: hidden ? current.hidden.filter(part => !ids.includes(part)) : [...new Set([...current.hidden, ...ids])] }, id);
  };
  const color = (partId: string, value: string) => {
    const current = read(instanceId), colors = { ...current.colors };
    for (const id of displayIds(partId)) { if (value) colors[id] = value; else delete colors[id]; }
    update({ ...current, colors });
  };
  const current = useMemo(() => read(instanceId), [read, instanceId]);
  return { current, read, update, toggle, color };
}
