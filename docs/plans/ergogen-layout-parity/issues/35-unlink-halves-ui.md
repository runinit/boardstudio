# 35: Unlink halves in the UI

Status: claimed
Type: build
Blocked by: None (all dependencies resolved; linked-half reflection and the exception lifecycle already exist)
Parent: [map.md](../map.md)
Map: [map.md](../map.md)

## Goal

Let the operator unlink a linked pair of halves from the UI with a
confirmation step, so unlinking ends the correspondence cleanly, leaves both
halves independently valid, and undoes as one entry.

## First RED / GREEN slice

Fail a mounted test: unlinking a linked pair through the confirmed UI action
leaves two valid independent halves with no live correspondence, and a
single Undo restores the linked pair with poses unchanged.

## Subsequent acceptance

- [ ] The unlink action asks for confirmation naming the consequence, then
      writes one edit with one Undo entry.
- [ ] After unlink, no orphaned correspondences remain: neither half follows
      the other and neither half references the former pair.
- [ ] Both halves stay valid after unlink: dimensions, layout membership and
      anchors hold, or the action refuses atomically with a reason.
- [ ] Relations and the canvas stop showing the pair relationship once
      unlinked and restore it on Undo.

## Checks

Mounted browser tests for confirmed unlink, no-orphan proof, and Undo
round-trip; affected quick/browser checks.

## Progress

- New test (no baseline edits):
  `mounted_unlink_halves_confirms_then_clears_link_with_one_undo` in
  `matrix_inspector_controller.rs::tests::queued_edits`.
- Proving test: 1/1 new unlink test passes; arm writes nothing, Cancel writes
  nothing, Confirm writes one `SetLayout` clearing the mirror link, Undo
  restores the link with poses unchanged.
- Counts: targeted 1/1 passes. Full layout browser suite not rerun here;
  final full quick across all five tickets will cover it.
- UI: `MatrixInspector` now arms an explicit confirmation naming the
  consequence (ends mirrored correspondence, both halves keep poses, one Undo
  entry) with Confirm/Cancel; `unlink_layout_resolver` unchanged (one
  `SetLayout` edit, one Undo entry).
- Quick: `cargo fmt --all --check` clean for both changed files; workspace
  `repo` doc-link and unrelated rustfmt drift fail on base too (pre-existing).
