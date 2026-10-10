# 110: Inverse-map matrix and part edits targeting linked mirrors

Status: ready-for-agent
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

- [ ] Outcome records coverage of this bounded family and links remaining inventory ownership to 112–114.
- [ ] Table-driven direct translation tests fail first, then cover every matrix/part variant in this slice
      in both directions, including rotated/translated maps, local targets and mixed batches.
- [ ] Transforming a linked mirror request yields the equivalent source request without
      changing unrelated IDs, parameters, bindings or labels.
- [ ] Deletion, ownership/reference translation and legacy/native conflict cases are covered.

Independent verifier: mutation-check inverse position, heading and identity conversion and
one mixed-batch/ownership case. Do not mark resolved with an unimplemented in-scope matrix/part family.

## Execution and verification gates

This is a child of [19](19-reconcile-mirrored-halves.md); resolving it does not resolve 19.
Core children execute serially with exclusive Core file/API leases. The user approved
independent A/F streams on 2026-10-10; follow the central lane registry and parallel-streams
decision for audited disjoint edits. Every shell/check command remains globally sequential;
activation and timing require exclusive windows. The prerequisite batch and delivered 37
verification slice are complete; root must refresh the bound index and grant this slice.
Use `/home/chris/01_Projects/ts-boardstudio2/.worktrees/lane-mirror-core`, branch `lane/mirror-core`;
report path/branch/status and inspect overlaps before editing. Every shell call has that
absolute `workdir`; commands run one at a time in the foreground.

Read `docs/agents/agent-workflow.md`, `verification-process.md`, `gitnexus-index.md` and
parent 19. Luna 6 high implements test-first and records `implemented (awaiting verification)`;
a fresh Sol 6.1 xhigh independently verifies/mutation-checks, then alone may set `resolved`.
Run bound GitNexus impact before every symbol edit; list callers, processes and risk.
`apply`/`layout::resolve` are known CRITICAL paths: warn before editing. UNKNOWN, stale or
missing answers need source confirmation and honest limitation reporting, not an all-clear.
Use `boardstudio-lane-mirror-core`; root alone refreshes. Run complete `detect_changes` before
any commit; truncated/partial evidence is not clean.

Request every test/build/browser/performance command slot from root before running it.
Affected checks: `repo lint test typecheck`, plus native/WASM boundary parity when its
behavior/transcript changes; targeted red/green and verifier mutations also need slots.
No full check per slice, no `release`, no background checks; Cargo uses 12 jobs.
Maps, ADRs, CONTEXT.md, scripts/check.py and generated goldens remain root-owned.
No merges/push/PR, stash/reset/discard or broad staging; explicit paths only.
Commit only after root permits it, with the actual-author trailer. Outcome records exact
commands/results/timings, mutation failures/restoration and every unrun or blocked gate.
