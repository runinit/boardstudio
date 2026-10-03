# 03: Resize selected keycaps with linked neighbor reflow

**Parent:** F6C.3 — Shared key-size drafts, selection scope, and linked reflow.

**What to build:** The Layout Inspector lets a designer resize the selected supported keycaps using TypeScript-equivalent width/height, Mixed, and Wide/Tall controls. The accepted result preserves the selected group's center, reflows row and column neighbors along their projected matrix axes, keeps linked halves consistent, updates the physical 2D caps, and participates in one Undo/Redo and normal project persistence operation.

**Blocked by:** F6C.1 — Keycaps projection, shared selection, and physical 2D view.

**Capability start gates:** Before source changes, verify on the implementation fixture that the accepted active-board projection exposes stable key and matrix IDs, effective cap placement/size, pitch and edge-gap data; the mounted Layout Inspector receives the accepted key/row/column/matrix selection and linked-layout source/partner metadata; and the existing accepted-document edit/history path can admit one plan with all affected target IDs. These proven capabilities authorize the fixture slice without waiting for full F3.2 or F3.5 completion. If any named capability is absent, record the exact missing capability and its source before starting. F3.2 and F3.5 remain F6C.3 parent acceptance joins.

**Status:** implementation in progress on `codex/keycaps-size-reflow-20261002` from integration `c701233e`; independent planning Spec/Standards are clear at the recorded exact hashes, and the fixture capability gates are proven in [the start receipt](../evidence/size-reflow/capability-start-gates.md). This status does not close the source review, browser journey, F6C.3 or F3.2/F3.5 joins.

### Paired linked-key browser follow-up — 2026-10-03

One selected linked key was resized from 1u×1u to 2u×1u through the mounted Layout Inspector in both pinned React and Dioxus candidate 34759. The mirror counterpart received the same size; the adjacent row placements moved symmetrically; the existing remaining-overlap warning remained visible; and Undo/Redo plus reload restored the accepted values. The bounded capture and derived fixture provenance are in [the paired receipt](../../dioxus-frontend-v1/evidence/keycaps-size-reflow-20261003/paired-linked-resize.md). This proves only the one-side selected linked-pair path. It does not cover both halves selected together, all mixed-selection/commit boundaries, or the F3.2/F3.5 joins, and it does not change the ticket's open status or acceptance checklist.

- [ ] The contextual Layout Inspector shows Key size only when the accepted selection contains supported keycaps, with React-equivalent title, placement, labels, size output, and selection scope.
- [ ] Width and height are derived from each selected placement, pitch, and edge gap using nearest-quarter-unit rounding; a heterogeneous selection visibly says Mixed.
- [ ] Width and height sliders use the reference 1–7u range and 0.25u step. Changing one axis preserves the other for all selected items.
- [ ] Wide and Tall derive one long/short pair from the current draft and apply that common pair to all selected items in one action; for Mixed selection the initial draft comes from the first selected item, and the control intentionally normalizes different selected sizes. Verify with at least two differently sized selected caps.
- [ ] Pointer release, blur, and keyboard release follow the React commit boundaries; delayed keyboard commit and pointer/blur follow-up do not create duplicate history entries.
- [ ] Changing the selection, board, project, or scope resets ephemeral slider drafts; empty, unsupported, unchanged, and stale selections do not create edit history.
- [ ] Accepted resizing reflows same-row/same-column keycaps only as needed, using projected axes for rotated, mirrored, or splayed matrices, and preserves the selected group center.
- [ ] A mirrored counterpart receives the corresponding key size while the canonical layout is planned once, whether one or both halves are selected.
- [ ] Existing disabled cells, saved rotations, unrelated parts/matrices, matrix-local offsets, and selection membership remain intact; remaining overlaps use the existing visible warning.
- [ ] One accepted resize produces one Undo entry with all affected target IDs; Undo/Redo restore sizes, moved neighbors and linked geometry; save and reload preserve the accepted result.
- [ ] The same fixture and user journey pass in pinned React and the source-stamped Dioxus build, with captured source/build/fixture hashes, project/board/matrix identities, control placement, event timing, accepted revision, geometry, history and reload evidence.
- [ ] Focused planner tests and mounted/browser checks cover unit conversion, mixed selection, each axis, Wide/Tall, row/column reflow, group centering, mirror deduplication, rotated/splayed offsets, draft resets and duplicate commit suppression.
- [ ] Required independent Spec/Standards reviews clear the exact source, and RF-005 is updated with the frontend-owned resize/reflow evidence; F6C.3 and F3.2/F3.5 joins remain open.
