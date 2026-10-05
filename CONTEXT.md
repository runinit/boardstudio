# Board Studio

Board Studio describes keyboard boards and their associated mechanical assemblies.

## Language

**Generated outline**:
A board boundary derived from the current placement of its included components
and the chosen outline settings; it updates as the layout changes.

**Outline refinement**:
An accepted alternative boundary that remains linked to its source components
and follows their placement. Freezing or manually editing it creates a separate
fixed copy while retaining the linked version.

**Edited outline**:
A manually modified copy of an outline, retained separately from its generated
source. Its geometry stays fixed when components move; clearance findings identify
where the changed layout no longer fits.

**Outline version**:
A named alternative board boundary belonging to one PCB design, with its own
cutouts, bridges, protected gaps and corner finishing. Versions share component
placement while retaining independent outline choices.

**Active outline**:
The outline version selected to define the current board shape and its exports.

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
part. Values not saved fall back to the generator's defaults.

**Generated support placement**:
A support position produced by the generator and not subsequently positioned or
pinned by the user; it remains eligible for automatic placement repair.

**User-positioned support**:
A support whose position the user has committed or explicitly pinned; automatic
placement repair preserves it until the user moves or resets it.
