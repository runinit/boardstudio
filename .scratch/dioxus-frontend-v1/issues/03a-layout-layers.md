# F3a: Layout keycaps, layers and footprint view

**Status:** implementing; pulled forward by the user's missing-function correction.
**Parent:** [F3](03-layout.md). Full F3 remains open.

Restore the pinned React Layout presentation now: default keycap outlines and the
Keys, Components, Keycaps, Footprints and Board visibility switches. Share the
Footprints switch with the canvas toolbar toggle. Start with all visible except
Footprints. Keep these options in presentation state across workspace switches;
do not write a project revision, history entry or saved schema field.

Use enabled projected matrix membership to classify keys and resolve cap size
(part override, definition, pitch minus edge gap). Also show eligible standalone
switch caps, exclude rotary input/encoders, and preserve selection/drag/Undo with
the complete cap as a hit target. Render footprints from existing definition
pads/drills and the same generated graphics as React, with local pose, rotation
and back-side mirroring. Hide key references when cap guides are visible. Render
board holes distinctly. The popover must not alter camera or canvas bounds.

The retained catalogue generator remains a service. Package its unchanged TS
source with type erasure and its verified generated catalogue; Dioxus/Rust owns
all SVG projection, state and controls. Include those service files in root and
subpath offline manifests and provenance. This does not migrate generator logic
or authorize a public Rust API change.

## Verification

- Existing demo reproduced the failure: no Layers, no Footprints, zero keycaps.
- Native projection tests cover Y-up coordinates, hidden/reference text, malformed
  geometry, native/legacy arcs and nested keepouts.
- Browser: default caps, every independent layer, shared Footprints state, actual
  pads/drills/graphics, no history/camera changes, cap edits and Undo, workspace
  retention, compact/Escape/focus, both themes and multi-board fixture scope.
- Root and subpath production builds and offline asset delivery, affected Rust
  fmt/check/Clippy/tests, repo checks and independent Standards/Spec review.
- Fresh visual comparison with React desktop and compact captures.

Full Layout authoring and 3D assembly remain in F3/F7. The reference's working 3D
assembly selector must be ported with its viewer; an inert selector is not an
acceptable substitute in this correction. Keep this boundary explicit in the
roadmap and do not mark Workbench, CanvasObjects or WorkbenchLayers fully ported.
