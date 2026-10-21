---
status: accepted
---

# Share reversible PCB routing, not half-specific firmware configuration

Two physical halves using one reversible PCB share its design and copper routing, not necessarily its assembled signal-to-MCU connections or effective firmware configuration. Each half's firmware must reflect its keymap positions and MCU pin mapping, including the solder-jumper configuration needed for its assembly orientation. Those mappings must agree with the actual assembly; this is not permission for arbitrary firmware pin overrides disconnected from the PCB. The user established this distinction on 2026-10-20.

This keeps one manufacturing design without forcing identical firmware configuration on both physical instances or copying the PCB solely to express assembly differences. A half-specific keymap or jumper-derived pin mapping must not silently rewrite shared copper. It extends [ADR-0010](0010-mirrored-half-is-a-core-maintained-reflection.md)'s reused-board identity and [ADR-0023](0023-distinguish-mounted-daughterboards-from-embedded-circuits.md)'s circuitry ownership; it does not choose a new persistence schema or require separately authored firmware projects/binaries where the firmware target can express both halves.

Current implementation evidence: [electrical resolution](../../core/src/electrical.rs), `resolve_jumpers` and `resolve`, derive population-face jumper recipes and terminal-to-firmware-GPIO aliases per selected instance. However, [electrical configuration](../../core/src/model.rs), `ElectricalBoardConfiguration`, still stores explicit jumper states and key-position bindings per Board. This confirms the intended product boundary, not complete support for independently persisted per-half configuration. The [topology research](../research/2026-10-19-hardware-topology-adr-gaps.md#1-board-electrical-authority-versus-physical-population-and-firmware-identity) distinguishes these implementation facts from unrecovered historical rationale.
