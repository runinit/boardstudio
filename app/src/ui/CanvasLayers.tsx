import { useEffect, useId, useRef, useState } from 'react';
import './canvas-layers.css';

type Layer = { id: string; label: string; accessibilityLabel?: string; availabilityLabel?: string; kind?: string; available?: boolean };

export function CanvasLayers({ groups, hidden, onToggle }: {
  groups: { title: string; layers: Layer[] }[];
  hidden: ReadonlySet<string>;
  onToggle: (id: string) => void;
}) {
  const listId = useId();
  const triggerRef = useRef<HTMLButtonElement>(null);
  const panelRef = useRef<HTMLElement>(null);
  const previousCompact = useRef<boolean | undefined>(undefined);
  const [compact, setCompact] = useState(false);
  const [open, setOpen] = useState(false);
  useEffect(() => {
    const panel = panelRef.current;
    if (!panel || typeof ResizeObserver === 'undefined') return;
    const update = (width: number) => {
      const nextCompact = width < 640;
      setCompact(nextCompact);
      if (nextCompact && previousCompact.current === false) setOpen(false);
      previousCompact.current = nextCompact;
    };
    update(panel.parentElement?.getBoundingClientRect().width ?? panel.getBoundingClientRect().width);
    const observer = new ResizeObserver(([entry]) => update(entry.contentRect.width));
    observer.observe(panel.parentElement ?? panel);
    return () => observer.disconnect();
  }, []);

  return <section ref={panelRef} className={`wb-stage-layers${compact ? ' is-compact' : ''}`} data-open={open} data-compact={compact} aria-label="Canvas layers" onKeyDown={event => {
    if (event.key === 'Escape' && open) {
      event.preventDefault(); event.stopPropagation(); setOpen(false); triggerRef.current?.focus();
    }
  }}>
    <button ref={triggerRef} type="button" className="wb-stage-layers-heading" aria-expanded={open} aria-controls={listId} onClick={() => setOpen(!open)}>
      <span>Layers</span><svg viewBox="0 0 20 20" aria-hidden="true"><path d={open ? 'm5 12 5-5 5 5' : 'm5 8 5 5 5-5'} /></svg>
    </button>
    {compact && open && <button type="button" className="wb-stage-layers-close" onClick={() => { setOpen(false); triggerRef.current?.focus(); }}>Close</button>}
    <div id={listId} className="wb-stage-layer-list" hidden={!open}>
      {groups.filter(group => group.layers.length).map(group => <div key={group.title}>
        {group.title && <div className="wb-stage-layer-group">{group.title}</div>}
        {group.layers.map(layer => <button type="button" key={layer.id} title={layer.available === false ? `${layer.label} — ${layer.availabilityLabel ?? 'Not generated'}` : layer.label} aria-pressed={!hidden.has(layer.id)} onClick={() => onToggle(layer.id)} aria-label={`${hidden.has(layer.id) ? 'Show' : 'Hide'} ${layer.accessibilityLabel ?? layer.label}`}>
          <span data-layer={layer.label} className={`wb-stage-layer-swatch is-${layer.kind ?? (['Board', 'PCB', 'Edge.Cuts'].includes(layer.id) ? 'board' : layer.id.includes('Cu') || layer.id === 'Pads' || layer.id === 'Copper' ? 'copper' : 'graphic')}`} />
          <span className="wb-stage-layer-label">{layer.label}</span>{layer.available === false && <small className="wb-stage-layer-state">{layer.availabilityLabel ?? 'Not generated'}</small>}
          <svg viewBox="0 0 20 20" aria-hidden="true"><path d="M2 10q8-12 16 0-8 12-16 0Z" /><circle cx="10" cy="10" r="2.5" />{hidden.has(layer.id) && <path d="m3 17 14-14" />}</svg>
        </button>)}
      </div>)}
    </div>
  </section>;
}
