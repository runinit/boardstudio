# 29: Mirror switches on X only and choose companion headings

Status: ready-for-agent
Type: build
Blocked by: [18](18-core-create-mirrored-half.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md#amendment-key-switches-and-their-companions-2026-10-10)

## Request

User, 2026-10-10: a mirrored key switch only mirrors on X. It must not take the 180-degree
turn of the general rule, or its LED would land on the wrong side. The diode and LED that
belong to the switch can be rotated, per variant (an RGB LED variant may be turned).

## What to do

- In `CreateMirroredHalf` (and, with 19, the reconcile step) give a key switch the mirrored
  position and the mirrored tilt `-r`, with no 180-degree turn, for switches in any matrix
  cell. Tests: a switch at 0, 15 and -20 degrees keeps its LED side and mirrors its tilt;
  mutation-check against the general `180 - r` rule.
- Companion diode and LED: mirrored position; heading chosen per variant (plain diode,
  single-colour LED, RGB LED). Start from the shipped recipes' placement of the right half
  and report any difference. Core adds a finding when the heading is uncertain.
- Never change a generator parameter, `reversible` or `side`.
- The uncertain-heading finding for non-symmetric footprints at a non-multiple of 90
  degrees (from the earlier amendment) must not fire for switches.

## Acceptance

- [ ] Switch headings follow the rule for every shipped split keyboard's mirrored half, by correspondence, within 1e-9.
- [ ] Diode and LED headings per variant, tested; uncertain cases produce a finding.
- [ ] The earlier general-rule tests (TRRS left, right, up, down; off-axis LED) still pass.

## Verification

Core tests, `repo lint test typecheck`, wasm boundary parity when the transcript changes.
