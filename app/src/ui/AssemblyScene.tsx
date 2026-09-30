import type { CaseDisplay } from './caseDisplay';
import { CanvasLayers } from './CanvasLayers';
import { defaultGasketLayout, moveGasket, gasketAnchors } from '../gasketEditing';
import { caseMountConstraints, moveCaseMount } from '../caseEditing';
import type { MechanicalGasketSupport } from '@boardstudio/v2-contracts';
import type { GenerationState } from '../generationState';
import { useEffect, useMemo, useRef, useState } from 'react';
import type { BoardReference, CaseBody, MechanicalAssembly, MechanicalConfiguration, Mount, PcbPreview, PreparedCaseAssemblyIR } from '@boardstudio/v2-contracts';
import { createRendererCanvas, type RendererCanvas } from '../renderClient';
import './assembly-preview.css';

import type { LoadedModel, AssemblyBody } from '../assemblyPreview';
export type { AssemblyBody } from '../assemblyPreview';
type AssemblyView = 'assembled' | 'exploded' | 'section';

export function AssemblyScene({ board, models, bodies = [], authoredCaseBodies = [], mechanical, generation, preparedCase, onGasketChange, onGasketDraft, onCaseMountChange, onCaseMountDraft, mechanicalConfiguration, selectedLayer = '', reference, onSelect, onSelectLayer, colorScheme, persistenceKey, display, onDisplayChange }: {
  board: PcbPreview;
  models: LoadedModel[];
  bodies?: AssemblyBody[];
  authoredCaseBodies?: CaseBody[];
  mechanical?: MechanicalAssembly;
  generation?: GenerationState;
  preparedCase?: PreparedCaseAssemblyIR;
  onGasketChange?: (config: MechanicalConfiguration) => void | Promise<boolean>;
  onGasketDraft?: (config: MechanicalConfiguration | null, disposition?: 'commit') => void;
  onCaseMountChange?: (bodyId: string, mounts: Mount[]) => void | Promise<boolean>;
  onCaseMountDraft?: (bodyId: string, mounts: Mount[] | null, disposition?: 'commit') => void;
  mechanicalConfiguration?: MechanicalConfiguration;
  selectedLayer?: string;
  reference?: BoardReference;
  onSelect?: (reference: string) => void;
  onSelectLayer?: (id: string) => void;
  colorScheme: 'light' | 'dark';
  persistenceKey?: string;
  display?: CaseDisplay;
  onDisplayChange?: (next: CaseDisplay) => void;
}) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const renderer = useRef<RendererCanvas | undefined>(undefined);
  const sceneRevision = useRef(0);
  const appliedBase = useRef<{ board: PcbPreview; models: LoadedModel[]; reference?: BoardReference; stackKey: string; batteryKey: string } | undefined>(undefined);
  const interacted = useRef(false);
  const fitted = useRef(false);
  const fittedModels = useRef(false);
  const selectRef = useRef(onSelect);
  selectRef.current = onSelect;
  const selectLayerRef = useRef(onSelectLayer);
  selectLayerRef.current = onSelectLayer;
  const gasketChangeRef = useRef(onGasketChange);
  const gasketDraftRef = useRef(onGasketDraft);
  const mountChangeRef = useRef(onCaseMountChange);
  const mountDraftRef = useRef(onCaseMountDraft);
  gasketChangeRef.current = onGasketChange;
  gasketDraftRef.current = onGasketDraft;
  mountChangeRef.current = onCaseMountChange;
  mountDraftRef.current = onCaseMountDraft;
  const draftFrame = useRef<number | undefined>(undefined);
  const pendingDraft = useRef<(() => void) | undefined>(undefined);
  const queueDraft = (callback: () => void) => {
    pendingDraft.current = callback;
    if (draftFrame.current !== undefined) return;
    draftFrame.current = requestAnimationFrame(() => {
      draftFrame.current = undefined;
      pendingDraft.current?.();
      pendingDraft.current = undefined;
    });
  };
  const preparedCaseRef = useRef(preparedCase);
  preparedCaseRef.current = preparedCase;
  const mechanicalRef = useRef(mechanical);
  mechanicalRef.current = mechanical;
  const [ready, setReady] = useState(false);
  const [error, setError] = useState('');
  const hiddenKey = `boardstudio:v2:layers:assembly:${persistenceKey ?? 'default'}`;
  const [localHidden, setLocalHidden] = useState<Set<string>>(() => { try { return new Set(JSON.parse(localStorage.getItem(hiddenKey) ?? '[]') as string[]); } catch { return new Set(); } });
  const hidden = useMemo(() => display ? new Set(display.hidden) : localHidden, [display?.hidden, localHidden]);
  const setHidden = (change: (current: Set<string>) => Set<string>) => {
    if (display && onDisplayChange) onDisplayChange({ ...display, hidden: [...change(hidden)] });
    else setLocalHidden(change);
  };
  const [explodeAmount, setExplodeAmount] = useState(1);
  const [sectionPlane, setSectionPlane] = useState('YZ');
  const [sectionPosition, setSectionPosition] = useState(0);
  const [showSectionPlane, setShowSectionPlane] = useState(true);
  const [showHidden, setShowHidden] = useState(false);
  const [selected, setSelected] = useState('');
  const [view, setView] = useState<AssemblyView>('assembled');
  const [displayMode, setDisplayMode] = useState<'shaded' | 'wireframe' | 'hybrid'>('hybrid');
  const stableBodies = useRef(bodies);
  if (bodies.length !== stableBodies.current.length || bodies.some((b,i) => b.id !== stableBodies.current[i]?.id || b.mesh.positions !== stableBodies.current[i]?.mesh.positions || b.color !== stableBodies.current[i]?.color)) stableBodies.current = bodies;
  const geometryBodies = stableBodies.current;
  // A commit may reuse the displayed draft buffers. Acknowledge its new exact
  // revision through the renderer's zero-delta path without forcing a redraw.
  const exactRevision = useRef<number | undefined>(undefined);
  if (generation?.status === 'ready') exactRevision.current = generation.revision;
  const stackKey = JSON.stringify(mechanical?.stack ?? []);
  const batteryKey = JSON.stringify(mechanicalConfiguration?.battery);
  const [preparing, setPreparing] = useState(false);
  const [editingGaskets, setEditingGaskets] = useState(false);
  const [gasketMessage, setGasketMessage] = useState('');
  const [activeGasket, setActiveGasket] = useState('');
  const [editingMounts, setEditingMounts] = useState(false);
  useEffect(() => {
    const gasketSelected = selectedLayer.startsWith('gasket:');
    setEditingGaskets(gasketSelected || selectedLayer === 'gaskets');
    setActiveGasket(gasketSelected ? selectedLayer.slice(7, -6) : '');
    if (gasketSelected || selectedLayer === 'gaskets') setEditingMounts(false);
  }, [selectedLayer]);
  useEffect(() => { try { localStorage.setItem(hiddenKey, JSON.stringify([...hidden])); } catch { /* view preference only */ } }, [hiddenKey, hidden]);

  useEffect(() => {
    const element = canvas.current;
    if (!element) return;
    let disposed = false;
    setError('');
    createRendererCanvas(element, (id) => {
      setSelected(id);
      if (id.startsWith('gasket:') || mechanicalRef.current?.stack.some((layer) => layer.id === id) || id === 'pcb' || id === 'battery') selectLayerRef.current?.(id);
      selectRef.current?.(id === 'pcb' ? 'PCB' : id);
    }, () => { interacted.current = true; }).then((instance) => {
      if (disposed) { instance.dispose(); return; }
      renderer.current = instance;
      setReady(true);
    }).catch(() => {
      if (!disposed) setError('WebGL2 could not start. The 2D editor remains available.');
    });
    return () => {
      disposed = true;
      setReady(false);
      renderer.current?.dispose();
      renderer.current = undefined;
    };
  }, []);

  useEffect(() => {
    const current = renderer.current;
    if (!ready || !current) return;
    const keepCamera = fitted.current && (models.length === 0 || fittedModels.current || interacted.current);
    const revision = ++sceneRevision.current;
    try {
      setPreparing(true);
      const packet = {
        revision,
        kind: 'assembly',
        theme: colorScheme,
        view,
        keepCamera,
        selectedLayer,
        hidden: [...hidden],
        board,
        models,
        bodies: geometryBodies,
        mechanicalStack: mechanical?.stack ?? [],
        battery: mechanicalConfiguration?.battery,
        reference,
      };
      const previous = appliedBase.current;
      const bodyOnly = previous?.board === board && previous.models === models && previous.reference === reference
        && previous.stackKey === stackKey && previous.batteryKey === batteryKey;
      const update = bodyOnly ? current.setSceneBodies({ revision, bodies: geometryBodies }) : current.setScene(packet);
      void update.then(accepted => {
      if (accepted) {
        appliedBase.current = { board, models, reference, stackKey, batteryKey };
        if (!keepCamera) fitted.current = true;
        if (models.length > 0) fittedModels.current = true;
        setError('');
        setPreparing(false);
      }
      }).catch(cause => { setError(String(cause)); setPreparing(false); });
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : 'The assembly preview could not be updated.');
    }
  }, [ready, board, models, geometryBodies, reference, stackKey, batteryKey, exactRevision.current]);

  useEffect(() => {
    renderer.current?.setState({ hidden: [...hidden, ...(!editingGaskets && !editingMounts ? ['GasketHandles'] : [])], selectedLayer, view, mode: displayMode, theme: colorScheme, explodeAmount, sectionPlane, sectionPosition, showSectionPlane, showHidden, colors: display?.colors ?? {} });
  }, [ready, hidden, selectedLayer, view, displayMode, colorScheme, editingGaskets, editingMounts, explodeAmount, sectionPlane, sectionPosition, showSectionPlane, showHidden, display?.colors]);

  useEffect(() => {
    const current = renderer.current;
    if (!current || !ready || (!mechanical && !authoredCaseBodies.length)) return;
    const original = mechanical?.gasketSupports ?? [];
    const z = mechanical?.stack.find(layer => layer.id === 'retainer');
    const handleZ = z ? z.z + z.thickness + 0.7 : 9;
    type MountTarget = { key: string; mounts: Mount[]; z: number; body?: CaseBody; field?: 'mounts' | 'closureMounts' };
    const targets: MountTarget[] = mechanicalConfiguration ? [
      ...(mechanicalConfiguration.mount !== 'gasket' ? [{ key: 'suspension', mounts: mechanicalConfiguration.mounts,
        z: mechanical?.stack.find(layer => layer.id === (mechanicalConfiguration.mount === 'rigid' ? 'plate' : 'bottom'))?.z ?? 0, field: 'mounts' as const }] : []),
      { key: 'closure', mounts: mechanicalConfiguration.closureMounts ?? [],
        z: mechanical?.stack.find(layer => layer.id === 'bottom')?.z ?? 0, field: 'closureMounts' },
    ] : authoredCaseBodies.map(body => ({ key: body.id, body, mounts: body.mounts ?? [], z: body.z ?? 0 }));
    const mountId = (target: MountTarget, mount: Mount) => `case-mount:${target.key}/${mount.id}`;
    let movingMount: { target: MountTarget; id: string; pending: Mount[]; constraints: ReturnType<typeof caseMountConstraints> } | undefined;
    const mountHandles = (invalid = false) => targets.flatMap(target =>
      (movingMount?.target === target ? movingMount.pending : target.mounts).map(mount => ({
        id: mountId(target, mount), at: mount.at, tangent: { x: 1, y: 0 }, normal: { x: 0, y: 1 },
        length: (mount.kind === 'boss' ? mount.bossDiameter ?? mount.holeDiameter : mount.holeDiameter) + 3,
        z: target.z + 0.8, invalid: invalid && movingMount?.id === mount.id && movingMount.target === target,
      })));
    const handles = (supports: MechanicalGasketSupport[], invalid = false) => supports.map(support => ({
      ...support, id: `gasket-handle:${support.id}`, z: handleZ, invalid: Boolean(support.fitError) || invalid,
    }));
    const mountDraft = (target: MountTarget, mounts: Mount[] | null, disposition?: 'commit') => {
      if (target.field && mechanicalConfiguration) gasketDraftRef.current?.(mounts ? { ...mechanicalConfiguration, [target.field]: mounts } : null, disposition);
      else if (target.body) {
        if (disposition) mountDraftRef.current?.(target.body.id, mounts, disposition);
        else mountDraftRef.current?.(target.body.id, mounts);
      }
    };
    let alive = true, committing = false;
    const commit = (save: () => void | Promise<boolean>, restore: () => void) => {
      committing = true;
      void (async () => {
        let saved = false;
        try { saved = await save() !== false; } catch { /* The owner reports the edit failure. */ }
        if (!alive) return;
        committing = false;
        if (!saved) { restore(); setGasketMessage('Move could not be saved'); }
      })();
    };
    const clearQueuedDraft = () => {
      if (draftFrame.current !== undefined) cancelAnimationFrame(draftFrame.current);
      draftFrame.current = undefined; pendingDraft.current = undefined;
    };
    current.setHandles(editingGaskets && mechanicalConfiguration ? handles(original) : editingMounts ? mountHandles() : []);
    let moving = '';
    let pending = original;
    let valid = true;
    current.setDrag(editingGaskets || editingMounts ? {
      start(id) {
        if (committing) return undefined;
        if (editingMounts) {
          for (const target of targets) {
            const mount = target.mounts.find(entry => mountId(target, entry) === id);
            if (!mount) continue;
            const sourceId = target.body?.id ?? (target.key === 'suspension' && mechanicalConfiguration?.mount === 'rigid' ? 'plate' : 'bottom');
            const prepared = preparedCaseRef.current?.bodies.find(entry => entry.body.id === sourceId);
            if (!prepared) {
              setGasketMessage('Update the case preview before moving mounts');
              return undefined;
            }
            // Capture committed boundaries once so draft replies cannot change an active gesture.
            movingMount = { target, id: mount.id, pending: target.mounts, constraints: caseMountConstraints(prepared) }; valid = true;
            setGasketMessage('Drag the mount · release to save its case position');
            return target.z + 0.8;
          }
          return undefined;
        }
        if (!id.startsWith('gasket-handle:') || !mechanicalConfiguration) return undefined;
        moving = id.slice('gasket-handle:'.length);
        setActiveGasket(moving); selectLayerRef.current?.(`gasket:${moving}:lower`); pending = original; valid = true;
        return handleZ;
      },
      move(point) {
        if (movingMount) {
          const next = moveCaseMount(point, movingMount.id, movingMount.target.mounts, 0.5, movingMount.constraints);
          valid = Boolean(next);
          if (next) {
            movingMount.pending = next;
            const target = movingMount.target;
            queueDraft(() => mountDraft(target, next));
          }
          current.setHandles(mountHandles(!valid));
          setGasketMessage(valid ? 'Release to save mount position' : 'Placement blocked · keep clearance from edges, holes, and other mounts');
          return;
        }
        if (!moving || !mechanicalConfiguration) return;
        const next = moveGasket(point, moving, original, mechanical?.gasketTracks ?? []);
        valid = Boolean(next);
        if (next) {
          pending = next;
        }
        current.setHandles(handles(pending, !valid));
        setGasketMessage(valid ? 'Release to place · fit is checked after placement · Escape cancels' : 'No perimeter found for this gasket');
      },
      end(cancelled) {
        clearQueuedDraft();
        if (movingMount) {
          const { target, pending: mounts } = movingMount;
          if (!cancelled && valid && mounts !== target.mounts) {
            mountDraft(target, mounts, 'commit');
            current.setHandles(mountHandles());
            movingMount = undefined;
            commit(() => target.field && mechanicalConfiguration
              ? gasketChangeRef.current?.({ ...mechanicalConfiguration, [target.field]: mounts })
              : target.body ? mountChangeRef.current?.(target.body.id, mounts) : undefined,
            () => { mountDraft(target, null); current.setHandles(mountHandles()); });
          } else {
            mountDraft(target, null); movingMount = undefined;
            current.setHandles(mountHandles());
          }
        } else if (moving) {
          if (!cancelled && valid && pending !== original && mechanicalConfiguration) {
            const layout = mechanicalConfiguration.gasketLayout ?? defaultGasketLayout();
            const configuration = { ...mechanicalConfiguration, gasketLayout: { ...layout, supports: gasketAnchors(layout, original, pending) } };
            current.setHandles(handles(pending));
            commit(() => gasketChangeRef.current?.(configuration),
              () => { gasketDraftRef.current?.(null); current.setHandles(handles(original)); });
          } else {
            gasketDraftRef.current?.(null); current.setHandles(handles(original));
          }
          moving = '';
        }
        setGasketMessage(cancelled ? 'Move cancelled' : !valid ? 'Blocked move was not saved' : '');
      },
    } : undefined);
    return () => {
      alive = false;
      clearQueuedDraft();
      if (movingMount) mountDraft(movingMount.target, null);
      if (moving) gasketDraftRef.current?.(null);
      current.setDrag(undefined);
    };
  }, [ready, editingGaskets, editingMounts, mechanical, authoredCaseBodies, mechanicalConfiguration, board]);

  const unlinkGasket = () => {
    if (!mechanical || !mechanicalConfiguration || !onGasketChange) return;
    const original = mechanical.gasketSupports ?? [];
    const next = original.map(support => support.id === activeGasket || support.pairId === activeGasket ? { ...support, unlinked: true } : support);
    const layout = mechanicalConfiguration.gasketLayout ?? defaultGasketLayout();
    onGasketChange({ ...mechanicalConfiguration, gasketLayout: { ...layout, supports: gasketAnchors(layout, original, next) } });
  };

  const toggle = (id: string) => setHidden((old) => {
    const next = new Set(old);
    next.has(id) ? next.delete(id) : next.add(id);
    return next;
  });
  const generatedBodyIds = new Set(bodies.map((body) => body.id));
  const controls = [
    ['PCB', 'PCB'],
    ['Copper', 'Copper'],
    ['Mask', 'Mask openings'],
    ['Silkscreen', 'Silkscreen'],
    ['Models', 'Models'],
    ['Keycaps', 'Keycaps'],
    ...bodies.map((body) => [body.id, body.name]),
    ...(mechanical?.stack ?? []).map((layer) => layer.id === 'pcb' ? ['PCB', 'PCB'] : [layer.id, layer.id]),
  ].filter(([id], index, all) => all.findIndex(([candidate]) => candidate === id) === index);
  const generatedBodyCount = mechanical
    ? bodies.filter((body) => mechanical.case.bodies.some((entry) => entry.body.id === body.id)).length
    : 0;
  const solidsBlocked = mechanical?.generationBlocked ?? false;
  const showView = (preset: 'fit' | 'top' | 'bottom' | 'isometric') => {
    renderer.current?.view(preset);
    interacted.current = true;
    if (preset === 'fit') fitted.current = true;
  };

  return <div className="wb-assembly-scene" aria-label="Complete PCB assembly preview">
    <div className="wb-scene-view-bar">
    <div className="wb-render-modes" role="group" aria-label="Display mode">
      {(['shaded', 'wireframe', 'hybrid'] as const).map(mode => <button key={mode} aria-pressed={displayMode === mode} onClick={() => setDisplayMode(mode)}>{mode[0].toUpperCase() + mode.slice(1)}</button>)}
      <button aria-label="Show hidden lines" title="Show hidden lines" aria-pressed={showHidden} onClick={() => setShowHidden(value => !value)}><svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true"><path d="m12 2 9 5v10l-9 5-9-5V7ZM3 7l9 5 9-5M12 12v10"/><path strokeDasharray="2 2" d="M12 2v10M3 17l9-5 9 5"/></svg></button>
    </div>
    {(mechanical || authoredCaseBodies.length > 0) && <div className="wb-mechanical-view-controls" role="group" aria-label="Mechanical assembly view">
      <button aria-pressed={view === 'assembled'} onClick={() => setView('assembled')}>Assembled</button>
      <button aria-pressed={view === 'exploded'} onClick={() => setView('exploded')}>Exploded</button>
      <button aria-pressed={view === 'section'} onClick={() => setView('section')}>Section</button>
      {Boolean(mechanical?.gasketSupports?.length) && mechanicalConfiguration && <button disabled={!ready || preparing} aria-pressed={editingGaskets} onClick={() => { setEditingGaskets(value => !value); setEditingMounts(false); setGasketMessage(editingGaskets ? '' : 'Drag directly to any side · resize a red gasket to make it fit'); setView('assembled'); renderer.current?.view('top'); }}>Edit gaskets</button>}
      {Boolean(mechanicalConfiguration ? (mechanicalConfiguration.mount !== 'gasket' && mechanicalConfiguration.mounts.length) || mechanicalConfiguration.closureMounts?.length : authoredCaseBodies.some(body => body.mounts?.length)) && (onCaseMountChange || onGasketChange) && <button disabled={!ready || preparing || (!editingMounts && !preparedCase)} title={!preparedCase ? 'Update the case preview to edit mounts' : undefined} aria-pressed={editingMounts} onClick={() => { setEditingMounts(value => !value); setEditingGaskets(false); setGasketMessage(editingMounts ? '' : 'Drag a case mount to preview its new position'); setView('assembled'); renderer.current?.view('top'); }}>Edit mounts</button>}
      {editingGaskets && activeGasket && <button onClick={unlinkGasket}>Unlink selected support</button>}
    </div>}
    </div>
    {view === 'exploded' && <div className="wb-scene-adjustments"><label>Separation <input aria-label="Exploded separation" type="range" min="0" max="10" step="0.1" value={explodeAmount} onChange={event => setExplodeAmount(Number(event.target.value))} /><output>{explodeAmount.toFixed(1)}×</output></label></div>}
    {view === 'section' && <div className="wb-scene-adjustments"><label>Plane <select aria-label="Section plane" value={sectionPlane} onChange={event => setSectionPlane(event.target.value)}>{['XY','XZ','YZ'].map(plane => <option key={plane}>{plane}</option>)}</select></label><label>Position <input aria-label="Section position" type="range" min="-100" max="100" value={sectionPosition} onChange={event => setSectionPosition(Number(event.target.value))} /><output>{sectionPosition}%</output></label><label><input type="checkbox" checked={showSectionPlane} onChange={event => setShowSectionPlane(event.target.checked)} />Show plane</label></div>}
    <div className="wb-layer-surface wb-assembly-drawing">
      <div className="wb-assembly-viewport">
    <canvas ref={canvas} aria-label="3D PCB assembly. Drag to orbit, scroll to zoom." />
    {preparing && <span role="status" className="wb-scene-preparing">Preparing 3D geometry…</span>}
    <div className="wb-assembly-controls" role="group" aria-label="Assembly camera">
      <button disabled={!ready || preparing} onClick={() => showView('fit')}>Fit</button>
      <button disabled={!ready || preparing} onClick={() => showView('top')}>Top</button>
      <button disabled={!ready || preparing} onClick={() => showView('bottom')}>Bottom</button>
      <button disabled={!ready || preparing} onClick={() => showView('isometric')}>Isometric</button>
    </div>
    {mechanical && <output role="status" className="wb-mechanical-preview-status">{solidsBlocked ? 'Case solids blocked' : generatedBodyCount > 0 && generatedBodyCount === mechanical.case.bodies.length && (!generation || generation.status === 'ready' && generation.revision === mechanical.revision) ? `Generated CAD solids · ${mechanical.case.bodies.length} parts at revision ${mechanical.revision}` : bodies.length ? 'Showing previous geometry' : 'No generated solids'}</output>}
    {gasketMessage && <output className="wb-gasket-message" role="status">{gasketMessage}</output>}
    {view === 'section' && <output className="wb-mechanical-section-label">{sectionPlane} section · {sectionPosition}% from centre</output>}
    <output className="wb-assembly-caption">{selected || `${models.length} / ${board.models.length} models · ${board.thickness} mm PCB`}</output>
    {error && <p className="wb-assembly-error" role="alert">{error}</p>}
      </div>
    <CanvasLayers hidden={hidden} onToggle={toggle} groups={[
      { title: 'Assembly', layers: controls.map(([id, label]) => ({ id, label: ({ plate: 'Plate', 'plate-foam': 'Plate foam', 'bottom-foam': 'Bottom foam', bottom: 'Bottom' } as Record<string, string>)[id] ?? label, available: id === 'PCB' || ['Copper', 'Mask', 'Silkscreen', 'Models', 'Keycaps'].includes(id) || generatedBodyIds.has(id) })) },
      { title: 'Components', layers: board.models.map(model => ({ id: model.id, label: `${model.reference} · ${model.path.split('/').pop()}`, kind: 'part', available: models.some(loaded => loaded.id === model.id), availabilityLabel: 'Missing model' })) },
    ]} />
    </div>
  </div>;
}
