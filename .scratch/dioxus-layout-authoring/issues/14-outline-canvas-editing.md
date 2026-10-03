# F3.4d: Drag and nudge outline perimeter points

**Parent:** F3.4 — Outline editing and refinement UI. Parent criteria, graph edges, history and public acceptance remain unchanged.

**What to build:** Add accessible canvas handles for the polygon perimeter already exposed by issue 13. Keep canvas and Inspector point selection synchronized. Drag a handle with live Core preview and the pinned React snap policy, then commit one transaction on release. Support Escape/capture-loss cancellation, grid nudge by arrows, Shift for a tenfold nudge, and Delete/Backspace while preserving at least three points. Generated edits create/activate a fixed copy on the first accepted change; fixed edits preserve the active polygon feature through `SetOutline`.

**Reference:** React `5a472a9426e6e38993361da402cd4ec730feb369`, current integration source `05c0f1373f6a61653ebe55ff7994a1155a98a84a`, supplied layered Sofle fixture SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`. React baseline screenshot: [f34d-react-baseline-20261003.png](../evidence/f34d-react-baseline-20261003.png).

**Spec:** [F3.4d direct outline editing](../specs/F34d-outline-canvas-editing.md).

**Capability boundary:** Existing F3.4c owner and Core `CopyOutline.edit`, `SetOutline`, `SelectOutline`, preview, commit and history operations are reused. No new public contract, schema, renderer or geometry algorithm is needed. Port the existing `outlineSnapping.ts` policy unchanged.

**Status:** candidate source and focused public evidence are implemented. The paired Generated-outline journey now covers trusted drag/preview, immediate release, Escape/out-and-back cancellation, one-step history, focused ArrowRight nudge, selected-point Delete, and reload persistence on the pinned layered Sofle fixture. Full F3.4d acceptance remains open for fixed-version edits, additional capture/owner-loss cases, and the parent acceptance joins.

**Focused paired receipt:** [F34d drag, cancel, nudge, delete, and reopen journey](../evidence/f34d-perimeter-paired-20261003/README.md). The 34749 public candidate predates the separate negative-half-grid follow-up; that source policy has its own focused production-seam RED/GREEN receipt in the same evidence packet.

- [ ] Expose point handles only for the same supported first Generated contour or first active fixed Polygon as issue 13. Keep point selection synchronized between Inspector list, numeric fields, and canvas handles.
- [ ] Drag captures one pointer, uses Layout snap settings and the existing outline snapping policy, streams live Core previews, and commits exactly one captured transaction with the final pointer sample on release.
- [ ] Escape, pointer cancellation, lost capture, owner unmount and replaced board/version/document/session cancel preview and release capture without committing. Ignore late preview results from the retired interaction.
- [ ] Focused point arrows nudge on the effective grid; Shift+Arrow uses ten times the step. Delete/Backspace removes only while at least three points remain. Each accepted keyboard action creates one ordinary Core history entry.
- [ ] Preserve Generated source when first drag/nudge/delete creates a fixed copy. Fixed version updates only the matching feature ID and preserves its anchor and operation. Show Saved only after accepted geometry and durable revision confirmation.
- [ ] Use the existing canvas interaction arbiter. Do not collide with part placement, mirrored-pair placement or pan; retain keyboard focus and accessible point names/selection state.
- [ ] Verify production browser parity on the pinned reference fixture, plus the paired Generated and fixed edit outcomes, Undo/Redo and save/reopen gates at parent acceptance. Do not add an exhaustive neighboring transition matrix.
- [ ] Record the browser preview boundary, bridge owner/removal criteria and React source reference. Preserve RF-001, RF-006 and RF-009; this child does not close F3.4/F3.7 or the 62-parent graph.

**Out of scope:** Rectangles, PartEnvelope connections, manual addition/cutout drafts (tracked by [F3.4e](18-outline-manual-polygon-drafts.md)), bridges, gaps, refinements, a new snap algorithm, parent closure, React deletion, renderer/CAD changes, public API/schema visibility, and global performance/accessibility qualification.
