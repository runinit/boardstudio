# 41: Inspector controls for the companion placement mode

Status: claimed
Type: build
Blocked by: [40](40-set-companion-placement-operation.md), [21](21-mirrored-half-web-adapter.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md#amendment-stabilisers-fallbacks-and-editing-the-mode-2026-10-10)

## Request

User, 2026-10-10: the Inspector panel edits the mode. The later human clarification limits
this to **switch companions only**: an individual companion, companions in a row/column or
matrix, and several selected companions. Independent controllers/connectors keep the general
mirror rule and do not gain placement-mode controls.

## What to do

- In the Layout Inspector, for eligible switch companions or key/matrix selections that identify companions,
  show **Companion placement** with literal mirror (default) / duplicate position / mirror with tweak
  (x, y, rotation fields), reflecting mixed values for a multi-selection. Independent parts are excluded.
- Build the `targets` list for one/several companions, companions in a selected row/column or matrix, and the
  pair default. Normalize source-side or mirror-side selection to 40's `(kind, mirrorId)` companion identity;
  deterministically deduplicate overlaps. Submit `SetCompanionPlacement` through the typed edit ticket path;
  a pending edit keeps the draft value (ADR-0005). Stabilisers show the control disabled with the reason:
  their mirrored-X position and unchanged heading follow 39 in every mode.
- Pair-default tweak controls must follow 40's root-reviewed technical contract; do not introduce a pair-wide
  tweak or independent-part coordinate semantics in the adapter while that review is pending.
- Follow DESIGN.md for controls and copy; read it before changing the panel. Mounted tests for each selection kind, a mixed selection,
  Undo, a refused edit showing the inline message, and exclusion of independent controllers/connectors.

## Acceptance

- [ ] Each selection kind changes eligible companions only; independent parts are excluded, stabiliser controls are disabled, Undo works, and mixed values display correctly.
- [ ] DESIGN.md gets one sentence only if a new visible control pattern is introduced.

## Verification

`repo lint typecheck browser`; mounted tests use the shared `wait_until` helper.

## User decision (2026-10-14)

Present the control in a named disclosure section, **Companion placement**.
