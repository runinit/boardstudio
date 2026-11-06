# Board Studio

Board Studio describes keyboard boards and their associated mechanical assemblies.

## Language

**Layout canvas view**:
The rendered presentation of a board's layout. It reflects the current accepted design and
any active preview; selection and gesture feedback affect its appearance without changing
the accepted design.

**Magnetic snap zone**:
A screen-space region around an enabled Layout snap target. Component moves, component
placement, and matrix stagger or offset drags move continuously outside the region and are
smoothly drawn toward a target within 8 CSS pixels. The target captures exactly within
3 CSS pixels and stays latched until the pointer is more than 14 CSS pixels away. Releasing
before exact capture keeps the displayed position; releasing after capture commits the
target. Alt temporarily disables snapping for the drag.

**Geometry snap target**:
A position where one component's physical envelope landmarks align with another's, or their
envelopes meet the configured gap. An eligible geometry target takes priority over a grid
target. Geometry snapping applies to one component at a time; a multi-component drag uses
grid snapping and preserves the group's relative placement.

**Drag-relative grid snap**:
Grid increments during component and matrix stagger or offset drags are measured from the
component or parameter value at the start of the drag, preserving its initial alignment.

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

**Closure boss**:
A local projection of the top case that supports a closure insert and can bring
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

**Board**:
A PCB design with its component placement, electrical wiring and board boundary.
One board may be used by more than one physical board instance.
_Avoid_: half (when the PCB design is meant)

**Physical board instance**:
One physical realization of a board in the keyboard, such as its left or right half.
Two instances remain distinct even when they use the same reversible board design.
_Avoid_: board (when one physical realization is meant)

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

**Column stagger**:
The vertical step of a matrix column from the previous column, in matrix
coordinates. The first column steps from the matrix origin. The column and every
later column inherit it.
_Avoid_: column offset (when the inherited step is meant)

**Column offset**:
An independent adjustment of one column only. Later columns do not inherit it.
_Avoid_: stagger (when the independent adjustment is meant)

**Row offset**:
An independent adjustment of one matrix row only.

**Key local offset**:
An adjustment of one key within its matrix. The key's attached companions move
with it.

**Structural relationship**:
Placement a selection inherits from its matrix (pitch, offsets, stagger, splay,
matrix frame) and from key assemblies. It is read from the accepted document;
there is no separate relationship record.

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

### Editing

**Accepted document**:
The project state the document engine has applied and saved; every view and
export reads from it.
_Avoid_: current doc, model, snapshot (when the saved state is meant)

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
