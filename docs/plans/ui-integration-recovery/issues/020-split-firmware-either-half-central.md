# 020: Export split firmware with either half as central

Status: ready-for-agent
Type: build
Blocked by: none
Parent: [001: Recovery specification](001-ui-integration-recovery-spec.md)
Related: [016: Audit ADR decisions](016-audit-adr-decisions.md), [ADR-0026](../../../adr/0026-decouple-firmware-role-from-physical-side.md), [ADR-0024](../../../adr/0024-share-reversible-pcb-routing-not-half-firmware-configuration.md), [Plan map](../map.md)

## What to build

Make split firmware export honor ADR-0026: either physical half may be the central
when its hardware and the selected transport support it, without renaming or
mirroring a PCB, swapping halves or changing logical keymap positions. The user
asked on 2026-11-06 for this to be updated after 016's initial audit found the gap.
ADR-0026 itself says this is intended behavior, not delivered.

Source facts at the time of writing, to re-verify:

- `web/crates/runtime/src/runtime.rs` (`firmware_bytes`, around lines 6753-6764)
  rejects split export unless the `central` instance is half `left` and the
  `peripheral` instance is half `right`.
- Core's generator is positional, not physical: `core/src/firmware.rs` names the
  central shield `boardstudio_left` and the peripheral `boardstudio_right`
  (around lines 497-580, 804, 844), adds the peripheral's keys after the
  central's rows with ids prefixed `right/` (around lines 479-495), and
  `firmware_request_adapter` passes primary then secondary. Removing the
  runtime guard alone would therefore put a physical right-half central's keys
  first and so change logical keymap positions, which ADR-0026 forbids.
- Which transports and hardware support an either-half central is not recorded
  here; ADR-0026 says "when its hardware and selected transport support that role".

## Acceptance criteria

- [ ] RED first at the real boundary: a split design whose central is the right
  half (and one whose central is the left half, as today) exported through the
  runtime path. The right-central case fails today with "Split firmware requires
  a left central and a right peripheral assembly".
- [ ] Either role assignment exports. Logical key positions, matrix pins and the
  keymap layout are those of the physical halves, not of the roles: swapping the
  central role must not renumber or reorder keys.
- [ ] Generated names, ids and files do not hard-code that left is central. Role
  and physical half are both recorded where a reader needs them, and the build
  matrix and Kconfig role flags follow the role.
- [ ] Unsupported combinations (a transport or hardware that cannot make the
  chosen half central) fail with a message that names the actual limitation, not
  a side convention. Record which combinations exist; do not invent support.
- [ ] Existing left-central output is unchanged for existing designs, or any
  difference is explained and approved by the user.
- [ ] ADR-0026's "intended behavior, not delivered" note is updated with a dated
  amendment when this lands; 016's 0026 finding is cross-referenced.

## Verification and completion

The new runtime-level tests plus the focused Core firmware and
`web/tests/firmware_request_adapter.rs` tests; record exact commands and results.
If implementation shows independent seams (runtime guard, adapter ordering, Core
generator naming, transport support) that need separate decisions, re-scope before
coding instead of widening this ticket.

## Comments

- 2026-11-06: The user confirmed that shield names identify physical sides:
  retain `boardstudio_left` and `boardstudio_right` and assign their role flags
  according to the selected central/peripheral roles. A split design assigning
  both halves as central must be rejected with a clear error. Preserve existing
  left-central output as required above. Hardware and transport support still
  need evidence; no new capability model is implied by this decision.
- 2026-11-06: Created from 016's initial audit finding at the user's request
  ("split firmware export needs to be updated"). Routed with `ask-matt`: the
  policy is already decided by an accepted ADR, so no grilling step; it looks
  like one session of work if transport support is already known, otherwise run
  `/to-spec` then `/to-tickets` first.
