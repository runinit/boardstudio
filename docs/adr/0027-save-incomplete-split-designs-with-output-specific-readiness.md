---
status: accepted
---

# Save incomplete split designs with output-specific readiness

A split keyboard may be an unfinished design: allow saving, reopening and continued editing when a half, controller or required hardware assignment is not yet present. Report what is missing and gate only operations that depend on it, such as firmware export requiring a complete split configuration; do not block project saving or unrelated valid outputs. The user confirmed both incomplete-design persistence and output-specific readiness on 2026-10-20, choosing incremental design over requiring a finished assembly before saving.

An unfinished design is not the same as a corrupt file. Keep structural integrity, input safety and [ADR-0020](0020-validate-portable-project-packages-in-core.md)'s archive validation; never fabricate missing hardware or claim an incomplete configuration is ready for an output that requires it. This extends [ADR-0001](0001-fixed-edited-outlines.md)'s distinction between editing/saving and dependent-output blockers into hardware topology. [ADR-0026](0026-decouple-firmware-role-from-physical-side.md) governs role eligibility independently of physical left/right.

The [topology research](../research/2026-10-19-hardware-topology-adr-gaps.md#contradictions-and-policy-questions) records single-instance split fixtures and stricter firmware requirements. Those are bounded implementation facts, not proof that every incomplete configuration already saves or that every output has correct readiness handling. This record establishes intended behavior, not implementation completion.
