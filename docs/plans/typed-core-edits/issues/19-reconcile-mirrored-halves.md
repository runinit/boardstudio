# 19: Reconcile mirrored halves on every edit, and exceptions

Status: ready-for-agent
Type: build
Blocked by: [18](18-core-create-mirrored-half.md), [26](26-mirrored-half-linked-mechanical-and-unchanged-parameters.md), [27](27-mirror-modules.md), [28](28-mirror-embedded-circuits.md), [29](29-mirror-switch-and-companion-headings.md), [30](30-independent-parts-use-the-full-heading-rule.md), [31](31-companion-placement-modes.md), [32](32-mirrored-half-labels.md), [39](39-stabilisers-and-companion-fallback.md), [105](105-mirrored-half-derivation-kernel.md), [106](106-reconcile-mirrored-matrices-layouts-parts.md), [107](107-reconcile-mirrored-hosted-electrical-items.md), [108](108-reconcile-mirrored-outlines-assemblies.md), [109](109-mirrored-half-exceptions-unlink.md), [110](110-inverse-map-mirrored-half-edits.md), [111](111-integrate-mirrored-half-reconciliation.md), [112](112-inverse-map-hosted-electrical-edits.md), [113](113-inverse-map-reviewed-outline-edits.md), [114](114-mirrored-edit-compatibility-constraints.md), [115](115-prepare-mirrored-edit-transaction.md), [116](116-prepare-mirrored-open-repair.md), [117](117-prepare-mirrored-preview-history-performance.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md) · [16](16-native-mirrored-halves.md)

## Request

Keep the mirror equal to its source: after every accepted edit and on Open, and for edits
made on the mirror.

## What to do

- `halves::reconcile(doc, changed)` after `apply` (`core/src/lib.rs`) and on Open; inverse-map
  an edit that targets a mirror item to its source (as `move_linked_component` does,
  `core/src/matrix/layout.rs:542`).
- `SetHalfException {pairId, kind, mirrorId, state: local|omitted|linked}` for Make local,
  Omit and Rejoin; `UnlinkHalves {pairId}` leaves the mirror as plain independent data.
  Removing a source item removes its mirror.
- Keep `LayoutMirrorLink` for same-board pairs; refuse both links on one matrix; stop using
  `boardstudio.mirroredComponentSource` for new pairs.
- A hand-edited mirror in an archive is re-derived with a warning.

## Acceptance

- [ ] A matrix, key, part, module, outline-version or net edit from either half moves both,
      in one Undo step.
- [ ] A local or omitted item survives edits and reopen; Rejoin reflects the source again.
- [ ] Performance guard: `core/tests/matrix_performance.rs` still passes; record the added cost.

## Verification

Core tests, boundary parity native and WASM, `lint`, `test`. `impact` on `apply` and
`layout::resolve` is CRITICAL: list callers before editing.

## Carried from 17's verification

- Record validation runs on mechanical edits, ReplaceDocument and Open only. Re-validate after every
  accepted edit, and drop or repair correspondences and exceptions when an unrelated edit removes
  an item they name; otherwise every later mechanical edit is refused with a located message about
  a record the user never touched.

## Contracts from other tickets

- Companion placement mode and tweak per item ([31](31-companion-placement-modes.md)): use
  `halves.companionMode` (absent means `literal-mirror`) unless the correspondence/exception supplies
  `companionMode`. Modes are `literal-mirror`, `duplicate-position`, `literal-mirror-with-tweak`.
  `companionTweak: {x, y, rotation}` is additive in the **mirrored switch's** local frame, in mm/degrees,
  and is valid only with effective mode 3; absent means zero adjustment. Creation's optional
  `companionTweak` seeds each matrix-assembly companion's correspondence, never a pair-wide tweak.
  Keep each item's override and tweak through edits, reopen, Local/Omit/Rejoin, and correspondence
  rebuilds. Reconciliation must use the stored settings, not reapply the creation default to every
  item. Duplicate mode keeps the source's actual local offset and relative heading. The reflected
  matrix assemblies and stored parts must agree. `reuse-board` copies no companions and refuses an
  explicit creation tweak, which would have no item record to retain it.
- Labels ([32](32-mirrored-half-labels.md)): reconciliation never rewrites instance labels.

- Stabilisers keep their heading and companions fall back to the X mirror ([39](39-stabilisers-and-companion-fallback.md)); the mode is editable later through [40](40-set-companion-placement-operation.md), which needs reconciliation to re-derive the affected companions.

## Execution split and readiness gate

Status remains `ready-for-agent`, but this parent is blocked until **all 105–117 are
independently resolved** and root explicitly grants a fresh verifier the final parent
acceptance review. Root coordinates that gate; it does not verify its dispatched work.
The original acceptance above is unchanged. The Core children form one sequential execution
batch with exclusive Core file/API leases, with a Luna 6 high implementer followed by a fresh Sol 6.1 xhigh
verifier per slice. Root owns scheduling, integration, the full batch check and graph refresh.
No partial reconciliation is activated in production before 111 joins the verified slices.
The user approved independent A/F streams on 2026-10-10: audited disjoint browser-test/component edits may overlap
Core preparation under the [central registry](../lane-registry.md) and [parallel-streams decision](../parallel-streams-review.md).
All shell/check commands remain globally sequential; activation and timing require exclusive windows.
Active 105's original scope and verification remain unchanged; new code streams start after its verification.

- [105](105-mirrored-half-derivation-kernel.md): private derivation kernel, creation unchanged.
- [106](106-reconcile-mirrored-matrices-layouts-parts.md): matrices/layouts/parts and membership.
- [107](107-reconcile-mirrored-hosted-electrical-items.md): hosted items and electrical data.
- [108](108-reconcile-mirrored-outlines-assemblies.md): outlines/versions/assemblies.
- [109](109-mirrored-half-exceptions-unlink.md): durable exceptions and Unlink helpers.
- [110](110-inverse-map-mirrored-half-edits.md): private matrix/part inverse translation.
- [112](112-inverse-map-hosted-electrical-edits.md): hosted/electrical/keycap inverse translation.
- [113](113-inverse-map-reviewed-outline-edits.md): outline/reviewed geometry inverse translation.
- [114](114-mirrored-edit-compatibility-constraints.md): compatibility/constraints and exhaustive
  operation coverage; `needs-info` until root reviews representability/design.
- [115](115-prepare-mirrored-edit-transaction.md): inert draft composition and final validation.
- [116](116-prepare-mirrored-open-repair.md): inert Open repair/warning preparation.
- [117](117-prepare-mirrored-preview-history-performance.md): inert preview/history preparation
  and paired measurement/boundary harness.
- [111](111-integrate-mirrored-half-reconciliation.md): bounded final atomic activation, followed
  by a separate fresh verifier for this parent's full acceptance.

The prerequisite lanes and delivered 37 verification slice were integrated and checked at `12a6784d`;
remaining 37 acceptance is separate A work. Root refreshes `boardstudio-lane-mirror-core` and grants exact ownership.
Every child needs its own root-granted execution/check slots; completion is not permission
to dispatch the next writer automatically. Generated transcripts and demo goldens stay root-owned.

## Durable association and owned exceptions

Root approved this technical policy on 2026-10-10, preserving ADR-0008/0010 behavior:
local preserves visible geometry; omitted remains absent; Rejoin derives from current source;
linked mirror edits inverse-map to source, and source removal removes its linked mirror.

An exception retains durable kind/source/mirror association even when omitted geometry is absent.
It affects the selected object and only the projection that object owns. Matrix-owned
cells/companions and embedded-circuit-owned members follow the owner exception. Independent
layout components, module host connectors, joined host nets and unrelated correspondences
retain their states. Store one owner exception rather than automatically making neighbors
local; preserve explicit child exceptions when rejoining the owner. Source removal follows
existing domain deletion closures and cleans only records for objects actually removed.

Unlink materializes effective linked mechanical data through the existing pair-dissolution
policy, removes the relationship, and preserves stored board data and visible placement.
See 109 for representation/compatibility tests and source evidence. These are technical
contracts, not permission to change heading/tweak semantics pending the separate ticket 40 decision.
