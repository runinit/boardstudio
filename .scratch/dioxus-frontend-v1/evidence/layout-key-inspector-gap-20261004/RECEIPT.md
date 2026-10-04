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
