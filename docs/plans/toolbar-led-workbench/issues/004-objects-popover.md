# 004: Layout Objects with active-outline and half-focus parity

Status: ready-for-agent
Type: build
Blocked by: [002](002-toolbar-accordion-label-fitting.md)
Parent: [map.md](../map.md)
Owner: Layout Objects projection/disclosure and existing tree/selection adapters

## Boundary

Project existing `objects.rs` / `ui-model/tree.rs` contexts into 009's searchable compact
popover: active Outline first, named halves/matrices, columns via explicit disclosure,
components with truthful anchor references. Preserve By layout/By part, categories, board
navigation, disclosure seeds/created-pair reveal and Replace/Add/Range/Toggle intentions.
Reuse TreeSelectRequest/ContextRequest, not a second object/selection model.

Every pick closes Objects (Replace/Add/Range/Toggle), never opens Inspect and does not
activate a different outline. Modifiers preserve their selection operation, not a new
keep-open behaviour; changing that interaction needs a separate user decision.
Canvas-boundary selection yields the same active-outline context; existing Outline and
OutlineVersion contexts remain valid where needed. Version lifecycle/bridges stay in 003.
Half selection aligns view-only focus with actual selected half; saved links/correspondences
are authoritative. No source swap, unlink/mirror action or document/history change. Anchor
labels reuse canvas association data (“Placed from Left main · R1/C6”), not screen side or
clearance. Keep unanchored/companion/local exceptions truthful.

Closed popover is inert; dismissal/focus/no reframe use 009. Preserve keyboard nudge on the
focused canvas selection after picking. Creation/overflow moves through 005; retain old
entry reachability until that destination lands, rather than deleting it in this ticket.

## Acceptance criteria

- [ ] Every Layout Objects inventory row has a reachable destination; no full dock/rail.
- [ ] Actual names/link states retained; Outline selection and activation remain separate.
- [ ] Search/filter, grouping and disclosure preserve accessible names and current selection.
- [ ] Component references match canvas data for anchored/detached/companion/local/mirrored fixtures.

## Proof map

- Outline parity — board with Generated plus active refinement/edited version → choose first
  Objects item, then pick boundary on canvas → same active-outline selection, version/shape/
  outputs unchanged; Objects closed and Inspect not opened · mounted · planned
- Reference truth — known anchor Left main row 1 column 6 plus detached/companion variants →
  filter/inspect rows → exact association matches canvas readout, no fabricated side/clearance · mounted · planned
- Half focus — linked pair with renamed/repositioned halves → pick each via Objects/canvas →
  focus follows identity, partner/source/correspondences and accepted revision/Undo unchanged · mounted · planned
- Navigation retention — collapsed named matrix with column/key, By part and another board →
  search/disclose/group/pick with existing modifier intentions, reopen → correct scoped
  selection and retained disclosure; nudge focused selected component on canvas still works · mounted · planned

## Checks and mutations

[Verification](../verification.md): `check.py quick`, `check.py lint`, layout browser subset
and focused `layout_split_focus_tests.rs` / anchor page files when changed. Native tree
coverage reused where sufficient, mounted tests cover pick/close wiring. Active-version,
link-source and reference-label proof-gap mutations must fail the named independent oracles.

## Progress 2026-11-11 (builder; no closure claim)

All four proof rows are GREEN at the mounted tier on the final tree
(`.evidence/toolbar-m1/004-final.log`: `layout_layers_tests` +
`layout_split_focus_tests`, 18 executed, 0 failed; RED first failures in
`004-red3.log`). Production changes: Objects rows no longer attach an
`OutlineAction::Activate` (version rows select only; lifecycle stays in
Inspect), and clicking the canvas board boundary (stroke-only hit area,
`data-board-boundary`) selects the active Outline context; Objects gained
the host-expected `Find an object` search field (label/detail filter, current
selection stays visible) and Replace/Add/Range/Toggle modifier modes on
click and keyboard picks. Component rows (By layout and By part) now carry
`tree::part_association_label` — `Placed from <matrix> · R<r>/C<c>` from the
same anchor record as the canvas readout, `Detached · <layout> origin`, or
`Local · own origin`; companion rows name their cell (`Automatic companion ·
R1/C1`); no pose, screen side or clearance is shown. Split-half identity is
decided by the saved mirror link alone (`split_focus::layout_half`: link
carrier = mirrored half, `source_id` target = source half) in `context_half`,
tree half grouping and badge halves, and `select_tree` now aligns the
view-only split focus on Objects picks as canvas picks already did. The old
ticket-14 expectation that a tree pick never moves focus is superseded by
this ticket for Objects picks and its test was updated accordingly. One
pre-existing commands test now asserts command-opened Objects focuses the
new search field instead of staying on the utility.

Checks: `cargo fmt --all --check` and `python3 scripts/check.py lint` pass
(`004-fmt-final.log`, `004-lint2.log`, under the build lock); native
`boardstudio-web-ui-model` 38/38 (`004-native-final.log`); layout-crate
`tree_projection` browser tests 2/2 (`004-regress-treeproj2.log`); anchor
badge page tests 5/5 (`004-regress-anchor.log`); native layout `outline`
tests 20/20. No full quick gate, mutations or review were run (per builder
instructions); the proof-gap mutations named above remain for the
coordinator. Limits/follow-ups: (1) returning to the first board via an
Objects board row from the empty second board did not navigate in the
mounted harness after three attempts (forward navigation and re-scoped tree
are proven; suspected stale routing generation in coordinator-owned
routing, logs `004-green5.log`–`004-green9.log`); (2) the mirrored matrix is
rebuilt by the mirror reconciler on open, so the companion fixture lives in
the source matrix (diagnostic `004-diag13.log`); (3) creation/overflow entry
reachability is unchanged, as 005 owns its move. This ticket is not closed.
