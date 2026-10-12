# 110: Inverse-map matrix and part edits targeting linked mirrors

Status: resolved
Type: build
Blocked by: [109](109-mirrored-half-exceptions-unlink.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

Implement a private operation-translation layer for linked mirror targets. An edit on a
linked mirror edits the authored source value through the inverse pair map. Activate it in 111.

## Bounded scope and coverage ownership

Root split the original all-family inventory on 2026-10-10. This slice owns only the private
canonicalization seam plus `SetMatrix`, `SetMatrixSplay`, `SetMatrixPlacement`, `RemoveMatrix`,
`SetMatrixCellsEnabled`, `RemoveSelectedMatrixCells`, `RemoveMatrixCells`, `RemoveMatrixRow`,
`RemoveMatrixColumn`, `MoveParts`, `AddPart`, `RemoveParts`. Preserve complete bulk semantics,
source identity allocation, mixed canonical-target conflicts and downstream constraint inputs.
No production call site changes; 115 composes the resulting request through resolution.

[112](112-inverse-map-hosted-electrical-edits.md) owns hosted/electrical/keycap families;
[113](113-inverse-map-reviewed-outline-edits.md) owns outline/reviewed geometry;
[114](114-mirrored-edit-compatibility-constraints.md) owns compatibility/constraints and the
final exhaustive ledger. The inventory below remains shared evidence, not a claim that 110
implements the other families. If this bounded scope still exceeds one session, report the
exact remaining family before requesting additional IDs. No unchecked family may disappear.

## Committed-source operation inventory

Enum: `core/src/model.rs::EditOperation`; dispatch: `core/src/lib.rs::apply`, at
`2478fb151`. Confirm freshness against synchronized HEAD before dispatch. This is
inventory evidence, not evidence of implemented or verified translation.

| Family | Variants | Required classification/translation |
| --- | --- | --- |
| Matrix payload | `SetMatrix` | IDs, board, world origin/heading, cells/assemblies; preserve numbering. |
| Matrix parameters | `SetMatrixSplay`, `SetMatrixPlacement` | Matrix ID; distinguish world origins from matrix-frame values. |
| Matrix structure | `RemoveMatrix`, `SetMatrixCellsEnabled`, `RemoveSelectedMatrixCells`, `RemoveMatrixCells`, `RemoveMatrixRow`, `RemoveMatrixColumn` | Whole multi-matrix requests before removal/renumbering; existing owned-member closure. |
| Parts | `MoveParts`, `AddPart`, `RemoveParts` | IDs, board, world positions; canonical allocation for additions; bulk collisions. |
| Hosted | `SetMountedModule`, `RemoveMountedModule`, `EmbedModuleCircuit`, `RemoveEmbeddedCircuit` | Pose, host/connector/upstream/bus/net references and namespace lifecycle. |
| Shared definition | `SetModuleDefinition` | Execute global definition once; reconcile affected instances, not definition geometry. |
| Outline legacy | `SetOutline`, `CopyOutline`, `SelectOutline`, `RenameOutline`, `RemoveOutline` | Board/version/features, anchored/world geometry and allocated identities. |
| Outline typed | `ApplyOutlineEdit` | All nested changes plus reviewed shape/seed/settlement semantics. |
| Electrical | `SetNet`, `SetInputScanMode`, `SetWiringMode` | Net/part/board IDs, pins and established side-name rules; scalar modes unchanged. |
| Keycaps | `SetKeycapBoard`, `SetMatrixKeycaps`, `SetKeycapKey` | Owner IDs; appearance/legend values unchanged. |
| Bindings | `EditKeymap`, `SetKeyBinding` | ADR-excluded: retain targeted-half semantics. |
| Layout | `SetLayout`, `CreateMirroredPair` | Ownership/membership and native/legacy conflict guards. |
| Constraints | `SetConstraint`, `RemoveConstraint` | Endpoint ownership and representability need design; not proven ordinary ID mapping. |
| Mechanical | `ApplyMechanicalEdit` | Delegate established bidirectional lifecycle; do not inverse twice. |
| Raw configuration | `SetMechanical`, `SetCase` | Explicit compatibility classification; not a generic point-map case. |
| Native creation | `CreateMirroredHalf` | Existing creation/label/map validation; no generic whole-request inverse. |
| Import/recovery | `ReplaceDocument` | Source-authoritative reconciliation; do not infer intended half from raw diff. |

`core/src/outline_edit.rs::OutlineChange` nested inventory:
`SetMargin`, `SetCorners`, `SetSize`, `SetBridgeWidth`, `SetRepairEnabled`,
`SetMaximumGapSpan`, `SetMinimumConnectionWidth`, `SetEdgeClearance`, `SetProtectedGap`,
`RemoveProtectedGap`, `AddFeature`, `SetFeature`, `RemoveFeature`, `AddConnection`,
`RemoveConnection`, `CreateGenerated`, `CreateLinkedRefinement`, `Copy`, `Freeze`,
`EditGeneratedPerimeter`, `EditFeaturePerimeter`, `Activate`, `Rename`, `Delete`.

Technical hazards to retain in split acceptance:

- `constraints::resolve_edit` receives the operation after apply: `MoveParts.positions`
  must be canonicalized for downstream resolution too, not only inside the apply arm.
- Existing mirror constraints encode vertical/horizontal axes and have no correspondence
  kind. Arbitrary rotated maps and linked/local endpoint mixtures are not proven representable.
- Reviewed outline contours use exact equality; transform reviewed shape, target, winding,
  controls and identity seeds consistently or valid edits may retire incorrectly.
- Matrix frame parameters, switch/part/module headings and stabilisers have distinct rules.
- Do not separate bulk movement from conflict/downstream resolution, circuit edits from
  joins/namespace cleanup, or reviewed geometry from its settlement checks.

For every top-level and nested variant, the final coverage table must say **translated**,
**delegated**, **deliberately unchanged**, or **unsupported with reason**. Unsupported entries
are reported blockers against applicable parent acceptance, not implicit permission to ship
partial behavior. No wildcard pass-through may conceal an unchecked family.

## Contracts

- Linked mirror requests canonicalize to source identities/coordinates before the existing
  operation executes. Use the complete inverse map, including translated/rotated maps and
  distinct switch, independent-part and companion heading rules.
- Local targets retain local execution. Resolve omitted/removed targets through the accepted
  document's durable association and existing intent retirement/refusal rules; do not recreate
  omitted geometry implicitly or guess a new product action.
- Mixed local/linked targets and composites preserve atomicity and existing operation order.
  Do not translate the same target twice when an operation already implements pair semantics.
- Source deletion removes its linked mirror; mirror-side linked deletion maps to that source
  deletion. Omit is the explicit mirror-only action. No new implicit local state.
- New additions and any ambiguous source/mirror collision in one request must be explicitly
  covered by the inventory; report unresolved operation semantics to root before implementing.
- Keep same-board `LayoutMirrorLink` support and refuse simultaneous native and legacy links
  on one matrix. Do not introduce `boardstudio.mirroredComponentSource` for native pairs.
- No production call-site integration, new visibility or ticket 40 independent-part semantics.

## File ownership

Exclusive during this slice: `core/src/mirrored_half.rs` and private child modules,
`core/src/mechanical/halves.rs`, inverse-translation tests in the mirrored-half test files.
`core/src/model.rs` may be read for the inventory; changing an operation payload requires root
review and ownership first. `core/src/lib.rs` is read-only until 111.

## Test-first and acceptance

- [x] Outcome records coverage of this bounded family and links remaining inventory ownership to 112–114.
- [x] Table-driven direct translation tests fail first, then cover every matrix/part variant in this slice
      in both directions, including rotated/translated maps, local targets and mixed batches.
- [x] Transforming a linked mirror request yields the equivalent source request without
      changing unrelated IDs, parameters, bindings or labels.
- [x] Deletion, ownership/reference translation and legacy/native conflict cases are covered.

Independent verifier: mutation-check inverse position, heading and identity conversion and
one mixed-batch/ownership case. Do not mark resolved with an unimplemented in-scope matrix/part family.

## Outcome (2026-10-12, after the review round)

Implemented in `core/src/mirrored_half/inverse.rs` (private child module, no call sites; 111
activates, 115 composes). `canonicalize(document, operation)` is one exhaustive dispatch over
every `EditOperation` variant (no wildcard arm: a new variant does not compile without a ledger
entry). It returns a `Canonical` with the source-equivalent request, its `Coverage`, the planned
`PartAddition` pairs for mirror-side `AddPart` and the planned `MatrixAddition` pairs for new
matrices, or a `Refusal` that carries the ledger entry of a family another ticket owns.

Ledger (translated / delegated / deliberately unchanged / unsupported with reason):

- **Translated:** `SetMatrix`, `SetMatrixSplay`, `SetMatrixPlacement`, `RemoveMatrix`,
  `SetMatrixCellsEnabled`, `RemoveSelectedMatrixCells`, `RemoveMatrixCells`, `RemoveMatrixRow`,
  `RemoveMatrixColumn`, `MoveParts`, `AddPart`, `RemoveParts`. Requests naming a source, an
  unpaired item or a Local exception run as written.
- **Delegated:** `ApplyMechanicalEdit` (the typed lifecycle is already two-way; not inverted twice).
- **Deliberately unchanged:** `EditKeymap`, `SetKeyBinding` (ADR-excluded; targeted-half semantics).
- **Unsupported here, owned elsewhere (refused, with owner and reason):** hosted, electrical,
  keycap and shared-definition families and circuit-member `MoveParts`/`RemoveParts` by 112;
  outline families by 113; `SetLayout`, `CreateMirroredPair`, `SetConstraint`,
  `RemoveConstraint`, `ConvertOffsetConstraint` (absent from the inventory above),
  `SetMechanical`, `SetCase`, `CreateMirroredHalf`, `ReplaceDocument` by 114.

User decisions recorded:

- Two entries naming one link are one action when they agree within 1e-6 mm, otherwise refused.
- A new matrix on a mirrored board is NOT run as written: it must also be mirrored and linked.
  110 supplies the seam: the source-equivalent `SetMatrix` (a mirror-board addition is inverted
  through the pair map; a source-board addition is returned as written) and a planned
  `MatrixAddition { source_id, mirror_id, source/mirror board, members }` with every key and
  companion as a source/mirror pair, identities allocated free of existing ones. The forward
  transition that creates the mirror matrix and mints its correspondences does not exist on dev
  (105 derives only an existing mirror; 106-108 add members to an existing matrix; 108 assigns
  minting to the membership slices), so [145](145-mirror-new-matrices-on-a-mirrored-board.md)
  owes it, blocked by 110 and named in 111 and 115.
- A `SetMatrix` payload that changes the mirror matrix's name, definition or board, or carries
  local companions, is refused.
- An Omitted mirror target is refused (omitted geometry is not recreated implicitly).
- Parts owned by a linked embedded circuit are refused here; 112 translates circuit edits.
- A moved linked companion translates, but forward validation refuses authored companion
  placement on linked slots; that is untested end to end and surfaces in 115.

Evidence, honestly graded. Red first, then green: the part, matrix-structure and `SetMatrix`
payload slices (including the literal-mode companion cases), the ledger categories and the new
matrix pairs (the last two slices went red as compile errors before green). Added after the
code, so evidenced by the mutation run rather than by a red phase: companion-mode `MoveParts`
(tweak, duplicate position, stabiliser), the round trips against production `CreateMirroredHalf`
output (axis, translated-along-the-line and slanted maps in every companion mode with a
stabiliser) and the all-variants table. 42 unit tests in `inverse/tests.rs`; mutations in
`.evidence/ticket110-mutations.log` (position, heading, identity, mixed batch, ownership, legacy
overlap, planned matrix pairs). Finding: `pair::forward` is not its own inverse when the map
translates along the mirror line, so `layout::reflected` cannot be reused for the origin.

Not covered by 110: the forward matrix-addition transition (145), circuit members (112), and
authored-companion moves end to end (115).
