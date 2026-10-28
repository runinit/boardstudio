# 16: Build the pure carried-layout kernel and fixture comparison

Status: resolved
Type: build
Blocked by: [15](15-generate-parity-fixtures.md)
Parent: [10](10-slice-the-build.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)

## Outcome

Implement the single pure geometry module, proposed at `core/src/matrix/kernel.rs`.
Stage layout value types/evaluation without changing persisted v6 documents or making
the application claim the new behavior. Reuse the eventual grouped record types;
keep staging input/adaptation private, not a second public document model or engine.

## First RED / GREEN slice

Through the agreed kernel seam, fail one literal default-step pose test and pass it.
Then add Shift followed by Bend versus offset-after-Bend as the next vertical case.
Feed committed fixtures through that same interface rather than private math helpers.

## Subsequent acceptance

- Implement the spec's column prefix/Splay versus own Column rotation, row Padding,
  summed row/key Shift then Bend, independent offsets/Key rotation and final mirror.
- Implement candidate Origin/Rotation then Follow, including the transported basis;
  expose unambiguous raw-anchor context versus composed-frame types/contract.
- Return exact shift/offset frames, column base/origin/rotation/axes and typed numeric
  contribution facts. Defaults, finite-value checks and no Spread/Padding at axis 0
  obey the agreed shape. Logical disabled/deleted values still carry.
- Compare all relevant upstream/BoardStudio fixtures at the agreed tolerances, including
  nonzero pivots, curved columns, X/Y mirror, key turns and independent offsets.
- Check cumulative Shift in matrix axes against a worked example, not raw vector sums.
  Do not include independent rotations in cumulative Bend.

## Scope and checks

No live schema activation or replacement of production edits in this slice; staging
success is not prepared-route support. Existing seams to replace later are
`core/src/matrix.rs` and `core/src/matrix/splay.rs`. Use focused
`cargo test -p boardstudio_core <new-kernel-test-filter>` after adding the named test,
then repository quick checks for the changed files. Register new integration coverage
in the existing Core integration harness (`autotests = false`) when applicable.

## Comments

### 2026-10-23 — staged kernel implementation

- Added `matrix::kernel`, exposed as `boardstudio_core::layout_kernel`, with one pure
  `evaluate` interface. Raw anchor-key context and evaluated world frames are distinct
  types; fixture/document adaptation and anchor lookup remain private to the integration
  test. Grouped geometry records do not activate a persisted schema or production route.
- Published column base means the base carried through preceding Splays, **before**
  this column's own Splay/Column rotation. The separate starting frame includes both
  turns. Spread/Stagger axes use the exact incoming-prefix translation and are checked
  against finite step differences under nonzero origins, Bend, rotation and X/Y mirror.
- RED/GREEN evidence covers the missing default seam, Shift-before-Bend versus
  offset-after-Bend, committed fixture poses, frame/contribution publication, axis-zero
  validation and overflow in published frames despite finite final key poses.
- Focused integration coverage passes 11 tests, including all 40 upstream and four
  BoardStudio-only cases: 381 numeric poses plus three absent-output/logical-pose
  checks. The Rust fixture consumer asserts version 1, the full pinned source commit
  and exact tolerances, using `1e-6` upstream and `1e-9` for worked/divergent poses.
  Evidence: `.evidence/ergogen-16/13-final-kernel-green.log` and numbered RED/GREEN logs.
- Changed-file `scripts/check.py quick` passed repository checks, native/WASM Clippy,
  typecheck, native tests, layout CSS and affected WASM browser suites: 1,959 native
  tests passed (nine existing ignored), 587 WASM tests passed, zero failures. Receipt:
  `.evidence/ergogen-16/quick-receipt.json` and `.json.md`; full log:
  `.evidence/ergogen-16/quick-final.log`. The earlier quick attempt timed out during
  Clippy and is not counted as passing. This is changed-file verification, not the
  full release/build, page-suite or product-smoke gate.
- Ticket stays claimed for review. Live schema/format activation, prepared-edit routing,
  graph/lifecycle ownership and replacing production geometry remain later tickets;
  staging success does not claim that support. Fixtures and the untracked prototype
  were not modified; no Node/npm tooling was run.

## Answer

Implemented as the staged pure kernel `core/src/matrix/kernel.rs`, not activated. Commits: `c121fb255` (kernel and 11 focused tests, with 381 fixture poses over 40 upstream and 4 BoardStudio-only cases at tolerances `1e-6` and `1e-9`); `8226b00d3` (single-test command documented); `be9714b85` (Standards fixes: shared rotate helper, fixture parsing moved test-side, test naming). Spec review passed on `c121fb255`; Standards re-review on `be9714b85` was clean. The quick check on `be9714b85` passed all steps (repo, lint, typecheck, test, layout-test-css, browser).

Carried forward: the geometry adaptation discards enabled, deleted, and omitted flags, so it proves supplied logical values carry but not production state selection. Tickets 17 and 22 must cover production selection. The no-live-activation boundary is shown by the diff, not by a regression test.
