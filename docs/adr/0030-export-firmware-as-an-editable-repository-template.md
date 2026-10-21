---
status: accepted
---

# Export firmware as an editable repository template

Firmware export currently means a source handoff: an editable repository template with the keyboard's firmware configuration, dependency/build information and instructions for building outside BoardStudio. It is not a promise of compiled, ready-to-flash binaries. The user selected this scope on 2026-10-20, choosing a repository-template handoff rather than making BoardStudio own firmware compilation and its toolchain for now.

An included external build workflow or local build script supports the handoff; it does not make in-app compilation, repository creation/publication or flashing part of this decision. A future binary-building feature needs a separate scope decision. Generated source is not proof of physical hardware operation; output readiness and supported hardware mappings remain separate from the delivery format. Preserve [ADR-0024](0024-share-reversible-pcb-routing-not-half-firmware-configuration.md)'s per-half assembly mapping and [ADR-0026](0026-decouple-firmware-role-from-physical-side.md)'s independent side/role boundary.

Evidence: [current firmware generator and output tests](../../core/src/firmware.rs), `generate`, `exports_reproducible_local_build_commands_for_each_half` and `emits_pinned_manifest_and_compact_keymap`, produce source/configuration, a pinned dependency manifest and local/external build entry points. These are inspected implementation facts, not a fresh build, a pinned-version policy or proof that every target already supports the accepted hardware decisions.

## Amendment: unfinished optional-peripheral support (2026-10-20)

The user confirmed that an otherwise valid keyboard source handoff may proceed when
an optional peripheral lacks supported firmware configuration. Export the valid
keyboard configuration and explicitly identify the unsupported peripheral and the
manual work remaining in the handoff; do not silently omit its limitation, invent
drivers or pin assignments, or claim the peripheral is operational.

Contradictions that invalidate the base keyboard configuration still block export.
Do not relabel required hardware as optional to bypass readiness checks, and retain
[ADR-0027](0027-save-incomplete-split-designs-with-output-specific-readiness.md)'s
gates for outputs that require missing hardware. This admits a useful editable
template with disclosed limitations, not a promise of complete functional firmware.
Current `validate_half` can reject the entire export for unsupported attached
hardware; this amendment establishes intended behavior, not implementation completion.
