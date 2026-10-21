---
status: accepted
---

# Decouple firmware role from physical side

Either physical half may be central when its hardware and selected transport support that role. Left/right identify physical position; central/peripheral identify communication responsibilities. Changing role must not require renaming or mirroring a PCB, swapping physical halves or changing their logical keymap positions merely to satisfy a side convention. The user confirmed this policy on 2026-10-20, choosing explicit role selection over the left-central/right-peripheral restriction.

Role changes must still respect actual assembly connections, controller capabilities and transport constraints; this is not permission to invent unsupported wiring. [ADR-0024](0024-share-reversible-pcb-routing-not-half-firmware-configuration.md) continues to bind each half's firmware mapping to its assembly and jumpers. Firmware target selection and physical/keymap ordering must not be conflated.

This is intended behavior, not delivered implementation. [Runtime firmware export](../../web/crates/runtime/src/runtime.rs), `firmware_bytes`, currently rejects split assemblies unless the central is labelled left and the peripheral right. The [topology research](../research/2026-10-19-hardware-topology-adr-gaps.md#contradictions-and-policy-questions) identifies that coupling; this decision replaces it rather than presenting it as a permanent product constraint.
