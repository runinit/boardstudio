# 56: Review the three rebuilt Sofles and obtain user sign-off

Status: ready-for-human
Type: task
Blocked by: none
Parent: [25](25-shipped-boards-review.md) · Coordination: [23](23-redo-keyboard-04-on-native-halves.md)

## Purpose and ownership

The user requires one reviewable Sofle project with v2, RGB, and Choc available as Inspector assembly choices, followed by a human sign-off before remaining boards proceed. This ticket owns that single artifact review and its assembly-swap evidence. It is a child of the board review, but it does not wait for all of ticket 25 to finish and has no dependency on the retired per-variant tickets 49, 52, or 55. Shared mirror/link behavior remains separately required under 19/22; this review does not duplicate it.

## Review packet

Capture one ordinary saved Sofle project authored through the existing Core/Archive/Artifact JSON request examples and common user operations. Record the saved artifact identity and the existing Inspector assembly choices. Demonstrate a reviewable swap between v2, RGB, and Choc choices and verify the resulting saved project through normal reopen/roundtrip operations.

Retain only genuine evidence gates: source and library identity, terminal mapping, geometry, electrical qualification, both-side physical fit, mounting and clearance evidence, and explicit limitations. Reuse sufficient current evidence and request a focused missing check rather than repeat a full batch. Successful serialization/export does not establish physical fit or electrical qualification.

The user reviews this concrete Sofle artifact and assembly-swap evidence and records approval to proceed with remaining boards, including any bounded accepted limitation. An unresolved evidence gap or declined sign-off keeps this gate open. No recipe, code, or golden work belongs here.

## Acceptance and verification

- [ ] One ordinary saved Sofle project presents v2, RGB, and Choc as Inspector assembly choices.
- [ ] Assembly-swap evidence and normal save/reopen behavior are recorded with artifact identity and current source pins.
- [ ] Remaining electrical and physical qualification gaps are explicit; no fit claim follows from saved data or export success.
- [ ] The user reviews the artifact and evidence and explicitly signs off proceeding with remaining boards. Record date, reviewed artifact/outputs, and any bounded accepted limitation.
- [ ] Tickets 57 and 79 remain gated on this sign-off. Ticket 23 reuses this evidence and retains cross-board review and integration obligations.

Docs-only repo for this ticket's record; schedule any needed browser/export run sequentially through the foreground owner. A substantive defect remains open and is routed to the smallest shared fix, not a board-specific workaround.
