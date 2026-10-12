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

## Outcome (2026-10-12)

Implemented in `core/src/mirrored_half/inverse.rs` (private child module, no call sites; 111
activates, 115 composes). `canonicalize(document, operation)` returns the source-equivalent
request, a `translated` flag and, for a mirror-side `AddPart`, the planned source/mirror identity
pair the forward independent-Add transition needs. `coverage` is the exhaustive ledger (no
wildcard arm).

Coverage of this slice (all **translated**): `SetMatrix`, `SetMatrixSplay`, `SetMatrixPlacement`,
`RemoveMatrix`, `SetMatrixCellsEnabled`, `RemoveSelectedMatrixCells`, `RemoveMatrixCells`,
`RemoveMatrixRow`, `RemoveMatrixColumn`, `MoveParts`, `AddPart`, `RemoveParts`. Requests naming a
source, an unpaired item or a Local exception are **deliberately unchanged**. Everything else is
refused by `canonicalize` and owned elsewhere: hosted, electrical, keycap and shared-definition
families by 112; outline families by 113; keymap, layout, constraints (including
`ConvertOffsetConstraint`, absent from the inventory above), mechanical, raw configuration,
native creation and import by 114.

Decisions made inside the slice (revisit if wrong):

- An Omitted mirror target is refused; Local and unpaired targets run as written.
- A `SetMatrix` payload that changes the mirror matrix's name, definition or board is refused
  (the forward pass never reflects them), as is one carrying local companions.
- A matrix reached by a native pair and a legacy layout link is refused from either half.
- A new matrix on a mirror board has no correspondence and runs as written (no matrix-addition
  transition exists in the forward kernels).
- Parts owned by a linked embedded circuit are refused (112 translates circuit edits).
- Two entries naming one link are one action when they agree to 1e-6 mm, otherwise refused.

Evidence: 37 unit tests in `inverse/tests.rs`, including round trips against production
`CreateMirroredHalf` output for an axis map, a map translated along the mirror line and a slanted
map in every companion mode with a stabiliser; 16 of 16 mutations killed
(`.evidence/ticket110-mutations.log`). Finding: `pair::forward` is not its own inverse when the
map translates along the mirror line, so `layout::reflected` cannot be reused for the origin.
