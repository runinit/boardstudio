# Ticket 115 research: prepared constraint resolver seam

## Question and decision

Should ticket 115 expose `constraints::resolve_prepared` as `pub(crate)`, or can it
compose through an existing narrower seam?

**Decision:** the owner approved making only `resolve_prepared` crate-visible for ticket 115 on
2026-10-12. This is the smallest existing seam that supports the accepted one-graph solver design.
It does not expose a public Rust or WASM interface because `constraints` itself is a private module
in `lib.rs`. Keep the solver implementation and framed model crate-private/private as they are; no
other visibility change is authorized.

## Source facts

- Ticket 115 requires the canonical operation to reach `apply` and both resolution passes
  with framed constraints in play. Its inherited section names `resolve_prepared` as the resolver
  and requires `pub(crate)` visibility, while the original scope and file-ownership sections
  prohibited visibility widening. The owner resolved that conflict by approving this one function
  as the only exception. ([ticket 115, scope and ownership](../issues/115-prepare-mirrored-edit-transaction.md#scope-and-evidence),
  [inherited requirement and owner decision](../issues/115-prepare-mirrored-edit-transaction.md#owner-decision-2026-10-12))
- In `constraints.rs`, `resolve_edit` is `pub(crate)`, extracts positions from the
  operation, then calls `resolve_with_positions`; that helper always passes `&[]` as the
  framed records. Thus the existing callable seam cannot carry the private prepared
  records. `resolve_prepared` accepts the document, positions, and framed records, but is
  private. ([`constraints.rs`](../../../../core/src/constraints.rs#L135),
  [`resolve_prepared`](../../../../core/src/constraints.rs#L220))
- A repository search finds one `resolve_edit` definition and exactly four calls: two
  consecutive passes in committed `CoreEngine::edit` and two in `preview_edit`. The edit
  calls are at `lib.rs:544` and `:552`; the preview calls are at `:651` and `:656`.
  Ticket 115 makes production call sites read-only, and assigns History/Open/preview work
  to 116/117. ([`lib.rs`, committed edit](../../../../core/src/lib.rs#L504),
  [`lib.rs`, preview](../../../../core/src/lib.rs#L644),
  [ticket 115 ownership](../issues/115-prepare-mirrored-edit-transaction.md#file-ownership))
- The prepared solver combines stored and framed constraints in one source-first graph,
  includes assembly placements, and writes resolved poses only after graph resolution.
  This is the shared behavior that 139's design says to retain. ([`constraints.rs`](../../../../core/src/constraints.rs#L220),
  [139 accepted design: resolver seam](139-generalized-mirrored-constraint-model.md#source-grounded-seams-and-impact))
- The compiler record `FramedConstraint` is already re-exported `pub(crate)`, while
  `lib.rs` declares `constraints` as `mod constraints;`. Making one function
  `pub(crate)` would permit sibling crate modules to call it without making the module or
  function externally public. ([`constraints.rs`](../../../../core/src/constraints.rs#L10),
  [`constraints.rs` crate-private records](../../../../core/src/constraints.rs#L16),
  [`lib.rs`](../../../../core/src/lib.rs#L1))
- Accepted design 139 says the prepared solver is the one engine and that 115 supplies
  the transaction half; its seam inventory also requires the same canonical operation
  to reach apply and both resolver passes. ([139 design: solver](139-generalized-mirrored-constraint-model.md#source-grounded-seams-and-impact),
  [139 design: prepared operation](139-generalized-mirrored-constraint-model.md#compiler-solver-and-canonical-edit-flow))
- In the current production path, `CoreEngine::edit` applies the operation, calls
  `resolve_edit`, synchronizes components, then calls `resolve_edit` again. Preview does
  the same pair of resolver passes. Ticket 115 keeps these production routes read-only;
  its prepared draft remains inert until ticket 111. ([`lib.rs`, edit](../../../../core/src/lib.rs#L504),
  [`lib.rs`, preview](../../../../core/src/lib.rs#L644), [ticket 115](../issues/115-prepare-mirrored-edit-transaction.md#file-ownership))

## Alternatives

| Alternative | Interface, depth, seam, locality | Tradeoff |
|---|---|---|
| Expose the existing `resolve_prepared` as `pub(crate)` | Reuses the solver's existing seam and implementation. The crate-only interface is low-level (`doc`, positions, framed records), but the behavior behind it is deep: compile/evaluate the combined graph and reconcile solved key poses. Solver changes stay local. | Smallest change and most direct match for 115/139. Any crate module can call the low-level entry, so the ticket must explicitly carve out this one visibility exception. It is not externally public due to the private parent module. |
| Extend existing `resolve_edit` to accept framed records and forward them to private `resolve_prepared` | No visibility widening: `resolve_edit` is already `pub(crate)`, and `FramedConstraint` is already crate-visible. It keeps the solver private and can preserve one combined graph. Passing the canonical operation remains part of the seam; `resolve_edit` can continue extracting MoveParts positions before forwarding. | Rust has no overloads or default function arguments, so its current four callers must all change to pass `&[]` (or equivalent) when they have no prepared records. Two are committed edit calls; two are preview calls that 115 marks read-only and leaves to 116/117. It also makes the existing production edit resolver interface carry prepared-only state. Therefore this alternative avoids a visibility modifier change but conflicts with ticket 115's callsite/preview ownership and broadens the existing interface's responsibilities. |
| Keep `resolve_prepared` private and add a transaction-specific crate-visible facade inside `constraints` | Gives callers a narrower, deeper interface if the facade accepts one prepared plan and owns when/how solving occurs; keeps solver details local. The `resolve_prepared` function itself remains private. | Still introduces a crate-visible seam, so it does not satisfy a literal “no visibility widening” rule. A thin pass-through facade adds a shallow module without leverage; a useful facade requires moving or reshaping transaction orchestration and coupling it to the constraints module. This is a viable redesign only if 115 explicitly leases that interface. |
| Call existing `resolve_edit` twice and omit prepared framed records | Uses the already crate-visible existing interface. | Not equivalent: the implementation always supplies an empty framed slice, so the two passes cannot evaluate the prepared relations together with stored constraints. This fails 115/139's combined-graph requirement and is not a valid substitute. |

## Decision boundary

The first alternative best preserves **locality** under the current ticket leases: one
solver implementation, one seam, no duplicate graph logic, and the narrowest coherent
code delta while leaving the four existing callers untouched. Extending `resolve_edit`
is the strongest alternative if avoiding any visibility modifier change is the priority;
it preserves operation-based resolution and one-graph evaluation, but changes the
existing interface and requires edits to all four callers, including preview, against
ticket 115's ownership rules. A transaction-specific facade may offer a narrower
**interface**, but only by adding crate visibility elsewhere and reshaping module
ownership; it is not a way around the ticket's visibility restriction. The existing
`resolve_edit` call with no framed records keeps its narrow interface but loses required
behavior, so its depth and leverage are insufficient for 115.

The owner-approved `pub(crate)` exception resolves the ticket contradiction. Ticket 115 can use
the existing prepared solver while leaving the four `resolve_edit` callers and preview path
untouched. The research supports that limited decision; it does not authorize widening the module,
editing production routing, or activating the prepared transaction before 111.
