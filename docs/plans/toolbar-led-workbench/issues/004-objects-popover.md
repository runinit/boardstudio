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
