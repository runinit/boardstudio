# 151: Provide explicit mirror-only Part placement before replacement pin resolution

Status: resolved
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

- [x] Dedicated command creates exactly one mirror-board Part, with atomic definition, board,
      envelope and optional layout registration; no source Part, correspondence or exception.
- [x] Ordinary AddPart's linked/default semantics remain unchanged. Only a unique native mirror
      board is accepted; invalid targets/IDs/definitions/layouts refuse without publication.
- [x] Subsequent edits, serde/Save/Open, Preview and one-step Undo/Redo preserve the unassociated
      mirror-only Part and leave source-half data unchanged.
- [x] Explicit “Only this half” placement choice emits the typed command; a prior Omit never
      changes ordinary placement intent. Follow existing product/placement conventions.
- [x] Independent Standards/Spec review and meaningful mutation probes cover accidental source
      insertion, missing membership, wrong target and preview/refusal publication. Required
      native/WASM/browser checks pass; root owns baseline/golden/integration gates.

This ticket provides explicit placement, not a second electrical assignment path. 109b consumes
the accepted route to perform its existing automatic-pin contract in the same transaction.

## Answer

2026-10-13.

Resolved and integrated locally in `773aee9ad`. Ordinary catalogue placement now creates one
source Part, one mirror Part and one correspondence from either paired board. AddPart accepts
optional atomic definition/layout metadata while preserving legacy payloads and unpaired behavior.
The dedicated AddMirrorOnlyPart command and explicit “Only this half” action create only the
named mirror-board Part, with no source Part, correspondence or exception.

Linked layouts map to their exact counterpart; Local/unmapped target layout membership remains
target-owned. Existing board/envelope membership is registered atomically and eligible independent
mirror envelope members survive later projection/Open. Invalid targets, identities, definitions
and ownership refuse before publication. Placement uses the activated prepared Preview/history
route, including matrixless pairs, archive Save/Open and one-step Undo/Redo.

Independent Standards and Spec reviews passed, including identity, golden, native inventory and
full-request assertion follow-ups. Eight unique compiled mutation faults were detected, with no
survivors or compile failures and exact restoration; five also received public integration
cross-checks. Full native verification recorded 1,924 executed tests; repository/tooling/lint/type
checks, full build and browser checks passed. The explicit real-WASM export replay reproduced
the native transcript, including placement and archive pack/unpack/Open. The official baseline
adds20 names and replaces one old source-add test; its full authored-operation assertion was
restored and passed. Post-gate source changes were confined to that reviewed test assertion.

Detailed requirement evidence and exact source hashes are in ignored
`.evidence/ticket151/integration/acceptance-matrix.md` and `acceptance-current.json`.
This resolves placement only. 109b automatic pin assignment and fresh parent19 acceptance,
including performance, remain required. Work is unpushed.
