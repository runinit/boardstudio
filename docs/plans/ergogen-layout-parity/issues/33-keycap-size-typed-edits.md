# 33: Keycap size through typed edits

Status: ready-for-agent
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
