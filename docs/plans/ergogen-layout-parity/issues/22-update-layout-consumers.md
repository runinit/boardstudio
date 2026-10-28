# 22: Adapt setup, alignment, selection and geometry consumers

Status: resolved
Type: build
Blocked by: [21](21-exception-lifecycle.md)
Parent: [10](10-slice-the-build.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)
Ownership: Owns only the consumer wiring (Setup, Align, selection) that calls the Core anchor-align edit; the Core edit itself is left to [19](19-anchor-layout-groups.md).

## Outcome

Finish semantic consumer adaptation beyond the mechanical field cutover: one layout
kernel supplies positions/frames, pitch supplies defaults/size/snap, and several
matrices do not create a false shared transform context.

## First RED / GREEN slice

Fail a real Column Align on a bent column that requires rigid translation of the
selected targets. Route it through Core's exact column edit instead of client column
offsets; then add an anchored/Follow refusal case with unchanged document/history.

## Subsequent acceptance

- Setup/resize adds default layout records; pitch changes only unpinned steps. Keep
  snap units at selected pitch × snap fraction and size-based split gap calculations.
- Matrix Align uses the actual frame, Column Align uses the shared exact Core edit,
  Key/Component Align retain their own pose routes and Row Align stays unavailable.
  Exact operations must not silently break Local pose ownership or drop directions.
- Drafts, pair previews, keycap fit/size, plate openings and actual canvas cells use
  resolved centres but pitch-minus-gap sizes; nonuniform steps never resize keycaps.
- Adapt tree/Objects/context/selection for all Layout matrices and anchors. Keep flat
  evaluation-order listing and Follows detail; multi-matrix selections get frame-free
  actions only and no fabricated common parameter values/axes.
- Verify outlines/collision, CAD/PCB geometry, companions, electrical scan/pin ordering
  and keymap identity consume final member poses/identities, not a recreated grid.
  Do not refactor unrelated providers or normalize hardware data.

## Checks

Current sources include `web/crates/catalogue/src/matrix_setup_operation.rs`,
`web/crates/layout/src/objects/layout_align_controller.rs`, `web/crates/layout/src/part_placement.rs`,
UI-model tree/selection and keycaps/pair geometry consumers. Run focused native
consumer/Core contracts and `check.py quick --files <changed-paths>`, plus affected
browser subsets (`layout keycaps parts catalogue ui-model` as applicable). Report
subset evidence as partial; include native/export invariants actually affected.

## Progress

Implementation lane on branch `docs/ergogen-layout-comparison`, base `32ac3c39c`.
Status remains claimed; no resolve, no push. `web/crates/layout/prototype/`
untouched.

Commits (`git log` SHAs):

- `3ff34b1` claim.
- `50d023f` test-only (RED): component placement snap must honor the selected
  matrix pitch. Failed as required: the 19.05 mm grid answered `(4.7625,-4.7625)`
  on a 20 mm-pitch board expecting `(5.0,-5.0)`.
- `42c5b35` fix (GREEN): `PlacementSnapOptions` gains `pitch: Option<Vec2>`
  (`None` keeps the 19.05 mm default); `grid_snap_point` steps per-axis from the
  selected pitch; the canvas-center call site and both `presentation.rs`
  component-placement routes pass the selected matrix pitch through the existing
  `matrix_snap_parameters` pattern. Existing 19.05 expectations unchanged.
- `0386ca1` Align consumer proofs (GREEN): routing test proves Column submits
  `ColumnTranslation` (never client offsets), Matrix its frame translation, Row
  stays unavailable, keys keep `MoveParts`; rigid end-to-end test bends a key
  then Column-Aligns and proves both targets share one nonzero rigid translation
  with rotations preserved; anchored Follow refusal test proves an unfollowed-X
  Matrix Align fails naming Follow with revision and parts byte-identical.
  Non-vacuity: sabotaging the Column arm to `None` REDs the routing and rigid
  tests while the refusal test stays green; sabotage reverted.

Verification performed so far (all green):

- `wasm-pack test web/crates/layout --lib --
  component_placement_snap_uses_the_selected_matrix_pitch`: 1 passed.
- `wasm-pack test web/crates/layout --lib -- layout_align_controller`: 6 passed,
  0 failed (routing, rigid, refusal + 3 pre-existing).
- `cargo test -p boardstudio_core --lib`: `ok. 808 passed; 0 failed`
  (Core untouched).
- `cargo test -p boardstudio_core --test integration` (`RUST_TEST_THREADS=4`):
  `ok. 815 passed; 0 failed` (matches the ticket-21 baseline).
- `python3 scripts/check.py quick --files
  web/crates/layout/src/part_placement.rs
  web/crates/layout/src/objects/layout_align_controller.rs
  web/src/presentation.rs` (`TMPDIR=/home/chris/.cache/tmp`,
  `NODE_PATH` at the sanctioned web-tools modules, `CHROME_BIN=/usr/bin/chromium`):
  `check: all steps passed: repo, lint, typecheck, test, layout-test-css,
  browser` in 471.9s (repo 1.3s, lint 10.2s, typecheck 3.1s, test 2.9s,
  layout-test-css 31.1s, browser 423.3s). Browser step includes the
  presentation.rs module batch (76 executed, 0 failed) covering the two edited
  component-placement routes, plus the layout-crate batch with the 4 new
  ticket-22 tests. Integrator notes: do NOT edit
`scripts/check-native-tests-baseline.json` here. New names needing reconciliation:
1 `part_placement::tests::component_placement_snap_uses_the_selected_matrix_pitch`
plus 3 `layout_align_controller::resolver_tests::*`
(`align_submits_the_exact_core_edits`,
`column_align_on_a_bent_column_moves_targets_rigidly`,
`anchored_follow_refusal_leaves_the_document_unchanged`). Zero removals or renames.

Consumers verified by reading against the acceptance bullets (no production change
needed; behavior already matches the ticket-08 decisions through tickets 17-21):
setup seeds empty layout records with kernel defaults (`matrix_setup_operation.rs`);
pitch changes move only default steps by construction while explicit records stay;
`preview_cells`, keycap fit/size and canvas cells use resolved centres with
pitch-minus-gap sizes; tree lists Layout matrices flat in scene (evaluation) order
with Follows detail and LayoutGroup/multi-matrix contexts get frame-free actions
only (Align disabled); outlines/collision, CAD/PCB, companions, electrical
scan/pin ordering and keymap bind materialized member poses/identities with no
`row * pitch` grid recreation; Core `set_matrix` truncates layout records on
shrink and the kernel treats absent records as defaults on grow, so resize needs
no record appending.

## Answer

Resolved on branch `docs/ergogen-layout-comparison` over base `32ac3c39c`
(ticket 21 closure). Implementation commits: `3ff34b1` (claim), `50d023f`
test-only RED (selected-pitch snap), `42c5b35` fix GREEN (`PlacementSnapOptions.pitch`,
per-axis snap, both `presentation.rs` component-placement routes), `0386ca1` Align
consumer proofs (routing + rigid + Follow-refusal), `6eb9086` (repo rustfmt),
`c629d457` (consumer proofs and quick-gate evidence, kept claimed). No native
baseline commit was needed: the 4 new tests are WASM-only, so
`scripts/check-native-tests-baseline.json` correctly records 2215 matched,
0 missing, 0 new at `4215f8538` (clean). Receipt:
`.evidence/ergogen-22-integration/native-baseline.json` (3/3 commands, exit 0).

- What delivered: component placement snap follows the selected matrix pitch
  (`None` keeps the 19.05 mm default; per-axis steps; existing 19.05 expectations
  unchanged); Matrix Align submits its frame translation, Column submits
  `ColumnTranslation` (never client offsets), Row stays unavailable, keys keep the
  `MoveParts` route; rigid Column Align on a bent column moves every target by one
  shared nonzero translation with rotations preserved; unfollowed-X Matrix Align
  refuses naming Follow with revision and parts byte-identical. Remaining
  consumers needed no production change (verified by reading): setup seeds empty
  layout records with kernel defaults; pitch changes move only default steps;
  previews/cells/keycaps use resolved centres with pitch-minus-gap sizes and never
  resize keycaps on nonuniform steps; tree lists Layout matrices flat in evaluation
  order with Follows detail and frame-free multi-matrix actions; outlines,
  collision, CAD/PCB, companions, electrical scan/pin ordering and keymap consume
  final member poses/identities.
- Reviews: integrator-performed combined read-only Spec + Standards review of
  `32ac3c39c..c629d457` against the ticket and `CODING_STANDARDS.md`: Standards
  Clean with no blockers (pitch threading reuses `matrix_snap_parameters`; the
  `snap_pitch` default carries a ticket-08 comment; new tests match sibling
  fixture/naming shape; no saved arithmetic is rewritten). Spec PASS for the
  consumer contract — first RED/GREEN slice plus every subsequent acceptance
  bullet is proven by test or by read-through with no behavior change claimed.
  Three non-blocking nits (no ticket change): (1) `grid_snap_point` duplicates
  the per-axis step computation for `pitch.x`/`pitch.y` instead of a shared
  per-axis helper; (2) `presentation.rs` repeats the `matrix_snap_parameters`
  pitch lookup in two component-placement routes instead of sharing one call
  site; (3) the resolver-test bent-column document hand-builds matrices/cells
  instead of reusing a shared layout-test fixture.
- Tests: `cargo test -p boardstudio_core --lib`: `ok. 808 passed; 0 failed`
  (Core untouched). `cargo test -p boardstudio_core --test integration`:
  `ok. 815 passed; 0 failed` (matches the ticket-21 baseline). WASM targeted:
  `part_placement::tests::component_placement_snap_uses_the_selected_matrix_pitch`:
  1 passed; `layout_align_controller` filter: 6 passed, 0 failed (routing, rigid,
  refusal + 3 pre-existing). Fresh capped `quick --files` (3 code paths):
  15/15 commands green (`repo, lint, typecheck, test, layout-test-css, browser`;
  layout WASM 161 passed with `resolver_tests` 5 and `part_placement` 31, page
  76/76). Fresh full: 55/55 commands green, native 2215 passed with 2215
  retained names matched and zero missing/new, layout WASM 161 passed, page suite
  77/77, drag and provider/CAD smokes passed. Receipts:
  `.evidence/ergogen-22-integration/quick.json` and `full.json` (plus `.md`
  summaries).

No release approval is implied.
