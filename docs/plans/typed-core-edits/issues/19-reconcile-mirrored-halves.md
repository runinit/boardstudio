# 19: Reconcile mirrored halves on every edit, and exceptions

Status: ready-for-agent
Type: build
Blocked by: [18](18-core-create-mirrored-half.md), [26](26-mirrored-half-linked-mechanical-and-unchanged-parameters.md), [27](27-mirror-modules.md), [29](29-mirror-switch-and-companion-headings.md), [30](30-independent-parts-use-the-full-heading-rule.md), [31](31-companion-placement-modes.md), [32](32-mirrored-half-labels.md)
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
