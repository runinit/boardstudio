# Capability inventory and destinations @ `cafaf430`

Parent: [map.md](map.md). Source base is local `main`
`cafaf4302635909b59666164ed190fa2137aabc5`, not the recovery branch. Inspected through native
Gortex in the planning worktree. This is a capability/owner inventory, not passing behaviour
or export evidence. **Retain** = same owner/path; **move** = same capability/new host;
**merge** = remove a duplicate entry, not its action; **retire** = user-approved shell
mechanism only. Proposed destinations await 001; no feature removals are implied.

## Shared shell and global surfaces

| Current capability / owner | Destination and disposition | Ticket |
|---|---|---|
| Six typed `WorkspaceObjectsInput`, `WorkspaceToolbarInput`, `WorkspaceInspectorInput` variants; `workspace_composition.rs::{objects,toolbar,inspector}` | Retain domain routing; shared host consumes domain content, not a universal object model | 009 |
| `ui-shared/src/panels.rs`: DockLayout, ObjectsPanel, InspectorPanel, CompactPanelState; desktop Sidenav / <980px Sheet, tabs, keep-mounted/inert and focus return | Move content to shared Objects/Inspect; retain lifetime/focus guarantees, retire old hosts only after six-adapter parity | 009, 008 |
| `panels/policy.rs`: PanelSettings/PanelMode, `boardstudio:v2:panel:*`, persisted Objects 200–420px / Inspector 280–480px; legacy autohide→pinned | Retire width/mode influence; stored obsolete values may remain; unrelated Theme/preferences untouched | 008 |
| Pin/collapse to 44px rail, pointer/arrow/Shift/Home/End dock resizing | Retire with explicit user approval; neither rail nor pinned alternative survives | 008 |
| Show/hide Objects/Inspector in palette/header Workbench menu (`presentation.rs`, `command_palette.rs`) | Move to shared utility requests, retain command reachability and disabled reasons | 009, 006 |
| Thirteen `pin_inspector_on_desktop` callers plus explicit/non-helper reveals | Central per-site disposition table in architecture.md includes Export Review wiring/case and InspectorActions; ordinary pick never opens, explicit requests use the target workspace's sole current authority. Preserve selection updates | 002–006, 010, 012 |
| Header identity/Project actions, six workspaces, Search Ctrl/Cmd+K, save control/popover, Undo/Redo, Settings Theme/Appearance, labelled Export, update notice; compact Workbench menu | Move visual hierarchy globally, retain actions, route/current labels and truthful saving/error/recovery state; project browser/Export remain separate surfaces | 006 |
| Save failure Retry/Download pending, recovery Reload currently carried in Layout toolbar input | Move to existing shell save/status alert area outside the 42px strip; never hide a blocking failure or manufacture Saved | 006 |
| Ctrl/Cmd+K and canvas-first Ctrl/Cmd+Z/Y (`workbench_shortcuts.rs`) | Retain; new toolbar/panel handlers do not steal editing/gesture keys | 009, 007 |
| Setup guide Project/Layout/Wiring/Case stages, dedicated viewer-side process surface | Retain dedicated process surface and existing control/preview owners; close compact panels on guide entry, do not create a second setup wizard | 005 |
| Theme/Appearance and semantic geometry/shell roles (`web/tailwind.css`, base/header/shell/toolbar styles) | Retain both Themes × light/dark; measure new rendered pairs, do not copy prototype token claims | 007 |

## Layout tools and canvas

Owners: `web/src/presentation/layout_workspace.rs`, `layout_canvas.rs`;
`web/crates/layout/src/objects/{layout_toolbar,layout_transform_toolbar,layout_align}.rs`.

| Capability | Proposed destination / disposition | Ticket |
|---|---|---|
| LayoutCommandPill + LayoutCommandMenu Select/Transform/Align/Snap, dismissal/focus helpers | Move to single 42px in-row accordion; shared utility host owns Objects/Inspect | 002, 009 |
| Matrix/Row/Column/Key/Part scopes, retained-cell conversion and inline empty guidance | Select section; retain `context_for_selection_kind` and `selection_kind_for_tree_context`, no scope popup | 002 |
| Stagger, Spread, Offset, Shift, Bend, Splay, Origin, Move, Rotate and Anchor activation | Transform section, existing `ToolActivation`/mounts; first-column Spread, Follow-rotation Rotate, locked/empty and local-exception rules unchanged | 002 |
| Six Align actions Left/Center X/Right/Top/Center Y/Bottom; reference Select/Combobox >12, disabled reasons/toasts, Relationships | Align section; existing reference detail overlay and explicit Relationships → Inspect | 002, 003 |
| Snap Off, 1/8u, 1/4u, 1/2u, 1u, 1mm, 0.5mm, 0.1mm; Geometry snap, Envelope gap, override; `snap_in_dock` split | Merge into Snap section + bounded detail surface; remove duplicate dock controls only after parity | 002, 008 |
| Keys/Components/Keycaps/Footprints/Board visibility, Canvas handles; duplicate bottom-left Footprints toggle; 2D/3D assembly toggle | Move into View, merge duplicate Footprints entry onto the same state; no second permanent row. Fit/zoom camera controls remain footer | 002 |
| Matrix/part drag rails, anchor-pick overlay, gesture feedback/toasts; boundary picking, split focus and Placed from readout/rings | Retain canvas owners; panel opens never remount/reframe. Half selection aligns view-only focus, not link source | 003, 004 |

## Layout Objects and Inspector

| Capability / current owner | Proposed destination / disposition | Ticket |
|---|---|---|
| `objects.rs`, `ui-model/tree.rs`: board→halves/layouts→matrices→rows/columns/keys; components/categories; outlines/versions/bridges, other-board navigation; grouping By layout/By part | Move navigation projection to compact searchable Objects; active Outline first; preserve grouping, named half/board navigation, category/disclosure reachability. Version lifecycle lives in Inspect, not an inactive-version geometry editor | 004 |
| ScopedTreeContext, TreeSelectRequest Replace/Add/Range/Toggle, TreeNudgeRequest; disclosure seeds/created-pair reveal | Retain selection admission/modifier intentions; row pick closes Objects without opening Inspect; disclosure survives reopening; arrow nudge available on focused canvas selection after pick | 004 |
| Physical-instance selector code in generic Objects is Case-gated; actual Case has its own tree | Do not invent a Layout physical-instance selector. Preserve actual Layout board navigation and Case assembly/instance navigation separately | 004, 013 |
| Component references derived from same key anchor as canvas, including companion/detached/local/mirrored parts | Objects association label, e.g. “Placed from Left main · R1/C6”; explicit unanchored/companion description where needed, not invented association or side/clearance validation | 004 |
| `LayoutComponentInspector`, LayoutWorkspaceState drafts, layout assignment, position/margin, constraints/anchors, local/mirrored edits, removal/pending/failures | Move unchanged editors into Inspect; no duplicate Position editor; owner-keyed draft/reset audit | 003 |
| CompanionPlacementInspector, MatrixInspector/presets/assembly/orientation, KeySizeMount, MatrixTransformInspector, origin/anchor picks/delete, BoardInspector | Move to Inspect with existing settlement, resolver and undo ownership | 003 |
| `outline/hook.rs::project_inspector`, OutlineVersionInspector / OutlineAction | Inspect Generated/linked refinement/fixed edited lifecycle, versions, bridges and explicit activation. Objects/canvas selection selects active boundary without activating | 003, 004 |
| MatrixRelationsPanel, reference hover, MatrixRelationshipEscape | Explicit Relationships opens Inspect; retain highlight/return-to-board escape behaviour | 003 |
| LayoutFindingsInspector, Back to selection, target navigation | Inspect findings; retain explicit navigation and unavailable-target text | 003 |
| Empty/context titles, precise drafts/errors/disclosures; PendingEditSignals | Retain; hide/inert is not sufficient proof against owner-reset effects | 003 |
| AddObjectEntry, parts chooser, setup/preview mounts and GeometryScriptsEditor | Move entry points/overflow per table below; no old dock-only workflow disappears | 005 |

## PCB adapter

Owners: `web/src/presentation/pcb_workspace.rs`; shared Layout Objects/tools;
`web/crates/pcb/src/{pcb_wiring.rs,pcb_wiring/}` and existing component/module inspectors.

| Capability | Proposed destination / disposition | Ticket |
|---|---|---|
| Shared Objects tree, board/part navigation, matrix setup, parts query/placement/error/Browse all parts | PCB Objects projection + shared Add task surface, same scope and placement owners | 010, 005 |
| LayoutCommandPill with Select/Transform/Align/Snap mounts; SVG PcbScene/module picking | PCB row uses only existing applicable mounts; preserve SVG pick/gesture ownership | 010 |
| PcbInspector Selection / Routing & nets, both kept mounted; selected component numeric position and module content | Both sections in Inspect, retain owner state while switching sections/panels; no new duplicate position controls | 010 |
| Wiring controller selection/carousel, matrix/direct mode/automatic resolution, named terminal assignments, pin locks/manual nets, conflicts/replacement and explicit remap review | Existing Routing & nets content/review surfaces reached from Inspect, with every readiness/pending/failure/admission guard intact | 010 |
| PCB Layers/copper/technical visibility, footprint detail and diagnostics | Retain workspace canvas/view owners; UI-only view changes do not alter routing or output data | 010 |

## Keymap adapter

Owners: `presentation/keymap_workspace.rs`; `keymap/src/keymap/panel.rs`, layer/binding/
macro/encoder controllers; `case/src/shared_viewer.rs::DesignViewToolbar`.

| Capability | Proposed destination / disposition | Ticket |
|---|---|---|
| KeymapNavigation: active layers, key search/reference+binding labels and selection | Compact Objects navigator with Layers/Keys; preserve search, layer selection and no-match state | 011 |
| Add layer, name draft/rename-on-blur, remove non-base; base/layer-limit/pending/save guards and feedback | Existing Layer operations editor in Inspect, explicit route from layer context; no inline editing stuffed into Objects | 011 |
| Keys / Macros / Encoders sections, BindingEditor, MacroEditor, EncoderEditor; export action | Inspect retains all editors/export and existing conditional-content semantics after draft audit; no new keymap model | 011 |
| Scope/layer/key/editor-instance keys, encoder identity generations and asynchronous admissions | Retain domain ownership; draft restoration cannot authorize stale/recreated input edits | 011 |
| DesignViewToolbar contextual tools, 2D/3D and Footprints; canvas key pick | Shared row with only applicable contextual/View controls, same viewer identity; pick never auto-opens Inspect | 011 |

## Keycaps adapter

Owners: `presentation/keycaps_workspace.rs`; existing Keycaps*SettingsEditor and fit owners;
`DesignViewToolbar` from `web/crates/case/src/shared_viewer.rs`.

| Capability | Proposed destination / disposition | Ticket |
|---|---|---|
| Shared Objects tree/key/matrix navigation; KeycapsBoardSettingsEditor currently below it | Navigation in Objects; **propose moving existing board defaults to Inspect**, without duplicating or dropping it | 012 |
| Matrix profiles disclosure, KeycapsMatrixSettingsEditor, selected-key KeyList + KeycapsSettingsEditor | Inspect, same board→matrix→key inheritance/override/reset owners | 012 |
| Fit inspector/focused finding, retry, explicit target navigation and pending-layout-fit lifetime | Inspect findings and existing owner-checked navigation; opening panel alone never fits | 012 |
| Contextual tools, 2D/3D/Footprints, canvas key selection | Row's applicable contextual/View controls, retain viewer/camera and no auto-open on pick | 012 |

## Case adapter

Owners: `presentation/case_workspace.rs::{objects,inspector,toolbar}`, `CaseSelection`,
`TreeAction`/DisplayRequest; `case/src/cad_presentation.rs::CasePanel` and existing viewers.

| Capability | Proposed destination / disposition | Ticket |
|---|---|---|
| Assembly roots/physical instances, generated stack/layers, gasket groups+fit details, legacy case bodies, PCB/components; disclosure and previous-geometry labels | Compact Objects with actual assembly/instance hierarchy and disclosure, not Layout TreeContext coercion; preserve navigation and diagnostic labels | 013 |
| Scoped visibility per root/body/layer/gasket/PCB and display state | Retain same CaseSelection/DisplayRequest owners, compact Objects controls and Inspect display details | 013 |
| Physical assembly, transport, PCB-design and flip controls; MechanicalSettings; display color/reset/visibility; CaseBodyInspector | Inspect, retained linked/shared construction and local battery/instance rules; hidden body editor drafts survive | 013 |
| Shared viewer Assembled / Exploded / Section, Shaded / Wireframe / Hybrid and Hidden lines controls | Propose Case View section/detail with the same transient viewer owner, no second permanent wrapping rail; camera remains footer. Gate any new interface with the M2 Case decision | 013 |
| CasePanel Live preview, Update preview, Cancel and gated Export geometry; command-palette entries; status/error/previous/current exact geometry, model-delivery diagnostics | Separate gated Preview entry/detail retains 44px task targets, exactly one generation/command owner and all currency checks; no footer exception or interface widening presumed. Status/model diagnostics and camera remain canvas/footer | 015 |
| CasePreviewViewer/CaseViewer and exact same-owner previous completed CadScene | Retain; panel switching never starts generation, reuses foreign geometry or authorizes stale export | 013 |

## Parts adapter

Owners: `presentation/parts_workspace.rs`, `parts/src/parts.rs::{PartsLibraryPanel,
PartsInspectorPanel,PartsPreviewWorkspace}`; PartsWorkspaceState selection/draft owners.

| Capability | Proposed destination / disposition | Ticket |
|---|---|---|
| Search by name/category, categorized definitions, key-assembly presets, switch orientation, loading/error/empty, selection preview | Compact catalog Objects navigator, retain query/selection/orientation and preview activation; actual definition/module IDs, not board tree contexts | 014 |
| Create key assembly, Import KiCad, new custom component | Larger catalog task surface through Project/Add object or explicit catalog task commands; retain native file picker/import/edit owners | 014 |
| SavedAssemblyEditor, preset placement, selected-definition metadata/custom fields, GeneratorSettingsEditor, ComponentModelEditor | Inspect detail/authoring entry points, larger existing editors where required; retained generator draft, pending/error and preview identity | 014 |
| Module inspector/variants, attachment and module placement; component placement, apply to selected key, controller Place in layout / Place | Existing Inspect/task surfaces with selection-generation and owner-admission guards; ordinary browse never places or auto-opens Inspect | 014 |
| 2D footprint/3D model preview, Layers/legend visibility and failure/retry/import; Back to PCB controller-browse route | Retain preview/view owners and explicit return route/context, without a second permanent toolbar | 014 |

## Overflow routing proposal

The current execution instruction adopts the recommended Layout M1 entries below; the
original “Y” alone did not. Later domain-specific destinations still need their M2 gate.
All entries keep their current controllers, previews, errors, commit/Undo and pending guards.
A larger task surface is bounded and dismissible, not a dock or a new permanent action row.

| Current workflow | Entry and proposed surface | Owner / ticket |
|---|---|---|
| Add object dock-header trigger `m1-add-object-trigger` | Project menu → Add object… and Command palette → larger contextual chooser; unavailable actions name reasons. No creation form inside compact Objects | Existing AddObjectEntry / 005; header plumbing 006 |
| Board/project setup, topology/mirrored-pair creation | Existing setup-guide/process overlay and canvas preview; explicit entry through Project/setup actions | Existing setup controllers; not every BoardSetupMount parameter currently renders in Objects / 005 |
| Matrix…, Mirror existing half… | Add chooser → existing MatrixSetupMount / ExistingHalfSetup overlay + real canvas preview; created-pair disclosure/selection retained | Existing setup/placement / 005 |
| Contextual Add row / Add column | Same chooser context → existing one-shot action, not a new wizard | Existing matrix action / 005 |
| Parts search/choose/place, placement error | Add chooser → bounded larger component chooser; failure stays with flow; Browse all parts → Parts carrying query/return context | AddObjectComponentChooser / 005, 010, 014 |
| Board outline… | Objects → first Outline item, or retained command → active boundary selection; explicit lifecycle request → Inspect | Existing Outline contexts/actions / 003, 004 |
| Geometry scripts… currently replacing Inspector | Explicit legacy action → dedicated larger editor surface, not compact Objects or small Inspect | GeometryScriptsEditor / 005. Deprecated ADR-0029 remains in force: no new scripting support or reduced safety |
| Keymap layer operations / Keycaps board defaults | Inspect's existing editor content; compact Objects only navigates | 011, 012 |
| Case construction / Parts import, custom/model/assembly authoring | Inspect entry to the current editor or bounded larger task surface when needed; preserve existing confirmations | Domain owners / 013, 014 |

## Completeness boundary

Every row has a destination and an owner; none proposes domain capability removal. Concrete
M2 routing/Keycaps-default/Case decisions remain human gates; M1 defaults and phase
limits are recorded in delivery.md. Before an
adapter closes, its completion record must tick every row and name any additional live
entry found while removing old fields. Use compiler errors and Gortex impact for changed
field/type inventories. Unaccounted live workflows block dock deletion, not permission to
silently de-scope them. This plan does not certify CAD/export/fabrication/accessibility.
