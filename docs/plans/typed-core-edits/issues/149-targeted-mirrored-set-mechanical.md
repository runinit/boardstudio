# 149: Give raw SetMechanical an explicit target for mirrored pairs

Status: resolved
Type: build
Blocked by: [117](117-prepare-mirrored-preview-history-performance.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

Design and implement raw `SetMechanical` support for native mirrored mechanical instances,
following the owner's 2026-10-13 choice to support mirror behavior. Ticket 114's refusal is safe
under its original payload but is not the final compatibility contract for parent 19.

## Source facts

- Before ticket 149, `EditOperation::SetMechanical` carried only
  `Option<Box<MechanicalConfiguration>>` (`core/src/model.rs`). Its apply arm replaces the one
  `ProjectDoc.mechanical` slot (`core/src/lib.rs`). The current private draft adds the
  owner-selected optional target; the prepared instance route is still unconnected.
- A physical instance instead stores its own optional configuration, and a native pair stores
  shared construction plus linked mechanical features in `HardwareConfiguration`
  (`core/src/model.rs`, `core/src/mechanical/inputs.rs`).
- `MechanicalEdit` already distinguishes `MechanicalTarget::Board` from
  `MechanicalTarget::Instance` (`core/src/mechanical/change.rs`). The current Case settings caller
  uses `ApplyMechanicalEdit`; no production caller of raw `SetMechanical` was found.
- Therefore the current payload cannot identify which instance a `None` replacement clears, and
  assigning a mirror-side configuration to the single project slot would not update the pair's
  per-instance storage or linked feature records.

## Owner decision (2026-10-13)

The owner chose an optional `MechanicalTarget` field on the existing `SetMechanical` operation.
Requests that omit it preserve the current board-slot behavior. New callers can name `Board` or
the exact `Instance`, including when clearing with `configuration: None`. Validate that a
provided configuration's `board_id` matches an explicit target.

The optional field preserves old serialized requests and does not change saved-project format.
Keeping the payload unchanged cannot target a native `new-board` instance safely: there is no
target for a mirror-side `None`, and the singleton board slot cannot represent per-instance data.

## Required behavior after the interface is chosen

- Prepare the entire replacement against the accepted document; mutate only a disposable draft.
  A refusal leaves the accepted document and history untouched.
- A targeted board replacement retains the existing board-slot semantics. Explicit Board `None`
  clears only a slot owned by that board; it preserves another board's slot. Do not write its value
  into `hardware.shared_construction` or an instance slot.
- An instance replacement uses the existing mechanical ownership model: shared construction stays
  pair-owned; board/instance-local data stays local; linked feature values and membership changes
  follow the existing feature rules from either member; Local and Omitted states are preserved.
  Resolve references by identity, never vector position.
- Keep the existing sparse shared-construction rule: an absent optional field falls back to each
  member's stored value; it does not clear that field across both members.
- `None` is scoped by the explicit target. For an instance with a linked pair, reuse the existing
  typed disable/unlink lifecycle so the surviving half keeps its visible mechanical data. When
  `target` is omitted, preserve the existing board-slot behavior: `Some` replaces the slot and
  `None` clears a populated slot (or is a no-op when the slot is already empty).
- A legacy pair without `halves` and a `reuse-board` pair retain existing semantics. A reuse-board
  pair is processed once and is not duplicated into two board configurations.
- Run existing downstream resolution, projection and final pair validation before publishing;
  return actual changed IDs. Keep production activation owned by 111.

## Suggested implementation seam

Use a private replacement plan (`between(accepted, target, requested)` then `apply(&mut draft)` or
one equivalent single entry point). It partitions a raw full replacement into the existing typed
mechanical lifecycle for linked features and construction, while making field ownership and
reference resolution explicit. This is pure in-process computation; no port/adapter is needed.
Do not duplicate geometry reflection or pair exception logic, widen public member visibility, or
route Open through this planner.

## Domain contract (2026-10-13 proposal review)

A Board identifies a PCB design. A physical board instance identifies one realization of it.
In `reuse-board`, both halves refer to the same Board, so only an Instance target can distinguish
their mechanical hardware. An explicit Board target continues to address the project-level
configuration; it does not select every physical instance using that board.

The submitted instance value is an effective mechanical configuration. It combines several
owners; setting it must respect those owners instead of treating the whole value as instance-local.

| Submitted change | Domain owner and consequence |
|---|---|
| Plate thickness, layer spacing, wall thickness or mounting method | Pair construction; both halves use the changed shared choice. |
| Battery, part profile, hardware specification or critical fit | Selected instance's hardware-specific settings; the other half keeps its own. |
| Changed linked mount, opening, closure mount or gasket support | Pair feature; the selected member becomes its geometry author and the counterpart is reflected. The construction source need not change. |
| New ordinary mechanical feature | Linked pair feature by default, unless an explicit exception says otherwise. |
| Linked feature absent from the submitted configuration | Delete the shared feature and its counterparts; this is different from Omit. |
| Changed Local feature | The selected member's independent counterpart; preserve its exception and other members' values. |
| Reintroduced Omitted member identity | Refuse; rejoining requires the explicit feature lifecycle. |
| Absent optional construction choice | Preserve the established fallback to each member's stored choice; this does not request clearing that choice on both halves. |
| No configuration for a paired instance | Unlink the pair and disable the selected mechanical stack. Preserve the surviving half's visible mechanical configuration. |

Consequently, "replacement" means applying a requested configuration through the existing
ownership rules. It does not promise that the resulting effective configuration is byte-for-byte
equal to the request: board thickness comes from the Board, identity reservations remain monotonic,
and absent optional construction fields retain their established fallback semantics.

The no-configuration case is especially consequential for native mirrored halves: the mechanical
pair also owns the `halves` relationship, so dissolving it ends automatic reflection of the board
items as well. This follows the existing explicit "unlink and disable this half" lifecycle in
`mechanical::edit::disable`; it is not a one-feature Omit or a clear of only an instance slot.
Any caller must present that action with its actual scope.

Concrete scenarios to preserve in prepared-path acceptance:

- Two physical halves use the same reversible Board. Changing the right battery height targets
  the right Instance only; changing its plate thickness updates the pair's construction.
- The right half moves a linked opening. The right member authors the new geometry; the left
  counterpart reflects it without losing either member's feature identity.
- The right half has an Omitted opening. Supplying that identity does not silently rejoin it.
- Removing a linked opening from a full request removes the shared feature; Omit is the separate
  action for retaining it on only one half.
- Clearing a paired instance unlinks it and disables its mechanical stack; the survivor retains
  the configuration it showed. This is broader than clearing an optional stored value.

The review exposed a missing case: a supplied configuration must also be able to establish a
disabled unpaired instance's mechanical stack, following the existing raw Board `Some` behavior.
The first private regression reproduced the refusal from reading `effective_inputs` before any
configuration existed. The implementation now handles that eligible absence explicitly, preserves
the supplied values and identity reservations, and still propagates errors from malformed effective
inputs. A Legacy instance can inherit from legacy shared construction or another configured
instance, so an absent stored value alone does not establish that it is disabled. Verification of
the complete prepared path remains required before acceptance.

The owner approved the single new crate-only entry point after the domain review on 2026-10-13.
It is named `apply_instance_configuration` to express its combined set/disable role. Existing
mechanical helpers retain their visibility, and the operation's optional-target request shape is
unchanged. No new ADR is needed for this reversible helper placement; pair and exception policies
remain governed by ADR-0008/0010. Production activation remains owned by 111.

## Acceptance

- [x] Owner selected the optional target field; old board-only serialized requests remain
      compatible.
- [x] Source- and mirror-side full replacements update the same pair atomically, including shared
      construction and linked feature additions, edits, moves, and removals.
- [x] Local features remain independent, Omitted features are never resurrected, and references
      resolve by identity or refuse without partial mutation.
- [x] Explicit instance `None` follows the existing disable/unlink lifecycle; absent legacy `None`
      stays a no-op. Legacy and reuse-board cases are covered.
- [x] No pair, configuration slot, or changed-ID record is partially updated on refusal.
- [x] Independent Spec and Standards reviews pass. The preparation remains inert until 111.

## Related

- [114](114-mirrored-edit-compatibility-constraints.md) records the prior unsupported disposition.
- [115](115-prepare-mirrored-edit-transaction.md) owns inert transaction composition.
- [111](111-integrate-mirrored-half-reconciliation.md) owns production activation.

## Comments

2026-10-13: Astra's architecture review recommends a private child module of `mechanical::edit`
so full replacement can reuse `persist_edit`, disable/unlink, identity reservations and clearance
projection without exposing those helpers. The private implementation is being checked directly.
Connecting it to `mirrored_half::reconcile::prepare::apply_canonical` needs one new `pub(crate)`
parent entry point; that separate visibility approval is pending. Existing helpers and production
routing remain unchanged by that proposed connection. The owner approvals recorded for ticket 115
cover its preparation/resolver seams and do not authorize this additional entry point.


2026-10-13 private-draft evidence: the mechanical edit tests pass (49), and the mechanical engine
tests pass (5). The existing mirrored-half integration module also passes (42). Independent
Standards and Spec reviews covered the recorded draft and the later
Board-clear correction. Spec found that explicit Board `None` erased another board's singleton;
a failing regression confirmed it, and the scoped clear now passes. The private lifecycle also
covers distinct per-member feature IDs, legacy paired members without a stored slot, identity
reservations and rollback after late validation failure. An independent verifier killed all six
compiled private mutations (zero survivors/compile failures), with exact file restoration.
Evidence is under `.evidence/ticket149/` in the main checkout (gitignored). These checks do not
complete ticket acceptance: the prepared instance route remains unconnected pending the separate
visibility approval, and its regressions still stop at the dispatcher guard. Full verification,
native baseline registration, connection review and local merge remain outstanding.


2026-10-13 approval and connection: the owner approved the proposed narrow crate-only seam after
requesting its domain review. `mechanical::edit::apply_instance_configuration` is the single new
`pub(crate)` entry point, with a private child implementation. Prepared instance-targeted
`SetMechanical` dispatches there; raw production apply still refuses that target until 111.
The earlier pending-approval comments and private review evidence describe the pre-connection
snapshot. The new connection and disabled-instance support require fresh changed-scope reviews,
mutations and full checks before local merge.


## Outcome (2026-10-13)

Implemented the optional target request shape and the approved crate-only
`mechanical::edit::apply_instance_configuration` preparation adapter. Omitted targets retain the
legacy board slot. Explicit Board clears are scoped to that board. Instance requests partition
construction, linked features and local hardware through the existing mechanical lifecycle,
preserve Local/Omitted states and identity reservations, support disabled unpaired instances,
and refuse invalid targets/geometry atomically. Paired clears materialize the survivor and unlink.
The generic lifecycle does not process the delegated mechanical replacement a second time.
Changed-ID comparison preserves distinct Board/Instance owners sharing an ID and includes
pair-owned data affecting effective member configurations. Production activation remains in 111.

Independent Standards and Spec reviews passed on the original draft, the recorded connection
scope, and the two-file Spec fix. Findings about another board's clear, Legacy fallback source
persistence and invalid Local gasket anchors were reproduced red and fixed. Optional lookup
consolidation notes remain maintenance heuristics, without a correctness or acceptance failure.

Final verification: 55 mechanical edit tests and 9 prepared instance-request tests pass; the full
`python3 scripts/check.py repo test lint --update-native-baseline` run passes with 1,817 executed
native test names. The baseline adds tests and intentionally renames the superseded raw-refusal
case to targeted delegation. An independent verifier killed all 14 compiled final mutations,
with zero survivors and exact restoration; an initial malformed m13 mutation was corrected and
rerun before being counted. Final source hashes match the independently reviewed scopes.

Evidence (gitignored, main checkout): `.evidence/ticket149/final-review-results.md`,
`full-checks-final.log`, `connected149-mutation-summary.json`, individual mutation logs and
red/green traces under `disabled-instance/`. Local merge and post-merge checks follow; nothing
is pushed. Parent 19 remains open for 150, 111, 109b and full activated acceptance.
