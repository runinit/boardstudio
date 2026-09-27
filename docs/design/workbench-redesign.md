# Board Studio v2 — direct editing workbench proposal

Status: Partially implemented. The unified workbench shell and the
documented interactions below are implemented in app; remaining ideas are
marked planned and remain design guidance.

## Implementation coverage

Implemented locally: linked Design branches; right-docked and resizable inspector;
labeled command menus; context-sensitive Add row/column; grid fractions and off;
standalone part landmark and rectangular envelope-gap snapping; align-once axis
bounds against an explicit reference; column splay with editable/pickable/draggable
origin, this-column or following-column effects, preview cancellation, undo/redo,
and project persistence. Existing offset and mirror constraints remain available.

Still proposed: arbitrary rotated edge constraints, persistent corner/edge/center
relationships, rich hover insert/remove affordances, added overlap/clearance
feedback, PCB nets/layers in the tree, and Case feature editing. Gap snapping is
limited to axis-aligned rectangular envelopes; courtyard fallback is labeled in
Snap. Alignment moves once and does not establish a persistent relationship.

The interaction blueprints below retain the broader design direction. This
coverage section defines what is implemented; unlisted blueprint capabilities
remain planned. Passing previews/tests does not establish fabrication readiness.

## Intent

Keep the Keyboard Lab's precise editing character, refresh its theme, and make
editing feel like one coherent tool.
The canvas should support everyday placement and shaping directly. The inspector
provides precise values, explanations, and less common options for the same edit.
Experienced keyboard designers should be able to see what an action affects,
preview its result, and undo it as one operation.

Scope: the main Design page, its menus, toolbars, selection states, direct
manipulation, inspector, snapping and relationships; shared rules for Parts,
footprint viewing/editing, PCB, Case, and Export. This is a desktop-first web
proposal with keyboard and narrow-screen alternatives.

## Selected direction for refinement

The implemented local direction replaces the bottom worktray with a **right-docked
inspector**, reduces top controls, brings Layout/PCB/Case into a unified Design
tree, and applies the approved light/dark token refresh. Parts remains a
separate page. This supersedes the previous bottom-dock hybrid.

The user confirmed that selecting a branch switches the main canvas view and its
tools while shared objects remain linked. It does not place all editing modes
on one combined canvas.

The local light and dark visual studies informed the implemented structure,
including the persisted System theme option. Those exploratory images are not
repository assets; see DESIGN.md for the current interface rules.

- **Global header:** Design and Parts on the left; Export, undo/redo, and the
  project menu on the right.
  PCB and Case leave the global navigation. Appearance remains in project settings.
- **Left navigator:** Main board contains Layout, PCB, and Case branches. Layout
  contains matrices and keys. PCB currently exposes board and parts; nets and
  layers are planned. Case currently exposes bodies; feature editing is planned.
  The sidebar is for navigation and selection,
  with no second inspector stacked beneath it.
- **Compact canvas toolbar:** four groups, **Select · Transform · Align · Snap**.
  Add opens a create view in Objects. Select names the current scope. Each group has a labeled menu; avoid
  rendering every subcommand as a separate permanent button. Fit and zoom join
  the view/status controls. Relevant direct-manipulation handles remain visible.
- **Right inspector:** a resizable dock uses the available screen height. Its
  selection title and breadcrumb lead into Properties/Relations and only the
  relevant expanded section. Splay & origin includes the angle, origin source,
  canvas picker, coordinates and affected set. Position, spacing and specialist
  controls can stay collapsed until needed.
- **Canvas:** gains vertical space by removing the bottom tray and long ribbon.
  A sparse grid and quieter unselected geometry give selected objects and their
  handles priority. Layout defaults to key envelopes; footprint detail can be
  shown deliberately. PCB/footprint views retain their richer physical detail.
- **Status:** a slim line for coordinates, units, snap summary, zoom and labeled
  findings. Blocking feedback stays next to the action it prevents. No permanent
  warning wall occupies the navigator.

On narrower desktop screens, the left navigator collapses at 980px and the
right inspector at 820px; phone access uses explicit drawers/sheets. The
inspector can be resized from 300–480px on desktop. Wide screens use the 60px /
64px / 60px header, toolbar, and footer rhythm from 1440px upward.

### Compact commands without losing direct editing

| Group | Contents and contextual behavior |
| --- | --- |
| Add in Objects | Matrix/part when nothing is selected; relevant key/row/column insertion for the current selection |
| Select: current scope | Named scopes appropriate to the active branch; current scope remains readable with the menu closed |
| Transform | Move, rotate, stagger, splay and origin when applicable; active tool opens the corresponding inspector section |
| Align | Pick source/reference geometry, align once, distribute, or retain a relationship |
| Snap | Independent grid step, geometry targets and gap settings, with a compact persistent status summary |

Duplicate and Remove belong to the selected object's action area in the right
inspector, not a permanent global toolbar. Direct insertion markers and transform
handles remain available on the selected canvas geometry. A user should not have
to reopen a menu for each drag. Prefer plain labeled menu triggers and separators
to a run of equally emphasized boxed buttons. Show one Add entry point in the
Objects panel.
Report the same warning count once in the status line; opening it reveals details.

### Linked views within Design

| Selected branch | Main canvas | Contextual tools and details |
| --- | --- | --- |
| Layout | Top-down key envelopes and board outline | Matrix/key/part scopes, stagger, splay, origin, alignment, snap |
| PCB | Board and selected parts | Placement and the implemented shared selection shell; nets and layers remain planned |
| Case | Selected case bodies | Body selection and the implemented shared selection shell; feature editing remains planned |

Activating a branch establishes the editing context; its disclosure arrow only
expands or collapses children. A visibility icon controls references independently
of the active context. Show the active branch and canvas breadcrumb so a user
cannot accidentally edit in a hidden mode. Preserve the view's camera and local
selection when returning. A shared part selected in Layout remains identifiable
in PCB; switching branches must not create duplicate document objects. Keep the
current selection only when it is meaningful in the destination view, otherwise
show the branch's own context. Switching away from an active drag or uncommitted
numeric edit must resolve that edit explicitly.

### Implemented refreshed theme

These are the implemented paired palette values recorded in the canonical
DESIGN.md and application tokens. Light and dark are paired treatments of one
system; the existing persisted System theme preference remains unchanged.

| Role | Light | Dark |
| --- | --- | --- |
| Panel | `#FFFFFF` | `#1c2632` |
| Canvas | `#E9EEF2` | `#101a22` |
| Primary text | `#182331` | `#EDF1F7` |
| Secondary text | `#576678` | `#AFBCCB` |
| Action | `#3858D6` | `#9AADFF` |
| On action | `#FFFFFF` | `#101725` |
| Selected text / surface | `#253EAC` / `#E9EDFF` | `#CDD6FF` / `#304373` |
| Board outline | `#397568` | `#6AB99D` |
| Pad / footprint detail | `#8E5AA5` | `#B98CD1` |
| Warning text / surface | `#8A4B00` / `#FFF1D9` | `#F4BA68` / `#332A1D` |

Cool white/mist surfaces and blue-ink dark surfaces replace the heavy charcoal
presentation. Indigo selection is quieter than electric cyan. Teal and plum
remain meaningful geometry roles, with lower visual weight on unselected items.
The original palette study’s ten proposed text/background pairs (primary, secondary, action, selection,
warning in each mode) were calculated at 5.87:1 or better. This validates those
specified values only; it is not a contrast audit of generated raster pixels or
proof of an implemented accessible interface.
The historical study’s eight proposed geometry/action-on-canvas pairs passed their 3:1 target; use DESIGN.md for current runtime pairs.
The exploratory palette calculations were local study artifacts; DESIGN.md
records the current runtime palette and its validation scope.

## Initial exploration

The initial options used the existing light/dark identity and role-led geometry
colors. They remain historical studies, not the current direction.

| Layout | Arrangement | Tradeoff |
| --- | --- | --- |
| Relationship bench | Object tree at left, canvas and inspector above a shallow relationship tray | Origin, alignment, and spacing stay visible; less vertical canvas space |
| Selection ribbon | Fixed selection and action ribbon above tree, canvas, and inspector | Predictable command locations; the ribbon reserves height even for simple tasks |
| Unified sidebar | Object tree and inspector stacked in one sidebar; toolbar above a wide canvas | More canvas width; long object trees compete with long property lists |

The relationship bench, selection ribbon, unified sidebar, and bottom-dock
hybrid were local exploratory studies. The current right-docked design
supersedes them; their mockups and temporary review server are not maintained.

These generated comps explore composition and affordances. Their drawn geometry,
small labels, and dimension placement are illustrative, not a measurement model.
The interaction rules below distinguish implemented local behavior from planned follow-up.
The approved build documents behavior already present in the application; the
remaining proposal language is explicitly labeled where it goes beyond the
implementation.

## Shared shell and control hierarchy

1. **Project header.** One project-name menu, Design/Parts/Export, local save state,
   undo, and redo. Remove redundant name labeling and repeated workspace titles.
2. **Stable tool strip.** Selection scope, creation, transforms, relationships,
   snapping, and view controls have predictable groups. Context changes the
   actions within a group without moving unrelated controls.
3. **Object hierarchy.** Layout/PCB/Case branches provide editing context; boards,
   matrices, rows/columns, keys, components and bodies remain directly selectable.
   The active selection is synchronized with the canvas.
4. **Canvas.** The selected geometry, handles, targets, and relationship guides
   explain the active operation. Handles appear on selection or tool activation.
5. **Right inspector.** Selection heading and breadcrumb, transform values, relationships,
   then useful advanced disclosures. Findings are summarized and expanded on
   demand; blocking findings remain visible.
6. **Status line.** Units, coordinates, zoom, and labeled analysis status. Saved,
   solving, preview available, and output validity remain distinct states.

Use consistent rectangular controls and icon-plus-label commands. Floating
content is reserved for anchored menus, transient measurements, and target
pickers. Avoid permanent pills scattered around the drawing surface.

## Selection and creation

The compact **Select: Column** control names the active scope and exposes
**Matrix · Row · Column · Key · Part** in its menu when in Layout.
The checked menu state, current label, and selection outline identify the scope.
Hovering another scope previews what it would select. A missing scope explains
why it is unavailable rather than silently ignoring input.

| Selection | Immediate contextual actions | Canvas affordances |
| --- | --- | --- |
| Nothing | Add matrix, Add part, Outline | Placement cursor and ghost preview |
| Matrix | Add row, Add column, Duplicate, Remove | Extents, corner rotation, row/column insertion markers |
| Row | Insert before/after, Duplicate, Remove row | Stagger/offset axis handle and insertion guide |
| Column | Insert before/after, Duplicate, Remove column | Stagger, splay arc, and origin crosshair |
| Key | Add adjacent key, Enable/disable cell, Duplicate, Remove | Move axes, rotation handle, anchor targets |
| Part | Add part, Duplicate, Remove | Move axes, rotation, geometry snap targets |
| Multiple objects | Move, Rotate, Align, Distribute, Remove | Group bounds, chosen reference, spacing dimensions |

Small plus markers appear only at relevant insertion boundaries, with text on
hover/focus such as “Insert column after Column 3.” A ghost shows the resulting
keys and affected geometry before committing. Every marker has an equivalent
named toolbar command. Remove is visually separated from creation; the preview
names the scope and affected count. Removing a generated key and disabling its
matrix cell are different actions and must be labeled accordingly.

## Direct transforms

- **Move/offset:** drag the selection freely or its X/Y handle for a single axis.
  A nearby readout shows the delta and unit. The inspector mirrors the same edit.
- **Stagger:** drag a labeled row/column axis handle. Highlight all affected rows
  or columns before movement and show a ghost of the original position.
- **Rotate:** use an arc around the selected object or group, with the pivot
  visible. Rotation of a whole matrix remains distinct from internal splay.
- **Splay:** activate the column arc. Show the chosen origin and the exact
  affected-column set, plus an angle readout and gap/collision feedback.
- **Precision:** clicking the canvas readout permits exact entry; arrow-key
  adjustment and the inspector are equivalent alternatives to dragging.
- **Commit:** pointer release or Enter commits one undoable change; Escape
  cancels the preview. Dragging does not create a separate undo item per frame.

## Splay origin

The origin is a visible, draggable crosshair connected to the active splay arc.
It can sit beyond the column bounds, where it is useful for fanning columns.

1. Select a column and activate **Splay**.
2. Choose **Origin**, then drag its crosshair or **Pick on canvas**. Preview
   centre, corner, edge midpoint, another object's landmark, or a custom point.
3. Moving the origin preserves the current geometry pose. The next angle change
   rotates about the new point; relocating a pivot must not make the layout jump.
4. Show **Affect: This column / This and following**. Preserve the established
   downstream splay option, while making its propagation visible and explicit.
5. Drag the arc. Show the previous pose, angle, affected set, and separation from
   neighbouring geometry. Overlap receives a labeled warning and hatched region.

An origin picked on another object can be attached to that named landmark or
left as a free point in board coordinates. The UI must distinguish those states.
Deleting an origin's reference marks the relationship unresolved; it must not
silently choose a different origin. **Reset origin** names the default location;
resetting the angle is a separate action.

A movable pivot alone does not guarantee a collision-free result. **Maintain
gap** is an explicit relationship that may also require translation. Preview the
affected translation and rotation; if the requested relationships cannot be
satisfied, show the conflict and preserve the previous valid placement.

## Snapping, spacing, and alignment

The **Snap** toolbar group exposes **Grid**, **Geometry**, and **Gap** in a compact
popover, without requiring the inspector. A short status summary keeps the
active snap setup visible without three permanent controls. These are independent aids:

- Grid: configured unit/pitch, fractions such as 1/8, 1/4, 1/2, 1u, and a custom
  millimetre step. A geometry or gap snap is allowed between grid intersections.
- Geometry: edge, centre, corner, and edge-midpoint candidates. Show the actual
  source and target with a guide and named label before committing.
- Gap: a shown spacing value, derived from layout spacing when appropriate,
  with an explicit override and an explicit reference envelope.

The user's spacing example is **19 mm centre pitch − 18 mm key envelope = 1 mm
edge gap**. It is illustrative configuration, not a change to existing defaults.
For unequal unrotated widths on the same axis, the gap is centre spacing minus
half each width. Rotated or irregular shapes use the chosen real boundaries and
displayed measurement direction; a rotated bounding box is not silently treated
as the physical part edge.

The reference selector names what clearance uses: **keycap envelope**, **body**,
**courtyard**, or another supported outline. Copper, body, and courtyard are not
interchangeable. Missing physical geometry shows **Gap unavailable** or an
explicitly named fallback; it never implies a verified clearance.

### Placement sequence

1. Drag a part near a neighbour. Its ghost shows compatible snap candidates.
2. The best candidate gets a guide, target marker, and label, for example
   **Keycap edge → keycap edge · gap 1.00 mm**. Unrelated guides stay quiet.
3. Keep the candidate stable while close to it; allow cycling nearby targets and
   temporarily suppressing snapping with a discoverable modifier.
4. Release to **Snap once**, or select **Keep constraint** to preserve the named
   relationship through later edits. Temporary snapping does not create a
   hidden constraint. A saved relationship shows a link icon and label.

### Align / constrain sequence

1. Select the objects and choose **Align**.
2. Pick a source landmark and target landmark: edges, centres, corners, or named
   origins. Choose the reference object explicitly; it remains fixed in preview.
3. Choose the relation: align/coincident, parallel, offset/gap, or equal spacing,
   only when applicable to the picked geometry. Corner-to-edge and edge-to-edge
   relations must state whether orientation or just position is affected.
4. Preview **Move once** or **Keep constraint**. Dimensions and relationship glyphs
   remain inspectable, keyboard reachable, and editable after creation.

Equal **centre pitch** and equal **edge gap** are separate distribution choices.
For mixed-size components, show which measurement is being equalized. The
constraint list names each target, relation, value, and state: satisfied,
pending, conflicting, or missing reference. Color supplements icons and text.

## Project menu and inspector

The project title is the menu trigger and has an explicit rename action. Group
New/Open/Save copy together; keep Appearance and workspace preferences separate
from Geometry scripts. Use consistent menu rows, visible keyboard shortcuts only
where supported, predictable dismissal, and focus returning to the trigger.

The inspector follows the selection, with a short breadcrumb and object count.
Its main sections are **Transform**, **Geometry/parameters**, **Relationships**,
and **Advanced** where applicable. Units live beside values; X/Y and related
parameters align in rows. Defaults, inheritance, overrides, and mixed selections
are distinguishable. A reset explains its scope. Selection changes must not
unexpectedly move keyboard focus or discard an in-progress value.

## Consistency across workspaces

The earlier workspace family study considered Parts/footprint viewing, custom
footprint editing, PCB placement, and Export together. Its old palette and
top-level PCB/Case navigation are superseded by the current unified Design proposal.

| Workspace | Left context | Main surface and contextual actions | Inspector / details |
| --- | --- | --- | --- |
| Design / Layout | Board and matrix tree | Scope, add/remove, transforms, origin, alignment, snapping | Selected geometry and relationships |
| Parts library | Search, categories, assemblies | Footprint/model preview; Place, Duplicate, Import | Identity, placement defaults, model and provenance |
| Footprint viewer | Part and layer list | Measure, reference origin, 2D/3D, view/layer controls | Read-only geometry, pins, supported parameters |
| Custom footprint editor | Primitives and layers | Pad/line/arc selection, add/remove, move, rotate, align, snap | Selected primitive and explicit units |
| Design / PCB | Boards, nets, components | Place/orient components, layers, alignment, snap, relationships | Position, side, nets, footprint and findings |
| Design / Case | Bodies and features | Inspect assembly, select body/feature, view and visibility controls | Dimensions, source geometry and readiness limits |
| Export | Artifact list and formats | Preview selected artifact; export current revision | Settings, dependencies, warnings and revision |

Standard footprints open in **View** with supported parameters; direct pad
editing belongs to a custom/imported definition, with **Make editable copy** as
the clear transition. Selecting a library part never exposes irrelevant raw pad
settings. Viewer and editor share preview navigation, axes, origin, layers,
selection styling, and measurement vocabulary.

PCB remains placement and output preparation in this proposal; final routing and
fabrication review remain downstream. Export reuses the shell, selection/details
pattern, menu language and status vocabulary, rather than showing meaningless
transform controls. Stale output is clearly marked and its export blocked.

## Essential states and access

- Empty document offers Add matrix / Add part. No selection shows board context.
- Multi-selection shows mixed values and a visible reference object for Align.
- Locked or constrained geometry explains why an operation is unavailable.
- Pending geometry preserves the draft/ghost and identifies the last resolved
  preview; failure keeps the previous valid geometry visible.
- At narrower widths, collapse the object tree before squeezing the canvas;
  expose the inspector as a deliberate drawer. Preserve labeled active tools.
- Touch uses larger handles and a tap-to-pick origin mode. Keyboard users can
  select every target through lists and edit every transform without dragging.
- Menus and popovers escape scrolling panels, keep visible focus, close on
  Escape, and restore focus. Reduced motion removes decorative transitions.
- Both themes retain semantic content/foreground pairs. Warnings use explicit
  labels; a local save, generated preview or successful export is never a
  fabrication-readiness claim.

## Design-review questions

Review whether the right dock and compact command groups leave enough
room for both the layout and the current operation on a widescreen display.
Before implementation,
validate splay propagation and attached-origin behavior
with a representative staggered layout; test the snap candidate hierarchy with
mixed-size and rotated parts. These are interaction validation needs, not an
invitation to silently invent solver behavior during implementation.
