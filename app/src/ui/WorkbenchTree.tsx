import React, { useEffect, useMemo, useRef, useState } from 'react';

export type TreeEntry = {
  id: string;
  label: string;
  detail?: string;
  hidden?: boolean;
  onVisibility?: () => void;
  level: number;
  expandable?: boolean;
  expanded?: boolean;
  selected?: boolean;
  kind: 'layout' | 'components' | 'pcb' | 'case' | 'board' | 'matrix' | 'row' | 'column' | 'key' | 'component' | 'outline' | 'outline-version' | 'bridge';
  onToggle?: () => void;
  onSelect: () => void;
  onKeyDown?: (event: React.KeyboardEvent<HTMLButtonElement>) => void;
};

const ROW_HEIGHT = 36;
const OVERSCAN = 6;

export const WorkbenchTree = ({ entries }: { entries: TreeEntry[] }) => {
  const [scrollTop, setScrollTop] = useState(0);
  const [viewportHeight, setViewportHeight] = useState(480);
  const viewportRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const viewport = viewportRef.current;
    if (!viewport) return;
    const resize = () => setViewportHeight(viewport.clientHeight || 480);
    resize();
    const observer = new ResizeObserver(resize);
    observer.observe(viewport);
    return () => observer.disconnect();
  }, []);
  const range = useMemo(() => {
    const first = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN);
    const count = Math.ceil(viewportHeight / ROW_HEIGHT) + OVERSCAN * 2;
    return { first, last: Math.min(entries.length, first + count) };
  }, [entries.length, scrollTop, viewportHeight]);

  return <div
    className="wb-tree-viewport"
    role="tree"
    aria-label="CAD structure"
    onScroll={(event) => setScrollTop(event.currentTarget.scrollTop)}
    ref={viewportRef}
  >
    <div className="wb-tree-space" style={{ height: entries.length * ROW_HEIGHT }}>
      {entries.slice(range.first, range.last).map((entry, offset) => {
        const index = range.first + offset;
        return <div className={`wb-tree-row is-${entry.kind} ${entry.selected ? 'is-selected' : ''}`} key={entry.id} style={{ top: index * ROW_HEIGHT, paddingLeft: 8 + entry.level * 14 }}>
          {entry.expandable ? <button className="wb-tree-disclosure" aria-label={`${entry.expanded ? 'Collapse' : 'Expand'} ${entry.label}`} aria-expanded={entry.expanded} onClick={entry.onToggle}>{entry.expanded ? '⌄' : '›'}</button> : <span className="wb-tree-spacer" />}
          <button className="wb-tree-select" role="treeitem" aria-level={entry.level + 1} aria-description={entry.onKeyDown ? 'Arrow keys move 0.1 mm; Shift+Arrow moves 1 mm. Delete removes the selection.' : undefined} aria-selected={entry.selected} aria-expanded={entry.expandable ? entry.expanded : undefined} onClick={entry.onSelect} onKeyDown={entry.onKeyDown}>
          <span className={`wb-tree-kind is-${entry.kind}`} aria-hidden="true"><TreeGlyph kind={entry.kind} /></span>
            <span className="wb-tree-label" title={entry.label}>{entry.label}</span>
            {entry.detail && <small title={entry.detail}>{entry.detail}</small>}
          </button>
          {entry.onVisibility && <button className="wb-tree-visibility" aria-label={`${entry.hidden ? 'Show' : 'Hide'} ${entry.label}`} aria-pressed={!entry.hidden} onClick={entry.onVisibility}><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12Z"/><circle cx="12" cy="12" r="3"/>{entry.hidden && <path d="m3 3 18 18"/>}</svg></button>}
        </div>;
      })}
    </div>
  </div>;
};

const TreeGlyph = ({ kind }: { kind: TreeEntry['kind'] }) => kind === 'pcb' ? <svg viewBox="0 0 16 16"><path d="m1 5 7-4 7 4-7 4ZM1 8l7 4 7-4M1 11l7 4 7-4" /></svg>
  : kind === 'outline' || kind === 'outline-version' ? <svg viewBox="0 0 16 16"><path d="M2 2h8v3h4v9H2z" />{kind === 'outline-version' && <path d="M5 8h6M5 11h4" />}</svg>
  : kind === 'bridge' ? <svg viewBox="0 0 16 16"><path d="M2 11V5h12v6M2 8h12" /><circle cx="2" cy="12" r="1.5" /><circle cx="14" cy="12" r="1.5" /></svg>
  : kind === 'case' ? <svg viewBox="0 0 16 16"><path d="m8 1 6 3v8l-6 3-6-3V4ZM2 4l6 3 6-3M8 7v8" /></svg>
  : kind === 'matrix' ? <svg viewBox="0 0 16 16"><rect x="2" y="2" width="12" height="12" rx="1" /><path d="M6 2v12M10 2v12M2 6h12M2 10h12" /></svg>
  : kind === 'components' ? <svg viewBox="0 0 16 16"><rect x="2" y="2" width="8" height="8" rx="1" /><rect x="6" y="6" width="8" height="8" rx="1" /><path d="M4 4h4M8 8h4" /></svg>
  : kind === 'key' ? <svg viewBox="0 0 16 16"><rect x="2" y="2" width="12" height="12" rx="2" /></svg>
    : kind === 'component' ? <svg viewBox="0 0 16 16"><path d="m8 1.8 6.2 6.2L8 14.2 1.8 8z" /><circle cx="8" cy="8" r="1.4" /></svg>
      : kind === 'column' ? <svg viewBox="0 0 16 16"><path d="M8 2v12M5 5l3-3 3 3M5 11l3 3 3-3" /></svg>
        : kind === 'row' ? <svg viewBox="0 0 16 16"><path d="M2 4h12M2 8h12M2 12h12" /></svg>
          : <svg viewBox="0 0 16 16"><path d="M2 3h12v10H2zM5 6h6M5 9h4" /></svg>;
