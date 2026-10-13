# 151: Provide explicit mirror-only Part placement before replacement pin resolution

Status: ready-for-human
Type: design
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

Owner operation-shape/placement-choice decision pending. The complete comparison, suggested
Rust/wire shape and affected-path inventory are in main-checkout ignored evidence
`.evidence/ticket109b/mirror-only-placement-proposal.md`. After the decision and 111 integration,
root assigns a bounded implementation lease and required checks/reviews. Do not add this family
inside 111's activation or alter its default AddPart behavior. 109b waits on this preparation;
111 can continue independently.
