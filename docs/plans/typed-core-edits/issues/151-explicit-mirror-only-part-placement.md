# 151: Provide explicit mirror-only Part placement before replacement pin resolution

Status: claimed
Type: build
Blocked by: [111](111-integrate-mirrored-half-reconciliation.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Related: [109b](109b-mirror-only-replacement-parts-auto-pins.md)

## Source-backed gap

109b requires replacement placement to leave the source half unchanged. Ordinary mirror-side
`AddPart` is deliberately inverse-mapped into a source addition and a linked correspondence.
It has no explicit mirror-only intent. The current UI placement producer builds ReplaceDocument;
110 reserves that operation for source-authoritative replacement/import/recovery, so a generic
document diff must not be interpreted as mirror-only placement intent.

## Reviewable proposal

Add a deliberate “Only this half” placement choice backed by a dedicated
`AddMirrorOnlyPart { part, boardId, definition?, layoutId? }` operation. Require the exact native
mirror board, preserve ordinary AddPart semantics, and reuse private placement/membership logic.
Optional definition/layout data let the existing placement producer submit one typed atomic
action. Refuse missing/ambiguous/non-mirror targets, conflicting identities/definitions, or an
invalid target layout; never fall back to linked placement.

A mirror-only Part has no source Part, correspondence or Local exception. Prove it survives
subsequent reconciliation, Save/Open and history, and keep the source data unchanged. Do not
infer intent from a previous Omit. 109b then reuses electrical resolution/materialization only on
the mirror draft, after reconciliation, with existing linked pin locks. No new resolver or
visibility widening is proposed.

Alternative: extend AddPart with an optional mirror-only intent whose absent/default value
preserves linked behavior. Both alternatives require explicit user intent in the placement flow;
the dedicated operation keeps the two ownership contracts distinct.

## Decision and implementation boundary

The owner approved the dedicated `AddMirrorOnlyPart` command and explicit “Only this half”
placement choice on 2026-10-13. The complete comparison, suggested Rust/wire shape and affected-path
inventory are in main-checkout ignored evidence `.evidence/ticket109b/mirror-only-placement-proposal.md`.
After 111 integration, root assigns a bounded implementation lease and required checks/reviews. Do not add this family
inside 111's activation or alter its default AddPart behavior. 109b waits on this preparation;
111 can continue independently.

## Accepted operation shape

`AddMirrorOnlyPart { part: Part, boardId: String, definition?: PartDefinition, layoutId?: String }`.
A supplied definition is added when absent, reused when identical, and refused on a conflicting
ID. A selected layout must belong to the named mirror board. The operation registers the new
Part on that board and its appropriate existing outline/layout memberships atomically. It
creates no source Part or correspondence/exception. All placement and pin changes use one
prepared draft, Preview and history route. Keep existing AddPart behavior; the earlier presence
of Omit does not change its meaning. Private helpers and the resolver stay private.

## Acceptance

- [ ] Dedicated command creates exactly one mirror-board Part, with atomic definition, board,
      envelope and optional layout registration; no source Part, correspondence or exception.
- [ ] Ordinary AddPart's linked/default semantics remain unchanged. Only a unique native mirror
      board is accepted; invalid targets/IDs/definitions/layouts refuse without publication.
- [ ] Subsequent edits, serde/Save/Open, Preview and one-step Undo/Redo preserve the unassociated
      mirror-only Part and leave source-half data unchanged.
- [ ] Explicit “Only this half” placement choice emits the typed command; a prior Omit never
      changes ordinary placement intent. Follow existing product/placement conventions.
- [ ] Independent Standards/Spec review and meaningful mutation probes cover accidental source
      insertion, missing membership, wrong target and preview/refusal publication. Required
      native/WASM/browser checks pass; root owns baseline/golden/integration gates.

This ticket provides explicit placement, not a second electrical assignment path. 109b consumes
the accepted route to perform its existing automatic-pin contract in the same transaction.

## Implementation scope after 111 (2026-10-13)

111 is resolved on local dev `d5028ff3`. The approved ordinary linked default is bilateral:
one source Part, one mirror Part and one correspondence for placement from either paired board.
Readiness found two gaps: catalogue AddObject emits ReplaceDocument, and source-side AddPart
does not create the forward addition plan. Closing these gaps fulfills the approved default;
preserving the existing missing linkage would not satisfy this ticket.

Extend existing AddPart only with optional `definition` and `layoutId` metadata, so catalogue
definition/Part/layout placement remains one atomic operation. This is not a mirror-only intent
flag. Preserve old JSON and unpaired behavior. Reuse private PartAddition and
IndependentPartTransition::Add for both linked directions, and the existing prepared
Preview/publication/history route for the dedicated mirror-only operation.

Selected linked mirror Layouts map to their exact source counterpart. Local or unmapped mirror
Layouts retain target-owned membership while the Part itself stays linked; source-side selected
Layout ownership is preserved too. Refuse stale, Omitted, duplicate, foreign or inconsistent
ownership. Register existing board PartEnvelope memberships on both halves for linked additions
and only on the named mirror for mirror-only additions. Later projection/Open keeps eligible
uniquely mirror-owned independent unassociated/Local members in stable order, excluding stale,
foreign, duplicate, generated and hosted ownership. Reuse existing ownership checks.

Root assigns Core source paths `core/src/model.rs`, `core/src/lib.rs`,
`core/src/mirrored_half/inverse.rs`, `inverse/hosted.rs`, `inverse/layout.rs`,
`reconcile.rs`, `reconcile/prepare.rs` and `reconcile/outlines.rs`, with their established
co-located inverse/preparation/outline tests. Public tests belong in `core/tests/core.rs`
and a bounded `core/tests/mirrored_half/placement.rs` module registered in its parent file;
`core/tests/generator_parameters.rs` may receive only required AddPart constructor fields.
Any additional exhaustive-match/compiler inventory path needs root assignment before editing.

Root separately assigns UI paths `web/crates/ui-model/src/state.rs`,
`web/crates/parts/src/parts.rs`, `web/crates/layout/src/objects.rs` and
`web/crates/layout/src/part_placement.rs`, including their established tests. Keep the default
catalogue click linked and expose a distinct one-shot “Only this half” action only for an
exact unique native mirror board. Preserve controller, inspector, apply-to-key, imported-source
and unpaired producers. Both actual paired catalogue producers must emit their typed operation.

No helper visibility widening, electrical/resolver change, generic Batch, format migration,
second transaction engine or unrelated cleanup is assigned. Core/UI changes require stable
handoffs, independent review axes, meaningful compiled mutation failures, public Core and
actual producer/mounted coverage, Save/Open/history proof and root-owned full gates before
integration. 109b and fresh parent 19 acceptance remain separate required work.

Readiness/design evidence is in main `.evidence/ticket151/`: `root-scope-notes.md`,
`default-linked-placement-design-579bc83c.md`, `design-review-before-implementation.md`,
`implementation-core-readiness-d502.md` and `implementation-ui-readiness-d502.md`.
