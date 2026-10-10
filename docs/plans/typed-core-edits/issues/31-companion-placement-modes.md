# 31: Companion placement modes for mirrored switches

Status: claimed
Type: build
Blocked by: [29](29-mirror-switch-and-companion-headings.md), [30](30-independent-parts-use-the-full-heading-rule.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md#amendment-companion-modes-pads-labels-2026-10-10)

## Request

User, 2026-10-10: a switch's diode and LED on the mirror have three placement modes: (1) literal mirror,
the default; (2) duplicate position, the same side as the source (what the shipped right halves do); (3) literal
mirror with an offset or tweak. Ticket [29](29-mirror-switch-and-companion-headings.md) left this open and
documented Core's current behaviour (literal mirror: local x -7.40 against the shipped +7.40).

## What to do

- Store the mode: a per-pair default (`companionMode`) and a per-item override on the correspondence or exception
  record (extend the ticket 17 `halves` types; absent means the default, so v5 documents stay valid), plus an
  optional tweak `{x, y, rotation}` in the switch's local frame for mode 3. Validation: mode names, finite tweak,
  tweak only with mode 3.
- `CreateMirroredHalf` takes the default mode in the request (default literal mirror) and applies it to every
  switch companion (diode, LED, RGB LED) at creation. Mode 2 places the companion at the source's local offset and
  heading relative to the mirrored switch; mode 3 adds the tweak to the literal mirror.
- Tests per mode with the Corne and the Sofle RGB fixtures: exact poses (1e-9), a rotated pair map, one Undo step,
  invalid combinations refused without mutation. Mutation-check each mode.
- Ticket 19 (reconcile) must keep each item's mode and tweak; note the contract in ticket 19.
- Never change a generator parameter, `reversible` or `side`.

## Acceptance

- [ ] Three modes implemented, stored and validated; literal mirror is the default.
- [ ] Shipped-fixture mirrors under mode 2 reproduce the shipped right-half diode placement within 1e-9.

## Verification

`repo lint test typecheck`; wasm boundary parity if the transcript changes. Part of the batch after Batch A.
