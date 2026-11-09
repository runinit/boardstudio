---
status: accepted
---

# Decouple firmware role from physical side

Either physical half may be central when its hardware and selected transport support that role. Left/right identify physical position; central/peripheral identify communication responsibilities. Changing role must not require renaming or mirroring a PCB, swapping physical halves or changing their logical keymap positions merely to satisfy a side convention. The user confirmed this policy on 2026-10-20, choosing explicit role selection over the left-central/right-peripheral restriction.

Role changes must still respect actual assembly connections, controller capabilities and transport constraints; this is not permission to invent unsupported wiring. [ADR-0024](0024-share-reversible-pcb-routing-not-half-firmware-configuration.md) continues to bind each half's firmware mapping to its assembly and jumpers. Firmware target selection and physical/keymap ordering must not be conflated.

## Amendment: delivered for generated firmware (2026-11-06)

[Ticket 020](../plans/ui-integration-recovery/issues/020-split-firmware-either-half-central.md) delivers this decision, closing the gap [ticket 016](../plans/ui-integration-recovery/issues/016-audit-adr-decisions.md) found. [Runtime firmware export](../../web/crates/runtime/src/runtime.rs), `firmware_bytes`, now accepts a split with one central and one peripheral half on opposite sides. Shield names keep identifying physical sides (`boardstudio_left`, `boardstudio_right`); the central role flags and Kconfig follow the role. Logical key positions, sensor order, matrix row offsets and the keymap run left half first whichever half is central, so swapping roles changes the role flags and each half's transport-direction settings, not key or sensor positions. Two central halves, two peripheral halves, or two halves on one side are rejected with an error naming the role or side problem. Left-central output is unchanged, except that a right-central README states the central half. The [topology research](../research/2026-10-19-hardware-topology-adr-gaps.md#contradictions-and-policy-questions) identified the side/role coupling this replaces.

Evidence limits: wireless export is exercised through the real Runtime and Core; the wired-UART path is covered by one adapter test and Core tests that build the request directly. ZMK's documentation states left-central is a convention and sets the central role per shield by Kconfig, and describes wired split as experimental. No transport or hardware combination is rejected beyond the existing electrical-plan readiness checks, and no real hardware has verified either central side, wired UART direction included.

## Evidence note (2026-11-08)

The earlier wired-path evidence limit is superseded: wired UART and encoder-bearing
right-central exports now pass through the real Runtime export path in
`split_export_wired_right_central_preserves_physical_layout_and_uart` and
`split_export_encoders_stay_physical_left_then_right_with_right_central`; see
[ticket 020's completion evidence](../plans/ui-integration-recovery/issues/020-split-firmware-either-half-central.md#answer).
The experimental wired-split and no-real-hardware-verification caveats remain unchanged.
