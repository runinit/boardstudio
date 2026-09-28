import { ProjectLibrary, ProjectLibraryIcon } from './ProjectLibrary';
import { SetupGuide } from './SetupGuide';
import { WorkflowNavigation, WorkflowReturn } from './WorkflowNavigation';
import { deriveSetupGuide, type SetupGuideStage } from './setupGuide';
import { useSetupGuide } from './useSetupGuide';
import { CaseInspectorPanel } from './CaseInspectorPanel';
import { PartsInspectorPanel } from './PartsInspectorPanel';
import { createCanvasCamera } from './createCanvasCamera';
import { createLibraryActions } from './createLibraryActions';
import { catalogue as ergogenCatalogue, parameters as ergogenParameterSchema, isErgogen, normalizeDefinition } from '@boardstudio/v2-ergogen';
import { importedPartDefinitions } from '../parts/catalogue';
import React, { lazy, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type {
  CaseBody,
  Constraint,
  EditCommand,
  JsonValue,
  Matrix,
  MatrixCell,
  MatrixSplayChange,
  MechanicalPartProfile,
  OutlineFeature,
  Part,
  PartDefinition,
  ProjectDoc,
  SceneDelta,
  Vec2
} from '../../../contracts/src/index';
import { defaultOutlineSettings } from '../../../contracts/src/index';
import { assemblyName, matrixPresetDefinitions, type MatrixPresetId } from './assemblyCatalog';
import { matrixWithAssembly } from './assemblyPlacement';
import { assemblyPreset, type SwitchOrientation } from './assemblyPresets';
import { aspectBounds, cameraBounds, getBounds } from './canvasBounds';
import { MatrixGhost, ScenePart, SplayHandles } from './CanvasObjects';
import { caseReadiness, mechanicalFindings } from './caseReadiness';
import { CommandMenu, ToolIcon } from './CommandMenu';
import { mirrorExistingHalf } from './existingHalf';
import { ExistingHalfSetup } from './ExistingHalfSetup';
import { FindingList } from './FindingList';
import { findingTarget, presentedFindings } from './findings';
import { ConstraintNumber, Coordinate, Measure } from './InspectorControls';
import { InspectorSection } from './InspectorSection';
import { findKeycapOverlaps, type KeycapPlacement } from './keycapReflow';
import { LibraryWorkspace } from './LibraryWorkspace';
import { matrixCellId, matrixSceneAdapter, type MatrixScene } from './matrixGeometry';
import { resizeMatrix } from './matrixResize';
import { MechanicalAssemblyPanel } from './MechanicalAssemblyPanel';
import { MatrixInspectorPanel } from './MatrixInspectorPanel';
import { MirrorPairIcon, MirroredPairSetup, pairAt } from './MirroredPairSetup';
import { OutlineInspector, PartOutlineControls } from './OutlineInspector';
import { OutlineControlOverlay } from './OutlineControlOverlay';
import { OutlineDrawingBar, OutlineToolIcon } from './OutlineTools';
import { OutlineDraftPreview } from './OutlineDraftPreview';
import { connectionEndpoint, snapOutline, type OutlineDrawMode, type OutlineSelection } from './outlineEditing';
import { partCatalogLabel, partCatalogSearchText, partChoices } from './partsCatalog';
import { movePlacement, type PlacementState } from './placementState';
import { createWorkbenchPlacementActions, documentWithPlacedPart } from './createWorkbenchPlacementActions';
import { assignNetPins, buildMirrorConstraint, buildOffsetConstraint, replacePartDefinition } from './createWorkbenchEditActions';
import { planKeycapResize } from './planKeycapResize';
import { createCanvasInteractions } from './createCanvasInteractions';
import { PartsLibrary } from './PartsLibrary';
import { alignmentDelta, snapPart } from './placementGeometry';
import { selectionOutline } from './selectionOutline';
import { useWorkbenchSelection } from './useWorkbenchSelection';
import { usePartsEditing } from './usePartsEditing';
import { useWorkbenchTree } from './useWorkbenchTree';
import { WiringPanel } from './WiringPanel';
import { PITCH_MM, createPart, isTyping, localMatrixDelta, makeId, nudgeLargeStep, nudgeStep, pointFromEvent, snapDelta, unit } from './workbenchGeometry';
import { ArrowIcon, BrandMark, CursorIcon, FitIcon, ModeIcon, PartGlyph, RedoIcon, ScopeIcon, UndoIcon } from './WorkbenchIcons';
import { WorkbenchLayers, footprintLayers } from './WorkbenchLayers';
import { WorkbenchTree } from './WorkbenchTree';
import { Drag, ExportKind, Mode, Props, SceneHandlers, SelectionScope, StaggerDrag, SplayDrag, PanDrag } from './workbenchTypes';
import { PanelIcon, WorkspacePanel, useCompactPanel, usePanelSettings } from './WorkspacePanel';

import './workbench.css';
import './inspector.css';
import './unified-workbench.css';
import './workspace-panels.css';
import './setup-guide.css';
import './outline-editor.css';
import './workflow.css';
import './project-library.css';

const AssemblyEditor = lazy(() => import('./AssemblyEditor').then(m => ({ default: m.AssemblyEditor })));
const AssemblyViewer = lazy(() => import('./AssemblyViewer').then(m => ({ default: m.AssemblyViewer })));
const BoardReferencePanel = lazy(() => import('./BoardReferencePanel').then(m => ({ default: m.BoardReferencePanel })));

type ThemePreference = 'system' | 'light' | 'dark';
const THEME_KEY = 'boardstudio:v2:theme';

const readThemePreference = (): ThemePreference => {
  try {
    const stored = localStorage.getItem(THEME_KEY);
    return stored === 'light' || stored === 'dark' ? stored : 'system';
  } catch {
    return 'system';
  }
};

const systemColorScheme = (): 'light' | 'dark' => {
  try {
    return matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  } catch {
    return 'light';
  }
};

const Workbench = ({ document, scene, saveStatus, projectSession, onEdit, onUndo, onRedo, onExport, compileFootprints, onNewProject, onOpenSavedProject, onOpenDemo, onImport, onImportFootprint, onImportModel, mechanicalAssembly, onResolveMechanical, onCancelGeneration, generation, livePreview, onLivePreviewChange, onCasePreviewDraft, onExportMechanical, onMechanicalProfile, onExtractMechanicalProfile, onDuplicateDesign, onProjectMatrices, onModeChange, caseBodies, casePreview, preparedCase, embedUsedModels = true, onEmbedUsedModelsChange, selectedBoardId: selectedBoardIdProp, onSelectBoard, physicalCaseDocument, physicalCaseScene, instanceControls, setupRequest, wiringStatus, wiring, onResolveWiring, onApplyWiring, onReviewWiring }: Props) => {
  const caseDocument = physicalCaseDocument ?? document;
  const caseScene = physicalCaseScene ?? scene;
  const [caseActionsTarget, setCaseActionsTarget] = useState<HTMLDivElement | null>(null);
  const [mode, setMode] = useState<Mode>('Design');
  const guide = useSetupGuide({ projectId: document.id, newProjectRequest: setupRequest });
  const guideVisible = guide.open && mode !== 'Library';
  const [lastDesignMode, setLastDesignMode] = useState<Mode>('Design');
  const [guidedPlacement, setGuidedPlacement] = useState(false);
  const navigationProject = useRef(document.id);
  const [commandMenu, setCommandMenu] = useState<string | null>(null);
  const [inspectorTab, setInspectorTab] = useState<'properties' | 'relations'>('properties');
  const [showFindings, setShowFindings] = useState(false);
  const [diagnosticsRequest, setDiagnosticsRequest] = useState(0);
  const findingsHeading = useRef<HTMLHeadingElement>(null);
  const [showFootprints, setShowFootprints] = useState(false);
  const [hiddenLayers, setHiddenLayers] = useState<ReadonlySet<string>>(new Set());
  const toggleLayer = (layer: string) => setHiddenLayers((current) => { const next = new Set(current); if (next.has(layer)) next.delete(layer); else next.add(layer); return next; });
  const [existingHalfOpen, setExistingHalfOpen] = useState(false);
  const [transformTool, setTransformTool] = useState<'stagger' | 'splay' | 'origin' | null>(null);
  const [originPicking, setOriginPicking] = useState(false);
  const [geometrySnap, setGeometrySnap] = useState(true);
  const [gapSnap, setGapSnap] = useState(true);
  const [gapOverride, setGapOverride] = useState<string>('');
  const [snapGuide, setSnapGuide] = useState<ReturnType<typeof snapPart>>();
  const [alignTargetId, setAlignTargetId] = useState('');
  const [splayAffect, setSplayAffect] = useState<'column' | 'following'>('following');
  const splayDrag = useRef<SplayDrag | null>(null);
  const objectsPanel = usePanelSettings('left');
  const inspectorPanel = usePanelSettings('right');
  const compactObjects = useCompactPanel('left');
  const compactInspector = useCompactPanel('right');
  const cameraViews = useRef(new Map<string, { zoom: number; pan: Vec2 }>());
  const designView = mode === 'Design' || mode === 'PCB' || mode === 'Case';
  const viewLabel = mode === 'Design' ? 'Layout' : mode === 'Library' ? 'Parts' : mode;

  const [themePreference, setThemePreference] = useState<ThemePreference>(readThemePreference);
  const [systemScheme, setSystemScheme] = useState(systemColorScheme);
  const [projectMenuOpen, setProjectMenuOpen] = useState(false);
  const [projectPage, setProjectPage] = useState<'project' | 'settings'>('project');
  const projectTriggerRef = useRef<HTMLButtonElement>(null);
  const projectFileRef = useRef<HTMLInputElement>(null);
  const [selected, setSelected] = useState<string[]>([]);
  const [scope, setScope] = useState<SelectionScope | null>(null);
  const [selectionMode, setSelectionMode] = useState<SelectionScope['kind']>('key');
  const [selectionAnchor, setSelectionAnchor] = useState<{ matrixId: string; row: number; column: number } | null>(null);
  const [expandedTree, setExpandedTree] = useState<Set<string>>(() => new Set());
  const [treeGrouping, setTreeGrouping] = useState<'column' | 'row'>(() => { try { return localStorage.getItem('boardstudio:v2:tree-grouping') === 'row' ? 'row' : 'column'; } catch { return 'column'; } });
  const [addPartOpen, setAddPartOpen] = useState(false);
  const [placement, setPlacement] = useState<PlacementState>({ kind: 'idle' });
  const pendingPart = placement.kind === 'part' ? placement.definition : null;
  const placementPoint = placement.kind === 'part' ? placement.point : { x: 0, y: 0 };
  const pendingLayoutId = placement.kind === 'part' ? placement.layoutId : '';
  const addPartRef = useRef<HTMLButtonElement>(null);
  const [hoveredPart, setHoveredPart] = useState<string | null>(null);
  const [librarySearch, setLibrarySearch] = useState('');
  const [libraryChoice, setLibraryChoice] = useState('');
  const [library3dOpen, setLibrary3dOpen] = useState(false);
  const [editingAssembly, setEditingAssembly] = useState<import('@boardstudio/v2-contracts').AssemblyDefinition | null>(null);
  const [assembly3d, setAssembly3d] = useState(false);
  const [selectedMechanicalLayer, setSelectedMechanicalLayer] = useState('');
  const [libraryAssembly, setLibraryAssembly] = useState<MatrixPresetId | null>(null);
  const [assemblyOrientation, setAssemblyOrientation] = useState<SwitchOrientation>('south');
  const matrixSetup = placement.kind === 'matrix-setup';
  const pairSetup = placement.kind === 'pair-setup';
  const pairPlacement = placement.kind === 'mirrored-pair' ? placement.pair : null;
  const [layoutTargetId, setLayoutTargetId] = useState('');
  const [matrixRows, setMatrixRows] = useState('');
  const [matrixColumns, setMatrixColumns] = useState('');
  const [matrixPreset, setMatrixPreset] = useState<MatrixPresetId>('mx-solder');
  const matrixGhost = placement.kind === 'matrix' || placement.kind === 'mirrored-pair' ? placement.matrix : null;
  const [matrixGhostScenes, setMatrixGhostScenes] = useState<MatrixScene[]>([]);
  const matrixDraftSeq = useRef(0);
  const cancelPlacement = () => {
    matrixDraftSeq.current += 1;
    setPlacement({ kind: 'idle' });
    setMatrixGhostScenes([]);
    setSnapGuide(undefined);
  };
  const matrixGhostProjections = useMemo(() => new Map(matrixGhostScenes.map((projection) => [projection.matrixId, matrixSceneAdapter(projection)])), [matrixGhostScenes]);
  const [snapFraction, setSnapFraction] = useState(0.25);
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState<Vec2>({ x: 0, y: 0 });
  const [leftOpen, setLeftOpen] = useState(false);
  const [rightOpen, setRightOpen] = useState(false);
  useEffect(() => {
    if (!guide.open) return;
    setLeftOpen(true);
    if (compactInspector) setRightOpen(false);
    if (!compactObjects) objectsPanel.setMode('pinned');
  }, [guide.open, document.id]);
  useEffect(() => {
    if (!guideVisible || (compactObjects ? !leftOpen : objectsPanel.mode !== 'pinned')) return;
    const frame = requestAnimationFrame(() => globalThis.document.querySelector<HTMLElement>('.wb-setup-guide h2')?.focus());
    return () => cancelAnimationFrame(frame);
  }, [guideVisible, document.id, compactObjects, leftOpen, objectsPanel.mode]);
  useEffect(() => {
    if (!guidedPlacement || placement.kind !== 'idle') return;
    setGuidedPlacement(false);
    guide.setOpen(true);
    setLeftOpen(true);
    if (compactInspector) setRightOpen(false);
  }, [guidedPlacement, placement.kind]);
  const [outlineDraft, setOutlineDraft] = useState<Vec2[]>([]);
  const [outlineActive, setOutlineActive] = useState(false);
  const [outlineSettingsOpen, setOutlineSettingsOpen] = useState(false);
  const [outlineOperation, setOutlineOperation] = useState<OutlineDrawMode>('add');
  const [outlineSelection,setOutlineSelection] = useState<OutlineSelection|null>(null);
  const [outlineGrid,setOutlineGrid] = useState(1);
  const [outlinePoint,setOutlinePoint] = useState(0);
  const [outlineDragBounds,setOutlineDragBounds] = useState<ReturnType<typeof getBounds>|null>(null);
  const outlineContext = useRef({documentId:document.id,projectSession});
  outlineContext.current = {documentId:document.id,projectSession};

  useEffect(() => { setOutlineDraft([]); setOutlineActive(false); setOutlineSelection(null); }, [mode, selectedBoardIdProp, document.id, projectSession]);
  const [newNetName, setNewNetName] = useState('');
  const [scriptsOpen, setScriptsOpen] = useState(false);
  const [activeScriptId, setActiveScriptId] = useState('');
  const [scriptName, setScriptName] = useState('');
  const [scriptSource, setScriptSource] = useState('');
  const [scriptEnabled, setScriptEnabled] = useState(true);
  const [caseBodyId, setCaseBodyId] = useState('');
  const [projectName, setProjectName] = useState(document.name);
  const [boardName, setBoardName] = useState('');
  const [localBoardId, setLocalBoardId] = useState('');
  const [definitionError, setDefinitionError] = useState('');
  const [constraintKind, setConstraintKind] = useState<Constraint['kind']>('offset');
  const [constraintSourceId, setConstraintSourceId] = useState('');
  const [constraintX, setConstraintX] = useState('0');
  const [constraintY, setConstraintY] = useState('0');
  const [constraintRotation, setConstraintRotation] = useState('0');
  const colorScheme = themePreference === 'system' ? systemScheme : themePreference;
  const inspectorPage = showFindings ? 'Layout findings' : scriptsOpen ? 'Geometry scripts' : outlineSettingsOpen && mode === 'Design' ? 'Board outline' : null;
  const closeInspectorPage = () => {
    setShowFindings(false);
    setScriptsOpen(false);
    setOutlineSettingsOpen(false);
    setOutlineSelection(null);
    setInspectorTab('properties');
    requestAnimationFrame(() => globalThis.document.querySelector<HTMLElement>('#wb-inspector .wb-inspector-content :is(input, select, button):not(:disabled)')?.focus());
  };
  const revealInspector = () => {
    setRightOpen(true);
    if (compactObjects) setLeftOpen(false);
    if (!compactInspector) inspectorPanel.setMode('pinned');
  };

  useEffect(() => {
    if (navigationProject.current === document.id) return;
    navigationProject.current = document.id;
    setMode('Design');
    setLastDesignMode('Design');
    setShowFindings(false);
    setScriptsOpen(false);
    setOutlineSettingsOpen(false);
    setCommandMenu(null);
    setProjectMenuOpen(false);
    setEditingAssembly(null);
    setAssembly3d(false);
    setGuidedPlacement(false);
    setScope(null);
    setSelected([]);
    setInspectorTab('properties');
  }, [document.id]);

  useEffect(() => {
    const media = matchMedia('(prefers-color-scheme: dark)');
    const update = (event: MediaQueryListEvent) => setSystemScheme(event.matches ? 'dark' : 'light');
    if (media.addEventListener) {
      media.addEventListener('change', update);
      return () => media.removeEventListener('change', update);
    }
    media.addListener(update);
    return () => media.removeListener(update);
  }, []);

  useEffect(() => {
    window.document.documentElement.dataset.theme = colorScheme;
    window.document.documentElement.style.colorScheme = colorScheme;
    const meta = window.document.querySelector<HTMLMetaElement>('meta[name="theme-color"]');
    if (meta) meta.content = colorScheme === 'dark' ? '#151c26' : '#f7f8fa';
  }, [colorScheme]);

  useEffect(() => {
    if (!addPartOpen) return;
    const dismiss = (event: PointerEvent) => {
      if (event.target instanceof Node && !window.document.getElementById('wb-add-part')?.contains(event.target) && !addPartRef.current?.contains(event.target)) setAddPartOpen(false);
    };
    window.document.addEventListener('pointerdown', dismiss);
    return () => window.document.removeEventListener('pointerdown', dismiss);
  }, [addPartOpen]);

  const chooseTheme = (preference: ThemePreference) => {
    setThemePreference(preference);
    try { localStorage.setItem(THEME_KEY, preference); } catch { /* Keep the current session usable. */ }
  };

  useEffect(() => {
    if (!projectMenuOpen) return;
    const dismiss = (event: PointerEvent) => {
      if (event.target instanceof Node && !window.document.querySelector('.wb-project-menu')?.contains(event.target)) {
        setProjectMenuOpen(false);
      }
    };
    const keydown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setProjectMenuOpen(false);
        projectTriggerRef.current?.focus();
      }
    };
    window.document.addEventListener('pointerdown', dismiss);
    window.document.addEventListener('keydown', keydown);
    return () => {
      window.document.removeEventListener('pointerdown', dismiss);
      window.document.removeEventListener('keydown', keydown);
    };
  }, [projectMenuOpen]);
  const [constraintAxis, setConstraintAxis] = useState<'vertical' | 'horizontal'>('vertical');
  const [constraintCoordinate, setConstraintCoordinate] = useState('0');
  const [keyboardStatus, setKeyboardStatus] = useState('');
  const svgRef = useRef<SVGSVGElement>(null);
  const wheelZoom = useRef<(event: WheelEvent) => void>(() => undefined);
  const dragRef = useRef<Drag | null>(null);
  const staggerDragRef = useRef<StaggerDrag | null>(null);
  const suppressClick = useRef(false);
  const spaceDown = useRef(false);
  const panDrag = useRef<PanDrag | null>(null);
  const transaction = useRef(makeId());
  const sceneHandlers = useRef<SceneHandlers | null>(null);

  const definitions = useMemo(() => new Map(document.definitions.map((item) => [item.id, item])), [document.definitions]);
  const bundledDefinitions = useMemo(() => [...ergogenCatalogue(), ...importedPartDefinitions()], []);
  const libraryDefinitions = useMemo(() => {
    const entries = new Map(bundledDefinitions.map((definition) => [definition.id, definition]));
    for (const definition of document.definitions) entries.set(definition.id, definition);
    return [...entries.values()];
  }, [bundledDefinitions, document.definitions]);
  const availableLibrary = useMemo(() => partChoices(libraryDefinitions), [libraryDefinitions]);
  const libraryRecipe = useMemo(() => libraryAssembly ? assemblyPreset(libraryAssembly, libraryDefinitions, assemblyOrientation) : undefined, [libraryAssembly, libraryDefinitions, assemblyOrientation]);
  const libraryRecipeEntries = useMemo(() => libraryRecipe?.members.map(member => {
    const definition = libraryDefinitions.find(entry => entry.id === member.definitionId)!;
    return { definition: normalizeDefinition({ ...definition, generator: definition.generator && { ...definition.generator, parameters: { ...definition.generator.parameters, ...member.parameters } } }), at: member.pose.at, rotation: member.pose.rotation, side: member.side };
  }) ?? [], [libraryRecipe, libraryDefinitions]);
  const libraryCompanions = useMemo(() => libraryRecipeEntries.slice(1), [libraryRecipeEntries]);
  const filteredLibrary = useMemo(() => {
    const query = librarySearch.trim().toLocaleLowerCase();
    return availableLibrary.filter((definition) => !query || partCatalogSearchText(definition).toLocaleLowerCase().includes(query));
  }, [availableLibrary, librarySearch]);
  const selectedLibraryDefinition = libraryDefinitions.find((definition) => definition.id === libraryChoice) ?? (!librarySearch.trim() ? availableLibrary.find(definition => definition.id === 'ergogen:ceoloide/switch_mx') : undefined) ?? filteredLibrary[0] ?? availableLibrary[0];
  const ergogenGenerator = isErgogen(selectedLibraryDefinition?.generator?.source);
  const recipePreviewDefinitions = useMemo(() => libraryRecipeEntries.map((entry) => entry.definition), [libraryRecipeEntries]);
  const { edits: libraryParameters, updateGenerator, generatorPreview, previewDefinition, compiledDefinitions: libraryCompiled, compilePending: previewCompilePending, compileError: previewCompileError, saveGenerator } = usePartsEditing({
    document, selectedDefinition: selectedLibraryDefinition, previewDefinitions: recipePreviewDefinitions,
    compileFootprints, onEdit, setDefinitionError,
  });
  const poses = useMemo(() => new Map(scene.transforms.map((item) => [item.id, item.pose])), [scene.transforms]);
  const treeParts = useMemo(() => new Map(document.parts.map((part) => [part.id, part])), [document.parts]);
  const parts = useMemo(() => new Map(document.parts.map((part) => [part.id, { ...part, pose: poses.get(part.id) ?? part.pose }])), [document.parts, poses]);
  const activeParts = selected.map((id) => parts.get(id)).filter((part): part is Part => Boolean(part));
  const activePart = activeParts[0];
  const activeDefinition = activePart ? definitions.get(activePart.definitionId) : undefined;
  const activeErgogenParams = activeDefinition?.generator && isErgogen(activeDefinition.generator.source) ? ergogenParameterSchema(activeDefinition.generator.source) : {};
  const updatePartGeneratorParameter = (key: string, value: JsonValue | undefined) => {
    if (!activePart) return;
    const generatorParameters = { ...(activePart.generatorParameters ?? {}) };
    if (value === undefined || value === '') delete generatorParameters[key]; else generatorParameters[key] = value;
    const next = { ...activePart, generatorParameters };
    emit({ kind: 'replace-document', document: { ...document, parts: document.parts.map((part) => part.id === next.id ? next : part) } }, [next.id]);
  };
  const activeScript = document.scripts.find((script) => script.id === activeScriptId);
  const selectedBoard = document.boards.find((board) => board.id === (selectedBoardIdProp ?? localBoardId)) ?? document.boards[0];
  const outlineFeature = outlineSettingsOpen && mode==='Design' && !assembly3d ? document.outline.find(feature=>feature.id===outlineSelection?.featureId && selectedBoard?.outlineIds.includes(feature.id) && (feature.kind!=='part-envelope' || feature.connections?.some(connection=>connection.id===outlineSelection.connectionId))) : undefined;
  const activeCaseBody = document.caseBodies.find((body) => body.id === caseBodyId && (!selectedBoard || body.boardId === selectedBoard.id))
    ?? document.caseBodies.find((body) => !selectedBoard || body.boardId === selectedBoard.id);
  const activeModelDefinition = selectedLibraryDefinition;
  const selectedBoardId = selectedBoard?.id ?? '';
  const boardPartIds = useMemo(() => new Set(selectedBoard?.partIds ?? document.parts.map((part) => part.id)), [selectedBoard, document.parts]);
  const treeVisibleParts = useMemo(() => document.parts.filter((part) => boardPartIds.has(part.id)), [document.parts, boardPartIds]);
  const visibleParts = useMemo(() => [...parts.values()].filter((part) => boardPartIds.has(part.id)), [parts, boardPartIds]);
  const pcbLayers = useMemo(() => [...new Set(visibleParts.flatMap((part) => { const definition = definitions.get(part.definitionId); return definition ? footprintLayers(definition, part.side) : []; }))].sort(), [visibleParts, definitions]);
  const visibleMatrices = useMemo(() => document.matrices.filter((matrix) => matrix.boardId ? matrix.boardId === selectedBoardId : matrix.partIds.some((id) => boardPartIds.has(id)) || (!matrix.partIds.length && document.boards.length <= 1)), [document.matrices, selectedBoardId, boardPartIds, document.boards.length]);
  const boardLayouts = useMemo(() => (document.layouts ?? []).filter((layout) => layout.boardId === selectedBoardId), [document.layouts, selectedBoardId]);
  const activeLayout = boardLayouts.find((layout) => layout.matrixId === scope?.matrixId || (activePart && (layout.partIds.includes(activePart.id) || document.matrices.find((matrix) => matrix.id === layout.matrixId)?.partIds.includes(activePart.id))));
  const linkedLayout = activeLayout ? activeLayout.mirrorLink ? activeLayout : boardLayouts.find((layout) => layout.mirrorLink?.sourceId === activeLayout.id) : undefined;
  const partnerLayout = linkedLayout ? boardLayouts.find((layout) => layout.id === (linkedLayout.id === activeLayout?.id ? linkedLayout.mirrorLink?.sourceId : linkedLayout.id)) : undefined;
  const mirroredComponentSourceId = typeof activePart?.properties?.['boardstudio.mirroredComponentSource'] === 'string'
    ? activePart.properties['boardstudio.mirroredComponentSource'] as string : undefined;
  const mirroredComponentIsSubstitute = activePart?.properties?.['boardstudio.mirroredComponentSubstitute'] === true;
  const mirroredComponentTarget = activePart
    ? visibleParts.find((part) => part.properties?.['boardstudio.mirroredComponentSource'] === activePart.id)
    : undefined;
  const hasMirroredComponent = Boolean(mirroredComponentSourceId || mirroredComponentTarget);
  useEffect(() => setLayoutTargetId(activeLayout?.id ?? ''), [activeLayout?.id]);
  const matrixMap = useMemo(() => new Map(visibleMatrices.map((matrix) => [matrix.id, matrix])), [visibleMatrices]);
  const matrixScenes = useMemo(() => new Map(scene.matrixScenes.map((item) => [item.matrixId, matrixSceneAdapter(item)])), [scene.matrixScenes]);
  const keyEnvelopes = useMemo(() => new Map(visibleMatrices.flatMap((matrix) => (matrixScenes.get(matrix.id)?.scene?.cells ?? []).flatMap((cell) => {
    if (!cell.enabled || !cell.memberId) return [];
    const part = parts.get(cell.memberId);
    const size = part?.keycap ?? definitions.get(part?.definitionId ?? matrix.definitionId)?.keycap ?? { x: Math.max(1, matrix.pitch.x - (matrix.edgeGap?.x ?? 1)), y: Math.max(1, matrix.pitch.y - (matrix.edgeGap?.y ?? 1)) };
    return [[cell.memberId, size] as const];
  }))), [visibleMatrices, matrixScenes, parts, definitions]);
  const unpairedMatrices = visibleMatrices.filter((matrix) => {
    const layout = boardLayouts.find((item) => item.matrixId === matrix.id);
    return !layout || (!layout.mirrorLink && !boardLayouts.some((item) => item.mirrorLink?.sourceId === layout.id));
  });
  const matrixCellOverrides = useMemo(() => new Map(visibleMatrices.map((matrix) => [
    matrix.id,
    new Map((matrix.cells ?? []).map((cell) => [`${cell.row}:${cell.column}`, cell])),
  ])), [visibleMatrices]);
  const memberMaps = useMemo(() => new Map(visibleMatrices.map((matrix) => [matrix.id, matrixScenes.get(matrix.id)?.members ?? new Map<string, string>()])), [visibleMatrices, matrixScenes]);
  const matrixPartLookup = useMemo(() => {
    const lookup = new Map<string, { matrixId: string; row: number; column: number; assemblyId?: string }>();
    for (const matrix of visibleMatrices) {
      for (const projected of matrixScenes.get(matrix.id)?.scene?.cells ?? []) {
        const { row, column, memberId } = projected;
        if (!memberId) continue;
        lookup.set(memberId, { matrixId: matrix.id, row, column });
        const cell = matrixCellOverrides.get(matrix.id)?.get(`${row}:${column}`);
        for (const assembly of cell?.assemblies ?? []) lookup.set(`${memberId}/${assembly.id}`, { matrixId: matrix.id, row, column, assemblyId: assembly.id });
      }
      for (const id of matrix.partIds) {
        if (lookup.has(id)) continue;
        const prefix = `matrix/${matrix.id}/r`;
        if (!id.startsWith(prefix)) continue;
        const coordinate = id.slice(prefix.length).match(/^(\d+)c(\d+)(?:\/(.+))?$/u);
        if (!coordinate) continue;
        lookup.set(id, { matrixId: matrix.id, row: Number(coordinate[1]), column: Number(coordinate[2]), assemblyId: coordinate[3] });
      }
    }
    return lookup;
  }, [visibleMatrices, matrixCellOverrides, matrixScenes]);
  const activeConstraint = activePart
    ? document.constraints.find((constraint) => constraint.targetPartId === activePart.id && boardPartIds.has(constraint.sourcePartId))
    : undefined;
  const defaultConstraintSourceId = visibleParts.find((part) => part.id !== activePart?.id)?.id ?? '';
  const constrainedTargetIds = useMemo(() => new Set(document.constraints
    .filter((constraint) => boardPartIds.has(constraint.sourcePartId) && boardPartIds.has(constraint.targetPartId))
    .map((constraint) => constraint.targetPartId)), [document.constraints, boardPartIds]);
  const selectedIds = useMemo(() => new Set(selected), [selected]);
  const keycapPlacements = useMemo<KeycapPlacement[]>(() => visibleParts.flatMap((part) => {
    const cell = matrixPartLookup.get(part.id);
    const size = keyEnvelopes.get(part.id);
    if (!cell || cell.assemblyId || !size) return [];
    return [{
      id: part.id,
      reference: part.reference,
      matrixId: cell.matrixId,
      row: cell.row,
      column: cell.column,
      at: part.pose.at,
      rotation: part.pose.rotation,
      size,
    }];
  }), [visibleParts, matrixPartLookup, keyEnvelopes]);
  const selectedKeycaps = useMemo(() => keycapPlacements.flatMap((placement) => {
    const matrix = matrixMap.get(placement.matrixId);
    if (!selectedIds.has(placement.id) || !matrix) return [];
    return [{
      placement,
      pitch: matrix.pitch,
      gap: matrix.edgeGap ?? { x: 1, y: 1 },
    }];
  }), [keycapPlacements, matrixMap, selectedIds]);
  const keycapOverlaps = useMemo(() => findKeycapOverlaps(keycapPlacements), [keycapPlacements]);
  const boardNets = useMemo(() => selectedBoard
    ? document.nets.filter((net) => selectedBoard.netIds.includes(net.id) || net.pins.some((pin) => selectedBoard.partIds.includes(pin.partId)))
    : document.nets, [document.nets, selectedBoard]);
  const visibleContours = selectedBoard
    ? scene.boardContours.find((entry) => entry.boardId === selectedBoard.id)?.contours ?? (document.boards.length === 1 ? scene.contours : [])
    : scene.contours;
  const assemblyDocument = mode === 'Case' ? caseDocument : document;
  const assemblyContours = mode === 'Case'
    ? caseScene.boardContours.find((entry) => entry.boardId === selectedBoardId)?.contours ?? (document.boards.length === 1 ? caseScene.contours : [])
    : visibleContours;
  // Instance meshes belong to the physical Case view; Layout shows the canonical PCB.
  const showCaseGeometry = mode === 'Case' || caseDocument === document;
  const selectedBoardReadiness = selectedBoard ? scene.boardReadiness?.find((entry) => entry.boardId === selectedBoard.id) : undefined;
  const boardReady = selectedBoardReadiness?.pcb ?? (document.boards.length <= 1 ? scene.readiness.pcb : false);
  const authoredCaseReady = Boolean(activeCaseBody) && (selectedBoardReadiness?.case ?? (document.boards.length <= 1 ? scene.readiness.case : false));
  const generatedCase = caseDocument.mechanical?.boardId === selectedBoardId;
  const mechanicalReadiness = caseReadiness({ revision: caseDocument.revision, sceneRevision: caseScene.revision,
    previewRevision: casePreview?.revision, boardId: selectedBoardId, configuredBoardId: caseDocument.mechanical?.boardId,
    generation, assembly: mechanicalAssembly, hasGeometry: Boolean(caseBodies?.length) });
  const generatedCaseReady = mechanicalReadiness.canExport;
  const visibleMechanicalFindings = generatedCase ? mechanicalFindings(mechanicalAssembly, document) : [];
  const revealMechanicalInspector = () => {
    setInspectorTab('properties'); setShowFindings(false); setRightOpen(true); setLeftOpen(false);
    if (!compactInspector) inspectorPanel.setMode('pinned');
  };
  const reviewMechanicalFindings = () => {
    changeMode('Case'); revealMechanicalInspector(); setDiagnosticsRequest(value => value + 1);
  };
  useEffect(() => {
    if (!showFindings) return;
    const frame = requestAnimationFrame(() => findingsHeading.current?.focus({ preventScroll: true }));
    return () => cancelAnimationFrame(frame);
  }, [showFindings]);

  const caseReady = generatedCase ? generatedCaseReady : authoredCaseReady;
  const [canvasSize, setCanvasSize] = useState({ width: 800, height: 600 });
  useEffect(() => {
    if (!svgRef.current) return;
    const observer = new ResizeObserver(([entry]) => {
      if (entry.contentRect.width && entry.contentRect.height) setCanvasSize({ width: entry.contentRect.width, height: entry.contentRect.height });
    });
    observer.observe(svgRef.current);
    return () => observer.disconnect();
  }, []);
  const bounds = useMemo(() => aspectBounds(getBounds(poses, visibleParts, visibleContours, visibleMatrices, matrixScenes, definitions, keyEnvelopes), canvasSize.width, canvasSize.height), [poses, visibleParts, visibleContours, visibleMatrices, matrixScenes, definitions, keyEnvelopes, canvasSize]);
  const viewBounds = useMemo(() => cameraBounds(dragRef.current?.bounds ?? splayDrag.current?.bounds ?? outlineDragBounds ?? bounds, zoom, pan), [bounds, outlineDragBounds, zoom, pan]);
  // Keep fine snapping available without drawing subpixel grid dots.
  const outlineGridSpacing = outlineGrid * Math.max(1, Math.ceil(8 * viewBounds.width / Math.max(canvasSize.width, 1) / outlineGrid));
  const { fitParts, startCanvasPan, zoomAt } = createCanvasCamera({ dragRef, splayDrag, staggerDragRef, poses, matrixScenes, definitions, keyEnvelopes, svgRef, bounds, canvasSize, setZoom, setPan, spaceDown, panDrag, pan, viewBounds });

  const readiness = scene.readiness;

  useEffect(() => {
    setScriptName(activeScript?.name ?? '');
    setScriptSource(activeScript?.source ?? '');
    setScriptEnabled(activeScript?.enabled ?? true);
  }, [activeScript?.id, activeScript?.source, activeScript?.enabled]);

  useEffect(() => setProjectName(document.name), [document.name]);
  useEffect(() => onModeChange?.(mode === 'Design' && assembly3d ? 'Case' : mode), [mode, assembly3d, onModeChange]);

  useEffect(() => {
    if (selectedBoardIdProp === undefined && selectedBoard) setLocalBoardId(selectedBoard.id);
    setBoardName(selectedBoard?.name ?? '');
    cancelPlacement();
    setAddPartOpen(false);
    setSelected([]);
    setScope(null);
    setSelectionMode('key');
    setSelectionAnchor(null);
  }, [selectedBoardIdProp, selectedBoard?.id]);

  useEffect(() => {
    if (scope) setSelectionMode(scope.kind);
  }, [scope?.kind]);

  useEffect(() => {
    if (!matrixGhost || !onProjectMatrices) {
      setMatrixGhostScenes([]);
      return;
    }
    const requestId = ++matrixDraftSeq.current;
    const pair = pairPlacement ? pairAt(matrixGhost, pairPlacement, { x: 0, y: 0 }) : undefined;
    const drafts = pair ? [pair.matrix, pair.preview] : [{ ...matrixGhost, origin: { x: 0, y: 0 } }];
    setMatrixGhostScenes([]);
    void onProjectMatrices(drafts).then((scenes) => {
      if (requestId !== matrixDraftSeq.current) return;
      setMatrixGhostScenes(scenes ?? []);
    }).catch(() => {
      if (requestId === matrixDraftSeq.current) setMatrixGhostScenes([]);
    });
    return () => { matrixDraftSeq.current += 1; };
  }, [matrixGhost?.id, matrixGhost?.rows, matrixGhost?.columns, matrixGhost?.definitionId, matrixGhost?.pitch, matrixGhost?.edgeGap, matrixGhost?.rowOffsets, matrixGhost?.columnOffsets, matrixGhost?.columnSplays, matrixGhost?.columnStaggers, matrixGhost?.columnOrigins, matrixGhost?.mirror, matrixGhost?.rotation, matrixGhost?.cells, pairPlacement, document.revision, onProjectMatrices]);

  useEffect(() => {
    setExpandedTree((current) => {
      const next = new Set(current);
      if (selectedBoardId) { next.add(`board:${selectedBoardId}`); next.add(`layout:${selectedBoardId}`); }
      for (const matrix of visibleMatrices) next.add(`matrix:${matrix.id}`);
      return next;
    });
  }, [selectedBoardId, visibleMatrices]);

  useEffect(() => {
    const sourceId = activeConstraint?.sourcePartId ?? defaultConstraintSourceId;
    setConstraintKind(activeConstraint?.kind ?? 'offset');
    setConstraintSourceId(sourceId);
    setConstraintX(activeConstraint?.kind === 'offset' ? String(activeConstraint.offset.x) : '0');
    setConstraintY(activeConstraint?.kind === 'offset' ? String(activeConstraint.offset.y) : '0');
    setConstraintRotation(activeConstraint?.kind === 'offset' ? String(activeConstraint.rotation) : '0');
    setConstraintAxis(activeConstraint?.kind === 'mirror' ? activeConstraint.axis : 'vertical');
    setConstraintCoordinate(activeConstraint?.kind === 'mirror' ? String(activeConstraint.coordinate) : '0');
  }, [activePart?.id, selectedBoardId, activeConstraint?.id, defaultConstraintSourceId]);

  useEffect(() => {
    const handleKey = (event: KeyboardEvent) => {
      if (event.code === 'Space' && !isTyping(event.target)) {
        spaceDown.current = true;
        event.preventDefault();
      }
      if (outlineActive && !isTyping(event.target)) {
        if (event.key === 'Escape') { event.preventDefault(); cancelOutlineDrawing(); return; }
        if (event.key === 'Backspace' || event.key === 'Delete' || ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'z')) {
          event.preventDefault(); setOutlineDraft(points => points.slice(0, -1)); return;
        }
      }
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'z') {
        event.preventDefault();
        if (cancelInteractions()) return;
        if (event.shiftKey) onRedo();
        else onUndo();
      }
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'y') {
        event.preventDefault();
        if (cancelInteractions()) return;
        onRedo();
      }
      if (event.key === 'Escape') {
        if (cancelInteractions()) return;
        if (existingHalfOpen) { setExistingHalfOpen(false); return; }
        if (transformTool) { setTransformTool(null); setOriginPicking(false); setSnapGuide(undefined); return; }
        if (originPicking) { setOriginPicking(false); return; }
        setOutlineDraft([]);
        setOutlineActive(false);
        setOutlineSelection(null);
        setSelected([]);
        setScope(null);
        setSelectionMode('key');
        setSelectionAnchor(null);
        cancelPlacement();
        setAddPartOpen(false);
        if (addPartOpen) addPartRef.current?.focus();
      }
      if (event.key === 'Enter' && !isTyping(event.target) && outlineActive) finishOutline();
      if (event.key === 'Delete' && !isTyping(event.target) && (selected.length > 0 || scope?.kind === 'matrix')) {
        if (scope?.kind === 'matrix' && scope.matrixId) {
          emit({ kind: 'remove-matrix', id: scope.matrixId }, [scope.matrixId, ...selected]);
          setScope(null);
        } else emit({ kind: 'remove-parts', ids: selected }, selected);
        setSelected([]);
      }
    };
    const releaseSpace = (event: KeyboardEvent) => {
      if (event.code === 'Space') spaceDown.current = false;
    };
    window.addEventListener('keydown', handleKey);
    window.addEventListener('keyup', releaseSpace);
    return () => {
      window.removeEventListener('keydown', handleKey);
      window.removeEventListener('keyup', releaseSpace);
    };
  }, [onRedo, onUndo, selected, document.revision, outlineActive, outlineDraft, outlineOperation, outlineGrid, addPartOpen, scope, originPicking, existingHalfOpen, transformTool]);

  const emit = (operation: EditCommand['operation'], targetIds: string[], phase: EditCommand['phase'] = 'commit', transactionId?: string) => {
    const currentTransaction = transactionId ?? (phase === 'preview' ? transaction.current : makeId());
    const committed = onEdit({
      baseRevision: document.revision,
      transactionId: currentTransaction,
      phase,
      targetIds,
      operation,
    });
    if (phase === 'commit') transaction.current = makeId();
    return committed;
  };

  const commitProjectName = () => {
    const name = projectName.trim();
    if (!name || name === document.name) {
      setProjectName(document.name);
      return;
    }
    emit({ kind: 'replace-document', document: { ...document, name } }, [document.id]);
  };

  const selectBoard = (id: string) => {
    cancelInteractions();
    if (onSelectBoard) onSelectBoard(id);
    else setLocalBoardId(id);
  };

  const changeMode = (next: Mode) => {
    if (next !== mode) {
      cancelInteractions();
      cameraViews.current.set(`${selectedBoardId}:${mode}`, { zoom, pan });
      const camera = cameraViews.current.get(`${selectedBoardId}:${next}`);
      setZoom(camera?.zoom ?? 1);
      setPan(camera?.pan ?? { x: 0, y: 0 });
    }
    if (next === 'Design' || next === 'PCB' || next === 'Case') setLastDesignMode(next);
    setCommandMenu(null);
    setOriginPicking(false);
    cancelPlacement();
    setShowFindings(false);
    setInspectorTab('properties');
    setLeftOpen(false);
    if (next === 'Export') setRightOpen(false);
    setMode(next);
    setScriptsOpen(false);
    setOutlineSettingsOpen(false);
    setTransformTool(null);
    setProjectMenuOpen(false);
    setAddPartOpen(false);

  };

  const changeScope = (kind: SelectionScope['kind']) => {
    setSelectionMode(kind);
    if (!scope) return;
    if (kind !== 'component' && !scope.matrixId) return;
    const next: SelectionScope = { ...scope, kind };
    if (scope.matrixId) {
      next.row = scope.row ?? 0;
      next.column = scope.column ?? 0;
      if (kind === 'component') next.partId = scope.partId ?? memberMaps.get(scope.matrixId)?.get(`${next.row}:${next.column}`) ?? matrixCellId(scope.matrixId, next.row, next.column);
      if (kind !== 'component') delete next.partId;
    }
    selectScope(next);
  };

  const addBoard = () => {
    const index = document.boards.length + 1;
    const board = { id: makeId(), name: `Board ${index}`, outlineIds: [] as string[], partIds: [] as string[], netIds: [] as string[], thickness: 1.6 };
    const envelope: OutlineFeature = { id: makeId(), kind: 'part-envelope', settings: defaultOutlineSettings, partIds: [], margin: 4, operation: 'add' };
    board.outlineIds = [envelope.id];
    emit({ kind: 'replace-document', document: { ...document, outline: [...document.outline, envelope], boards: [...document.boards, board] } }, [board.id, envelope.id]);
    selectBoard(board.id);
  };

  const commitBoardName = () => {
    if (!selectedBoard) return;
    const name = boardName.trim();
    if (!name || name === selectedBoard.name) {
      setBoardName(selectedBoard.name);
      return;
    }
    const boards = document.boards.map((board) => board.id === selectedBoard.id ? { ...board, name } : board);
    emit({ kind: 'replace-document', document: { ...document, boards } }, [selectedBoard.id]);
  };

  const saveOutline = (feature: OutlineFeature) => {
    if (!selectedBoard) return;
    const outlineIds = selectedBoard.outlineIds.includes(feature.id) ? selectedBoard.outlineIds : [...selectedBoard.outlineIds, feature.id];
    const boards = document.boards.map((board) => board.id === selectedBoard.id ? { ...board, outlineIds } : board);
    emit({ kind: 'replace-document', document: { ...document, outline: [...document.outline, feature], boards } }, [feature.id, selectedBoard.id]);
  };
  const { choosePart, selectScope } = useWorkbenchSelection({ outlineActive, pendingPart, setOutlineSettingsOpen, suppressClick, matrixPartLookup, selectionAnchor, matrixMap, memberMaps, boardPartIds, setSelectionMode, setScope, setSelected, setRightOpen, selected, setSelectionAnchor, selectionMode });

  const toggleTree = useCallback((id: string) => setExpandedTree((current) => {
    const next = new Set(current);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    return next;
  }), []);

  const nudgePart = (event: React.KeyboardEvent<Element>, part: Part) => {
    if (!event.key.startsWith('Arrow') || part.locked) return;
    event.preventDefault();
    const step = event.shiftKey ? nudgeLargeStep : nudgeStep;
    const delta = event.key === 'ArrowLeft' ? { x: -step, y: 0 }
      : event.key === 'ArrowRight' ? { x: step, y: 0 }
        : event.key === 'ArrowUp' ? { x: 0, y: step }
          : event.key === 'ArrowDown' ? { x: 0, y: -step }
            : undefined;
    if (!delta) return;
    const ids = selected.includes(part.id) ? selected : [part.id];
    const moving = ids.map((id) => parts.get(id)).filter((item): item is Part => item !== undefined && !item.locked);
    if (moving.length === 0) return;
    setSelected(moving.map((item) => item.id));
    setRightOpen(true);
    emit({ kind: 'move-parts', positions: moving.map((item) => ({
      id: item.id,
      at: { x: item.pose.at.x + delta.x, y: item.pose.at.y + delta.y },
    })) }, moving.map((item) => item.id));
    const nextPose = moving.find((item) => item.id === part.id)?.pose.at;
    const x = (nextPose?.x ?? part.pose.at.x) + delta.x;
    const y = (nextPose?.y ?? part.pose.at.y) + delta.y;
    setKeyboardStatus(`${part.reference} moved ${event.key.slice(5).toLowerCase()} ${step} mm. Position X ${x.toFixed(1)}, Y ${y.toFixed(1)} mm.`);
  };

  const { treeEntries } = useWorkbenchTree({ document, mechanicalAssembly, selectedBoardId, expandedTree, selectedBoard, visibleParts, toggleTree, setScope, setSelected, setOutlineSettingsOpen, visibleMatrices, boardLayouts, memberMaps, treeParts, scope, selectScope, matrixScenes, treeGrouping, matrixCellOverrides, definitions, nudgePart, treeVisibleParts, mode, changeMode, setExpandedTree, matrixMap, selected, activeCaseBody, setCaseBodyId, setRightOpen, selectedMechanicalLayer, setSelectedMechanicalLayer });

  const updatePosition = (axis: 'x' | 'y', value: number) => {
    if (!Number.isFinite(value) || activeParts.length === 0) return;
    const delta = value - activeParts[0].pose.at[axis];
    const positions = activeParts.map((part) => ({
      id: part.id,
      at: { ...part.pose.at, [axis]: part.pose.at[axis] + delta },
    }));
    emit({ kind: 'move-parts', positions }, selected);
  };

  const updateComponentDefinition = (definitionId: string, detachMirror = false) => {
    if (!activePart) return;
    const properties = { ...(activePart.properties ?? {}) };
    if (detachMirror) properties['boardstudio.mirroredComponentSubstitute'] = true;
    const definition = libraryDefinitions.find((item) => item.id === definitionId);
    if (!definition) return;
    const nextDocument = replacePartDefinition(document, activePart, definition);
    emit({ kind: 'replace-document', document: {
      ...nextDocument,
      parts: nextDocument.parts.map((part) => part.id === activePart.id ? { ...part, properties } : part),
    } }, [activePart.id]);
  };

  const setComponentSubstitute = (substitute: boolean) => {
    if (!activePart) return;
    const ids = new Set([activePart.id]);
    if (!substitute || !mirroredComponentSourceId) {
      if (mirroredComponentSourceId) ids.add(mirroredComponentSourceId);
      if (mirroredComponentTarget) ids.add(mirroredComponentTarget.id);
    }
    const parts = document.parts.map((part) => {
      if (!ids.has(part.id)) return part;
      const properties = { ...(part.properties ?? {}) };
      if (substitute) properties['boardstudio.mirroredComponentSubstitute'] = true;
      else delete properties['boardstudio.mirroredComponentSubstitute'];
      return { ...part, properties };
    });
    emit({ kind: 'replace-document', document: { ...document, parts } }, [...ids]);
  };

  const assignNet = (padId: string, netId: string) => {
    if (!activePart) return;
    const nets = assignNetPins(document.nets, activePart.id, [padId], netId);
    emit({ kind: 'replace-document', document: { ...document, nets } }, [activePart.id, padId, netId]);
  };

  const assignTerminal = (terminal: string, padIds: string[], netId: string) => {
    if (!activePart) return;
    const nets = assignNetPins(document.nets, activePart.id, padIds, netId);
    emit({ kind: 'replace-document', document: { ...document, nets } }, [activePart.id, ...padIds, terminal]);
  };

  const saveConstraint = () => {
    if (!activePart || !selectedBoard || !boardPartIds.has(activePart.id) || !boardPartIds.has(constraintSourceId) || constraintSourceId === activePart.id) return;
    const id = activeConstraint?.id ?? makeId();
    let constraint: Constraint | undefined;
    if (constraintKind === 'offset') {
      constraint = buildOffsetConstraint(id, constraintSourceId, activePart.id, constraintX, constraintY, constraintRotation);
    } else {
      constraint = buildMirrorConstraint(id, constraintSourceId, activePart.id, constraintAxis, constraintCoordinate);
    }
    if (!constraint) return;
    emit({ kind: 'set-constraint', constraint }, [constraint.id, constraint.sourcePartId, constraint.targetPartId]);
  };

  const removeConstraint = () => {
    if (!activeConstraint) return;
    emit({ kind: 'remove-constraint', id: activeConstraint.id }, [activeConstraint.id, activeConstraint.targetPartId]);
  };

  const addNet = () => {
    const name = newNetName.trim();
    if (!name) return;
    const net: ProjectDoc['nets'][number] = { id: makeId(), name, pins: [] };
    const boards = document.boards.map((board) => board.id === selectedBoard?.id ? { ...board, netIds: [...board.netIds, net.id] } : board);
    emit({ kind: 'replace-document', document: { ...document, nets: [...document.nets, net], boards } }, [net.id, selectedBoard?.id ?? ''].filter(Boolean));
    setNewNetName('');
  };

  const addScript = () => {
    const script: ProjectDoc['scripts'][number] = { id: makeId(), name: `Script ${document.scripts.length + 1}`, source: '', enabled: false };
    emit({ kind: 'replace-document', document: { ...document, scripts: [...document.scripts, script] } }, [script.id]);
    setActiveScriptId(script.id);
  };

  const applyScript = () => {
    if (!activeScript || !scriptSource.trim()) return;
    const scripts = document.scripts.map((script) => script.id === activeScript.id
      ? { ...script, name: scriptName, source: scriptSource, enabled: scriptEnabled }
      : script);
    emit({ kind: 'replace-document', document: { ...document, scripts } }, [activeScript.id]);
  };

  const addCaseBody = () => {
    if (!selectedBoard) return;
    const body: CaseBody = {
      id: makeId(),
      name: `${selectedBoard.name} plate`,
      boardId: selectedBoard.id,
      kind: 'plate',
      thickness: 3,
      clearance: 0.5,
      materialId: document.materials.find((material) => material.id === 'pla' || material.name.toLowerCase() === 'pla')?.id ?? 'pla',
      z: 0,
      wallHeight: 14,
      wallThickness: 2,
      mounts: [],
    };
    emit({ kind: 'set-case', body }, [body.id]);
    setCaseBodyId(body.id);
  };

  const updateCaseBody = (changes: Partial<CaseBody>) => {
    if (!activeCaseBody) return;
    emit({ kind: 'set-case', body: { ...activeCaseBody, ...changes } }, [activeCaseBody.id]);
  };

  const updateMount = (mountId: string, changes: Partial<NonNullable<CaseBody['mounts']>[number]>) => {
    if (!activeCaseBody) return;
    const mounts = (activeCaseBody.mounts ?? []).map((mount) => mount.id === mountId ? { ...mount, ...changes } : mount);
    updateCaseBody({ mounts });
  };

  const addMount = () => {
    if (!activeCaseBody) return;
    const mount = { id: makeId(), at: { x: 0, y: 0 }, kind: 'hole' as const, holeDiameter: 2.5, bossDiameter: 5, height: 5 };
    updateCaseBody({ mounts: [...(activeCaseBody.mounts ?? []), mount] });
  };

  const removeMount = (mountId: string) => {
    if (!activeCaseBody) return;
    updateCaseBody({ mounts: (activeCaseBody.mounts ?? []).filter((mount) => mount.id !== mountId) });
  };

  const editDefinition = selectedLibraryDefinition;
  const libraryActions = createLibraryActions({ activeModelDefinition, onImportModel, document, emit, editDefinition, setDefinitionError });

  const addCustomDefinition = () => {
    const definition: PartDefinition = {
      id: makeId(),
      name: `Custom component ${document.definitions.length + 1}`,
      kind: 'custom',
      courtyard: [{ x: -5, y: -3 }, { x: 5, y: -3 }, { x: 5, y: 3 }, { x: -5, y: 3 }],
      pads: [],
    };
    emit({ kind: 'replace-document', document: { ...document, definitions: [...document.definitions, definition] } }, [definition.id]);
    setLibrarySearch('');
    setScriptsOpen(false);
    setRightOpen(true);
    setLeftOpen(false);
    setLibraryChoice(definition.id);
    setLibraryAssembly(null);
    setDefinitionError('');
  };

  const documentWithPart = (part: Part, definition: PartDefinition, layoutId: string): ProjectDoc => documentWithPlacedPart(document, selectedBoardId, part, definition, layoutId);

  const assignPartLayout = (part: Part, layoutId: string) => {
    const layouts = (document.layouts ?? []).map((layout) => ({ ...layout, partIds: [...layout.partIds.filter((id) => id !== part.id), ...(layout.id === layoutId ? [part.id] : [])] }));
    emit({ kind: 'replace-document', document: { ...document, layouts } }, [part.id, layoutId].filter(Boolean));
  };

  const addDefinition = (definition: PartDefinition) => {
    const part = createPart(definition, document.parts);
    emit({ kind: 'replace-document', document: documentWithPart(part, definition, layoutTargetId) }, [part.id]);
    setSelected([part.id]);
    setScope({ kind: 'component', partId: part.id });
    setMode('Design');
    setRightOpen(true);
  };

  const commitMatrix = (matrix: Matrix, phase: EditCommand['phase'] = 'commit', transactionId?: string, definitions?: PartDefinition[]) => {
    const isLinkedTarget = boardLayouts.some((layout) => layout.matrixId === matrix.id && layout.mirrorLink);
    const current = document.matrices.find((item) => item.id === matrix.id);
    const next = isLinkedTarget && current ? {
      ...matrix,
      cells: (matrix.cells ?? []).map((cell) => {
        const previous = current.cells?.find((item) => item.row === cell.row && item.column === cell.column);
        const assemblyChanged = previous?.definitionId !== cell.definitionId
          || previous?.variant !== cell.variant
          || JSON.stringify(previous?.assemblies ?? []) !== JSON.stringify(cell.assemblies ?? []);
        return assemblyChanged ? { ...cell, assembliesLocal: true } : cell;
      }),
    } : matrix;
    emit({ kind: 'set-matrix', matrix: next, definitions }, [matrix.id, ...(definitions ?? []).map((definition) => definition.id)], phase, transactionId);
  };

  const commitSplay = (matrix: Matrix, column: number, change: MatrixSplayChange, phase: EditCommand['phase'] = 'commit', transactionId?: string) => {
    emit({ kind: 'set-matrix-splay', matrixId: matrix.id, column, change }, [matrix.id], phase, transactionId);
  };

  const setCell = (matrix: Matrix, row: number, column: number, changes: Partial<MatrixCell>, definitions?: PartDefinition[]) => {
    const cells = matrix.cells ?? [];
    const current = cells.find((cell) => cell.row === row && cell.column === column);
    const nextCell: MatrixCell = { row, column, enabled: current?.enabled ?? true, ...current, ...changes };
    const nextCells = cells.filter((cell) => cell.row !== row || cell.column !== column).concat(nextCell);
    commitMatrix({ ...matrix, cells: nextCells }, 'commit', undefined, definitions);
  };

  const createGuidedMatrix = () => { setPlacement({ kind: 'matrix-setup' }); setAddPartOpen(false); setLeftOpen(true); if (!compactObjects) objectsPanel.setMode('pinned'); setMatrixRows(''); setMatrixColumns(''); };

  const placementActions = createWorkbenchPlacementActions({
    document, selectedBoard, selectedBoardId, definitions, libraryDefinitions, matrixMap, matrixCellOverrides,
    scope, layoutTargetId, snapFraction, viewBounds, orientation: assemblyOrientation, placement, setPlacement,
    cancelPlacement, emit, setAddPartOpen, setLeftOpen, setRightOpen, setOutlineActive, setOutlineSettingsOpen,
    setMode, setScope, setExpandedTree, setSelected, selectScope,
    addDefinition, setCell,
  });
  const { beginMatrixPlacement, placeLibraryDefinition, beginPartPlacement, placePendingPart, placeMatrixAt } = placementActions;

  useEffect(() => {
    if ((pendingPart || matrixGhost) && mode === 'Design') svgRef.current?.focus();
  }, [pendingPart, Boolean(matrixGhost), mode]);

  const snapPlacement = (point: Vec2, free: boolean): Vec2 => {
    if (!pendingPart || free) { setSnapGuide(undefined); return point; }
    const grid = snapDelta(point, { x: PITCH_MM, y: PITCH_MM }, snapFraction);
    const moving: Part = { id: 'placement-preview', reference: pendingPart.name, definitionId: pendingPart.id, pose: { at: point, rotation: 0 }, side: 'front' };
    const snap = geometrySnap ? snapPart(moving, visibleParts, new Map([...definitions, [pendingPart.id, pendingPart]]), 2, gapSnap ? effectiveGap : null) : undefined;
    setSnapGuide(snap);
    return snap?.at ?? grid;
  };



  const drawOutline = (event: React.MouseEvent<SVGSVGElement>) => {
    if (suppressClick.current) return;
    if (originPicking && scope?.matrixId && scope.column !== undefined) {
      const matrix = matrixMap.get(scope.matrixId);
      const point = pointFromEvent(event, svgRef.current);
      if (matrix && point) commitSplay(matrix, scope.column, { kind: 'origin', world: snapSplayOrigin(point, matrix, event.altKey) });
      setOriginPicking(false); setRightOpen(true); return;
    }
    if (pendingPart && event.button === 0) {
      const point = pointFromEvent(event, svgRef.current);
      if (point) placePendingPart(snapPlacement(point, event.altKey));
      return;
    }
    if (mode === 'Design' && matrixGhost && event.button === 0) {
      const point = pointFromEvent(event, svgRef.current);
      if (point) placeMatrixAt(point);
      return;
    }
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

  const cancelOutlineDrawing = () => {
    setOutlineDraft([]); setOutlineActive(false); setRightOpen(true);
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
    setRightOpen(true);
  };

  wheelZoom.current = (event: WheelEvent) => {
    const nextZoom = Math.min(4, Math.max(0.25, zoom * Math.exp(-event.deltaY * 0.001)));
    zoomAt(event.clientX, event.clientY, nextZoom);
  };

  useEffect(() => {
    const canvas = svgRef.current;
    if (!canvas) return;

    // A non-passive listener keeps wheel input inside the drawing.
    const handleWheel = (event: WheelEvent) => {
      event.preventDefault();
      wheelZoom.current(event);
    };
    canvas.addEventListener('wheel', handleWheel, { passive: false });
    return () => canvas.removeEventListener('wheel', handleWheel);
  }, []);

  const selectedMatrix = (scope?.matrixId ? matrixMap.get(scope.matrixId) : undefined);
  const selectedKeycapOverlapReferences = [...new Set(keycapOverlaps
    .filter(({ first, second }) => selectedIds.has(first.id) || selectedIds.has(second.id))
    .flatMap(({ first, second }) => [first.reference, second.reference]))];
  const effectiveGap = gapOverride.trim() && Number.isFinite(Number(gapOverride)) && Number(gapOverride) >= 0 ? Number(gapOverride) : selectedMatrix?.edgeGap?.x ?? 1;

  const { startDrag, moveDrag, endDrag, startStagger, startSplay, moveCanvasPointer, endCanvasPointer, snapSplayOrigin, cancelInteractions } = createCanvasInteractions({
    refs: { dragRef, staggerDragRef, splayDrag, panDrag, svgRef, suppressClick, spaceDown, transaction },
    selection: { selected, scope, selectionMode, setScope, setSelectionMode, setSelectionAnchor, setSelected, setOutlineSettingsOpen, setRightOpen, selectScope },
    layout: { matrixPartLookup, matrixMap, matrixCellOverrides, matrixScenes, parts, definitions, visibleParts, bounds },
    snapping: { snapFraction, geometrySnap, gapSnap, effectiveGap, setSnapGuide },
    placement: { outlineActive, pendingPart, matrixGhost, mode, originPicking, selectedMatrix, splayAffect, snapPlacement, setPlacement },
    camera: { viewBounds, setPan },
    edits: { emit, commitMatrix, commitSplay },
  });
  const interactionContext = useRef({ documentId: document.id, projectSession });
  useEffect(() => {
    const previous = interactionContext.current;
    const sameProject = previous.documentId === document.id && previous.projectSession === projectSession;
    cancelInteractions(sameProject ? 'restore' : 'discard');
    setSelectedMechanicalLayer('');
    interactionContext.current = { documentId: document.id, projectSession };
  }, [document.id, projectSession, selectedBoardId]);
  useEffect(() => () => { cancelInteractions('discard'); }, []);

  const linkedSelection = partnerLayout && (scope?.kind !== 'component' || (activePart && matrixPartLookup.has(activePart.id) && !matrixPartLookup.get(activePart.id)?.assemblyId));
  const movingCounterparts = new Set(linkedSelection ? matrixMap.get(partnerLayout.matrixId)?.partIds ?? [] : []);
  const alignTargets = visibleParts.filter((part) => !selectedIds.has(part.id) && !movingCounterparts.has(part.id));
  const alignTarget = alignTargets.find((part) => part.id === alignTargetId) ?? alignTargets[0];
  const selectionLocked = activeParts.some((part) => part.locked || constrainedTargetIds.has(part.id));
  const alignSelection = (axis: 'x' | 'y', anchor: 'min' | 'center' | 'max') => {
    if (!alignTarget || !activeParts.length || selectionLocked) return;
    const delta = alignmentDelta(selectionOutline(activeParts, definitions), selectionOutline([alignTarget], definitions), axis, anchor);
    if (!delta) return;
    if (selectedMatrix && scope && ['matrix', 'row', 'column'].includes(scope.kind)) {
      if (scope.kind === 'matrix') commitMatrix({ ...selectedMatrix, origin: { x: selectedMatrix.origin.x + delta.x, y: selectedMatrix.origin.y + delta.y } });
      else {
        const local = localMatrixDelta(selectedMatrix, delta, scope.column ?? 0);
        const key = scope.kind === 'row' ? 'rowOffsets' : 'columnOffsets';
        const index = scope.kind === 'row' ? scope.row ?? 0 : scope.column ?? 0;
        const offsets = [...(selectedMatrix[key] ?? [])];
        while (offsets.length <= index) offsets.push({ x: 0, y: 0 });
        offsets[index] = { x: offsets[index].x + local.x, y: offsets[index].y + local.y };
        commitMatrix({ ...selectedMatrix, [key]: offsets });
      }
    } else emit({ kind: 'move-parts', positions: activeParts.map((part) => ({ id: part.id, at: { x: part.pose.at.x + delta.x, y: part.pose.at.y + delta.y } })) }, selected);
    setKeyboardStatus(`Aligned selection to ${alignTarget.reference}. Reference unchanged.`);
  };
  const commitKeycapResize = (units: Vec2, axis?: 'x' | 'y') => {
    const plan = planKeycapResize({ document, layouts: boardLayouts, matrices: matrixMap, projections: matrixScenes, placements: keycapPlacements, selectedIds, units, axis });
    if (plan) {
      emit({ kind: 'replace-document', document: plan.document }, plan.targetIds);
    }
  };
  const groupOutline = useMemo(() => selected.length > 1 || (scope && ['matrix', 'row', 'column'].includes(scope.kind))
    ? selectionOutline(visibleParts.filter((part) => selectedIds.has(part.id)), definitions)
    : [], [selected.length, scope, visibleParts, selectedIds, definitions]);
  const hoverOutline = useMemo(() => {
    if (!hoveredPart) return [];
    const member = matrixPartLookup.get(hoveredPart);
    const kind = scope?.kind ?? 'key';
    const group = visibleParts.filter((part) => {
      if (!member || !['matrix', 'row', 'column'].includes(kind)) return part.id === hoveredPart;
      const candidate = matrixPartLookup.get(part.id);
      return candidate?.matrixId === member.matrixId && (kind === 'matrix' || (kind === 'row' ? candidate.row === member.row : candidate.column === member.column));
    });
    return selectionOutline(group, definitions);
  }, [hoveredPart, scope?.kind, visibleParts, matrixPartLookup, definitions]);

  const selectionTitle = scope?.kind === 'component' && activePart ? activePart.reference : selectedMatrix
    ? `${activeLayout?.name || selectedMatrix.name?.trim() || `Matrix ${document.matrices.indexOf(selectedMatrix) + 1}`}${scope?.kind === 'row' ? ` · Row ${(scope.row ?? 0) + 1}` : scope?.kind === 'column' ? ` · Column ${(scope.column ?? 0) + 1}` : scope?.kind === 'key' ? ` · Key ${(scope.column ?? 0) + 1}.${(scope.row ?? 0) + 1}` : ''}`
    : activePart?.reference ?? selectedBoard?.name ?? 'Board';

  const [pendingFinding, setPendingFinding] = useState<SceneDelta['findings'][number] | null>(null);
  const showFinding = (finding: SceneDelta['findings'][number]) => {
    const target = findingTarget(finding, document, mechanicalAssembly);
    if (target.mechanicalLayer) {
      changeMode('Case'); setSelectedMechanicalLayer(target.mechanicalLayer.id); setShowFindings(false); setRightOpen(true);
      return;
    }
    const board = target.board ?? document.boards.find((item) => target.part ? item.partIds.includes(target.part.id) : target.matrix ? target.matrix.partIds.some((id) => item.partIds.includes(id)) : target.body?.boardId === item.id);
    if (board && board.id !== selectedBoardId) { setPendingFinding(finding); selectBoard(board.id); return; }
    changeMode(target.body ? 'Case' : 'Design');
    setScriptsOpen(false); setShowFindings(false); setInspectorTab('properties');
    if (target.outline || (!target.part && !target.matrix && !target.body)) {
      setOutlineSettingsOpen(true);
      const ids = target.outline?.kind === 'part-envelope' ? target.outline.partIds : board?.partIds ?? [];
      fitParts(document.parts.filter((part) => ids.includes(part.id)), board ? scene.boardContours.find((entry) => entry.boardId === board.id)?.contours ?? [] : scene.contours);
    } else if (target.part) { selectScope({ kind: 'component', partId: target.part.id }); fitParts([target.part]); }
    else if (target.matrix) { selectScope({ kind: 'matrix', matrixId: target.matrix.id }); fitParts(document.parts.filter((part) => target.matrix!.partIds.includes(part.id))); }
    else if (target.body) setCaseBodyId(target.body.id);
    setRightOpen(true);
    if (!compactInspector) inspectorPanel.setMode('pinned');
    requestAnimationFrame(() => globalThis.document.querySelector<HTMLElement>('#wb-inspector button, #wb-inspector input')?.focus());
  };

  useEffect(() => {
    if (pendingFinding) { setPendingFinding(null); showFinding(pendingFinding); }
  }, [selectedBoardId, pendingFinding]);

  const getModeDetails = () => {
    if (scriptsOpen) return <>
      <div className="wb-inspect-head"><h2>Geometry scripts</h2></div>
        <div className="wb-script-heading"><h3 className="wb-subtitle">Scripts</h3><button onClick={addScript}>+ New script</button></div>
        {document.scripts.length > 0 && <label className="wb-script-select-label">Active script<select aria-label="Active script" value={activeScript?.id ?? ''} onChange={(event) => setActiveScriptId(event.target.value)}>
          {document.scripts.map((script) => <option key={script.id} value={script.id}>{script.name}</option>)}
        </select></label>}
        {activeScript ? <div className="wb-script-editor">
          <label className="wb-script-name">Name<input value={scriptName} onChange={(event) => setScriptName(event.target.value)} /></label>
          <label className="wb-script-source-label">Rhai source<textarea spellCheck={false} value={scriptSource} onChange={(event) => setScriptSource(event.target.value)} placeholder="// Describe generated geometry and component groups" /></label>
          <label className="wb-script-enabled"><input type="checkbox" checked={scriptEnabled} onChange={(event) => setScriptEnabled(event.target.checked)} /> Enable on Apply</label>
          <button className="wb-primary wb-script-apply" disabled={!scriptSource.trim()} onClick={applyScript}>Apply script <ArrowIcon /></button>
          <div className="wb-script-diagnostics"><span>Core findings</span><FindingList document={document} onShow={showFinding} findings={scene.findings} /></div>
        </div> : <p className="wb-empty-state">Scripts generate named geometry groups. Apply a script to run it in the core.</p>}
    </>;

    if (mode === 'Design' && outlineSettingsOpen) return <><OutlineInspector document={document} board={selectedBoard}
      onChange={(next, ids) => emit({ kind: 'replace-document', document: next }, ids)}
      selection={outlineSelection} selectedPoint={outlinePoint} onSelectPoint={setOutlinePoint} grid={outlineGrid} onGrid={setOutlineGrid}
      onSelect={selection=>{setOutlineSelection(selection);setOutlinePoint(0);setSelected([]);setScope(null);setOutlineActive(false);setOutlineDraft([]);}}
      onDraw={(operation) => { cancelInteractions(); cancelPlacement(); setTransformTool(null); setOriginPicking(false); setCommandMenu(null); setAssembly3d(false); setOutlineOperation(operation); setOutlineSelection(null); setSelected([]); setScope(null); setOutlineDraft([]); setOutlineActive(true); setRightOpen(false); setLeftOpen(false); }} /><div className="wb-findings-head"><h3 className="wb-subtitle">Findings</h3></div><FindingList document={document} onShow={showFinding} findings={scene.findings} /></>;

    if (mode === 'PCB') {
      const builtinSwitch = activePart && activeDefinition?.kind === 'switch';
      if (wiring && (!activePart || activeDefinition?.kind === 'controller')) return <WiringPanel {...wiring} onResolve={onResolveWiring} onApply={onApplyWiring} onReview={onReviewWiring} onToggleLock={onReviewWiring} />;
      if (builtinSwitch && activePart && activeDefinition) return <><div className="wb-inspect-head"><h2>{activePart.reference} · {activeDefinition.name}</h2><span className="wb-mini-tag">{selectedBoard?.name ?? 'Board'} / PCB</span></div><p className="wb-empty-note">Switch wiring is inherited from its key assembly and resolved by the board wiring plan.</p><InspectorSection title="Named terminals" detail={`${Object.keys(activeDefinition.terminals ?? {}).length}`} defaultOpen><div className="wb-net-map">{Object.entries(activeDefinition.terminals ?? {}).map(([terminal, padIds]) => { const net = boardNets.find((entry) => entry.pins.some((pin) => pin.partId === activePart.id && padIds.includes(pin.padId))); return <div className="wb-net-map-row" key={terminal}><span>{terminal}</span><strong>{net?.name ?? 'Unmapped'}</strong></div>; })}</div></InspectorSection><button className="wb-inspector-link" onClick={() => { setScope(null); setSelected([]); }}>Edit board wiring <ArrowIcon /></button></>;
      const assigned = boardNets.reduce((total, net) => total + net.pins.filter((pin) => selectedBoard?.partIds.includes(pin.partId) ?? true).length, 0);
      return <>
        <div className="wb-inspect-head"><h2>{activePart && activeDefinition ? `${activePart.reference} · ${activeDefinition.name}` : 'Board setup'}</h2><span className="wb-mini-tag">{activePart && activeDefinition ? `${selectedBoard?.name ?? 'Board'} / PCB` : `${document.boards.length} board${document.boards.length === 1 ? '' : 's'}`}</span></div>
        <InspectorSection title="Board details" detail={selectedBoard?.name}>
        {selectedBoard ? <dl className="wb-measure-list">
          <Measure label="Board" value={selectedBoard.name} />
          <Measure label="Thickness" value={`${selectedBoard.thickness.toFixed(2)} mm`} />
          <Measure label="Placed parts" value={`${selectedBoard.partIds.length}`} />
          <Measure label="Net assignments" value={`${assigned}`} />
        </dl> : <p className="wb-empty-note">Add a board in the project setup to begin mapping nets.</p>}
        </InspectorSection>
        <div className="wb-findings-head"><h3 className="wb-subtitle">Connections</h3><span>{activeDefinition?.pads.length ?? 0}</span></div>
        {activePart && activeDefinition ? <div className="wb-net-map">
          {Object.entries(activeDefinition.terminals ?? {}).map(([terminal, padIds]) => {
            const assigned = [...new Set(padIds.map((padId) => boardNets.find((net) => net.pins.some((pin) => pin.partId === activePart.id && pin.padId === padId))?.id).filter((id): id is string => Boolean(id)))];
            return <label className="wb-net-map-row" key={`terminal:${terminal}`}><span>{terminal} terminal</span><select aria-label={`Net for terminal ${terminal}`} value={assigned.length === 1 ? assigned[0] : ''} onChange={(event) => assignTerminal(terminal, padIds, event.target.value)}>
              <option value="">Unmapped</option>{boardNets.map((net) => <option key={net.id} value={net.id}>{net.name}</option>)}
            </select></label>;
          })}
          {activeDefinition.pads.filter((pad) => pad.plated !== false && pad.number !== '' && !isErgogen(activeDefinition.generator?.source) && !Object.values(activeDefinition.terminals ?? {}).some((padIds) => padIds.includes(pad.id))).map((pad) => {
          const assigned = boardNets.find((net) => net.pins.some((pin) => pin.partId === activePart.id && pin.padId === pad.id));
          return <label className="wb-net-map-row" key={pad.id}><span className="wb-pad-number">{pad.number}</span><select aria-label={`Net for pad ${pad.number}`} value={assigned?.id ?? ''} onChange={(event) => assignNet(pad.id, event.target.value)}>
            <option value="">Unmapped</option>{boardNets.map((net) => <option key={net.id} value={net.id}>{net.name}</option>)}
          </select></label>;
        })}</div> : <p className="wb-empty-note">Select a placed part to map its pads to nets.</p>}
        {activePart && Object.keys(activeErgogenParams).some((key) => activeErgogenParams[key].type === 'net' || activeErgogenParams[key].type === 'anchor') && <>
          <div className="wb-panel-rule" /><h3 className="wb-subtitle">Ergogen bindings</h3>
          <div className="wb-net-map">{Object.entries(activeErgogenParams).filter(([key, parameter]) => (parameter.type === 'net' && !activeDefinition?.terminals?.[key]) || parameter.type === 'anchor').map(([key, parameter]) => {
            const value = activePart.generatorParameters?.[key];
            if (parameter.type === 'net') return <label className="wb-net-map-row" key={key}><span>{key}</span><select aria-label={`Ergogen net ${key}`} value={typeof value === 'string' ? value : ''} onChange={(event) => updatePartGeneratorParameter(key, event.target.value || undefined)}><option value="">Default</option>{boardNets.map((net) => <option key={net.name} value={net.name}>{net.name}</option>)}</select></label>;
            const anchor = value && typeof value === 'object' && !Array.isArray(value) ? value as { x?: unknown; y?: unknown } : {};
            const updateAnchor = (axis: 'x' | 'y', raw: string) => {
              const next = { ...anchor };
              if (raw.trim() === '') delete next[axis];
              else {
                const coordinate = Number(raw);
                if (!Number.isFinite(coordinate)) return;
                next[axis] = coordinate;
              }
              updatePartGeneratorParameter(key, Object.keys(next).length ? next as JsonValue : undefined);
            };
            return <div className="wb-net-map-row wb-anchor-row" key={key}><span>{key}</span><label>{key} X<input type="number" aria-label={`Ergogen anchor ${key} X`} value={typeof anchor.x === 'number' ? anchor.x : ''} placeholder="Part X" onChange={(event) => updateAnchor('x', event.target.value)} /></label><label>{key} Y<input type="number" aria-label={`Ergogen anchor ${key} Y`} value={typeof anchor.y === 'number' ? anchor.y : ''} placeholder="Part Y" onChange={(event) => updateAnchor('y', event.target.value)} /></label></div>;
          })}</div>
        </>}
        <form className="wb-new-net" onSubmit={(event) => { event.preventDefault(); addNet(); }}>
          <input aria-label="New net name" placeholder="New net name" value={newNetName} onChange={(event) => setNewNetName(event.target.value)} />
          <button type="submit" disabled={!newNetName.trim()}>Add net</button>
        </form>
        <div className="wb-panel-rule" />
        <InspectorSection title="Electrical nets" detail={String(boardNets.length)}>
        <div className="wb-net-list">{boardNets.map((net) => <div className="wb-net-row" key={net.id}>
          <span className="wb-net-swatch" /> <span>{net.name}</span><small>{net.pins.filter((pin) => selectedBoard?.partIds.includes(pin.partId) ?? true).length} pins</small>
        </div>)}</div>
        </InspectorSection>
      </>;
    }
    if (mode === 'Case') {
      const configuredBoard = document.boards.find((board) => board.id === caseDocument.mechanical?.boardId);
      const instanceSetup = !selectedMechanicalLayer && !(guideVisible && guide.currentStep === 'project') && instanceControls && <div className="wb-case-instance-controls">{instanceControls}</div>;
      const mechanicalPanel = caseDocument.mechanical && !generatedCase
        ? <section className="wb-mechanical-panel"><h3>Mechanical stack belongs to {configuredBoard?.name ?? caseDocument.mechanical.boardId}</h3><p>This board shows its authored case bodies. Select a physical assembly to configure a separate case.</p><button className="wb-secondary" disabled={!configuredBoard} onClick={() => configuredBoard && selectBoard(configuredBoard.id)}>Show configured board</button></section>
        : <MechanicalAssemblyPanel livePreview={livePreview} onLivePreviewChange={onLivePreviewChange} readiness={mechanicalReadiness} diagnosticsRequest={diagnosticsRequest} onDiagnosticsShown={() => setDiagnosticsRequest(0)} generationTarget={caseActionsTarget} onRevealDiagnostics={revealMechanicalInspector} document={caseDocument} boardId={selectedBoardId} projectSession={projectSession} definitions={libraryDefinitions} configuration={caseDocument.mechanical} assembly={mechanicalAssembly} onChange={(configuration) => emit({ kind: 'set-mechanical', configuration }, [document.id])} onResolve={onResolveMechanical} onCancel={onCancelGeneration} generation={generation} onExport={onExportMechanical} onMechanicalProfile={onMechanicalProfile} onExtractMechanicalProfile={onExtractMechanicalProfile} onEditParts={(definitionId) => { changeMode('Library'); setLibraryChoice(definitionId); setLibraryAssembly(null); setLibrary3dOpen(false); setRightOpen(true); setLeftOpen(false); }} onShowFinding={showFinding} selectedLayer={selectedMechanicalLayer} onSelectLayer={setSelectedMechanicalLayer} />;
      if (generatedCase) return <>{instanceSetup}{mechanicalPanel}{!selectedMechanicalLayer && <p className="wb-empty-note">Generated assembly preview · {document.caseBodies.filter((body) => body.boardId === selectedBoardId).length} authored case bodies remain saved. Disable the mechanical stack to preview and edit them.</p>}</>;
      return <CaseInspectorPanel bodies={document.caseBodies} activeCaseBody={activeCaseBody} selectedBoard={selectedBoard}
        findings={scene.findings} findingDocument={document} onShowFinding={showFinding}
        mechanicalPanel={mechanicalPanel} instanceSetup={instanceSetup} caseActionsTarget={caseActionsTarget}
        generation={generation} livePreview={livePreview} onLivePreviewChange={onLivePreviewChange} canConfigure={!document.mechanical} previewCurrent={casePreview?.revision === scene.revision && caseReady}
        onConfigure={() => { revealMechanicalInspector(); requestAnimationFrame(() => globalThis.document.querySelector<HTMLButtonElement>('.wb-mech-start button')?.focus()); }}
        onResolveMechanical={onResolveMechanical} onCancelGeneration={onCancelGeneration}
        setCaseBodyId={setCaseBodyId} addCaseBody={addCaseBody} updateCaseBody={updateCaseBody}
        addMount={addMount} updateMount={updateMount} removeMount={removeMount}
      />;
    }
    if (mode === 'Library') return <PartsInspectorPanel
      document={document} libraryAssembly={libraryAssembly} libraryDefinitions={libraryDefinitions}
      assemblyOrientation={assemblyOrientation} setAssemblyOrientation={setAssemblyOrientation} setEditingAssembly={setEditingAssembly}
      onPlaceAssembly={(preset) => beginMatrixPlacement(1, 1, preset)}
      onPlaceDefinition={(definition) => scope?.kind === 'key' ? placeLibraryDefinition(definition) : beginPartPlacement(definition)}
      applyToKey={scope?.kind === 'key'} selectedLibraryDefinition={selectedLibraryDefinition}
      editDefinition={editDefinition} activeModelDefinition={activeModelDefinition} onImportModel={onImportModel}
      definitionError={definitionError} actions={libraryActions}
      generator={{ enabled: ergogenGenerator, edits: libraryParameters, error: generatorPreview?.error,
        compilePending: previewCompilePending, compileError: previewCompileError, onChange: updateGenerator, onSave: saveGenerator }}
    />;
    if (mode === 'Export') {
      const outputs: { label: string; ready: boolean; detail: string; kind: ExportKind }[] = [
        { label: 'KiCad board', ready: boardReady && wiring?.ready === true, detail: 'Placements, resolved wiring, and board edges for KiCad', kind: 'kicad' },
        { label: 'Draft KiCad board', ready: boardReady, detail: 'Incomplete wiring with a findings report', kind: 'kicad-draft' },
        { label: 'ZMK firmware', ready: Boolean(wiring?.ready && wiring.controller && wiring.assignments?.length), detail: 'ZMK v0.3.0 configuration and editable starter keymap', kind: 'firmware' },
        { label: 'KiCad footprints', ready: document.definitions.length > 0, detail: 'Component footprint library', kind: 'footprints' },
        { label: 'SVG board outline', ready: selectedBoardReadiness?.outline ?? readiness.outline, detail: 'Resolved board contour', kind: 'svg' },
        { label: 'DXF board outline', ready: selectedBoardReadiness?.outline ?? readiness.outline, detail: 'Resolved board contour', kind: 'dxf' },
        { label: generatedCase ? 'Authored Case STEP' : 'Case STEP', ready: authoredCaseReady, detail: 'Saved authored case bodies', kind: 'case-step' },
      ];
      return <>
        <div className="wb-inspect-head"><h1>Export package</h1></div>
        <p className="wb-export-intro">Choose a handoff for {selectedBoard?.name ?? 'this board'}. Downloads use the current committed design; routing and mechanical fit still need downstream review.</p>
        <h2>Design files</h2>
        <div className="wb-export-list">{outputs.map((output) => <div className="wb-export-row" key={output.label}>
          <span className={`wb-ready-dot ${output.ready ? 'is-ready' : ''}`} />
          <span><strong>{output.label}</strong><small>{output.detail}</small>{!output.ready && <small className="wb-export-reason">{output.kind === 'firmware' || output.kind === 'kicad' ? 'Review the layout and resolve controller wiring in PCB.' : output.kind === 'case-step' ? 'Add and generate case geometry in Case.' : 'Review the board outline and layout findings.'}</small>}</span>
          <button className="wb-export-status wb-export-button" aria-label={`Export ${output.label}`} disabled={!output.ready} onClick={() => onExport(output.kind, selectedBoardId || undefined)}>{output.ready ? 'Export' : 'Needs work'}</button>
        </div>)}</div>
        {generatedCase && <div className="wb-export-row"><span className={`wb-ready-dot ${generatedCaseReady ? 'is-ready' : ''}`} /><span><strong>Generated mechanical package</strong><small>STEP/STL parts, outlines, specifications and FR4 plate project</small></span><button className="wb-export-status wb-export-button" disabled={!generatedCaseReady} onClick={onExportMechanical}>{generatedCaseReady ? 'Export' : 'Needs work'}</button></div>}
        <div className="wb-export-next"><span>Need to finish something?</span><button onClick={() => { changeMode('PCB'); setScope(null); setSelected([]); revealInspector(); }}>Review wiring</button><button onClick={() => { changeMode('Case'); revealInspector(); }}>Review case</button></div>
        <h2>Portable project</h2><p>Keep an editable copy of the whole project, including all boards.</p>
        {onEmbedUsedModelsChange && <label className="wb-export-option"><input type="checkbox" aria-label="Embed used models" checked={embedUsedModels} onChange={(event) => onEmbedUsedModelsChange(event.target.checked)} /><span><strong>Embed used models</strong><small>Include attached 3D model files used in this project.</small></span></label>}
        <button className="wb-primary wb-export-action" onClick={() => onExport('project')}>Save .boardstudio project <ArrowIcon /></button>
      </>;
    }
    if (selectedMatrix && scope && scope.kind !== 'component') return <MatrixInspectorPanel
      title={selectionTitle}
      matrix={selectedMatrix}
      scope={scope}
      layout={activeLayout}
      partnerLayout={partnerLayout}
      linkedLayout={linkedLayout}
      selectedKeycaps={selectedKeycaps}
      overlapReferences={selectedKeycapOverlapReferences}
      onKeycapResize={commitKeycapResize}
      onEdit={onEdit}
      onLayoutChange={(layout) => emit({ kind: 'set-layout', layout }, [layout.id])}
      onUnlink={(layout) => emit({ kind: 'set-layout', layout: { ...layout, mirrorLink: undefined } }, [layout.id])}
      onDuplicateDesign={onDuplicateDesign}
      document={document}
      catalog={libraryDefinitions}
      projection={matrixScenes.get(selectedMatrix.id)}
      onSplay={(column, change) => commitSplay(selectedMatrix, column, change)}
      splayAffect={splayAffect}
      onSplayAffectChange={setSplayAffect}
      onPickOrigin={() => { setTransformTool('origin'); setOriginPicking(true); setRightOpen(false); }}
      parts={treeParts}
      members={memberMaps.get(selectedMatrix.id) ?? new Map()}
      isMirrorTarget={Boolean(activeLayout?.mirrorLink)}
      onChange={(matrix, definitions) => commitMatrix(matrix, 'commit', undefined, definitions)}
      onDelete={() => { emit({ kind: 'remove-matrix', id: selectedMatrix.id }, [selectedMatrix.id, ...selectedMatrix.partIds]); setScope(null); setSelected([]); }}
    />;
    return <>
      <div className="wb-inspect-head"><h2>{selectionTitle}</h2>
        {activePart?.locked && <span className="wb-mini-tag is-locked">Locked</span>}</div>
      {activePart && activeDefinition && scope?.kind === 'component' ? <>
        <p className="wb-part-title">{activeDefinition.name}<span>{activeDefinition.kind}</span></p>
        {hasMirroredComponent && <InspectorSection title="Mirrored component" detail={mirroredComponentIsSubstitute ? 'Substituted' : 'Linked'} defaultOpen>
          <p className="wb-empty-note">{mirroredComponentIsSubstitute ? 'This component pair is independent. Changes on either half stay local.' : 'Placement and component changes follow the paired half.'}</p>
          <label className="wb-script-select-label">Replace with<select aria-label="Replace mirrored component" value={activePart.definitionId} onChange={(event) => updateComponentDefinition(event.target.value, Boolean(mirroredComponentSourceId))}>
            {partChoices(libraryDefinitions, activePart.definitionId).map((definition) => <option key={definition.id} value={definition.id}>{partCatalogLabel(definition)}</option>)}
          </select></label>
          {!mirroredComponentIsSubstitute
            ? <button className="wb-secondary" onClick={() => setComponentSubstitute(true)}>Keep only on this half</button>
            : <button className="wb-secondary" onClick={() => setComponentSubstitute(false)}>Use mirrored component</button>}
        </InspectorSection>}
        {boardLayouts.length > 0 && !matrixPartLookup.has(activePart.id) && <label className="wb-component-layout">Layout<select aria-label="Component layout" value={activeLayout?.id ?? ''} onChange={(event) => assignPartLayout(activePart, event.target.value)}><option value="">Board / ungrouped</option>{boardLayouts.map((layout) => <option key={layout.id} value={layout.id}>{layout.name}</option>)}</select></label>}
        {activeDefinition.envelopeNotice && <p className="wb-empty-note">{activeDefinition.envelopeNotice}</p>}
        <h3 className="wb-subtitle">Position <small>millimetres</small></h3>
        <div className="wb-coordinate-grid">
          <Coordinate label="X" value={activePart.pose.at.x} onCommit={(value) => updatePosition('x', value)} />
          <Coordinate label="Y" value={activePart.pose.at.y} onCommit={(value) => updatePosition('y', value)} />
        </div>
        <InspectorSection title="Board outline" detail={activePart.outline?.excluded ? 'Excluded' : 'Included'}>
        <PartOutlineControls part={activePart} definition={activeDefinition} onChange={(next) => emit({ kind: 'replace-document', document: { ...document, parts: document.parts.map((part) => part.id === next.id ? next : part) } }, [next.id])} />
        </InspectorSection>
        <InspectorSection title="Layout constraint" detail={activeConstraint ? 'Active' : 'Optional'} defaultOpen={Boolean(activeConstraint)}>
        {visibleParts.length > 1 ? <div className="wb-constraint-form">
          <label>Relationship
            <select aria-label="Constraint type" value={constraintKind} onChange={(event) => setConstraintKind(event.target.value as Constraint['kind'])}>
              <option value="offset">Offset from part</option>
              <option value="mirror">Mirror placement across axis</option>
            </select>
          </label>
          <label>Source part
            <select aria-label="Constraint source part" value={constraintSourceId} onChange={(event) => setConstraintSourceId(event.target.value)}>
              {visibleParts.filter((part) => part.id !== activePart.id).map((part) => <option key={part.id} value={part.id}>{part.reference}</option>)}
            </select>
          </label>
          {constraintKind === 'offset' ? <>
            <div className="wb-constraint-number-grid">
              <ConstraintNumber label="Offset X (mm)" value={constraintX} onChange={setConstraintX} />
              <ConstraintNumber label="Offset Y (mm)" value={constraintY} onChange={setConstraintY} />
            </div>
            <ConstraintNumber label="Rotation (degrees)" value={constraintRotation} onChange={setConstraintRotation} />
          </> : <>
            <label>Axis
              <select aria-label="Mirror axis" value={constraintAxis} onChange={(event) => setConstraintAxis(event.target.value as 'vertical' | 'horizontal')}>
                <option value="vertical">Vertical</option>
                <option value="horizontal">Horizontal</option>
              </select>
            </label>
            <ConstraintNumber label="Axis coordinate (mm)" value={constraintCoordinate} onChange={setConstraintCoordinate} />
            <p className="wb-constraint-note">Footprint geometry stays unchanged. Use a handed definition where needed.</p>
          </>}
          {activeConstraint && <p className="wb-constraint-note">{parts.get(activeConstraint.sourcePartId)?.reference ?? 'A part'} drives {activePart.reference}.</p>}
          <div className="wb-constraint-actions">
            <button type="button" onClick={saveConstraint} disabled={!constraintSourceId}>{activeConstraint ? 'Save constraint' : 'Add constraint'}</button>
            {activeConstraint && <button type="button" className="is-remove" onClick={removeConstraint}>Remove</button>}
          </div>
        </div> : <p className="wb-empty-note">Add another part on this board to create a layout constraint.</p>}
        </InspectorSection>
        <button className="wb-inspector-link" onClick={() => changeMode('PCB')}>Edit electrical connections <ArrowIcon /></button>
        {activeParts.length > 1 && <p className="wb-selection-count">{activeParts.length} parts selected. Position edits apply to selection.</p>}
      </> : <>
        <p className="wb-empty-note">Select a key, component, or matrix to edit it.</p>
        {selectedBoard && <label className="wb-object-name">Board name<input aria-label="Board name" value={boardName} onChange={(event) => setBoardName(event.target.value)} onBlur={commitBoardName} onKeyDown={(event) => { if (event.key === 'Enter') event.currentTarget.blur(); if (event.key === 'Escape') setBoardName(selectedBoard.name); }} /></label>}
        <div className="wb-empty-measure"><span>Outline</span><strong>{readiness.outline ? 'Resolved' : 'Not defined'}</strong></div>
        <div className="wb-empty-measure"><span>Placed parts</span><strong>{visibleParts.length}</strong></div>

      </>}

    </>;
  };

  const guideTargets = new Set(guideVisible ? [document.id, selectedBoardId, ...(selectedBoard?.partIds ?? []), ...(selectedBoard?.outlineIds ?? []), ...visibleMatrices.map(matrix => matrix.id)] : []);
  const guideStages = guideVisible ? deriveSetupGuide({
    document, boardId: selectedBoardId,
    wiring: wiringStatus ?? { current: false, ready: false, applied: false },
    caseReadiness: { configured: generatedCase || Boolean(activeCaseBody), canExport: caseReady, message: generatedCase ? mechanicalReadiness.message : authoredCaseReady ? 'Authored case geometry is ready.' : 'Review the authored case geometry.' },
    layoutErrorCount: presentedFindings(scene.findings, document).filter(finding => finding.severity === 'error' && (!finding.targetIds.length || finding.targetIds.some(id => guideTargets.has(id)))).length,
  }) : undefined;
  const chooseSetupStage = (step: SetupGuideStage) => {
    guide.setCurrentStep(step);
    changeMode(step === 'wiring' ? 'PCB' : step === 'case' ? 'Case' : step === 'review' ? 'Export' : 'Design');
    setScope(null); setSelected([]);
    setLeftOpen(true);
    if (compactInspector) setRightOpen(false);
    else if (step === 'wiring' || step === 'case' || step === 'review') inspectorPanel.setMode('pinned');
  };
  const showSetupControls = (next: Mode) => {
    changeMode(next); setScope(null); setSelected([]);
    setRightOpen(true);
    if (!compactInspector) inspectorPanel.setMode('pinned');
    requestAnimationFrame(() => globalThis.document.querySelector<HTMLElement>('#wb-inspector .wb-inspector-content :is(input, select, button):not(:disabled)')?.focus());
  };
  const dismissGuide = () => {
    guide.setOpen(false);
    requestAnimationFrame(() => projectTriggerRef.current?.focus({ preventScroll: true }));
  };
  const reopenGuide = () => {
    if (mode === 'Library') changeMode('Design');
    guide.setOpen(true); setLeftOpen(true);
    if (compactInspector) setRightOpen(false);
    if (!compactObjects) objectsPanel.setMode('pinned');
  };
  const guideContent = guide.currentStep === 'project' ? <>
    <label>Project name<input aria-label="Project name" value={projectName} onChange={event => setProjectName(event.target.value)} onBlur={commitProjectName} onKeyDown={event => { if (event.key === 'Enter') event.currentTarget.blur(); }} /></label>
    <label>Board<select aria-label="Setup board" value={selectedBoardId} onChange={event => selectBoard(event.target.value)}>{document.boards.map(board => <option key={board.id} value={board.id}>{board.name}</option>)}</select></label>
    {instanceControls}
    <button className="wb-primary" onClick={() => chooseSetupStage('layout')}>Continue to layout</button>
  </> : guide.currentStep === 'layout' ? <>
    <p>Place keys with their switch, diode, and optional lighting assembly. Edit the layout on the canvas.</p>
    <button className="wb-primary" onClick={() => { guide.setOpen(false); changeMode('Design'); setGuidedPlacement(true); createGuidedMatrix(); }}>Add key matrix</button>
    <button className="wb-secondary" onClick={dismissGuide}>Edit existing objects</button>
    <button className="wb-secondary" onClick={() => chooseSetupStage('wiring')}>Continue to wiring</button>
  </> : guide.currentStep === 'wiring' ? <>
    <p>Place a controller, then review automatic pin assignments and reversible jumpers in PCB settings.</p>
    <button className="wb-secondary" onClick={() => { guide.setOpen(false); changeMode('Library'); setLibrarySearch('controller'); setLeftOpen(true); if (!compactObjects) objectsPanel.setMode('pinned'); }}>Choose controller</button>
    <button className="wb-primary" onClick={() => showSetupControls('PCB')}>Review controller & wiring</button>
    <button className="wb-secondary" onClick={() => chooseSetupStage('case')}>Continue to case</button>
  </> : guide.currentStep === 'case' ? <>
    <p>Configure construction and clearances, then generate geometry when you are ready. PCB and firmware exports are available separately.</p>
    <button className="wb-primary" onClick={() => showSetupControls('Case')}>Open case settings</button>
    <button className="wb-secondary" onClick={() => chooseSetupStage('review')}>Continue to review</button>
  </> : <>
    <p>Review the selected board’s findings and available outputs. Export checks still apply to each output.</p>
    <button className="wb-primary" onClick={() => showSetupControls('Export')}>Open export options</button>
    <button className="wb-secondary" onClick={dismissGuide}>Finish guide</button>
  </>;

  const pairPreview = matrixGhost && pairPlacement ? pairAt(matrixGhost, pairPlacement, matrixGhost.origin) : undefined;

  const addContent = <>
    <section className="wb-add-section" aria-label="Layouts"><h3>Layouts</h3>
      <button className="wb-add-matrix wb-pair-icon" aria-label="Mirrored pair…" onClick={() => { changeMode('Design'); setPlacement({ kind: 'pair-setup' }); setRightOpen(false); }}><MirrorPairIcon /><span>Mirrored pair…<small>Linked halves, independent hardware</small></span></button>
      <button className="wb-add-matrix" disabled={!unpairedMatrices.length} onClick={() => { setExistingHalfOpen(true); setAddPartOpen(false); }}><MirrorPairIcon /><span>Mirror existing half…<small>Link a copy of your current layouts</small></span></button>
      <button className="wb-add-matrix" aria-label="Matrix…" onClick={createGuidedMatrix}><ScopeIcon kind="matrix" /><span>Matrix…<small>Rows, columns & key assemblies</small></span></button>
    </section>
    {selectedMatrix && <section className="wb-add-section" aria-label="Selected matrix"><h3>{selectedMatrix.name || 'Selected matrix'}</h3><div className="wb-add-inline"><button data-close-menu onClick={() => commitMatrix(resizeMatrix(selectedMatrix, selectedMatrix.rows + 1, selectedMatrix.columns))}><ToolIcon name="add" />Add row</button><button data-close-menu onClick={() => commitMatrix(resizeMatrix(selectedMatrix, selectedMatrix.rows, selectedMatrix.columns + 1))}><ToolIcon name="add" />Add column</button></div></section>}
    <section className="wb-add-section" aria-label="Parts"><h3>Parts</h3>
      {boardLayouts.length > 0 && <label className="wb-placement-layout">Place in<select aria-label="Part placement layout" value={layoutTargetId} onChange={(event) => setLayoutTargetId(event.target.value)}><option value="">Board / ungrouped</option>{boardLayouts.map((layout) => <option key={layout.id} value={layout.id}>{layout.name}</option>)}</select></label>}
      <input autoFocus type="search" aria-label="Search parts" placeholder="Find a part…" value={librarySearch} onChange={(event) => setLibrarySearch(event.target.value)} />
      {librarySearch.trim() ? <div className="wb-add-part-results">{filteredLibrary.map((definition) => <button key={definition.id} onClick={() => beginPartPlacement(definition)}><PartGlyph kind={definition.kind} /><span>{partCatalogLabel(definition)}</span></button>)}{!filteredLibrary.length && <p>No parts match this search.</p>}</div> : [
        { name: 'Components', match: (item: PartDefinition) => /power|reset|battery/i.test(item.name) },
        { name: 'Controllers', match: (item: PartDefinition) => item.kind === 'controller' },
        { name: 'Displays', match: (item: PartDefinition) => /display|oled|nice.view/i.test(item.name) },
        { name: 'Encoders', match: (item: PartDefinition) => item.kind === 'encoder' || /encoder/i.test(item.name) },
      ].map((group) => <details className="wb-add-category" key={group.name} open={group.name === 'Components'}><summary>{group.name}</summary><div className="wb-add-part-results">{availableLibrary.filter(group.match).map((definition) => <button key={definition.id} onClick={() => beginPartPlacement(definition)}><PartGlyph kind={definition.kind} /><span>{partCatalogLabel(definition)}</span></button>)}</div></details>)}
      <button className="wb-add-browse" data-close-menu onClick={() => { changeMode('Library'); setLeftOpen(true); if (!compactObjects) objectsPanel.setMode('pinned'); }}>Browse all parts<ArrowIcon /></button>
    </section>
    <section className="wb-add-section" aria-label="Board geometry"><h3>Board geometry</h3><button data-close-menu onClick={() => { changeMode('Design'); setOutlineSettingsOpen(true); revealInspector(); }}>Board outline…</button><button data-close-menu onClick={() => { changeMode('Design'); setScriptsOpen(true); revealInspector(); }}>Geometry scripts…</button></section>
  </>;

  sceneHandlers.current = { startDrag, moveDrag, endDrag, choosePart, nudgePart, hoverPart: setHoveredPart };

  return <main className={`wb-root wb-unified is-${mode.toLowerCase()}`} data-guide-open={guideVisible} data-objects-mode={objectsPanel.mode} data-inspector-mode={inspectorPanel.mode} data-revision={document.revision} style={{
    ...(objectsPanel.width ? { '--wb-navigator-width': `${objectsPanel.width}px` } : {}),
    ...(inspectorPanel.width ? { '--wb-inspector-width': `${inspectorPanel.width}px` } : {}),
  } as React.CSSProperties}>
    <header className="wb-topbar">
      <div className="wb-project-menu">
        <button ref={projectTriggerRef} className="wb-project-trigger" aria-expanded={projectMenuOpen} aria-controls="wb-project-dropdown" aria-label="Project" title={`Project menu — ${document.name}`} onClick={() => { setProjectMenuOpen(open => !open); setProjectPage('project'); setCommandMenu(null); }}><span className="wb-brand-mark"><BrandMark /></span><span className="wb-project-name">{document.name}</span><ToolIcon name="chevron" /></button>
        {projectMenuOpen && <div id="wb-project-dropdown" className="wb-project-dropdown" data-page={projectPage} aria-label="Project menu" onKeyDown={event => { if (event.key === 'Escape') { event.stopPropagation(); setProjectMenuOpen(false); projectTriggerRef.current?.focus({ preventScroll: true }); } }}>
          <div className="wb-project-menu-heading"><strong>{projectPage === 'settings' ? 'Workspace settings' : 'Keyboards'}</strong><button type="button" aria-label="Close project menu" onClick={() => { setProjectMenuOpen(false); projectTriggerRef.current?.focus({ preventScroll: true }); }}><svg viewBox="0 0 20 20" aria-hidden="true"><path d="m5 5 10 10M15 5 5 15" /></svg></button></div>
          {projectPage === 'settings' ? <div className="wb-workspace-preferences">
            <button className="wb-back-link" onClick={() => { setProjectPage('project'); requestAnimationFrame(() => globalThis.document.getElementById('wb-preferences-entry')?.focus({ preventScroll: true })); }}><ArrowIcon />Back to project menu</button>
          <label>Appearance<select aria-label="Color theme" value={themePreference} onChange={event => chooseTheme(event.target.value as ThemePreference)}><option value="system">System</option><option value="light">Light</option><option value="dark">Dark</option></select></label>
          <div className="wb-menu-section"><strong>Panels</strong><p>Restore the Objects and Inspect panels to their default size and pinned position.</p><button onClick={() => { objectsPanel.reset(); inspectorPanel.reset(); setLeftOpen(false); setRightOpen(false); }}>Restore default panels</button></div>
          <p>Workspace preferences apply in this browser. Board dimensions, wiring, and case settings belong to their editing views.</p>
          </div> : <>
          <div className="wb-keyboard-library-actions">
            {onNewProject && <button className="wb-library-new" onClick={() => { setProjectMenuOpen(false); onNewProject(); }}><ProjectLibraryIcon name="new" />New project</button>}
            {onImport && <button className="wb-open-project" onClick={() => projectFileRef.current?.click()}><ProjectLibraryIcon name="open" />Open project…</button>}
          </div>
          <div className="wb-keyboard-current-project">
            <label className="wb-project-title"><span>Current project</span><input aria-label="Project name" title="Rename project" value={projectName} onChange={event => setProjectName(event.target.value)} onBlur={commitProjectName} onKeyDown={event => { if (event.key === 'Enter') event.currentTarget.blur(); if (event.key === 'Escape') setProjectName(document.name); }} /></label>
            <button title="Save project copy…" onClick={() => { setProjectMenuOpen(false); onExport('project'); }}><ProjectLibraryIcon name="download" /><span>Save project copy…</span></button>
          </div>
          {onImport && <input ref={projectFileRef} className="wb-project-file-input" type="file" accept=".boardstudio" tabIndex={-1} onChange={event => {
            const file = event.currentTarget.files?.[0];
            if (file) onImport(file);
            event.currentTarget.value = '';
            setProjectMenuOpen(false);
          }} />}
          <ProjectLibrary document={document} onOpen={onOpenSavedProject && (id => { setProjectMenuOpen(false); onOpenSavedProject(id); })} onOpenDemo={onOpenDemo && (id => { setProjectMenuOpen(false); onOpenDemo(id); })} />
          <div className="wb-project-workspace-actions">
            <div className="wb-project-history"><button className="wb-compact-action" onClick={() => { setProjectMenuOpen(false); onUndo(); }}><UndoIcon />Undo</button><button className="wb-compact-action" onClick={() => { setProjectMenuOpen(false); onRedo(); }}><RedoIcon />Redo</button></div>
            <button onClick={() => { setProjectMenuOpen(false); reopenGuide(); }}><ProjectLibraryIcon name="guide" />Setup guide</button>
            <button id="wb-preferences-entry" onClick={() => { setProjectPage('settings'); requestAnimationFrame(() => globalThis.document.querySelector<HTMLElement>('.wb-workspace-preferences .wb-back-link')?.focus({ preventScroll: true })); }}><ProjectLibraryIcon name="settings" />Workspace settings</button>
          </div>
          </>}
        </div>}
      </div>
      <details className={`wb-save-state is-${saveStatus ?? 'unknown'}`} onKeyDown={(event) => { if (event.key === 'Escape') { event.stopPropagation(); event.currentTarget.open = false; event.currentTarget.querySelector('summary')?.focus(); } }}><summary aria-label={saveStatus === 'saved' ? 'Saved locally' : saveStatus === 'saving' ? 'Saving locally' : saveStatus === 'failed' ? 'Local save failed' : 'Local save status unavailable'}><i aria-hidden="true" /><span>{saveStatus === 'saved' ? 'Saved locally' : saveStatus === 'saving' ? 'Saving…' : saveStatus === 'failed' ? 'Save failed' : 'Save status'}</span></summary><div role="status">{saveStatus === 'saved' ? 'Changes are saved in this browser. Use Save project copy for a portable backup.' : saveStatus === 'saving' ? 'Saving changes in this browser…' : saveStatus === 'failed' ? 'Changes could not be saved locally. Check available browser storage before editing further.' : 'Local save status is unavailable.'}</div></details>
      <WorkflowNavigation mode={mode} onNavigate={changeMode} />
      <div className="wb-top-actions">
        {compactObjects && mode !== 'Export' && <button id="wb-objects-toggle" className="wb-icon-button wb-panel-toggle" aria-label="Objects" title="Objects" aria-expanded={leftOpen} aria-controls="wb-inventory" onClick={() => { setLeftOpen(!leftOpen); setRightOpen(false); }}><PanelIcon side="left" /></button>}
        {compactInspector && mode !== 'Export' && <button id="wb-inspector-toggle" className="wb-icon-button wb-panel-toggle" aria-label={mode === 'Case' ? 'Case settings' : 'Inspect'} title={mode === 'Case' ? 'Case settings' : 'Inspect'} aria-expanded={rightOpen} aria-controls="wb-inspector" onClick={() => { setRightOpen(!rightOpen); setLeftOpen(false); }}><PanelIcon side="right" /></button>}
        <button className="wb-icon-button wb-export-trigger" aria-label="Export" title="Export" aria-pressed={mode === 'Export'} onClick={() => changeMode('Export')}><svg viewBox="0 0 20 20" aria-hidden="true"><path d="M4 12v5h12v-5M10 13V3M6 7l4-4 4 4" /></svg><span>Export</span></button>
      </div>
    </header>
    <p className="wb-keyboard-status" role="status" aria-label="Keyboard movement" aria-live="polite">{keyboardStatus}</p>
    <section className="wb-workspace" id="workbench">
      {(mode !== 'Export' || guideVisible) && <WorkspacePanel options={<label>Group objects<select aria-label="Tree grouping" value={treeGrouping} onChange={(event) => { const value = event.target.value as 'row' | 'column'; setTreeGrouping(value); try { localStorage.setItem('boardstudio:v2:tree-grouping', value); } catch { /* Session only. */ } }}><option value="column">Columns</option><option value="row">Rows</option></select></label>} side="left" label={guideVisible ? 'Setup guide' : 'Part inventory'} settings={objectsPanel} compact={compactObjects} open={leftOpen} onClose={() => setLeftOpen(false)} toolbar={<>
        <span className="wb-panel-title">{mode === 'Library' ? 'Parts' : guideVisible ? 'Setup' : 'Objects'}</span>
      </>} primaryAction={!guideVisible && (mode === 'Design' || mode === 'PCB') && <button ref={addPartRef} className="wb-add-trigger" aria-label="Add object" aria-expanded={addPartOpen} aria-controls="wb-add-part" onClick={() => { setAddPartOpen((open) => !open); setCommandMenu(null); }}><ToolIcon name="add" /> Add object</button>}>

        {guideVisible && guideStages ? <SetupGuide boardName={selectedBoard?.name ?? 'Select a board'} stages={guideStages} currentStep={guide.currentStep} onStepChange={chooseSetupStage} onDismiss={dismissGuide} currentStepContent={guideContent} /> : mode === 'Library' ? <PartsLibrary definitions={libraryDefinitions} assemblies={(Object.keys(matrixPresetDefinitions) as MatrixPresetId[]).map((id) => ({ id, name: assemblyName(id), definitionId: matrixPresetDefinitions[id].definitionId }))} query={librarySearch} selected={libraryAssembly ? `assembly:${libraryAssembly}` : selectedLibraryDefinition?.id ?? ''} onCreate={addCustomDefinition} onImport={onImportFootprint} onSearch={setLibrarySearch} onSelect={(id) => { setEditingAssembly(null); setScriptsOpen(false); setLibraryChoice(id); setLibraryAssembly(null); setLibrary3dOpen(false); setRightOpen(true); setLeftOpen(false); }} onAssembly={(id) => { setEditingAssembly(null); setScriptsOpen(false); const preset = id as MatrixPresetId; setLibraryAssembly(preset); setLibraryChoice(matrixPresetDefinitions[preset].definitionId); setLibrary3dOpen(false); setRightOpen(true); setLeftOpen(false); }} /> : addPartOpen ? <div id="wb-add-part" className="wb-add-inventory" role="dialog" aria-label="Add" onKeyDown={(event) => { if (event.key === 'Escape') { event.stopPropagation(); setAddPartOpen(false); addPartRef.current?.focus(); } }} onClick={(event) => { if ((event.target as Element).closest('[data-close-menu]')) setAddPartOpen(false); }}><div className="wb-add-heading"><strong>Add to {selectedBoard?.name ?? 'board'}</strong><button className="wb-back-link" onClick={() => { setAddPartOpen(false); addPartRef.current?.focus(); }}><ArrowIcon />Back to objects</button></div>{addContent}</div> : <>

        <div className="wb-board-picker"><label htmlFor="wb-board-select">Board</label><div><select id="wb-board-select" aria-label="Selected board" value={selectedBoardId} onChange={(event) => selectBoard(event.target.value)}>{document.boards.map((board) => <option key={board.id} value={board.id}>{board.name}</option>)}</select><button type="button" onClick={addBoard} aria-label="New board"><ToolIcon name="add" /></button></div></div>

        {matrixSetup && <form className="wb-matrix-setup" aria-label="New matrix" onSubmit={(event) => { event.preventDefault(); const rows = Number(matrixRows); const columns = Number(matrixColumns); if (Number.isInteger(rows) && rows > 0 && Number.isInteger(columns) && columns > 0 && rows * columns <= 4096) beginMatrixPlacement(rows, columns, matrixPreset); }}>
          <h3>New matrix</h3><label>Rows<input autoFocus aria-label="New matrix rows" type="number" min="1" max="4096" required value={matrixRows} onChange={(event) => setMatrixRows(event.target.value)} /></label><label>Columns<input aria-label="New matrix columns" type="number" min="1" max="4096" required value={matrixColumns} onChange={(event) => setMatrixColumns(event.target.value)} /></label>
          <label>Key assembly<select aria-label="New matrix assembly" value={matrixPreset} onChange={(event) => setMatrixPreset(event.target.value as MatrixPresetId)}>{(Object.keys(matrixPresetDefinitions) as MatrixPresetId[]).map((id) => <option key={id} value={id}>{assemblyName(id)}</option>)}</select></label>
          <p>Choose dimensions, then click the board to place.</p><button type="submit" disabled={!matrixRows || !matrixColumns || Number(matrixRows) * Number(matrixColumns) > 4096}>Continue to placement</button><button type="button" onClick={cancelPlacement}>Cancel</button>
        </form>}
        <WorkbenchTree entries={treeEntries} />
        <div className="wb-inventory-foot"><strong>{selectionTitle.split(' · ').at(-1)}</strong><span>{selectedBoard?.name} / {viewLabel}{selectedMatrix ? ` / ${activeLayout?.name || selectedMatrix.name || selectionTitle.split(' · ')[0]}` : ''}</span><span>{selected.length ? `${selected.length} selected` : 'Select an object to edit'}</span></div>
        </>}
      </WorkspacePanel>}

      {mode === 'Export' ? <section className="wb-export-workspace" aria-label="Export workspace"><div className="wb-export-content"><WorkflowReturn returnMode={lastDesignMode} onNavigate={changeMode} />{getModeDetails()}</div></section> : <section className={`wb-canvas-column${mode === 'Case' ? ' is-case-workbench' : ''}`} aria-label={`${mode === 'Library' ? 'Parts' : mode} canvas`}>
        {mode === 'Library' && <div className="wb-task-return"><WorkflowReturn returnMode={lastDesignMode} onNavigate={changeMode} /></div>}
        <div hidden={outlineActive} className={`wb-canvas-toolbar ${mode === 'Design' || mode === 'PCB' ? `is-floating ${commandMenu ? 'is-expanded' : ''}` : ''}`} role="toolbar" aria-label={`${viewLabel} commands`}>
          {outlineFeature ? <div className="wb-outline-canvas-context"><OutlineToolIcon kind={outlineFeature.kind === 'part-envelope' ? 'connect' : outlineFeature.operation} /><strong>Outline points</strong><span>{outlineGrid} mm snap</span><button aria-label="Finish editing outline points" onClick={() => setOutlineSelection(null)}>Done</button></div> : (mode === 'Design' && !assembly3d) || mode === 'PCB' ? <>
            <CommandMenu triggerClassName="wb-compact-select" panelClassName="wb-compact-select-panel" attached id="wb-select-menu" label={`Select: ${selectionMode === 'component' ? 'Part' : selectionMode[0].toUpperCase() + selectionMode.slice(1)}`} icon={<ScopeIcon kind={selectionMode} />} open={commandMenu === 'select'} onOpenChange={(open) => { setCommandMenu(open ? 'select' : null); setAddPartOpen(false); }}>
              <p>{!scope ? 'Choose a selection type, then click a key.' : 'Choose the extent of your selection. The change applies now.'}</p>
              <div role="group" aria-label="Selection scope">{(['matrix', 'row', 'column', 'key', 'component'] as const).map((kind) => <button key={kind} data-close-menu aria-pressed={selectionMode === kind} onClick={() => changeScope(kind)}><ScopeIcon kind={kind} />{kind === 'component' ? 'Part' : kind[0].toUpperCase() + kind.slice(1)}</button>)}</div>

            </CommandMenu>
            <CommandMenu attached id="wb-transform-menu" label="Transform" icon={<ToolIcon name="transform" />} open={commandMenu === 'transform'} onOpenChange={(open) => setCommandMenu(open ? 'transform' : null)}>
              <p>Drag the selection to move it. Arrow keys move parts precisely.</p>
              {(['stagger', 'splay', 'origin'] as const).map((tool) => <button key={tool} aria-pressed={transformTool === tool} disabled={!selectedMatrix} onClick={() => { setTransformTool(transformTool === tool ? null : tool); if (tool !== 'stagger' || scope?.kind !== 'row') changeScope('column'); setCommandMenu(null); }}><ToolIcon name={tool} />{tool[0].toUpperCase() + tool.slice(1)}</button>)}
              <button data-close-menu onClick={() => { setInspectorTab('properties'); revealInspector(); }} disabled={!scope}>Position & rotation</button>
              <button data-close-menu disabled={!selectedMatrix} onClick={() => { changeScope('column'); setInspectorTab('properties'); revealInspector(); }}>Splay & origin</button>
              <button data-close-menu disabled={!selectedMatrix} onClick={() => { changeScope('row'); setInspectorTab('properties'); revealInspector(); }}>Row offsets</button>
            </CommandMenu>
            <CommandMenu attached id="wb-align-menu" label="Align" icon={<ToolIcon name="align" />} open={commandMenu === 'align'} onOpenChange={(open) => setCommandMenu(open ? 'align' : null)}>
              <label>Reference part<select aria-label="Alignment reference" disabled={!alignTargets.length} value={alignTarget?.id ?? ''} onChange={(event) => setAlignTargetId(event.target.value)}>{!alignTargets.length && <option value="">No independent reference parts</option>}{alignTargets.map((part) => <option value={part.id} key={part.id}>{part.reference}</option>)}</select></label>
              <p>Align envelope bounds once. The reference stays fixed. Rotated shapes use axis bounds.</p>
              <div className="wb-align-grid">{([['x', 'min', 'Left'], ['x', 'center', 'Center X'], ['x', 'max', 'Right'], ['y', 'max', 'Top'], ['y', 'center', 'Center Y'], ['y', 'min', 'Bottom']] as const).map(([axis, anchor, label]) => <button data-close-menu key={label} disabled={!activeParts.length || !alignTarget || selectionLocked || scope?.kind === 'row'} onClick={() => alignSelection(axis, anchor)}>{label}</button>)}</div>
              {(selectionLocked || scope?.kind === 'row') && <p>{selectionLocked ? 'Unlock or remove the driving constraint before aligning.' : 'A row can cross differently splayed columns. Align individual keys or a column.'}</p>}
              <button data-close-menu disabled={!scope} onClick={() => { setInspectorTab('relations'); revealInspector(); }}>Relationships</button>
            </CommandMenu>
            <CommandMenu attached id="wb-snap-menu" label="Snap" icon={<ToolIcon name="snap" />} open={commandMenu === 'snap'} onOpenChange={(open) => setCommandMenu(open ? 'snap' : null)}>
              <label>Grid step<select aria-label="Snap increment" value={snapFraction} onChange={(event) => setSnapFraction(Number(event.target.value))}><option value={0}>Off</option><option value={0.125}>⅛u</option><option value={0.25}>¼u</option><option value={0.5}>½u</option><option value={1}>1u</option></select></label>
              <p>Uses the selected matrix pitch. Hold Alt to place freely.</p>
              <label className="wb-menu-check"><input type="checkbox" checked={geometrySnap} onChange={(event) => setGeometrySnap(event.target.checked)} /> Geometry snap</label>
              <label className="wb-menu-check"><input type="checkbox" checked={gapSnap} onChange={(event) => setGapSnap(event.target.checked)} disabled={!geometrySnap} /> Envelope gap</label>
              <label>Gap (mm)<input aria-label="Snap gap" type="number" min="0" step="0.1" placeholder={String(selectedMatrix?.edgeGap?.x ?? 1)} value={gapOverride} onChange={(event) => setGapOverride(event.target.value)} /></label>
              <p>Standalone parts snap at corners, midpoints and centers. Gaps use rectangular edges; keycap envelope when available, courtyard otherwise.</p>
            </CommandMenu>
          </> : <div className="wb-canvas-context"><ModeIcon mode={mode} /><strong>{viewLabel}</strong><span>{mode === 'Case' ? 'Assembly & components' : mode === 'Library' ? 'Footprint & model preview' : assembly3d ? 'PCB assembly' : 'Artifacts & readiness'}</span></div>}
          {transformTool && <button className="wb-command-trigger wb-transform-active" aria-label="Finish transform" title="Finish transform (Esc)" onClick={() => { setTransformTool(null); setOriginPicking(false); }}><ToolIcon name={transformTool} />{transformTool[0].toUpperCase() + transformTool.slice(1)} · Done</button>}
        </div>
          {mode === 'Design' && !outlineActive && <div className="wb-design-view-toggle" role="group" aria-label="Design view"><button aria-pressed={!assembly3d} onClick={() => setAssembly3d(false)}>2D</button><button aria-pressed={assembly3d} onClick={() => { cancelInteractions(); setCommandMenu(null); setTransformTool(null); cancelPlacement(); setAssembly3d(true); }}>3D assembly</button>{!assembly3d && <button aria-pressed={showFootprints} onClick={() => setShowFootprints(!showFootprints)}>Footprints</button>}</div>}
        {mode === 'Case' && <div className="wb-case-action-bar" ref={setCaseActionsTarget} />}
        <div className={`wb-canvas-stage wb-layer-surface ${leftOpen || rightOpen ? 'has-drawer' : ''} ${mode === 'Design' && !showFootprints ? 'is-layout-simplified' : ''}`}>
          <svg tabIndex={0} onKeyDown={(event) => { if (matrixGhost) {
            if (event.key === 'Enter') { event.preventDefault(); placeMatrixAt(matrixGhost.origin); }
            if (event.key.startsWith('Arrow')) { event.preventDefault(); const step = PITCH_MM * (snapFraction || 0.25); setPlacement((current) => current.kind === 'matrix' || current.kind === 'mirrored-pair' ? movePlacement(current, { x: current.matrix.origin.x + (event.key === 'ArrowRight' ? step : event.key === 'ArrowLeft' ? -step : 0), y: current.matrix.origin.y + (event.key === 'ArrowUp' ? step : event.key === 'ArrowDown' ? -step : 0) }) : current); }
            return;
          } if (!pendingPart) return; if (event.key === 'Enter') { event.preventDefault(); placePendingPart(placementPoint); } if (event.key.startsWith('Arrow')) { event.preventDefault(); const step = snapFraction ? PITCH_MM * snapFraction : nudgeStep; setPlacement((current) => current.kind === 'part' ? movePlacement(current, { x: current.point.x + (event.key === 'ArrowRight' ? step : event.key === 'ArrowLeft' ? -step : 0), y: current.point.y + (event.key === 'ArrowUp' ? step : event.key === 'ArrowDown' ? -step : 0) }) : current); } }} ref={svgRef} className={`wb-canvas ${matrixGhost ? 'is-placing-matrix' : ''}`} style={{ display: mode === 'Case' || mode === 'Library' || (mode === 'Design' && assembly3d) ? 'none' : undefined }} viewBox={`${viewBounds.minX} ${-viewBounds.maxY} ${viewBounds.width} ${viewBounds.height}`} role="application" aria-label="Board layout canvas. Use the CAD tree to select objects; use wheel to zoom and Space-drag to pan." onPointerDown={startCanvasPan} onPointerMove={moveCanvasPointer} onPointerUp={endCanvasPointer} onPointerCancel={endCanvasPointer} onLostPointerCapture={endCanvasPointer} onClickCapture={(event) => { if (matrixGhost) { event.stopPropagation(); drawOutline(event); } }} onClick={drawOutline} onDoubleClick={finishOutline}>
            <defs><pattern id="wb-grid-small" width={outlineActive || outlineFeature ? outlineGridSpacing : unit / 2} height={outlineActive || outlineFeature ? outlineGridSpacing : unit / 2} patternUnits="userSpaceOnUse"><circle cx="0" cy="0" r={outlineActive || outlineFeature ? viewBounds.width / Math.max(canvasSize.width, 1) * 0.7 : 0.12} fill="var(--wb-grid-large)" stroke="none" /></pattern></defs>
            <rect x={viewBounds.minX} y={-viewBounds.maxY} width={viewBounds.width} height={viewBounds.height} fill="url(#wb-grid-small)" />
            <g transform="scale(1,-1)" style={outlineActive || outlineFeature || pendingPart || originPicking ? { pointerEvents: 'none' } : undefined}>
              {!hiddenLayers.has(mode === 'PCB' ? 'Edge.Cuts' : 'Board') && visibleContours.map((contour, index) => <polygon key={`contour-${index}`} points={contour.points.map((point) => `${point.x},${point.y}`).join(' ')} className={`wb-outline-shape ${contour.hole ? 'is-hole' : ''}`} />)}
              {mode === 'Design' && matrixGhost && <g transform={`translate(${matrixGhost.origin.x} ${matrixGhost.origin.y})`}>
                {pairPreview && <><line className="wb-mirror-axis" x1="0" x2="0" y1="-16" y2={(matrixGhost.rows - 1) * matrixGhost.pitch.y + 16} /><MatrixGhost matrix={pairPreview.preview} projection={matrixGhostProjections.get(pairPreview.preview.id)} scope={null} ghost onSelect={() => undefined} onStagger={() => undefined} /></>}
                <MatrixGhost matrix={pairPreview?.matrix ?? matrixGhost} projection={matrixGhostProjections.get(matrixGhost.id)} scope={null} ghost onSelect={() => undefined} onStagger={() => undefined} />
              </g>}
              {mode === 'Design' && !hiddenLayers.has('Keys') && visibleMatrices.map((matrix) => <MatrixGhost key={matrix.id} matrix={matrix} projection={matrixScenes.get(matrix.id)} parts={parts} definitions={definitions} scope={scope} onSelect={(row, column) => selectScope({ kind: 'key', matrixId: matrix.id, row, column })} onStagger={(event, axis, index) => startStagger(event, matrix, axis, index)} />)}
              {visibleParts.filter((part) => !hiddenLayers.has(keyEnvelopes.has(part.id) ? 'Keys' : 'Components')).map((part) => {
                const definition = definitions.get(part.definitionId);
                return <ScenePart key={part.id} part={part} definition={definition} active={selectedIds.has(part.id)} constrained={constrainedTargetIds.has(part.id)} handlers={sceneHandlers} hiddenLayers={hiddenLayers} keycap={keyEnvelopes.get(part.id) ?? (definition?.kind === 'switch' ? part.keycap ?? definition.keycap : undefined)} pcb={mode === 'PCB'} footprints={mode === 'PCB' || showFootprints} />;
              })}
              {hoverOutline.length > 2 && <polygon className="wb-scope-outline is-hover" points={hoverOutline.map((point) => `${point.x},${point.y}`).join(' ')} />}
              {groupOutline.length > 2 && <polygon className="wb-scope-outline is-selected" data-scope={scope?.kind} points={groupOutline.map((point) => `${point.x},${point.y}`).join(' ')} />}
              {mode === 'Design' && !hiddenLayers.has('Keys') && (transformTool === 'splay' || transformTool === 'origin') && selectedMatrix && scope?.kind === 'column' && !originPicking && <SplayHandles handleScale={viewBounds.width / (svgRef.current?.getBoundingClientRect().width || 800) * 3} originOnly={transformTool === 'origin'} projection={matrixScenes.get(selectedMatrix.id)} matrix={selectedMatrix} column={scope.column ?? 0} parts={activeParts} onStart={startSplay}
                onOrigin={(point) => commitSplay(selectedMatrix, scope.column ?? 0, { kind: 'origin', world: point })}
                onAngle={(value) => commitSplay(selectedMatrix, scope.column ?? 0, { kind: 'angle', angle: value, affect: splayAffect })} />}
              {mode === 'Design' && !hiddenLayers.has('Keys') && transformTool === 'stagger' && selectedMatrix && scope && activePart && <g className="wb-transform-gizmo" transform={`translate(${activePart.pose.at.x} ${activePart.pose.at.y + 12})`} role="button" tabIndex={0} aria-label="Drag to stagger" onKeyDown={(event) => {
                if (!event.key.startsWith('Arrow')) return;
                event.preventDefault(); event.stopPropagation();
                const axis = scope.kind === 'row' ? 'row' : 'column';
                const index = axis === 'row' ? scope.row ?? 0 : scope.column ?? 0;
                const values = [...(axis === 'row' ? selectedMatrix.rowOffsets ?? [] : selectedMatrix.columnOffsets ?? [])];
                while (values.length <= index) values.push({ x: 0, y: 0 });
                const step = event.shiftKey ? 1 : .1;
                values[index] = { x: values[index].x + (event.key === 'ArrowRight' ? step : event.key === 'ArrowLeft' ? -step : 0), y: values[index].y + (event.key === 'ArrowUp' ? step : event.key === 'ArrowDown' ? -step : 0) };
                commitMatrix(axis === 'row' ? { ...selectedMatrix, rowOffsets: values } : { ...selectedMatrix, columnOffsets: values });
              }} onPointerDown={(event) => startStagger(event, selectedMatrix, scope.kind === 'row' ? 'row' : 'column', scope.kind === 'row' ? scope.row ?? 0 : scope.column ?? 0)}><circle r="4" /><path d="M0-3v6M-1.5-1.5 0-3l1.5 1.5M-1.5 1.5 0 3l1.5-1.5" /></g>}
              {snapGuide && <g className="wb-snap-guide"><circle cx={snapGuide.to.x} cy={snapGuide.to.y} r="1.3" /><path d={`M${snapGuide.from.x} ${snapGuide.from.y}L${snapGuide.to.x} ${snapGuide.to.y}`} /></g>}
              {pendingPart && <g className="wb-placement-preview" transform={`translate(${placementPoint.x} ${placementPoint.y})`}><polygon points={pendingPart.courtyard.map((point) => `${point.x},${point.y}`).join(' ')} /><path d="M-2 0h4M0-2v4" /></g>}
              {outlineActive && <OutlineDraftPreview points={outlineDraft} mode={outlineOperation} grid={outlineGrid} scale={viewBounds.width/(svgRef.current?.getBoundingClientRect().width||800)} />}
            </g>
            {outlineFeature && <OutlineControlOverlay key={`${document.id}/${projectSession}/${selectedBoardId}/${outlineSelection?.featureId}/${outlineSelection?.connectionId??''}`} feature={outlineFeature} connectionId={outlineSelection?.connectionId} selectedPoint={outlinePoint} onSelectPoint={setOutlinePoint} parts={document.parts} grid={outlineGrid} onDragChange={active=>setOutlineDragBounds(active?bounds:null)} scale={viewBounds.width/(svgRef.current?.getBoundingClientRect().width||800)} onChange={(feature,phase,transactionId)=>{
              if(outlineContext.current.documentId===document.id && outlineContext.current.projectSession===projectSession) emit({kind:'set-outline',feature},[feature.id],phase,transactionId);
            }} />}
          </svg>
          {(mode === 'Design' || mode === 'PCB') && !assembly3d && <WorkbenchLayers layers={mode === 'PCB' ? [...pcbLayers, 'Edge.Cuts', 'Courtyards', 'Pads', 'Holes', 'References'] : ['Keys', 'Components', 'Keycaps', 'Footprints', 'Board']} hidden={mode === 'Design' ? new Set([...hiddenLayers, ...(!showFootprints ? ['Footprints'] : [])]) : hiddenLayers} onToggle={(layer) => layer === 'Footprints' ? setShowFootprints(!showFootprints) : toggleLayer(layer)} />}
          {existingHalfOpen && <ExistingHalfSetup matrices={unpairedMatrices} axis={Math.max(0, ...selectionOutline(visibleParts, definitions).map((point) => point.x)) + 12} onCancel={() => setExistingHalfOpen(false)} onCreate={(matrices, axis) => { try { const next = mirrorExistingHalf(document, matrices, axis, makeId); emit({ kind: 'replace-document', document: next }, matrices.map((matrix) => matrix.id)); setExistingHalfOpen(false); setZoom(1); setPan({ x: 0, y: 0 }); return undefined; } catch (error) { return String(error instanceof Error ? error.message : error); } }} />}
          {pairSetup && mode === 'Design'  && <MirroredPairSetup presets={(Object.keys(matrixPresetDefinitions) as MatrixPresetId[]).map((id) => ({ id, name: assemblyName(id) }))} onPreview={(setup) => beginMatrixPlacement(setup.rows, setup.columns, setup.preset as MatrixPresetId, setup)} onCancel={() => { cancelPlacement(); (compactObjects && !leftOpen ? window.document.getElementById('wb-objects-toggle') : addPartRef.current)?.focus(); }} />}
          {mode === 'Library' && !editingAssembly && <LibraryWorkspace document={document} definition={previewDefinition} title={libraryAssembly ? assemblyName(libraryAssembly) : previewDefinition ? partCatalogLabel(previewDefinition) : undefined} companions={libraryCompanions} rotation={libraryRecipeEntries[0]?.rotation ?? 0} compiled={libraryCompiled} compilePending={previewCompilePending} compileError={previewCompileError} mechanicalProfile={(previewDefinition as (PartDefinition & { mechanicalProfile?: MechanicalPartProfile }) | undefined)?.mechanicalProfile} onSaveMechanicalProfile={(profile) => { if (!previewDefinition) return; const snapshot = { ...previewDefinition, mechanicalProfile: profile }; const nextDefinitions = document.definitions.some((entry) => entry.id === previewDefinition.id) ? document.definitions.map((entry) => entry.id === previewDefinition.id ? snapshot : entry) : [...document.definitions, snapshot]; emit({ kind: 'replace-document', document: { ...document, definitions: nextDefinitions } }, [previewDefinition.id]); }} onMechanicalProfile={onMechanicalProfile} onExtractMechanicalProfile={onExtractMechanicalProfile} show3d={library3dOpen} onViewChange={setLibrary3dOpen} colorScheme={colorScheme} />}
          {mode === 'Library' && editingAssembly && <React.Suspense fallback={<p>Loading assembly editor…</p>}><AssemblyEditor key={editingAssembly.id} document={document} initial={editingAssembly} matrixName={selectedMatrix ? selectedMatrix.name?.trim() || `Matrix ${document.matrices.indexOf(selectedMatrix) + 1}` : 'selected matrix'} onApply={selectedMatrix ? assembly => { const prepared = matrixWithAssembly(selectedMatrix, assembly, libraryDefinitions, document.revision); emit({ kind: 'set-matrix', ...prepared }, [selectedMatrix.id]); } : undefined} definitions={libraryDefinitions} boardId={selectedBoard?.id} colorScheme={colorScheme} onChange={next => emit({ kind: 'replace-document', document: next }, [editingAssembly.id])} onClose={() => setEditingAssembly(null)} onPlace={next => { emit({ kind: 'replace-document', document: next }, []); setEditingAssembly(null); changeMode('Design'); setAssembly3d(true); }} /></React.Suspense>}
          {(mode === 'Case' || (mode === 'Design' && assembly3d)) && selectedBoard && <React.Suspense fallback={<p role="status">Loading assembly viewer…</p>}><AssemblyViewer document={assemblyDocument} boardId={selectedBoard.id} contours={assemblyContours} bodies={showCaseGeometry ? caseBodies?.map(body => ({ id: body.id, name: body.name, mesh: body })) : undefined} preparedCase={showCaseGeometry ? preparedCase : undefined} generation={showCaseGeometry ? generation : undefined} onCasePreviewDraft={onCasePreviewDraft} onCaseMountChange={(bodyId, mounts) => { const body = document.caseBodies.find(entry => entry.id === bodyId); if (body) return emit({ kind: 'set-case', body: { ...body, mounts } }, [bodyId]); }} onGasketChange={configuration => emit({ kind: 'set-mechanical', configuration }, [document.id])} mechanical={showCaseGeometry && generatedCase ? mechanicalAssembly : undefined} selectedLayer={selectedMechanicalLayer} onSelectLayer={setSelectedMechanicalLayer} colorScheme={colorScheme} onSelect={reference => { const part = document.parts.find(p => p.reference === reference); if (part) choosePart(part.id); }} /></React.Suspense>}
          {mode !== 'Case' && mode !== 'Library' && !assembly3d && !matrixGhost && !pendingPart && visibleParts.length === 0 && visibleContours.length === 0 && visibleMatrices.length === 0 && <div className="wb-canvas-empty"><div className="wb-empty-cursor"><CursorIcon /></div><h2>Build your keyboard layout</h2><p>Start with a key matrix. Its switches and companions create the board outline as you edit.</p><button className="wb-primary" onClick={() => { dismissGuide(); changeMode('Design'); createGuidedMatrix(); }}>Add key matrix <ArrowIcon /></button><button className="wb-empty-secondary" onClick={() => changeMode('Library')}>Browse individual parts</button></div>}
          {snapGuide && <div className="wb-canvas-hint" role="status">{snapGuide.label}</div>}
          {originPicking && <div className="wb-canvas-hint" role="status">Pick splay origin · Click the canvas · Esc cancels</div>}
          {matrixGhost && pairPlacement && <div className="wb-canvas-hint" role="status">Place linked halves · Click to place · Esc cancels</div>}
          {pendingPart && <div className="wb-canvas-hint" role="status">Place {pendingPart.name}{pendingLayoutId ? ` in ${boardLayouts.find((layout) => layout.id === pendingLayoutId)?.name}` : ''} · Click or Enter to place · Esc cancels</div>}
          {outlineActive && <OutlineDrawingBar mode={outlineOperation} count={outlineDraft.length} grid={outlineGrid} onGrid={setOutlineGrid} onUndo={() => setOutlineDraft(points => points.slice(0, -1))} onCancel={cancelOutlineDrawing} onFinish={finishOutline} />}
        </div>

      </section>}

      {mode !== 'Export' && <WorkspacePanel side="right" label={`${mode === 'Library' ? 'Parts' : mode} inspector`} settings={inspectorPanel} compact={compactInspector} open={rightOpen} onClose={() => setRightOpen(false)}>
        {inspectorPage && <div className="wb-inspector-return"><button className="wb-back-link" onClick={closeInspectorPage}><ArrowIcon />Back to selection</button><span>{inspectorPage}</span></div>}
        <div className="wb-inspector-content" onKeyDown={event => { if (event.key === 'Escape' && inspectorPage) { event.stopPropagation(); closeInspectorPage(); } }} key={`${mode}:${scriptsOpen}:${mode === 'Library' ? libraryAssembly ?? selectedLibraryDefinition?.id : `${selectedBoardId}:${scope?.matrixId}:${scope?.kind}:${scope?.row}:${scope?.column}:${activePart?.id}`}`}>
          {designView && !inspectorPage && <>{mode === 'Design' && selectedMatrix && scope?.kind !== 'component' && <><div className="wb-inspect-head wb-selection-heading"><h2 aria-label={selectionTitle}>{selectionTitle.split(' · ').at(-1)}</h2></div><p className="wb-selection-summary">{scope?.kind === 'matrix' ? `${selectedMatrix.rows} rows · ${selectedMatrix.columns} columns` : `${activeParts.length} keys selected`}</p></>}<div className="wb-selection-breadcrumb">{selectedBoard?.name} / {viewLabel}{selectedMatrix ? ` / ${activeLayout?.name || selectedMatrix.name || selectionTitle.split(' · ')[0]}` : ''}</div>{mode === 'Design' && Boolean(scope || activePart) && <div className="wb-inspector-tabs" role="tablist" aria-label="Inspector details"><button role="tab" aria-selected={inspectorTab === 'properties'} onClick={() => setInspectorTab('properties')}>Properties</button><button role="tab" aria-selected={inspectorTab === 'relations'} onClick={() => setInspectorTab('relations')}>Relations</button></div>}</>}
          {showFindings ? <><div className="wb-inspect-head"><h2 ref={findingsHeading} tabIndex={-1}>Layout findings</h2></div><FindingList document={document} onShow={showFinding} findings={scene.findings} /></> : inspectorTab === 'relations' && designView && !scriptsOpen && !outlineSettingsOpen ? <><h2>Relationships</h2><p className="wb-empty-note">{partnerLayout ? `Key assemblies, diodes and components mirror with ${partnerLayout.name}. Replace a component on one half to keep it local.` : activeLayout ? 'This layout is independent. Its geometry and components can be edited separately.' : activeConstraint ? `${parts.get(activeConstraint.sourcePartId)?.reference} drives ${activePart?.reference}.` : 'No saved placement relationship on this selection.'}</p>{activePart && <button className="wb-secondary" onClick={() => { changeMode('Design'); setScope({ kind: 'component', partId: activePart.id }); setInspectorTab('properties'); }}>Edit placement relationship</button>}<p className="wb-empty-note">Matrix rows and columns share pitch, stagger and splay. Edit those in Properties.</p></> : mode === 'Case' ? null : getModeDetails()}
          {mode === 'Case' && <div hidden={showFindings || inspectorTab === 'relations'}>{getModeDetails()}</div>}
          {(mode === 'Design' || mode === 'Case') && !inspectorPage && selectedBoard && <React.Suspense fallback={null}><BoardReferencePanel document={document} boardId={selectedBoard.id} onChange={next => emit({ kind: 'replace-document', document: next }, [selectedBoard.id])} /></React.Suspense>}
        </div>
      </WorkspacePanel>}
      {((compactObjects && leftOpen && (mode !== 'Export' || guideVisible)) || (compactInspector && rightOpen && mode !== 'Export')) && <button className="wb-drawer-scrim" aria-label="Close panels" onClick={() => { setLeftOpen(false); setRightOpen(false); }} />}
    </section>
    {mode !== 'Export' && <footer className="wb-canvas-footer">
      <div className="wb-edit-history"><button className="wb-history-action" aria-label="Undo" title="Undo (Ctrl+Z)" onClick={onUndo}><UndoIcon /></button><button className="wb-history-action" aria-label="Redo" title="Redo (Ctrl+Shift+Z)" onClick={onRedo}><RedoIcon /></button></div>
      {mode === 'Case' || (mode === 'Design' && assembly3d) || (mode === 'Library' && editingAssembly) ? <>
        <span className="wb-footer-view">3D · mm</span>
        <div className="wb-footer-center"><span>Drag to orbit · Scroll to zoom</span></div>
      </> : <>
      <div className="wb-footer-coords"><span>mm</span><span>X <b>{activePart?.pose.at.x.toFixed(2) ?? '0.00'}</b></span><span>Y <b>{activePart?.pose.at.y.toFixed(2) ?? '0.00'}</b></span></div>
      <div className="wb-footer-center">{outlineActive || outlineFeature ? <span>Outline snap {outlineGrid} mm</span> : <><button onClick={() => setCommandMenu('snap')} disabled={mode !== 'Design' && mode !== 'PCB'}>Grid {snapFraction === 0 ? 'off' : snapFraction === 1 ? '1u' : snapFraction === .5 ? '½u' : snapFraction === .125 ? '⅛u' : '¼u'}</button><span>{geometrySnap ? 'Geometry snap on' : 'Geometry snap off'}</span></>}</div>
      <div className="wb-footer-zoom"><button className="wb-view-button" aria-label="Fit board" title="Fit entire board" onClick={() => fitParts(visibleParts, visibleContours, visibleMatrices)}><FitIcon />Fit board</button><button className="wb-view-button" disabled={!activeParts.length} onClick={() => fitParts(activeParts)}>Fit selection</button><button aria-label="Zoom out" onClick={() => zoomAt(window.innerWidth / 2, window.innerHeight / 2, Math.max(.25, zoom / 1.2))}>−</button><span>{Math.round(zoom * 100)}%</span><button aria-label="Zoom in" onClick={() => zoomAt(window.innerWidth / 2, window.innerHeight / 2, Math.min(4, zoom * 1.2))}>+</button></div>
      </>}
      <div className="wb-findings-scopes">
        <button className="wb-findings-status" onClick={() => { setShowFindings(!showFindings); setRightOpen(true); setLeftOpen(false); if (!compactInspector) inspectorPanel.setMode('pinned'); }}><ToolIcon name="warning" />Layout findings: {presentedFindings(scene.findings, document).length}</button>
        {generatedCase && <button className="wb-findings-status" onClick={reviewMechanicalFindings}>Mechanical findings: {visibleMechanicalFindings.length}</button>}
      </div>
    </footer>}
  </main>;
};

export { Workbench };
