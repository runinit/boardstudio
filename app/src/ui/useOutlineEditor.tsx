import { useEffect, useRef, useState } from 'react';
import type { MouseEvent, RefObject } from 'react';
import { defaultOutlineSettings, type Board, type EditCommand, type OutlineFeature, type ProjectDoc, type SceneDelta, type Vec2 } from '@boardstudio/v2-contracts';
import { getBounds } from './canvasBounds';
import { OutlineInspector } from './OutlineInspector';
import { OutlineControlOverlay } from './OutlineControlOverlay';
import { OutlineDraftPreview } from './OutlineDraftPreview';
import { OutlineDrawingBar, OutlineToolIcon } from './OutlineTools';
import { connectionEndpoint, snapOutline, type OutlineDrawMode, type OutlineSelection } from './outlineEditing';
import { FindingList } from './FindingList';
import { makeId, pointFromEvent, isTyping } from './workbenchGeometry';
import type { Mode } from './workbenchTypes';

type Bounds = ReturnType<typeof getBounds>;
type Inputs = {
  document: ProjectDoc; board?: Board; mode: Mode; projectSession?: number; inspecting: boolean; assembly3d: boolean;
  svgRef: RefObject<SVGSVGElement | null>; viewport: () => { bounds: Bounds; viewBounds: Bounds; width: number };
  emit: (operation: EditCommand['operation'], ids: string[], phase?: EditCommand['phase'], transactionId?: string) => unknown;
  showFinding: (finding: SceneDelta['findings'][number]) => void;
  scene: SceneDelta; onSelect: () => void; onDraw: () => void; onFinish: () => void;
};

export function useOutlineEditor({ document, board: selectedBoard, mode, projectSession, inspecting, assembly3d, svgRef, viewport, emit, showFinding, scene, onSelect, onDraw, onFinish }: Inputs) {
  const [outlineDraft, setOutlineDraft] = useState<Vec2[]>([]);
  const [outlineActive, setOutlineActive] = useState(false);
  const [outlineOperation, setOutlineOperation] = useState<OutlineDrawMode>('add');
  const [outlineSelection,setOutlineSelection] = useState<OutlineSelection|null>(null);
  const [outlineGrid,setOutlineGrid] = useState(1);
  const [outlinePoint,setOutlinePoint] = useState(0);
  const [outlineDragBounds,setOutlineDragBounds] = useState<ReturnType<typeof getBounds>|null>(null);
  const selectedBoardId = selectedBoard?.id ?? '';
  const outlineContext = useRef({ documentId: document.id, projectSession, boardId: selectedBoardId });
  outlineContext.current = { documentId: document.id, projectSession, boardId: selectedBoardId };
  useEffect(() => { setOutlineDraft([]); setOutlineActive(false); setOutlineSelection(null); setOutlineDragBounds(null); }, [mode, selectedBoardId, document.id, projectSession]);
  const outlineFeature = inspecting && mode === 'Design' && !assembly3d ? document.outline.find(feature => feature.id === outlineSelection?.featureId && selectedBoard?.outlineIds.includes(feature.id) && (feature.kind !== 'part-envelope' || feature.connections?.some(connection => connection.id === outlineSelection.connectionId))) : undefined;
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
    const snapped = event.altKey ? point : snapOutline(point, outlineGrid);
    setOutlineDraft((current) => current.some((p) => p.x === snapped.x && p.y === snapped.y) ? current : [...current, snapped]);
  };

  const saveOutline = (feature: OutlineFeature) => {
    if (!selectedBoard) return;
    const outlineIds = selectedBoard.outlineIds.includes(feature.id) ? selectedBoard.outlineIds : [...selectedBoard.outlineIds, feature.id];
    const boards = document.boards.map((board) => board.id === selectedBoard.id ? { ...board, outlineIds } : board);
    emit({ kind: 'replace-document', document: { ...document, outline: [...document.outline, feature], boards } }, [feature.id, selectedBoard.id]);
  };
  const cancelOutlineDrawing = () => {
    setOutlineDraft([]); setOutlineActive(false); onFinish();
  };

  const finishOutline = () => {
    if (!outlineActive || outlineDraft.length < (outlineOperation==='connect'?2:3) || !selectedBoard) return;
    let selection: OutlineSelection;
    if (outlineOperation==='connect') {
      const envelope=document.outline.find(feature=>selectedBoard.outlineIds.includes(feature.id)&&feature.kind==='part-envelope');
      if (!envelope || envelope.kind!=='part-envelope') return;
      const eligible=document.parts.filter(part=>envelope.partIds.includes(part.id)&&!part.outline?.excluded);
      const connection={id:makeId(),width:(envelope.settings??defaultOutlineSettings).bridgeWidth,points:outlineDraft.map((at,index)=>index===0||index===outlineDraft.length-1?connectionEndpoint(at,eligible):{at})};
      emit({kind:'set-outline',feature:{...envelope,connections:[...envelope.connections??[],connection]}},[envelope.id]);
      selection={featureId:envelope.id,connectionId:connection.id};
    } else {
      const feature: OutlineFeature = {id:makeId(),kind:'polygon',points:outlineDraft,operation:outlineOperation};
      saveOutline(feature);
      selection={featureId:feature.id};
    }
    setOutlineDraft([]);
    setOutlineActive(false);
    setOutlineSelection(selection);
    setOutlinePoint(0);
    onFinish();
  };

  const inspector = () => <><OutlineInspector document={document} board={selectedBoard}
      onChange={(next, ids) => emit({ kind: 'replace-document', document: next }, ids)}
      selection={outlineSelection} selectedPoint={outlinePoint} onSelectPoint={setOutlinePoint} grid={outlineGrid} onGrid={setOutlineGrid}
      onSelect={select}
      onDraw={beginDrawing} /><div className="wb-findings-head"><h3 className="wb-subtitle">Findings</h3></div><FindingList document={document} onShow={showFinding} findings={scene.findings} /></>;

  const overlay = () => {
    const { bounds, viewBounds } = viewport();
    return <>
      {outlineActive && <g transform="scale(1,-1)" pointerEvents="none"><OutlineDraftPreview points={outlineDraft} mode={outlineOperation} grid={outlineGrid} scale={viewBounds.width/(svgRef.current?.getBoundingClientRect().width||800)} /></g>}
            {outlineFeature && <OutlineControlOverlay key={`${document.id}/${projectSession}/${selectedBoardId}/${outlineSelection?.featureId}/${outlineSelection?.connectionId??''}`} feature={outlineFeature} connectionId={outlineSelection?.connectionId} selectedPoint={outlinePoint} onSelectPoint={setOutlinePoint} parts={document.parts} grid={outlineGrid} onDragChange={active=>setOutlineDragBounds(active?bounds:null)} scale={viewBounds.width/(svgRef.current?.getBoundingClientRect().width||800)} onChange={(feature,phase,transactionId)=>{
              if(outlineContext.current.documentId===document.id && outlineContext.current.projectSession===projectSession && outlineContext.current.boardId===selectedBoardId) emit({kind:'set-outline',feature},[feature.id],phase,transactionId);
            }} />}    </>;
  };
  const bar = () => <>
          {outlineActive && <OutlineDrawingBar mode={outlineOperation} count={outlineDraft.length} grid={outlineGrid} onGrid={setOutlineGrid} onUndo={() => setOutlineDraft(points => points.slice(0, -1))} onCancel={cancelOutlineDrawing} onFinish={finishOutline} />}
  </>;
  const toolbar = () => outlineFeature && (<div className="wb-outline-canvas-context"><OutlineToolIcon kind={outlineFeature.kind === 'part-envelope' ? 'connect' : outlineFeature.operation} /><strong>Outline points</strong><span>{outlineGrid} mm snap</span><button aria-label="Finish editing outline points" onClick={() => setOutlineSelection(null)}>Done</button></div>);
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
  return { active: outlineActive, setActive: setOutlineActive, feature: outlineFeature, grid: outlineGrid, dragBounds: outlineDragBounds,
    inspector, overlay, bar, toolbar, click, finish: finishOutline, clear, clearSelection: () => setOutlineSelection(null), handleKey,
    gridSpacing: () => outlineGrid * Math.max(1, Math.ceil(8 * viewport().viewBounds.width / Math.max(viewport().width, 1) / outlineGrid)),
  };
}
