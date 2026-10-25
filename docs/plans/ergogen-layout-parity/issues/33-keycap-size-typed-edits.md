# 33: Keycap size through typed edits

Status: claimed
Type: build
Blocked by: None (all dependencies resolved; the Inspector keycap surface and the typed-edit Core contract already exist)
Parent: [map.md](../map.md)
Map: [map.md](../map.md)

## Goal

Make the Inspector keycap size control submit the operator's typed value as a
single named typed keycap edit, so a keycap resize commits exactly what was
typed, validates before writing, and undoes and redoes as one entry.

## First RED / GREEN slice

Fail a mounted test: typing a valid keycap size in the Inspector and
committing resizes the selected key's keycap, and a single Undo restores the
prior keycap size with nothing else changed.

## Subsequent acceptance

- [ ] Committing a valid typed keycap size writes one typed edit with one
      Undo entry; the keycap and plate opening follow the tied size rule.
- [ ] Committing an invalid keycap size is refused with an inline message and
      writes nothing; history and the saved document are untouched.
- [ ] One Undo restores the prior keycap size exactly; Redo restores the
      committed size exactly.
- [ ] The control never submits a whole-document replacement for a keycap
      resize; history shows the single named typed entry.

## Checks

Mounted browser tests for the typed keycap commit, refusal, and Undo/Redo
round-trip; affected quick/browser checks.

## Progress

- Investigated 2026-10-25: the Layout key-size surface (`keycap_size_controller.rs`
  `resize_resolver` + `keycap_resize.rs::plan_resize`) submits
  `EditOperation::ReplaceDocument` for every keycap resize. No Core typed
  variant writes `Part.keycap` size: `SetKeycapBoard/Matrix/Key` cover keycap
  *settings* (`KeycapKeyChange::Units` etc. in `core/src/keycaps/edits.rs`),
  not the physical `Part.keycap: Option<Vec2>` size tied to pitch minus edge
  gap. `grep` for `part.keycap =` shows only `ReplaceDocument`, matrix
  creation, and mirrored-half lifecycle writers; no typed size edit exists.
- Ticket requires a new named typed keycap-size Core edit (model.rs type
  change) to replace the whole-document path. Per task rules (no model.rs
  type changes unless a ticket requires it — if so, stop and report), stopping
  implementation here and reporting to parent. No code changed; ticket stays
  claimed for parent review.
