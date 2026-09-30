import { displayIds, type useCaseDisplay } from './caseDisplay';
import type { Board, BoardOutlineScene, MechanicalAssembly, ProjectDoc } from '@boardstudio/v2-contracts';
import type { Dispatch, KeyboardEvent, SetStateAction } from 'react';
import { useMemo } from 'react';
import type {
  CaseBody,
  Layout,
  Matrix,
  MatrixCell,
  Part,
  PartDefinition
} from '../../../contracts/src/index';
import type { MatrixProjection } from './matrixGeometry';
import { matrixCellId } from './matrixGeometry';
import type { TreeEntry } from './WorkbenchTree';
import { Mode, SelectionScope } from './workbenchTypes';

type Inputs = {
  caseDisplay: ReturnType<typeof useCaseDisplay>;
  selectedCaseInstanceId?: string;
  onSelectCaseInstance: (id: string, boardId: string) => void;
  document: ProjectDoc;
  mechanicalAssembly?: MechanicalAssembly;
  selectedBoardId: string;
  expandedTree: Set<string>;
  selectedBoard: Board;
  visibleParts: Part[];
  toggleTree: (id: string) => void;
  setScope: Dispatch<SetStateAction<SelectionScope | null>>;
  setSelected: Dispatch<SetStateAction<string[]>>;
  setOutlineSettingsOpen: Dispatch<SetStateAction<boolean>>;
  visibleMatrices: Matrix[];
  boardLayouts: Layout[];
  memberMaps: Map<string, Map<string, string>>;
  treeParts: Map<string, Part>;
  scope: SelectionScope | null;
  selectScope: (next: SelectionScope) => void;
  matrixScenes: Map<string, MatrixProjection>;
  treeGrouping: "row" | "column";
  matrixCellOverrides: Map<string, Map<string, MatrixCell>>;
  definitions: Map<string, PartDefinition>;
  nudgePart: (event: KeyboardEvent<Element>, part: Part) => void;
  treeVisibleParts: Part[];
  mode: Mode;
  changeMode: (next: Mode) => void;
  setExpandedTree: Dispatch<SetStateAction<Set<string>>>;
  matrixMap: Map<string, Matrix>;
  selected: string[];
  activeCaseBody: CaseBody | undefined;
  setCaseBodyId: Dispatch<SetStateAction<string>>;
  setRightOpen: Dispatch<SetStateAction<boolean>>;
  selectedMechanicalLayer: string;
  outlineScene?: BoardOutlineScene;
  outlineOpen: boolean;
  selectedBridgeId: string;
  onOutline: (versionId?: string | null) => void;
  onBridge: (id: string) => void;
  setSelectedMechanicalLayer: Dispatch<SetStateAction<string>>
};

export function useWorkbenchTree({ caseDisplay, selectedCaseInstanceId, onSelectCaseInstance, document, mechanicalAssembly, selectedBoardId, expandedTree, selectedBoard, visibleParts, toggleTree, setScope, setSelected, setOutlineSettingsOpen, visibleMatrices, boardLayouts, memberMaps, treeParts, scope, selectScope, matrixScenes, treeGrouping, matrixCellOverrides, definitions, nudgePart, treeVisibleParts, mode, changeMode, setExpandedTree, matrixMap, selected, activeCaseBody, setCaseBodyId, setRightOpen, selectedMechanicalLayer, setSelectedMechanicalLayer, outlineScene, outlineOpen, selectedBridgeId, onOutline, onBridge }: Inputs) {
  const treeEntries = useMemo<TreeEntry[]>(() => {
    const rows: TreeEntry[] = [];
    if (mode === 'Case') {
      const choose = (id: string) => { setSelectedMechanicalLayer(id); setScope(null); setSelected([]); setRightOpen(true); };
      const entries: TreeEntry[] = [];
      const instances = document.hardware?.instances.length ? document.hardware.instances : document.boards.map(board => ({id:board.id, boardId:board.id, name:board.name}));
      for (const instance of instances) {
        const active = instance.id === (selectedCaseInstanceId ?? selectedBoardId);
        const hidden = caseDisplay.read(instance.id).hidden.includes('Assembly');
        const rootKey = `case-assembly:${instance.id}`;
        const collapsed = expandedTree.has(rootKey);
        entries.push({id:`case-assembly:${instance.id}`,label:`${instance.name.replace(/ half$| PCB$/i, '').replace(/^./, letter => letter.toUpperCase())} case assembly`,kind:'case',level:0,
          selected:active && !selectedMechanicalLayer, expandable:true,expanded:active && !collapsed,
          onToggle:() => { if (!active) onSelectCaseInstance(instance.id, instance.boardId); else toggleTree(rootKey); },
          onSelect:() => { onSelectCaseInstance(instance.id, instance.boardId); if (collapsed) toggleTree(rootKey); choose(''); },
          hidden, onVisibility:() => caseDisplay.toggle('Assembly', instance.id)});
        if (!active || collapsed) continue;
      const pcbKey = `case-pcb:${instance.id}`;
      const pcbOpen = expandedTree.has(pcbKey);
      const bodies = document.caseBodies.filter(body => body.boardId === selectedBoardId);
      const layers = new Set((mechanicalAssembly?.stack ?? []).map(layer => layer.id));
      if (mechanicalAssembly?.gasketSupports.length) layers.add('retainer');
      const labels: Record<string,string> = {plate:'Plate',bottom:'Bottom case',retainer:'Top case','plate-foam':'Plate foam','bottom-foam':'Bottom foam','middle-frame':'Middle frame',battery:'Battery'};
      for (const id of layers) if (id !== 'pcb' && !id.startsWith('gasket:')) entries.push({
        id:`case-generated:${id}`,label:labels[id] ?? id,kind:'case',level:1,
        selected:selectedMechanicalLayer === id,onSelect:() => choose(id),
      });
      const supports = mechanicalAssembly?.gasketSupports ?? [];
      if (supports.length) {
        const groupKey = `case-gaskets:${selectedBoardId}`;
        const open = expandedTree.has(groupKey);
        entries.push({ id:groupKey,label:'Gaskets',detail:`${supports.length} pairs`,kind:'components',level:1,
          expandable:true,expanded:open,selected:selectedMechanicalLayer === 'gaskets',
          onToggle:() => toggleTree(groupKey),onSelect:() => choose('gaskets') });
        if (open) supports.forEach((support,index) => {
          const id = `gasket:${support.id}:lower`;
          const side = Math.abs(support.normal.x) > Math.abs(support.normal.y) ? support.normal.x > 0 ? 'Right' : 'Left' : support.normal.y > 0 ? 'Top' : 'Bottom';
          entries.push({id:`case-generated:${id}`,label:`Gasket ${index+1} · ${side}`, detail:`${support.length} × ${support.width} mm${support.fitError ? ' · Does not fit' : ''}`,
            kind:'case',level:2,selected:selectedMechanicalLayer === id || selectedMechanicalLayer === `gasket:${support.id}:upper`,onSelect:() => choose(id)});
        });
      }
      if (!mechanicalAssembly) for (const body of bodies) entries.push({id:`case:${body.id}`,label:body.name,kind:'case',level:1,
        selected:activeCaseBody?.id === body.id,onSelect:() => {setCaseBodyId(body.id);setRightOpen(true);} });
      entries.push({ id:pcbKey, label:'PCB', detail:`${visibleParts.length} components`, kind:'pcb', level:1,
        expandable:true, expanded:pcbOpen, selected:selectedMechanicalLayer === 'pcb',
        onToggle:() => toggleTree(pcbKey), onSelect:() => choose('pcb') });
      if (pcbOpen) for (const part of treeVisibleParts) entries.push({ id:`Case:${part.id}`,label:part.reference,
        detail:definitions.get(part.definitionId)?.name ?? 'Component', kind:'component',level:2,
        selected:scope?.kind === 'component' && scope.partId === part.id,
        hidden:caseDisplay.current.hidden.includes(part.reference), onVisibility:() => caseDisplay.toggle(part.reference),
        onSelect:() => { choose('pcb'); selectScope({kind:'component',partId:part.id}); } });
      for (const entry of entries.filter(entry => entry.level > 0 && !entry.onVisibility)) {
        const id = entry.id === pcbKey ? 'pcb' : entry.id.startsWith('case-gaskets:') ? 'Gaskets' : entry.id.replace(/^case-generated:|^case:|^Case:/, '');
        entry.hidden = displayIds(id).every(part => caseDisplay.current.hidden.includes(part));
        entry.onVisibility = () => caseDisplay.toggle(id);
      }
      }
      return entries;
    }
    const boardKey = `board:${selectedBoardId}`;
    const boardExpanded = expandedTree.has(boardKey);
    rows.push({
      id: boardKey,
      label: selectedBoard?.name ?? 'Board',
      detail: `${visibleParts.length} parts`,
      level: 0,
      kind: 'board',
      expandable: true,
      expanded: boardExpanded,
      onToggle: () => toggleTree(boardKey),
      onSelect: () => { setScope(null); setSelected([]); setOutlineSettingsOpen(false); if (!boardExpanded) toggleTree(boardKey); },
    });
    if (!boardExpanded) return rows;
    const outlineKey = `outline:${selectedBoardId}`;
    const outlineExpanded = expandedTree.has(outlineKey);
    const outline = document.boardOutlines?.find(state => state.boardId === selectedBoardId);
    const activeVersion = outline?.versions.find(version => version.id === outline.activeVersionId);
    const bridges = outlineScene?.bridges ?? [];
    const bridgeEntry = (bridge: typeof bridges[number], index: number, level: number, context: string): TreeEntry => ({
      id: `${context}:bridge:${bridge.id}`, label: `Bridge ${index + 1}`, detail: `${bridge.width} mm`, kind: 'bridge', level,
      selected: outlineOpen && selectedBridgeId === bridge.id, onSelect: () => onBridge(bridge.id),
    });
    const outlineRows: TreeEntry[] = [{ id: outlineKey, label: 'Outline', detail: activeVersion?.name ?? 'Generated', kind: 'outline', level: 1,
      selected: outlineOpen && !selectedBridgeId, expandable: true, expanded: outlineExpanded,
      onToggle: () => toggleTree(outlineKey), onSelect: () => { if (!outlineExpanded) toggleTree(outlineKey); onOutline(); },
    }];
    if (outlineExpanded) {
      outlineRows.push({ id: `${outlineKey}:generated`, label: 'Generated', detail: !activeVersion ? 'Active' : 'Follows layout', kind: 'outline-version', level: 2,
        selected: outlineOpen && !activeVersion && !selectedBridgeId, onSelect: () => onOutline(null) });
      for (const version of outline?.versions ?? []) outlineRows.push({ id: `${outlineKey}:version:${version.id}`, label: version.name,
        detail: version.id === activeVersion?.id ? 'Active' : 'Fixed', kind: 'outline-version', level: 2,
        selected: outlineOpen && version.id === activeVersion?.id && !selectedBridgeId, onSelect: () => onOutline(version.id) });
      outlineRows.push(...bridges.map((bridge, index) => bridgeEntry(bridge, index, 2, outlineKey)));
    }
    if (mode === 'PCB') {
      const entries: TreeEntry[] = [rows[0], ...outlineRows];
      for (const part of treeVisibleParts) entries.push({
        id: `${mode}:${part.id}`, label: part.reference,
        detail: mode === 'PCB' ? `${definitions.get(part.definitionId)?.name ?? 'Component'} · ${part.side}` : `${definitions.get(part.definitionId)?.name ?? 'Part'} · Reference`,
        kind: 'component', level: 1, selected: scope?.kind === 'component' && scope.partId === part.id,
        onSelect: () => selectScope({ kind: 'component', partId: part.id }),
      });
      return entries;
    }
    const memberIds = new Set<string>();
    for (const matrix of visibleMatrices) {
      for (const id of matrix.partIds) memberIds.add(id);
      const matrixKey = `matrix:${matrix.id}`;
      const half = boardLayouts.find((layout) => layout.matrixId === matrix.id);
      const isExpanded = expandedTree.has(half ? `half:${half.id}` : matrixKey);
      rows.push({
        id: matrixKey,
        label: matrix.name?.trim() || `Matrix ${document.matrices.indexOf(matrix) + 1}`,
        detail: `${[...(memberMaps.get(matrix.id)?.values() ?? [])].filter((id) => treeParts.has(id)).length} keys`,
        level: 1,
        kind: 'matrix',
        expandable: true,
        expanded: isExpanded,
        selected: scope?.kind === 'matrix' && scope.matrixId === matrix.id,
        onToggle: () => toggleTree(matrixKey),
        onSelect: () => selectScope({ kind: 'matrix', matrixId: matrix.id }),
      });
      if (isExpanded) rows.push(...bridges.flatMap((bridge, index) => bridge.matrixIds.includes(matrix.id) ? [bridgeEntry(bridge, index, 2, matrixKey)] : []));
      const projection = matrixScenes.get(matrix.id)?.scene;
      if (!isExpanded || !projection) continue;
      const columnCount = projection.columns.length;
      const rowCount = projection.cells.length / columnCount;
      const groupCount = treeGrouping === 'column' ? columnCount : rowCount;
      const keyCount = treeGrouping === 'column' ? rowCount : columnCount;
      for (let group = 0; group < groupCount; group++) {
        const groupKey = `${treeGrouping}:${matrix.id}:${group}`;
        const groupExpanded = expandedTree.has(groupKey);
        const enabledCount = Array.from({ length: keyCount }, (_, index) => treeParts.has(memberMaps.get(matrix.id)?.get(treeGrouping === 'column' ? `${index}:${group}` : `${group}:${index}`) ?? '')).filter(Boolean).length;
        rows.push({
          id: groupKey, label: `${treeGrouping === 'column' ? 'Column' : 'Row'} ${group + 1}`,
          detail: `${enabledCount} keys`, level: 2, kind: treeGrouping, expandable: true, expanded: groupExpanded,
          selected: scope?.kind === treeGrouping && scope.matrixId === matrix.id && scope[treeGrouping] === group,
          onToggle: () => toggleTree(groupKey),
          onSelect: () => selectScope({ kind: treeGrouping, matrixId: matrix.id, [treeGrouping]: group }),
        });
        if (!groupExpanded) continue;
        for (let index = 0; index < keyCount; index++) {
          const row = treeGrouping === 'column' ? index : group;
          const column = treeGrouping === 'column' ? group : index;
          const part = treeParts.get(memberMaps.get(matrix.id)?.get(`${row}:${column}`) ?? '');
          const cell = matrixCellOverrides.get(matrix.id)?.get(`${row}:${column}`);
          const keyId = `key:${matrix.id}:${row}:${column}`;
          const components = [
            ...(cell?.assemblies ?? []).map((assembly) => ({ id: assembly.id, definitionId: assembly.definitionId, label: definitions.get(assembly.definitionId)?.name ?? 'Component' })),
          ];
          rows.push({
            id: keyId,
            label: `Key ${column + 1}.${row + 1}`,
            detail: !part ? 'Empty slot' : part.reference,
            level: 3,
            kind: 'key',
            expandable: components.length > 0,
            expanded: expandedTree.has(keyId),
            selected: scope?.kind === 'key' && scope.matrixId === matrix.id && scope.row === row && scope.column === column,
            onToggle: () => toggleTree(keyId),
            onSelect: () => selectScope({ kind: 'key', matrixId: matrix.id, row, column }),
            onKeyDown: part ? (event) => nudgePart(event, part) : undefined,
          });
          if (expandedTree.has(keyId)) {
            for (const component of components) {
              const partId = `${memberMaps.get(matrix.id)?.get(`${row}:${column}`) ?? matrixCellId(matrix.id, row, column)}/${component.id}`;
              rows.push({
                id: `component:${partId}`,
                label: component.label,
                detail: component.id === 'diode' ? 'Automatic companion' : 'Cell component',
                level: 4,
                kind: 'component',
                selected: scope?.kind === 'component' && scope.partId === partId,
                onSelect: () => selectScope({ kind: 'component', matrixId: matrix.id, row, column, partId }),
              });
            }
          }
        }
      }

    }
    for (const part of treeVisibleParts) {
      if (memberIds.has(part.id)) continue;
      rows.push({
        id: `component:${part.id}`,
        label: part.reference,
        detail: `${definitions.get(part.definitionId)?.name ?? 'Component'} · ${part.pose.at.x.toFixed(1)}, ${part.pose.at.y.toFixed(1)}`,
        level: 1,
        kind: 'component',
        selected: scope?.kind === 'component' && scope.partId === part.id,
        onSelect: () => selectScope({ kind: 'component', partId: part.id }),
        onKeyDown: (event) => nudgePart(event, part),
      });
    }
    const splitAxis = boardLayouts.find((layout) => layout.mirrorLink)?.mirrorLink?.axisX;
    const groups = new Map<string, TreeEntry[]>();
    const componentsByLayout = new Map<string, TreeEntry[]>();
    let owner: Layout | undefined;
    for (const entry of rows.slice(1)) {
      if (entry.kind === 'matrix') owner = boardLayouts.find((layout) => `matrix:${layout.matrixId}` === entry.id);
      if (entry.level === 1 && entry.kind === 'component') owner = boardLayouts.find((layout) => layout.partIds.some((id) => `component:${id}` === entry.id));
      const key = owner?.id ?? '';
      const children = groups.get(key) ?? [];
      const wrapped = {
        ...entry, level: owner ? Math.max(2, entry.level) : entry.level + 1, selected: mode === 'Design' && entry.selected,
        onSelect: () => { changeMode('Design'); entry.onSelect(); }
      };
      if (splitAxis !== undefined && entry.level === 1 && entry.kind === 'component' && owner) {
        const components = componentsByLayout.get(owner.id) ?? [];
        components.push(wrapped);
        componentsByLayout.set(owner.id, components);
      } else {
        if (!owner || entry.kind !== 'matrix') children.push(wrapped);
        groups.set(key, children);
      }
    }
    const result: TreeEntry[] = [rows[0], ...outlineRows];
    const layoutKey = `layout:${selectedBoardId}`;
    const layoutExpanded = expandedTree.has(layoutKey);
    if (groups.has('') || boardLayouts.length === 0) result.push({
      id: layoutKey, label: 'Layout', kind: 'layout', level: 1, expandable: true,
      expanded: layoutExpanded, selected: mode === 'Design' && !scope,
      onToggle: () => toggleTree(layoutKey), onSelect: () => { changeMode('Design'); setExpandedTree((current) => new Set([...current, layoutKey])); },
    }, ...(layoutExpanded ? groups.get('') ?? [] : []));
    const halfGroups = splitAxis === undefined ? [{ label: '', layouts: boardLayouts }] : [
      { label: 'Left half', layouts: boardLayouts.filter((layout) => (matrixMap.get(layout.matrixId)?.origin.x ?? 0) < splitAxis) },
      { label: 'Right half', layouts: boardLayouts.filter((layout) => (matrixMap.get(layout.matrixId)?.origin.x ?? 0) >= splitAxis) },
    ];
    for (const half of halfGroups) {
      const groupKey = `half-group:${selectedBoardId}:${half.label}`;
      const open = !half.label || !expandedTree.has(groupKey);
      if (half.label && half.layouts.length) result.push({
        id: groupKey, label: half.label, kind: 'layout', level: 1, expandable: true, expanded: open,
        selected: half.layouts.some((layout) => layout.partIds.some((id) => selected.includes(id))), onToggle: () => toggleTree(groupKey), onSelect: () => { changeMode('Design'); setScope(null); setSelected([...new Set(half.layouts.flatMap((layout) => [...layout.partIds, ...(matrixMap.get(layout.matrixId)?.partIds ?? [])]))]); }
      });
      if (!open) continue;
      const halfComponents = half.layouts.flatMap((layout) => componentsByLayout.get(layout.id) ?? []);
      const componentGroupKey = `components:${selectedBoardId}:${half.label || 'layout'}`;
      const componentsExpanded = !expandedTree.has(componentGroupKey);
      if (halfComponents.length) result.push({
        id: componentGroupKey,
        label: 'Components',
        detail: `${halfComponents.length} parts`,
        kind: 'components',
        level: half.label ? 2 : 1,
        expandable: true,
        expanded: componentsExpanded,
        selected: false,
        onToggle: () => toggleTree(componentGroupKey),
        onSelect: () => toggleTree(componentGroupKey),
      }, ...(componentsExpanded ? halfComponents.map((entry) => ({ ...entry, level: half.label ? 3 : 2 })) : []));
      for (const layout of half.layouts) {
        const key = `half:${layout.id}`;
        const expanded = expandedTree.has(key);
        const linked = Boolean(layout.mirrorLink || boardLayouts.some((other) => other.mirrorLink?.sourceId === layout.id));
        result.push({
          id: key, label: layout.name, kind: 'layout', level: half.label ? 2 : 1, expandable: true, expanded,
          detail: linked ? 'Linked' : 'Independent', selected: mode === 'Design' && scope?.kind === 'matrix' && scope.matrixId === layout.matrixId,
          onToggle: () => toggleTree(key), onSelect: () => { changeMode('Design'); selectScope({ kind: 'matrix', matrixId: layout.matrixId }); },
        }, ...(expanded ? (groups.get(layout.id) ?? []).map((entry) => ({ ...entry, level: entry.level + (half.label ? 1 : 0) })) : []));
      }
    }
    return result;
  }, [caseDisplay.current, caseDisplay.read, selectedCaseInstanceId, document.hardware, mode, selected, document.caseBodies, activeCaseBody?.id, selectedBoardId, selectedBoard?.name, visibleMatrices, treeVisibleParts, expandedTree, scope, treeParts, definitions, matrixCellOverrides, selectScope, toggleTree, treeGrouping, memberMaps, matrixScenes, document.matrices, boardLayouts, mechanicalAssembly, selectedMechanicalLayer, document.boardOutlines, outlineScene, outlineOpen, selectedBridgeId, onOutline, onBridge]);
  return { treeEntries };
}
