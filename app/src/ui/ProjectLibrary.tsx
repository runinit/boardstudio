import type { ProjectDoc } from '@boardstudio/v2-contracts';
import { useEffect, useId, useMemo, useRef, useState } from 'react';
import { demos, type DemoId } from '../demos/keyboards';
import { keyboardPreview } from '../demos/keyboardPreviews';
import { listProjects } from '../storage';

export function ProjectLibraryIcon({ name }: { name: 'new' | 'open' | 'download' | 'keyboard' | 'search' | 'guide' | 'settings' | 'check' | 'delete' }) {
  const paths = {
    new: 'M10 3v14M3 10h14',
    open: 'M2 6V4h6l2 2h8v3M2 6v11h14l2-8H5l-3 8',
    download: 'M10 2v10m-4-4 4 4 4-4M3 13v4h14v-4',
    keyboard: 'M2 5h16v11H2ZM5 8h.1M8 8h.1M11 8h.1M14 8h.1M5 11h.1M8 11h.1M11 11h.1M14 11h.1M6 14h8',
    search: 'M13 13l5 5M15 8a7 7 0 1 1-14 0 7 7 0 0 1 14 0',
    guide: 'M3 3h5l2 2 2-2h5v13h-5l-2 2-2-2H3ZM10 5v13',
    settings: 'M3 5h14M3 10h14M3 15h14M6 3v4M14 8v4M8 13v4',
    check: 'm4 10 4 4 8-8',
    delete: 'M3 5h14M7 5V3h6v2M5 5l1 12h8l1-12M8 8v6M12 8v6',
  };
  return <svg viewBox="0 0 20 20" aria-hidden="true"><path d={paths[name]} /></svg>;
}

function previewKeys(document: ProjectDoc) {
  const definitions = new Map(document.definitions.map(definition => [definition.id, definition]));
  return document.parts.flatMap(part => {
    const definition = definitions.get(part.definitionId);
    if (definition?.kind !== 'switch') return [];
    const size = part.keycap ?? definition.keycap ?? { x: 18, y: 18 };
    return [{ id: part.id, x: part.pose.at.x, y: -part.pose.at.y, angle: -part.pose.rotation, width: size.x, height: size.y }];
  });
}

function KeyboardPreview({ keys }: { keys: ReturnType<typeof previewKeys> }) {
  if (!keys.length) return <div className="wb-keyboard-preview is-empty"><ProjectLibraryIcon name="keyboard" /><span>No keys placed</span></div>;
  const extents = keys.map(key => {
    const angle = key.angle * Math.PI / 180;
    return { ...key, dx: (Math.abs(Math.cos(angle)) * key.width + Math.abs(Math.sin(angle)) * key.height) / 2, dy: (Math.abs(Math.sin(angle)) * key.width + Math.abs(Math.cos(angle)) * key.height) / 2 };
  });
  const left = Math.min(...extents.map(key => key.x - key.dx)) - 8;
  const top = Math.min(...extents.map(key => key.y - key.dy)) - 8;
  const width = Math.max(...extents.map(key => key.x + key.dx)) - left + 8;
  const height = Math.max(...extents.map(key => key.y + key.dy)) - top + 8;
  return <div className="wb-keyboard-preview"><svg viewBox={`${left} ${top} ${width} ${height}`} aria-hidden="true">
    {keys.map(key => <rect key={key.id} transform={`translate(${key.x} ${key.y}) rotate(${key.angle})`} x={-key.width / 2} y={-key.height / 2} width={key.width} height={key.height} rx="2" />)}
  </svg></div>;
}

function KeyboardTile({ name, keys, boardCount, current, demo, onOpen, onDelete }: { name: string; keys: ReturnType<typeof previewKeys>; boardCount: number; current?: boolean; demo?: boolean; onOpen: () => void; onDelete?: () => void }) {
  return <div className="wb-keyboard-card"><button type="button" className={`wb-keyboard-tile${current ? ' is-current' : ''}`} aria-label={`${demo ? 'Start' : 'Open'} ${name}`} aria-current={current ? 'true' : undefined} onClick={onOpen}>
    <KeyboardPreview keys={keys} />
    <span className="wb-keyboard-tile-title">{name}</span>
    <span className="wb-keyboard-tile-detail">{keys.length} keys · {boardCount > 1 ? `${boardCount} boards` : 'Single board'}{current && <span className="wb-keyboard-current"><ProjectLibraryIcon name="check" />Current</span>}</span>
  </button>{onDelete && <button type="button" className="wb-keyboard-delete" aria-label={`Delete ${name}`} title={`Delete ${name}`} onClick={onDelete}><ProjectLibraryIcon name="delete" /></button>}</div>;
}

export function ProjectLibrary({ document, onOpen, onOpenDemo, onDelete }: { document: ProjectDoc; onOpen?: (id: string) => void; onOpenDemo?: (id: DemoId) => void; onDelete?: (id: string) => Promise<boolean> }) {
  const [projects, setProjects] = useState<ProjectDoc[]>([]);
  const [status, setStatus] = useState<'loading' | 'ready' | 'failed'>('loading');
  const [retry, setRetry] = useState(0);
  const [query, setQuery] = useState('');
  const [pendingDelete, setPendingDelete] = useState<ProjectDoc | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState('');
  const dialog = useRef<HTMLDialogElement>(null);
  const search = useRef<HTMLInputElement>(null);
  const confirmationId = useId();

  useEffect(() => {
    if (pendingDelete) dialog.current?.showModal();
    else dialog.current?.close();
  }, [pendingDelete?.id]);

  async function confirmDelete() {
    if (!pendingDelete || !onDelete || deleting) return;
    setDeleting(true);
    setDeleteError('');
    try {
      if (!await onDelete(pendingDelete.id)) throw new Error('Deletion failed');
      setProjects(saved => saved.filter(project => project.id !== pendingDelete.id));
      setPendingDelete(null);
      setRetry(value => value + 1);
      requestAnimationFrame(() => search.current?.focus());
    } catch {
      setDeleteError('This keyboard could not be deleted. Try again.');
    } finally {
      setDeleting(false);
    }
  }

  const demoPreviews = useMemo(() => demos.map(demo => ({ ...demo, ...keyboardPreview(demo.id) })), []);

  useEffect(() => {
    let cancelled = false;
    setStatus('loading');
    void listProjects().then(saved => {
      if (cancelled) return;
      setProjects(saved);
      setStatus('ready');
    }).catch(() => { if (!cancelled) setStatus('failed'); });
    return () => { cancelled = true; };
  }, [document.id, retry]);

  const saved = [document, ...projects.filter(project => project.id !== document.id).sort((a, b) => a.name.localeCompare(b.name))];
  const matches = saved.filter(project => project.name.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase()));

  return <div className="wb-keyboard-library-scroll">
    <section aria-label="Your keyboards" className="wb-keyboard-section">
      <div className="wb-keyboard-section-heading"><h3>Your keyboards <span>{status === 'ready' ? saved.length : ''}</span></h3><span>Saved in this browser</span></div>
      <div className="wb-keyboard-search"><ProjectLibraryIcon name="search" /><input ref={search} type="search" aria-label="Search saved keyboards" placeholder="Search your keyboards" value={query} onChange={event => setQuery(event.target.value)} />{query && <button onClick={() => setQuery('')}>Clear search</button>}</div>
      {status === 'loading' && <p role="status">Loading saved keyboards…</p>}
      {status === 'failed' && <p role="alert">Saved keyboards could not be loaded. <button className="wb-library-text-action" onClick={() => setRetry(value => value + 1)}>Try again</button></p>}
      <div className="wb-keyboard-grid">{matches.map(project => <KeyboardTile key={project.id} name={project.name} keys={previewKeys(project)} boardCount={project.boards.length} current={project.id === document.id} onDelete={onDelete ? () => { setDeleteError(''); setPendingDelete(project); } : undefined} onOpen={() => onOpen?.(project.id)} />)}</div>
      {!matches.length && <p className="wb-keyboard-empty">No keyboards match your search.</p>}
    </section>
    {onOpenDemo && <section aria-label="Demo keyboards" className="wb-keyboard-section wb-keyboard-demos">
      <div className="wb-keyboard-section-heading"><h3>Demo keyboards</h3><span>Start an editable copy</span></div>
      <div className="wb-keyboard-grid">{demoPreviews.map(demo => <KeyboardTile key={demo.id} {...demo} demo onOpen={() => onOpenDemo(demo.id)} />)}</div>
    </section>}
    <dialog ref={dialog} className="wb-keyboard-delete-dialog" aria-labelledby={`${confirmationId}-title`} aria-describedby={`${confirmationId}-description`}
      onKeyDown={event => event.stopPropagation()} onCancel={event => { event.preventDefault(); if (!deleting) setPendingDelete(null); }}>
      <h2 id={`${confirmationId}-title`}>Delete “{pendingDelete?.name}”?</h2>
      <p id={`${confirmationId}-description`}>This removes the keyboard saved in this browser. This cannot be undone. Any project copies you downloaded will be kept.</p>
      {deleteError && <p role="alert">{deleteError}</p>}
      <div className="wb-keyboard-delete-actions">
        <button type="button" autoFocus disabled={deleting} onClick={() => setPendingDelete(null)}>Cancel</button>
        <button type="button" className="wb-keyboard-delete-confirm" disabled={deleting} onClick={() => void confirmDelete()}>{deleting ? 'Deleting…' : 'Delete keyboard'}</button>
      </div>
    </dialog>
  </div>;
}
