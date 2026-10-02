# Draft child ticket: first real Parts 2D preview and controls

Parent: F4.4 Inspect isolated 2D and 3D library previews. This child makes the earliest independently demoable 2D path explicit; it does not replace the parent’s 3D/shared-viewer acceptance.

## Goal

After a user selects a real component in Parts, show its actual compiled/projected 2D footprint in the existing center canvas and expose reference-compatible per-layer visibility controls. Keep the component catalogue and Inspector in their existing panels.

## Start / acceptance edges

- Start after F4.1’s current accepted catalogue/detail projection and its actual root-owned Parts mount contract.
- The 2D projection path can start without waiting for F7.3 because the existing F3a footprint SVG/projector is already used by the app; verify it is page-binary reachable before implementation.
- Full F4.4 acceptance remains joined to F7.3 and INT.2. This child cannot mark F4.4 complete or claim 3D/model-camera parity.

## Behavior

- Display the selected definition’s source-backed pads, drills, text/graphics, courtyard/keycap outline and only supported companions/pose. Show pending, no-selection, compile-error, and diagnostic states truthfully.
- Provide named per-footprint layer controls matching React’s library preview. Layer visibility is private presentation state, keyed to the selected library projection; it is not Session part selection or project data.
- Query, row selection, layer changes and 2D/3D tab selection do not mutate the authoritative ProjectDoc, revision or history.
- Build only from current catalogue entry/accepted scope and immutable selected-definition projection. Capture selection/scope for asynchronous compile; stale results cannot replace current content. Do not keep a deep cloned ProjectDoc per repaint.
- Use an isolated ephemeral sample ProjectDoc where the existing projector requires one; never submit edits or retain a second authoritative Session.
- No synthetic preview geometry, hard-coded rows, renderer copy, new generic preview abstraction, or API/schema/CAD changes.

## Acceptance

- Paired React/Dioxus public browser journey: same fixture, select representative imported, Ergogen and project-override definitions; compare 2D pads/graphics/courtyard and initial layers; toggle a non-default layer and change selection; verify stale pending completion is ignored; verify project revision/history and Session selection are unchanged.
- Exercise empty selection, pending projection, supported diagnostic, and compile error states with actual sources.
- Desktop/theme checks now; compact panel interaction joins F2.3/shared acceptance. Capture screenshot and action trail.
- Root owns mounting/shared CSS; implementation owns only a new private preview component/typed projection until root approves its mount.
