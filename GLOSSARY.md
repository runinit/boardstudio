# Board Studio glossary

Board Studio describes keyboard boards and their associated mechanical assemblies.

## Language

**Layout canvas view**:
The rendered presentation of a board's layout. It reflects the current accepted design and
any active preview; selection and gesture feedback affect its appearance without changing
the accepted design.

**Generated outline**:
A board boundary derived from the current placement of its included components
and the chosen outline settings; it updates as the layout changes.

**Outline refinement**:
An accepted alternative boundary that remains linked to the board's current
included components and follows changes to their placement, geometry and membership.
Each refinement owns its outline choices independently. Freezing or manually editing
its perimeter creates a separate fixed copy while retaining the linked version.

**Edited outline**:
A manually modified copy of an outline, retained separately from its generated
source. Its geometry stays fixed when components move; clearance findings identify
where the changed layout no longer fits.

**Outline version**:
A named alternative board boundary belonging to one PCB design, with its own
cutouts, bridges, protected gaps and corner finishing. Versions share component
placement while retaining independent outline choices.

**Active outline**:
The generated outline, refinement or edited outline selected to define the current
board shape and its exports.

**Outline connection**:
An authored path and width requesting PCB material between component groups within
an outline version. Its control points may follow named components; the resulting
material is an outline bridge.

**Outline bridge**:
PCB material connecting component groups within one outline version. A bridge
may appear in multiple matrix contexts while remaining one connection.

**Protected gap**:
A deliberately retained recess in a generated board boundary, excluded from
automatic gap filling through the Keep gap action. Protection follows its source
components on generated/refined outlines and remains fixed on edited copies.

**Case exterior**:
The outer boundary shared by a case's tray and top, sized from the board and the
space required by its mechanical features and clearances.

**Collision check**:
An explicit comparison of physical items in an assembly for intersection or required
clearance. Findings belong to the accepted design being checked and remain uncertain
when the available geometry cannot support the requested accuracy.

**Approximate collision finding**:
A measured gap or potential interference in the geometry available for a collision
check. It is not a guarantee of physical clearance when geometric error or model
completeness is unknown.

**Vendor model preview**:
A display representation of an imported vendor model, distinct from its preserved
original. It may contain repairs whose geometric error is unknown; successful display
does not establish collision accuracy or fabrication readiness.

**Incomplete vendor preview**:
A vendor model preview with missing geometry, displayed with an explicit error.
It cannot establish clearance for the complete model.

**Generated exact geometry**:
The case or PCB geometry used for exact manufacturing output, distinct from a display
mesh. Exact describes its geometric representation, not a guarantee of fabrication
readiness or collision accuracy.

**Current case result**:
Generated exact case geometry and its associated mechanical information that still
represent the accepted document for the selected physical board instance. A previous
result may remain visible without being current.

**Nominal wall thickness**:
The case wall thickness before local gasket pockets remove material.

**Remaining wall**:
The solid exterior wall left behind a gasket pocket, distinct from the nominal
wall thickness.

**Vertical travel**:
The permitted upward and downward displacement of the floating plate/PCB assembly
from its nominal assembled position, excluding tilt and sideways displacement.

**Hardware preset**:
A specified screw or insert variant with its thread, physical dimensions, and
applicable installation requirements; a thread size alone is not a hardware preset.

**Screw drive**:
The tool-engagement feature of a screw, such as Torx or an internal hex socket.

**Screw head profile**:
The external shape and dimensions of the screw head that determine its bearing
surface and the space needed to recess it in the case.

**Case boss**:
A solid projection from a case body that surrounds a mounting hole, wider than the
hole it surrounds, such as a closure boss on the top case or a standoff rising from the tray to hold
the board.
_Avoid_: boss (when it could mean a closure boss specifically)

**Closure boss**:
A case boss on the top case that supports a closure insert and can bring
its threaded entrance closer to the bottom screw, separate from a gasket support.

**Footprint generator**:
A built-in parametric footprint identified by a stable source ID. It produces a
component's pads, outline graphics, nets and 3D model placements from its
generator parameters; it is not tied to any external tool's format or name.

**Generator parameters**:
The values saved for one footprint generator on a part definition or a placed
part. Values not saved fall back to the generator's defaults. Each value is typed by
the generator's declaration: a length is a number, not text. Core refuses an edit that
writes a value of the wrong type or a name the generator does not declare. A saved project
that holds an undeclared name, or text where a number is declared, opens with a warning
instead of being refused.

**Generated support placement**:
A support position produced by the generator and not subsequently positioned or
pinned by the user; it remains eligible for automatic placement repair.

**User-positioned support**:
A support whose position the user has committed or explicitly pinned; automatic
placement repair preserves it until the user moves or resets it.

**Mirror-only Part**:
A Part belonging to the mirror half without a corresponding source Part. A Local Part retains
its source relationship and an exception; a mirror-only Part has no such relationship.
_Avoid_: Local Part (when there is no source Part)

### Mechanical relationships

**Mounted daughterboard**:
A separate physical PCB attached to a host board. Its constituent components and
copper belong to the daughterboard, not the host PCB.

**Embedded circuit copy**:
A source circuit copied into the host PCB as independently owned components and
internal nets, with explicit connections to host signals.

**Board**:
A PCB design with its component placement, electrical wiring and board boundary.
One board may be used by more than one physical board instance.
_Avoid_: half (when the PCB design is meant)

**Physical board instance**:
One physical realization of a board in the keyboard, such as its left or right half.
Two instances remain distinct even when they use the same reversible board design.
_Avoid_: board (when one physical realization is meant)

**Reversible PCB**:
A PCB design that can serve opposite physical halves through the appropriate assembly
orientation and connections. The halves share its copper routing but may require
different firmware key positions and MCU pin mappings.

**Reversibility conversion**:
An explicit, reviewed change between a split using one shared reversible PCB design
and a split using distinct PCB designs. It is not merely a change of footprint options.

**Jumper configuration**:
The selected open or solder-bridged jumper connections on one physical board instance.
These determine its assembled electrical connections, including applicable MCU pin mapping.

**Central half**:
The half that communicates with the computer and receives input from the peripheral
half. It may be physically left or right.
_Avoid_: left half (when the communication role is meant)

**Peripheral half**:
The half that sends its input to the central half. It may be physically left or right.
_Avoid_: right half (when the communication role is meant)

**Incomplete split design**:
A split keyboard design whose halves or required hardware assignments are not yet
complete. It is an unfinished design, not by itself a corrupt project.

**Firmware handoff**:
An editable repository template containing keyboard firmware sources, configuration
and build information. It is not a compiled image ready to flash.

**PCB handoff**:
A package of PCB design data and wiring/assembly information for downstream work.
It is distinct from a firmware handoff and does not establish that a PCB was manufactured.

**Export availability**:
An output's current eligibility to begin from the accepted design and its known prerequisites.
It does not guarantee delivery if the design changes or generation fails.

**Mechanical mirror pair**:
Two physical halves whose shared mechanical features stay reflected when edited
from either half. They may use one reversible PCB design or different PCB designs,
and individual features may be half-specific exceptions. New pairs share their
construction settings within the pair; existing independent halves link explicitly.
Everything in a pair is linked by default, including proposed and generated
features; only an explicitly recorded exception is not
([ADR-0008](docs/adr/0008-linked-halves-stay-linked.md)).

**Mirrored half**:
The second half of a mechanical mirror pair, kept as a reflection of the first in
its matrices, keys, components, outlines and mechanical features. Differences are
explicit per-item exceptions, and the pair never drifts silently
([ADR-0010](docs/adr/0010-mirrored-half-is-a-core-maintained-reflection.md)).

**Pair construction**:
The construction choices shared by a mechanical mirror pair, such as mounting method,
plate thickness, layer spacing and wall thickness.
_Avoid_: shared configuration (which also suggests sharing half-specific hardware)

**Effective mechanical configuration**:
The mechanical configuration applicable to one board or physical board instance,
including its shared construction, hardware-specific settings and feature exceptions.
_Avoid_: stored configuration (when the complete applicable configuration is meant)

**Construction source**:
The member of a mechanical mirror pair whose geometry is the reference for its
construction and generated closure proposals.

**Mechanical feature author**:
The member whose frame defines a linked mechanical feature's authored geometry.
It may differ from the pair's construction source.

**Linked mechanical feature**:
A mechanical feature whose counterparts on mirrored halves remain related as
reflected geometry when the feature changes.

**Local mechanical feature**:
A counterpart of a pair feature with an explicit exception giving that half its
own independently editable geometry.

**Omitted mechanical feature**:
A counterpart of a pair feature explicitly excluded from one half, while the
feature remains part of the pair.
_Avoid_: deleted feature (when only one counterpart is omitted)

**Half-specific mechanical feature**:
A mechanical feature required by one half's hardware or assembly, such as a
screen opening or an encoder clearance. It is an explicit exception to mirrored
sharing and may be a local feature or an omission on that half.

### Matrix layout

**Layout**:
A group of one or more matrices on one board, together with the independent components placed
among them. Matrix anchors stay inside one layout, and linked halves on one board mirror whole
layouts.
_Avoid_: zone, matrix (when the whole group is meant)

**Column stagger**:
The vertical step of a matrix column from the previous column, in matrix
coordinates. The first column steps from the matrix origin. The column and every
later column inherit it.
_Avoid_: column offset (when the inherited step is meant)

**Column spread**:
The horizontal step of a matrix column from the previous column. The column and every later
column inherit it.
_Avoid_: column offset (when the inherited step is meant)

**Column offset**:
An independent adjustment of one column only. Later columns do not inherit it, and it does not
move the column's splay origin.
_Avoid_: stagger, spread, shift (when the independent adjustment is meant)

**Splay origin**:
The point a column's splay turns about, measured from the column's base: its first key's
position after spread and stagger.
_Avoid_: pivot (in user-facing text)

**Column rotation**:
A turn of one column about its splay origin. No later column inherits it.
_Avoid_: splay (when only this column turns), bend

**Matrix pitch**:
A matrix's key unit. Key and keycap size are the pitch minus the edge gap. Columns and rows with
no spread or padding of their own step by one pitch, and snapping steps in fractions of it.
_Avoid_: spacing (when the step between particular keys is meant)

**Row padding**:
The step from one row of a matrix column to the next.
_Avoid_: row offset, pitch (when a single row's step is meant)

**Matrix anchor**:
A key of another matrix in the same layout whose final pose a matrix's frame follows, such as
a thumb cluster following the main matrix — or the average or intersection of several such
keys, whose combined pose the frame follows instead. The matrix may follow the target's X, Y
and rotation, or only some of them.
_Avoid_: parent, reference (unqualified)

**Part anchor**:
A key of a matrix in the part's own layout that an independent part follows. Spread,
Stagger and Splay carry in the anchor frame; Offset and Rotation apply for the part alone.
The part may follow the target's X, Y and rotation, or only some of them (Follow). A dropped
part anchors to the nearest key of its half and keeps the drop point, upright; Detach clears
the anchor and places the part from the half origin.
_Avoid_: parent, reference (unqualified)

**Row offset**:
An independent adjustment of one matrix row only.
_Avoid_: shift (when the independent adjustment is meant)

**Key local offset**:
An adjustment of one key within its matrix. The key's attached companions move
with it.
_Avoid_: shift (when the independent adjustment is meant)

**Local key**:
A counterpart of a linked key with an explicit exception giving it an independent world
pose. Shared matrix layout changes do not move it, and neighbouring keys remain linked.
_Avoid_: key local offset (when the linked-half exception is meant)

**Local matrix**:
A counterpart of a linked matrix with its own independently editable layout and dimensions.
Making it Local keeps its pose and removes its inherited anchor; it may be explicitly
anchored again.

**Shift**:
A move of one row (row shift) or one key (key shift) that carries to every later row of its
column, along the key's heading.
_Avoid_: offset (when the carried move is meant), nudge

**Bend**:
A turn of one row (row bend) or one key (key bend) that carries to every later row of its
column. The later rows swing about it, so the column curves.
_Avoid_: rotate, rotation (when the carried turn is meant), curve

**Key rotation**:
A turn of one key about its centre. No other key inherits it.
_Avoid_: bend

**Structural relationship**:
Placement a selection inherits from matrix steps, carried transforms, independent
adjustments, its matrix frame and anchors, and key assemblies. It is read from the
accepted document; there is no separate relationship record.

**Explicit constraint**:
A separately authored placement relationship from a source part to a target part.
It takes priority over a companion's attachment to its key.
_Avoid_: relationship (unqualified)

**Offset constraint**:
An explicit constraint that holds its target at a fixed distance from its source along the
board's axes, and at a fixed angle to it. Turning the source turns the target but does not
swing it around the source.
_Avoid_: offset (unqualified, when the axes matter)

**Part-offset constraint**:
An explicit constraint that holds its target at a fixed distance and angle measured along the
source part's own axes. Turning the source swings the target around it, as a following
companion moves with its key.
_Avoid_: attach constraint (an attached companion is one that follows its key)

**Split gap**:
The stored editable distance between the two halves of a split board, measured key-edge to key-edge. It defaults from projected key envelopes at creation and never moves on its own: the adaptive pass widens the Current spacing around it instead, and narrowing back stops at the stored floor.
_Avoid_: spacing (when the stored half distance is meant)

**Current spacing**:
The evaluated key-edge distance between the two halves. It never goes below the Split gap: it widens when the halves need the space and narrows back toward the stored floor.
_Avoid_: Split gap (when the evaluated half distance is meant)

**Half frame**:
Each half's own placement frame for its matrices, positioned from the split axis by half the Split gap. A matrix origin on a stored-gap half is a within-half value; changing the gap moves both halves symmetrically without rewriting it.
_Avoid_: board coordinates (when a stored-gap half's within-half placement is meant)

**Half-frame part**:
An independent Part on a stored-gap half, stored in that half's frame and evaluated at board-space through the half offset. Spacing changes never rewrite it; drags, constraints and spacing all work board-space.
_Avoid_: board-space part (when a stored-gap half's within-half stored pose is meant)

**Mirrored part**:
A half-frame part with a Part correspondence on the mirror link. Either copy edits both; the partner realizes the reflected half-frame pose on every sync.
_Avoid_: linked part (when the correspondence-mirrored half-frame pair is meant)

**Local part**:
A half-frame part with no Part correspondence, pinned local by the substitute flag so the default mirror never claims it. Removing the pin re-mirrors it at the reflected pose. A local part's overhang past the split axis still counts in the split minimum.
_Avoid_: mirror-only part (when a pinned unmirrored half-frame part is meant)

### Editing

**Format reset**:
A project-format change that refuses older project data instead of converting it.
It does not delete existing saved projects; starting fresh creates a separate project.
_Avoid_: factory reset (when the format compatibility change is meant)

**Editor session**:
One opening of a project for editing. Reopening the same project starts a new
editor session even when its saved contents have not changed.
_Avoid_: session (when the lifetime of one project opening is meant)

**Accepted document**:
The project state the document engine has applied and saved; every view and
export reads from it.
_Avoid_: current doc, model, snapshot (when the saved state is meant)

**Replacement input**:
A newly created keyboard input following removal of an earlier input, even when it
occupies the same matrix position. It is distinct from a surviving input moved by a layout edit.

**Temporarily disabled input**:
An existing keyboard input made inactive rather than deleted and recreated.
Re-enabling it resumes its prior bindings as the same input.

**Draft value**:
A value typed into a field that has not yet become part of the accepted document.

**Pending edit**:
An edit the user has committed that has neither landed nor failed. Queued pending
edits are resolved against the accepted document when they run, so each applies
on top of the edits accepted before it.
_Avoid_: in-flight edit, queued command

**Landed edit**:
A pending edit that produced, or confirmed without change, an accepted document
revision. An edit that fails or whose target has gone is not landed; its field
shows the accepted value again with an explanation.

**Field edit**:
An edit that sets a value the user entered, such as a name, a coordinate or a
setting. Committing it again with the same value changes nothing, so field edits
always queue; the latest committed value wins.

**One-shot action**:
An edit that creates, removes, imports, uploads or applies something. Committing
it again does it again, so its control is unavailable while it is pending.
_Avoid_: command, button edit

**Edit preview**:
A transient display of what a drag or typed value would change, shown before the
user commits. It never becomes part of the accepted document or Undo history;
the direct Session preview event carries only the operation, targets and transaction
identity. It has no commit phase. Committing submits a pending edit intent through
`ResolveEdit`, which resolves against the accepted document when it runs.
_Avoid_: live edit, uncommitted edit

**Theme**:
The visual style of the whole workbench: Original (the established Board Studio look)
or shadcn. It sets colours and component styling, never what a control does, and is
chosen independently of Appearance.
_Avoid_: skin, palette

**Appearance**:
Whether the workbench is shown light, dark, or following the system setting. Every
Theme has both a light and a dark form.
_Avoid_: theme (for light/dark), color mode

**Workspace**:
One of the main working views of a project: Layout, PCB, Keymap, Keycaps, Case or Parts. It is chosen
from the header and shows its own canvas with the Objects and Inspector docks. Export is a separate
page, not a workspace.
_Avoid_: stage, mode, Design (the former name of Layout)

**Dock**:
One of the two side panels beside the canvas: Objects on the left and Inspector on the right. A dock is
either shown beside the canvas or collapsed, and on narrow screens both appear as one sheet over the
lower part of the screen so the canvas stays visible.
_Avoid_: sidebar, drawer, pane

**Command palette**:
A searchable list, opened with Ctrl/Cmd+K, of the workbench's existing actions and of the project's objects by name. Choosing an
entry navigates, opens a tool or selects an object; it never performs a destructive or numeric edit itself.
_Avoid_: command bar, quick open
