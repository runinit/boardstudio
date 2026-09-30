import { useEffect, useRef, useState } from 'react';
import type { MouseEvent, RefObject, ReactNode } from 'react';
import { defaultOutlineSettings, type Board, type EditCommand, type EditOperation, type EditPhase, type Part, type PartDefinition, type OutlineFeature, type ProjectDoc, type SceneDelta, type Vec2 } from '@boardstudio/v2-contracts';
import { getBounds } from './canvasBounds';
import { OutlineInspector } from './OutlineInspector';
import { OutlineControlOverlay } from './OutlineControlOverlay';
import { OutlineDraftPreview } from './OutlineDraftPreview';
import { OutlineDrawingBar, OutlineToolIcon, snapGridLabel } from './OutlineTools';
import { connectionEndpoint, type OutlineDrawMode, type OutlineSelection } from './outlineEditing';
import { editableOutlineFeatures, nextOutlineName, outlineVersion } from './boardOutlines';
import { snapOutlinePoint, type OutlineSnap, type OutlineSnapContext } from './outlineSnapping';
import { snapOrigin } from './placementGeometry';
import { outlineWorld } from './outlineEditing';
import { FindingList } from './FindingList';
import { makeId, pointFromEvent, isTyping } from './workbenchGeometry';
import type { Mode } from './workbenchTypes';

type Bounds = ReturnType<typeof getBounds>;
type Inputs = {
  document: ProjectDoc; board?: Board; mode: Mode; projectSession?: number; inspecting: boolean; assembly3d: boolean;
  svgRef: RefObject<SVGSVGElement | null>; viewport: () => { bounds: Bounds; viewBounds: Bounds; width: number };
  emit: (operation: EditCommand['operation'], ids: string[], phase?: EditCommand['phase'], transactionId?: string) => unknown;
  showFinding: (finding: SceneDelta['findings'][number]) => void;
  snap: () => { grid: number; selection: number; pitch: Vec2; enabled: boolean; parts: Part[]; definitions: Map<string, PartDefinition>; contours: SceneDelta['contours'] }; onGrid: (value: number) => void; snapControls: () => ReactNode;
  bridge: string; onCloseBridge: () => void; onFocusGap: (gap: SceneDelta['contours'][number]) => void; onVersion: () => void;
  scene: SceneDelta; onSelect: () => void; onDraw: () => void; onFinish: () => void;
};

export function useOutlineEditor({ document, board: selectedBoard, mode, projectSession, inspecting, assembly3d, svgRef, viewport, emit, showFinding, scene, onSelect, onDraw, onFinish, snap, onGrid, snapControls, bridge, onCloseBridge, onFocusGap, onVersion }: Inputs) {
  const [outlineDraft, setOutlineDraft] = useState<Vec2[]>([]);
  const [outlineActive, setOutlineActive] = useState(false);
  const [outlineOperation, setOutlineOperation] = useState<OutlineDrawMode>('add');
  const [outlineSelection,setOutlineSelection] = useState<OutlineSelection|null>(null);
  const { grid: outlineGrid, selection: snapFraction } = snap();
  const outlineSnapMemory = useRef<OutlineSnap | undefined>(undefined);
  const outlineSnapTargets = useRef<Vec2[][] | undefined>(undefined);
  const [outlinePoint,setOutlinePoint] = useState(0);
  const [outlineDragBounds,setOutlineDragBounds] = useState<ReturnType<typeof getBounds>|null>(null);
  const selectedBoardId = selectedBoard?.id ?? '';
  const outlineContext = useRef({ documentId: document.id, projectSession, boardId: selectedBoardId });
  outlineContext.current = { documentId: document.id, projectSession, boardId: selectedBoardId };
  useEffect(() => { setOutlineDraft([]); setOutlineActive(false); setOutlineSelection(null); setOutlineDragBounds(null); }, [mode, selectedBoardId, document.id, projectSession]);
  const boardOutlineScene = scene.boardOutlineScenes?.find(entry => entry.boardId === selectedBoard?.id);
  const activeOutlineVersion = selectedBoard ? outlineVersion(document, selectedBoard.id) : undefined;
  const outlineFeatures = selectedBoard ? editableOutlineFeatures(document, selectedBoard, boardOutlineScene) : [];
  const outlineFeature = inspecting && mode === 'Design' && !assembly3d
    ? outlineSelection?.contour !== undefined ? outlineFeatures[outlineSelection.contour]
      : outlineFeatures.find(feature => feature.id === outlineSelection?.featureId)
        ?? document.outline.find(feature => feature.id === outlineSelection?.featureId && selectedBoard?.outlineIds.includes(feature.id) && feature.kind === 'part-envelope' && feature.connections?.some(connection => connection.id === outlineSelection.connectionId))
    : undefined;
  const select = (selection: OutlineSelection | null) => { setOutlineSelection(selection); setOutlinePoint(0); setOutlineActive(false); setOutlineDraft([]); onSelect(); };
  const beginDrawing = (operation: OutlineDrawMode) => { onDraw(); setOutlineOperation(operation); setOutlineSelection(null); setOutlineDraft([]); setOutlineActive(true); };
  const click = (event: MouseEvent<SVGSVGElement>) => {
    const { viewBounds } = viewport();
    if (mode !== 'Design' || !outlineActive || event.button !== 0) return;
    const point = pointFromEvent(event, svgRef.current);
    if (!point) return;
    if (event.detail > 1) return;
    if (outlineOperation !== 'connect' && outlineDraft.length >= 3 && Math.hypot(point.x - outlineDraft[0].x, point.y - outlineDraft[0].y) < viewBounds.width / (svgRef.current?.getBoundingClientRect().width || 800) * 8) {
      finishOutline(); return;
    }
    const snapped = snapOutlineGeometry(point, { anchor: outlineDraft.at(-1), previous: outlineDraft.at(-2) }, event.altKey).at;
    setOutlineDraft((current) => current.some((p) => p.x === snapped.x && p.y === snapped.y) ? current : [...current, snapped]);
  };

  const saveOutline = (feature: OutlineFeature) => {
    if (!selectedBoard) return;
    if (!activeOutlineVersion) return emit({ kind: 'copy-outline', boardId: selectedBoard.id, versionId: makeId(), name: nextOutlineName(document, selectedBoard.id), feature }, [selectedBoard.id]);
    return emit({ kind: 'replace-document', document: { ...document, boardOutlines: document.boardOutlines?.map(state => state.boardId !== selectedBoard.id ? state : {
      ...state, versions: state.versions.map(version => version.id !== activeOutlineVersion.id ? version : {
        ...version, geometry: { ...version.geometry, features: [...version.geometry.features, feature] },
      }),
    }) } }, [feature.id, selectedBoard.id]);
  };
  const editOutlineFeature = (feature: OutlineFeature, phase: EditPhase = 'commit', transactionId = makeId()) => {
    if (!selectedBoard) return;
    if (!activeOutlineVersion && feature.id.startsWith('outline-source:') && feature.kind === 'polygon' && outlineSelection?.contour !== undefined) {
      return emit({ kind: 'copy-outline', boardId: selectedBoard.id, versionId: `fixed-${transactionId}`, name: nextOutlineName(document, selectedBoard.id), edit: { contour: outlineSelection.contour, points: feature.points } }, [selectedBoard.id], phase, transactionId);
    }
    return emit({ kind: 'set-outline', feature }, [feature.id], phase, transactionId);
  };
  const changeOutlineVersion = (operation: EditOperation) => {
    if (operation.kind === 'select-outline' && operation.versionId === (activeOutlineVersion?.id ?? null)) return;
    onVersion();
    setOutlineDraft([]); setOutlineActive(false); setOutlineSelection(null);
    emit(operation, selectedBoard ? [selectedBoard.id] : []);
  };
  const snapOutlineGeometry = (point:Vec2, context:OutlineSnapContext={}, free=false):OutlineSnap => {
    const { viewBounds, width: canvasWidth } = viewport();
    const { pitch: snapPitch, enabled: geometrySnap, parts: visibleParts, definitions, contours: visibleContours } = snap();
    const tolerance=viewBounds.width / Math.max(canvasWidth,1)*7;
    const grid={x:snapFraction<0?-snapFraction:snapPitch.x*snapFraction,y:snapFraction<0?-snapFraction:snapPitch.y*snapFraction};
    const result=snapOutlinePoint(point,{...context,free,grid,tolerance,enabled:geometrySnap,paths:outlineSnapTargets.current??boardOutlineScene?.sourceContours.map(contour=>contour.points)??visibleContours.map(contour=>contour.points)},outlineSnapMemory.current);
    if(!free&&geometrySnap&&!result.guides.length){
      const component=snapOrigin(point,visibleParts,definitions,tolerance);
      if(component) result.at=component.at;
    }
    outlineSnapMemory.current=result;
    return result;
  };
  const cancelOutlineDrawing = () => {
    setOutlineDraft([]); setOutlineActive(false); onFinish();
  };

  const finishOutline = async () => {
    if (!outlineActive || outlineDraft.length < (outlineOperation==='connect'?2:3) || !selectedBoard) return;
    let selection: OutlineSelection;
    if (outlineOperation==='connect') {
      const envelope=document.outline.find(feature=>selectedBoard.outlineIds.includes(feature.id)&&feature.kind==='part-envelope');
      if (!envelope || envelope.kind!=='part-envelope') return;
      const eligible=document.parts.filter(part=>envelope.partIds.includes(part.id)&&!part.outline?.excluded);
      const connection={id:makeId(),width:(envelope.settings??defaultOutlineSettings).bridgeWidth,points:outlineDraft.map((at,index)=>index===0||index===outlineDraft.length-1?connectionEndpoint(at,eligible):{at})};
      if (await emit({kind:'set-outline',feature:{...envelope,connections:[...envelope.connections??[],connection]}},[envelope.id]) === false) return;
      selection={featureId:envelope.id,connectionId:connection.id};
    } else {
      const feature: OutlineFeature = {id:makeId(),kind:'polygon',points:outlineDraft,operation:outlineOperation};
      if (await saveOutline(feature) === false) return;
      selection={featureId:feature.id, contour: outlineFeatures.length};
    }
    setOutlineDraft([]);
    setOutlineActive(false);
    setOutlineSelection(selection);
    setOutlinePoint(0);
    onFinish();
  };

  const inspector = () => <><OutlineInspector document={document} board={selectedBoard}
      onChange={(next, ids) => emit({ kind: 'replace-document', document: next }, ids)}
      outlineScene={boardOutlineScene} onVersion={changeOutlineVersion} onFeatureEdit={feature => editOutlineFeature(feature)}
      bridge={boardOutlineScene?.bridges.find(item => item.id === bridge)} onCloseBridge={onCloseBridge}
      onFocusGap={gap => onFocusGap({ hole: false, points: gap.points.map(point => outlineWorld(point.at, document.parts.find(part => part.id === point.partId))) })}
      selection={outlineSelection} selectedPoint={outlinePoint} onSelectPoint={setOutlinePoint} grid={outlineGrid} gridSelection={snapFraction} onGrid={onGrid}
      onSelect={select}
      onDraw={beginDrawing} /><div className="wb-findings-head"><h3 className="wb-subtitle">Findings</h3></div><FindingList document={document} onShow={showFinding} findings={scene.findings} /></>;

  const overlay = () => {
    const { bounds, viewBounds } = viewport();
    return <>
      {outlineActive && <g transform="scale(1,-1)" pointerEvents="none"><OutlineDraftPreview points={outlineDraft} mode={outlineOperation} grid={outlineGrid} snap={snapOutlineGeometry} scale={viewBounds.width/(svgRef.current?.getBoundingClientRect().width||800)} /></g>}
            {outlineFeature && <OutlineControlOverlay key={`${document.id}/${projectSession}/${selectedBoardId}/${outlineSelection?.contour ?? outlineSelection?.featureId}/${outlineSelection?.connectionId??''}`} feature={outlineFeature} connectionId={outlineSelection?.connectionId} selectedPoint={outlinePoint} onSelectPoint={setOutlinePoint} parts={document.parts} grid={outlineGrid} snap={snapOutlineGeometry} onDragChange={active=>{ setOutlineDragBounds(active?bounds:null); outlineSnapTargets.current=active?boardOutlineScene?.sourceContours.map(contour=>contour.points):undefined; outlineSnapMemory.current=undefined; }} scale={viewBounds.width/(svgRef.current?.getBoundingClientRect().width||800)} onCancel={() => emit({ kind: 'select-outline', boardId: selectedBoardId, versionId: activeOutlineVersion?.id ?? null }, [selectedBoardId], 'preview')} onChange={(feature,phase,transactionId)=>{
              if(outlineContext.current.documentId===document.id && outlineContext.current.projectSession===projectSession && outlineContext.current.boardId===selectedBoardId) editOutlineFeature(feature,phase,transactionId);
            }} />}    </>;
  };
  const bar = () => <>
          {outlineActive && <OutlineDrawingBar mode={outlineOperation} count={outlineDraft.length} grid={outlineGrid} selection={snapFraction} onGrid={onGrid} snapControls={snapControls()} onUndo={() => setOutlineDraft(points => points.slice(0, -1))} onCancel={cancelOutlineDrawing} onFinish={finishOutline} />}
  </>;
  const toolbar = () => outlineFeature && (<div className="wb-outline-canvas-context"><OutlineToolIcon kind={outlineFeature.kind === 'part-envelope' ? 'connect' : outlineFeature.operation} /><strong>Outline points</strong><span>{snapGridLabel(snapFraction)} grid</span>{snapControls()}<button aria-label="Finish editing outline points" onClick={() => setOutlineSelection(null)}>Done</button></div>);
  const clear = () => { setOutlineDraft([]); setOutlineActive(false); setOutlineSelection(null); };
  const handleKey = (event: KeyboardEvent) => {
    if (!outlineActive || isTyping(event.target)) return false;
    if (event.key === 'Escape') { event.preventDefault(); cancelOutlineDrawing(); return true; }
    if (event.key === 'Backspace' || event.key === 'Delete' || ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'z')) {
      event.preventDefault(); setOutlineDraft(points => points.slice(0, -1)); return true;
    }
    if (event.key === 'Enter') { event.preventDefault(); finishOutline(); return true; }
    return false;
  };
  return { scene: boardOutlineScene, changeVersion: changeOutlineVersion, active: outlineActive, setActive: setOutlineActive, feature: outlineFeature, grid: outlineGrid, dragBounds: outlineDragBounds,
    inspector, overlay, bar, toolbar, click, finish: finishOutline, clear, clearSelection: () => setOutlineSelection(null), handleKey,
    gridSpacing: () => Math.max(outlineGrid, .1) * Math.max(1, Math.ceil(8 * viewport().viewBounds.width / Math.max(viewport().width, 1) / Math.max(outlineGrid, .1))),
  };
}
