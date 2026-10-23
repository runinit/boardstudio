# 18: Implement named placement, inverse moves and exact origin compensation

Status: claimed
Type: build
Blocked by: [17a](17a-grouped-model-and-core-callers.md), [17c](17c-kernel-production-routing.md) (parent: [17](17-cut-over-format-and-model.md))
Parent: [10](10-slice-the-build.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)

## Outcome

Extend existing typed placement/splay seams for Spread, Padding, row/key Shift/Bend,
Column rotation, independent offsets and origin hold-still/swing. Core owns inverses,
exactness and changed IDs; callers do not rebuild matrices with their own geometry.

## First RED / GREEN slice

Fail a public prepared edit that moves a key in a turned/curved column by its requested
world delta without moving its later keys. Implement the offset-frame inverse; then
add one carried Shift/Bend case and its preview/cancel/Undo behavior.

## Subsequent acceptance

- Field, gesture and nudge inputs resolve against accepted state. Preserve companion
  carry/constraint precedence and independent key identity, changed IDs and no-op rules.
- Column rotation affects only its column; switching Splay scope changes no value.
- Origin hold-still uses this/next step compensation, with matrix Origin for column 0;
  cover Column rotation, previous Splay, Bend, mirrors and nonzero custom origins.
- Inverse-solve a dragged/Picked final origin after compensation, rather than merely
  transforming its initial pointer delta. Fields/nudges hold; Ctrl swing writes origin
  only and is locked at pointer-down; Reset holds and Ctrl base snap swings.
- Implement reusable Core rigid-column translation for Align, with the correct next
  step compensation under Splay, not column/per-key offset scattering under Bend.
- Refuse nonrepresentable exact requests without changing state. Keep Local ownership
  promises; anchor/linked paths in 19–21 extend the same implementation, not another
  inverse. Preserve explicit Spread/pitch behavior and one operation/Undo per edit.
- Keep Y-only Convert exact eligibility: no Splay, Column rotation or carried Bend;
  transfer to this Stagger, subtract next, clear offset. Core owns the blocked reason.

## Checks

Extend `core/src/matrix/{placement,splay,layout}.rs` and existing placement, companion,
splay reconciliation and part-offset tests through public Core/Session routes. Run
focused `boardstudio_core` tests, then quick checks. Private numeric tests supplement,
never replace, prepared-route preview/commit/refusal/Undo evidence.

## Progress

### Parallel implementation claim — 2026-10-23

Claimed for an isolated implementation lane. Acceptance, TDD, review and gates are
unchanged; integration owns shared baselines, test aggregation, map and closure.

### Isolated implementation progress — 2026-10-23

Worktree `t3-d32e0528`, branch `feat/core-placement-edits`, claim base
`964762b519d9c57adf9cab3dda2683c2560fda08`. Status remains claimed; parent owns
independent review, shared baselines, integration and closure.

- Added named Spread/Padding and row/key Shift/Bend intents to the existing placement
  seam, with finite/bounds checks and no-op preservation. Independent key world moves
  already used the kernel inverse; the first regression found excessive changed IDs,
  not an incorrect inverse. Reports now retain only moved members and the matrix ID.
- Added relative-origin hold and world-origin swing intents. Hold/Pick solves through
  the published fully turned column frame, so a representable partial-Follow request
  is no longer refused by unrepresentable unit probes. Compensation retains the agreed
  this/next step arithmetic; swing writes only the relative origin. Column 0 retains
  its mirror-aware incoming X axis in the kernel, without adding a Spread handle.
  Unchanged placement and splay requests preserve authored records, revision and history.
- Exact rigid translation refuses Local or constraint-owned keys before writing,
  including corresponding linked matrices. Existing column translation passes curved,
  splayed, turned and X/Y-mirrored cases without scattering independent offsets.
- Real Core accepted scene/document, native prepared pair and Session
  preview/cancel/persist/commit/Undo/Redo coverage lives in existing test modules.
  Focused native checks passed (27 placement, 14 kernel, 11 splay, 9 companions, 20 part-offset,
  one native prepared pair, one Session test). No shared aggregator or fixture/golden
  changed. New native test names need parent baseline reconciliation; no new WASM tests.

Local plan, RED/GREEN logs, exact commands and final gate status are recorded under
`.evidence/ergogen-18/` in the assigned worktree. Follow-on UI/lifecycle tickets and
parent merged gates remain outside this implementation.

Changed-file quick stopped at the retained native boundary golden:
`transcript.core[4:preview].bodyDigest` is `6579:bd874ef70034195d`, expected
`6684:6191b7cfa510ee5e`. The 105-byte reduction matches removing five unchanged
member IDs from the key-move preview. The fixture is unchanged for parent review;
no blessing or baseline-update flags were used. Repo checks, native/WASM Clippy
and page typecheck passed; the receipt records 2,008 native passes, one golden
failure, zero missing retained names and 15 new names within the executed scope.
The CSS/browser phase did not run. See `.evidence/ergogen-18/quick.json` and its
summary. Production source stayed unchanged after that run; the native prepared
test then gained a mirror-side Swing assertion with separate focused verification.
