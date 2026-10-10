# 41: Inspector controls for the companion placement mode

Status: ready-for-agent
Type: build
Blocked by: [40](40-set-companion-placement-operation.md), [21](21-mirrored-half-web-adapter.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md#amendment-stabilisers-fallbacks-and-editing-the-mode-2026-10-10)

## Request

User, 2026-10-10: the Inspector panel edits the mode, supporting an individual part, a row or column, a matrix, and several selected parts.

## What to do

- In the Layout Inspector, when the selection includes parts or keys of a mirrored half, show **Companion placement** with
  literal mirror (default) / duplicate position / mirror with tweak (x, y, rotation fields), reflecting mixed values for a multi-selection.
- Build the `targets` list from the selection: one part, several parts, a row or column of a matrix (from the matrix selection), a whole
  matrix, and the pair default. Submit `SetCompanionPlacement` as a typed intent through the edit ticket path; a pending edit keeps the draft value
  (ADR-0005); stabilisers show the control disabled with the reason.
- Follow DESIGN.md for controls and copy; read it before changing the panel. Mounted tests for each selection kind, a mixed selection,
  Undo, and a refused edit showing the inline message.

## Acceptance

- [ ] Each selection kind changes the right companions only; Undo works; mixed values display correctly.
- [ ] DESIGN.md gets one sentence only if a new visible control pattern is introduced.

## Verification

`repo lint typecheck browser`; mounted tests use the shared `wait_until` helper.
