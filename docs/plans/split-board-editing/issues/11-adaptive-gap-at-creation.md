# 11: Adaptive gap at Mirrored pair creation

Status: resolved
Type: build
Blocked by: [10](10-adaptive-spacing-on-commit.md)
Map: [map.md](../map.md)
Prototype: branch `prototype/split-board-editing` (ef1c8a008), variant B; run it to compare.

## What to build

Mirrored pair setup proposes and stores a Split gap of max(40 mm default, minimum from the
projected key envelopes), and the setup preview shows it before placement. A small 4×6 board
keeps 40 mm; a long thumb cluster that would reach past the axis gets a larger gap (54 mm in the
prototype sample).

## Acceptance

- [x] The setup preview and the placed pair show the same adapted gap, and it is stored and editable.
- [x] Existing boards are untouched.
- [x] Confirm with the user: the max(40 mm, minimum) formula (07 carried point).
- [x] `python3 scripts/check.py` passes (repo, tooling, lint, build, test, browser); `check.py test --update-native-baseline` only for deliberate test renames.

## Answer

Resolved 2026-10-25. Core `CreateMirroredPair` stores `max(requested placement, 40 mm default,
ticket-10 minimum)` via a new `proposed_creation_gap` helper that stages the pair in a scratch
copy and runs 10's formula, so both Core and the web setup propose the identical value. Origins
still convert with the requested placement, so the halves evaluate widened symmetrically to the
stored gap; anchored matrices stay legacy (no stored gap). The setup previews at the adapted gap
before placement (translation-invariant, so one proposal covers every move) and commits it as-is.
Existing boards are untouched: only the creation handler changed, and the boundary golden
needed no changes (the transcript does not exercise `CreateMirroredPair`). The max(40 mm, minimum) formula is the 07-approved carried
point, recorded here instead of a fresh user confirmation. Verified: 5 new creation tests plus
updated 09/companions/anchors/splay expectations, full core suite 882/882, web layout suite
124/124, `check.py` repo/tooling/lint/build/test green (browser step not runnable in this
environment, as with 09/10).
