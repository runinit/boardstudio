# 113: Inverse-map outline and reviewed-geometry edits

Status: ready-for-agent
Type: build
Blocked by: [112](112-inverse-map-hosted-electrical-edits.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

Prepare private translation of the outline families and settlement inputs listed in 110.
Do not route production outline operations through the translator until 111.

## Scope and evidence

At `2478fb151`, top-level `SetOutline`, `CopyOutline`, `SelectOutline`, `RenameOutline`,
`RemoveOutline` and `ApplyOutlineEdit` target board/version/feature geometry and identities.
`core/src/outline_edit.rs::OutlineChange` has 24 variants enumerated in 110. Reviewed contours
use exact equality: mapping only the target can incorrectly retire a valid reviewed edit.

- Classify each of the 24 changes. Scalar settings retain values; geometry/refs map through
  108's world/part-anchor rules; lifecycle requests retain allocation/seed/settlement meaning.
- Translate board/version/feature IDs, geometry, reviewed shapes, nested controls/bridges/gaps,
  winding/index associations and ownership together. Preserve authored identity reservations.
- Test linked versus fixed versions, additions/removals and relevant-versus-unrelated review
  retirement. Do not change existing outline lifecycle or exact review comparison semantics.
- Keep this as a private prepared adapter. If all 24 cases cannot fit a bounded session, report
  the exact remaining lifecycle family to root before requesting further IDs or coding broadly.

## File ownership

Exclusive: private mirrored-half translation module(s), established mirrored-half tests.
`core/src/outline_edit.rs` and outline/mechanical implementation files are read-only evidence;
request root ownership for any narrowly necessary helper change. No visibility widening,
operation schema change, production route or generated golden edit.

## Test-first and acceptance

- [ ] Direct translation tests fail first and classify every top-level/nested outline variant.
- [ ] Axis and rotated/translated-map geometry, anchored controls and reviewed contours map
      consistently; source-equivalent reviewed edits remain admissible.
- [ ] Relevant changed review inputs retire, unrelated changes do not; identity/seed allocation
      and independent local exceptions preserve existing behavior.
- [ ] No unclassified variant hides behind wildcard pass-through; production remains unchanged.

Independent verifier: mutate reviewed-contour conversion, winding/anchor mapping and lifecycle
identity translation separately. A missing lifecycle family blocks resolution, not parent scope.
