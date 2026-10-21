---
status: accepted
---

# Preserve PCB handoff protection across Undo

Once PCB handoff protection is established, ordinary Undo/Redo must not erase the recorded signal-assignment baseline or silently authorize different wiring. A handed-off design may already be routed or used outside BoardStudio; restoring an earlier project snapshot cannot undo that external knowledge. The user confirmed protection surviving Undo on 2026-10-20. Changing protected assignments requires an explicit reviewed remap rather than ordinary history restoration.

This is a narrow exception for the handed-off mapping, not a general prohibition on Undo or proof that a board was manufactured. Protect PCB signal-to-terminal assignments separately from the assembly-specific MCU aliases described by [ADR-0024](0024-share-reversible-pcb-routing-not-half-firmware-configuration.md). It does not change [ADR-0030](0030-export-firmware-as-an-editable-repository-template.md)'s firmware repository-template scope or [ADR-0016](0016-export-ownership-is-separate-from-preview-currency.md)'s export ownership rules.

## Protection begins before download delivery (2026-10-20)

The user confirmed that successful package creation, current captured ownership and
the saved protection commit establish the baseline before download delivery. A later
delivery failure or cancelled download does not remove that protection. This is a
conservative boundary: BoardStudio cannot reliably establish whether exported files
were saved or used elsewhere. Failed packaging or a stale owner must not establish
new protection; a protection commit that has not saved is not accepted, following
[ADR-0013](0013-save-before-publication-and-explicit-recovery.md). Establishing the
baseline is not proof of completed download or fabrication.

Evidence: [electrical protection](../../core/src/electrical.rs), `preserve_handoff`, retains prior protection; [Core history](../../core/src/lib.rs) reapplies it on Undo/Redo. [PCB packaging](../../web/crates/runtime/src/pcb_handoff.rs), `package_then_protect`, packages first, checks captured ownership, commits protection and then checks delivery. Its `failed_package_retry_and_stale_owner_use_session_protection_order` regression covers package failure, retry and stale ownership. These are inspected implementation facts, not newly executed test evidence.
