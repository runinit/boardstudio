# 39: Stabilisers keep their heading, and companions fall back to the X mirror

Status: ready-for-agent
Type: build
Blocked by: [31](31-companion-placement-modes.md), [32](32-mirrored-half-labels.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md#amendment-stabilisers-fallbacks-and-editing-the-mode-2026-10-10)

## Request

User, 2026-10-10 (answering the ticket 31 verification follow-ups): do not rotate stabilisers; there should be no unmaterialised
parts, and the default is the X-coordinate mirror.

## What to do

- Stabilisers (the imported `STAB_MX_*` matrix members) keep their source heading in every companion mode (literal mirror, duplicate,
  tweak) and only their position is mirrored on X; a tweak's rotation never applies to them. Tests at relative heading 0, 90 and 15 degrees under an
  axis map and a rotated map, mutation-checked.
- If a cell's source part is not materialised, mode 2 and mode 3 fall back to the X mirror of the switch's companion position instead of
  returning an error. Add a test that builds such a cell; the fallback must not hide a real validation error (a missing switch is still an error).
- Never change a generator parameter, `reversible` or `side`.

## Acceptance

- [ ] Stabiliser headings unchanged in all modes; fallback test passes; the earlier ticket 31 tests unchanged.

## Verification

`repo lint test typecheck`; wasm boundary parity only if the transcript changes.
