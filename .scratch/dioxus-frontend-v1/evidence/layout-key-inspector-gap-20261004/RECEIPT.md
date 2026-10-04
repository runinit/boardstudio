# Layout key Inspector parity probe — 34814 vs TypeScript 5175

Paired Sofle v2 demo copies, Left PCB, 2D. Empty selection: both show Board context (name, Outline, Placed parts).
Single key (Key 4.1) and shift-selected pair ("Key 4.2 / 2 keys selected", Local Y -0.05): selection counts, Properties/Relations
tabs, Key size and Local X/Y/rotation/Reset local transform match; the Relations tab text is identical.

Reproduced gap: the Dioxus key Properties tab lacks, compared with `app/src/ui/MatrixInspector.tsx`:
- `Enabled` checkbox (cell.enabled)
- `Key Assembly` select (cell.definitionId / variant)
- `Attached components` list with per-component Replace select and Remove, plus mirror-target
  "Use mirrored components" / local-override note and the "No attached components" empty note
- `Independent layout` ownership card and the `Left PCB / Layout / keys` breadcrumb line
Source search finds no Dioxus implementation (`web/src/presentation/objects/matrix_transform_inspector.rs` only has the
transform fields). The Wide/Tall button also shows no pressed state at 1u in Dioxus (Wide is pressed in TS).
Not repaired in this session.

## Enabled toggle repair — candidate 34815 (source 0d37fc1d, page-only reuse build from 34814)
Added `KeyEnabled` through the existing set-matrix path (`matrix_transform_operation.rs`, controller projection/baseline, Inspector checkbox
`Key enabled`). Focused tests: 9/9 executed (new `key_enabled_toggle_preserves_cell_and_creates_missing_cell`; no separate RED run
was captured, the test used new symbols). Page wasm and native `cargo check` pass.
Public check on Sofle v2 Left PCB, key 4.1: uncheck -> key leaves the canvas, 70 -> 68 parts, Inspector "Empty slot"; re-check -> 70 parts;
Undo of the re-check -> unchecked/68 parts. Reopen persistence and Redo were not exercised.
Still missing: Key Assembly select, Attached components (replace/remove, mirror-target override), ownership card/breadcrumb, Wide/Tall pressed state.
