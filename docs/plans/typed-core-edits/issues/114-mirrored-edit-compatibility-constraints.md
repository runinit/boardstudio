# 114: Classify compatibility and constraint translation

Status: claimed
Type: build
Blocked by: [113](113-inverse-map-reviewed-outline-edits.md), [139](139-generalized-mirrored-constraint-model.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

Finish the inverse-operation coverage ledger and resolve compatibility/constraint behavior
before preparing its private adapter. This draft is not implementation-ready for constraints.

## Explicit technical design gate

At `2478fb151`, constraints have no correspondence kind and mirror-axis constraints encode
vertical/horizontal axes. Arbitrary rotated pair maps and linked/local endpoint mixtures have
not been shown representable. Root must review a source-grounded design before implementation;
no schema/visibility widening, new public payload or silent unsupported pass-through is authorized.
Determine which operations delegate existing behavior and which require canonical endpoint/axis
handling. Report exact unrepresentable cases and affected parent acceptance rather than guessing.

## Source-grounded design checkpoint (2026-10-11)

Bounded Luna high planning packet: `/home/chris/.cache/boardstudio-baselines/ticket114-planning/decision-packet.md`, stable lane sources at86821996751c8e1c80c1f5102c7c77c58f22c0e5. Root reviewed the packet; this is planning evidence, not implementation or independent behavior acceptance. Same-pair linked/linked Offset is representable with vector reflection, negated relative rotation and durable constraint ID; Mirror is representable only when its transformed axis remains horizontal/vertical. RemoveConstraint delegates by exact ID; local/local remains local. The current two-axis/global constraint model does not generally represent slanted axes or mixed linked/local endpoint relations. Canonical operation must feed apply and both downstream resolve_edit calls.

The user chose support through a separately scoped model extension; [139](139-generalized-mirrored-constraint-model.md) owns the approved extension design/implementation. Root accepted the representation and endpoint semantics in139 and ADR0010;114 is ready-for-agent subject to its113/139 implementation dependencies. Do not silently narrow parent19 or claim unsupported cases complete. This decision does not block current106 correction,138 retirement or already approved independent work. Reuse110’s ownership ledger; no second inventory/audit is required.

## Remaining inventory families

`SetConstraint`, `RemoveConstraint`, `ConvertOffsetConstraint` (added after 110's inventory);
`SetLayout`, `CreateMirroredPair`;
`ApplyMechanicalEdit`, `SetMechanical`, `SetCase`; `EditKeymap`, `SetKeyBinding`;
`CreateMirroredHalf`, `ReplaceDocument`. Check every 40-variant entry in 110 has an owner.

- Delegate typed mechanical pair semantics once. Raw whole configurations require explicit
  compatibility classification; they are not generic point-map requests.
- Preserve bindings on the targeted half (ADR exclusion), creation labels/map validation,
  and source-authoritative ReplaceDocument reconciliation rather than inferring intent from diffs.
- Keep native/legacy same-board layouts; refuse overlapping link systems without silently unlinking.
- Constraints must use the canonical operation throughout apply and downstream resolution.
  Cover linked/linked, local/local and mixed endpoints, rotated maps and durable IDs explicitly.
- After root's design review, prepare only approved private helpers; activate nothing before 111.

## File ownership

Read-only design inventory: `core/src/model.rs`, `core/src/lib.rs`, constraint and layout modules.
Once root grants the design, exclusive writer scope is private mirrored-half translation modules,
association lookup in `core/src/mechanical/halves.rs`, and established mirrored-half test files.
Any constraint implementation/schema file needs separate explicit ownership before editing.

## Test-first and acceptance

- [ ] Root-reviewed constraint/compatibility design is recorded, with representability limits.
- [ ] Every remaining variant is translated/delegated/deliberately unchanged/unsupported with
      reason; applicable unsupported parent behavior remains a blocker.
- [ ] Approved private adapter cases have expected red/green tests for endpoints/axes,
      native/legacy conflict, bindings, raw compatibility and import/recovery policy.
- [ ] No production behavior is enabled and no independent legacy data is linked implicitly.

Independent verifier: mutate one constraint conversion/delegation, legacy ownership guard and
ADR-excluded binding pass-through. Do not claim readiness or resolution from inventory alone.

## Inherits from 139

[139](139-generalized-mirrored-constraint-model.md) resolved the private model: framed endpoints,
slanted `mirror-line` and `part-offset`, key/companion projection, `retune_target` and
`constraints::driving_constraint` (`core/src/constraints/`; crate-visible, no `ProjectDoc`
forwarder). A companion frame is unbound (`key: None`) until the solver supplies the key's
resolved pose, and the key-reading rules refuse while unbound, so a caller of `project` or
`retune_target` outside the solver must bind it with `with_key` first. 114 owns what 139 handed on:
translating `SetConstraint`/`RemoveConstraint` and `MoveParts` through these, editing from either
linked half, and the endpoint rewrites on MakeLocal/Omit/Rejoin/Unlink per design.md's transition
table (109 is on dev; what is missing is the persisted arm and the operation seam). The
persisted arm and format 6 are 111's.

Two deferrals 139 left for 114's own edit of `core/src/constraints.rs`: the solver's
`(parts, constraints, assemblies, resolved, visiting)` argument clump should become one value
when the operation seam adds more state to it, and the accessors
`ConstraintEndpoint::visible_part_id` and `authoritative_part_id`, read only by tests, are to be
deleted if nothing in 114 uses them.

## Root source review and first implementation lease (2026-10-12)

The product/design gate is resolved by the accepted [139 design](../design/139-generalized-mirrored-constraint-model.md)
and its source checkpoint above. Root rechecked the current `dev` sources after 113/139 landed:
the 139 endpoint, relation, lookup and resolver helpers are present; the persisted `Constraint::Framed`
arm and format-6 migration remain 111-owned; the prepared transaction and propagation through apply
and both resolver calls remain 115-owned. No product question is open. The current 41-operation enum
is exhaustively classified in `inverse::canonicalize`; the runtime ledger test is intentionally a
sample, so the documented ledger and exhaustive dispatch are the completeness evidence.

Exclusive first-unit lease on `typed-core-edits/114-mirrored-compatibility`:
`core/src/mirrored_half/inverse.rs`, new `core/src/mirrored_half/inverse/constraints.rs` and its
module-local tests, `core/src/constraints.rs`, and `core/src/constraints/{endpoints,relation,retune}.rs`
with their module-local tests. This unit prepares the private Set/Remove/Convert constraint mapping,
legacy-to-live endpoint compilation and shared retuning seam required by the accepted design. It may
add narrowly crate-visible prepared-value constructors/types only if the downstream prepared
transaction needs them; these remain non-public and are not a production route. Do not edit
`core/src/model.rs`, `core/src/lib.rs`, `core/src/matrix/layout.rs`, `core/src/matrix/placement.rs`,
`core/src/mechanical/edit.rs`, migration, web adapters or goldens in this unit. Root will grant any
additional shared-path lease after reviewing the first stable handoff. Production activation and
persisted schema remain with 111; transaction composition remains with 115.
