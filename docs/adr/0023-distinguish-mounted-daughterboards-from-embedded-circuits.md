---
status: accepted
---

# Distinguish mounted daughterboards from embedded circuit copies

A mounted daughterboard is a separate physical PCB: its constituent footprints and copper remain source-owned and do not become host fabrication contents merely because the daughterboard is displayed. An embedded circuit copy instead creates independently owned host components and internal nets, retains source provenance, and joins host signals explicitly. The user confirmed this boundary on 2026-10-19, choosing explicit ownership and connections over an ambiguous operation that conflates mounting a board with copying its circuit.

Embedded copies must not accidentally share internal component or net identities. Missing host joins or incompatible power rails produce findings rather than guessed connections. Imported, previewable and fabrication-ready remain separate claims. This does not impose new blanket restrictions on editing source variants; [ADR-0010](0010-mirrored-half-is-a-core-maintained-reflection.md) continues to govern mirrored instances and reflection.

Evidence: [component onboarding ownership and acceptance rules](../hardware/component-onboarding.md#onboarding-workflow), [mounted/embedded ownership regressions](../../core/tests/mounted_modules.rs) and [explicit electrical-join regressions](../../core/tests/module_circuit_electrical.rs). These establish existing behavior; the original selection rationale was not fully recovered, so this record documents the present confirmation rather than inventing an earlier comparison.
