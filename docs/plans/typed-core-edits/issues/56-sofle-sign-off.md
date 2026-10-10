# 56: Review the three rebuilt Sofles and obtain user sign-off

Status: ready-for-human
Type: task
Blocked by: [49](49-sofle-v2-native-mechanics.md), [52](52-sofle-rgb-native-mechanics.md), [55](55-sofle-choc-native-mechanics.md)
Parent: [25](25-shipped-boards-review.md) · Coordination: [23](23-redo-keyboard-04-on-native-halves.md)

## Purpose and ownership

The user explicitly requires Sofle v2, RGB and Choc fixed/independently verified first, then user sign-off before remaining boards. One reviewer/planner owns this ticket's evidence and sign-off record; the user supplies approval. No recipe/code/golden work belongs to this gate. Do not resolve parent 25 or keyboard 04 here.

## Review packet

Collect the current source/commit pins, native assembly/peripheral/mechanical Outcomes from tickets 47–55 and hardware answers in 46. Reuse sufficient current verification evidence; request a missing targeted pass rather than repeat the full batch:

- Per variant: measured-key/native-profile correspondence, one authored half and native mirror, approved companion modes, independent-part headings, side/parameters, source TH1–TH7 versus generated mounts, real component/pin/datum evidence and explicit exceptions.
- Actual PCB, plate and case output, selected mount/profile and updated numerical closure/component/switch/stabiliser/access clearances on both halves. Keep physical fit and electrical qualification separate from successful generation.
- Before this sign-off, the coordinator dispatches a fresh Sol 6.1 xhigh verifier for a targeted Sofle mounted pass covering all three rebuilt variants: live edit from both halves, local/omitted exception where deliberately supported, Undo/Redo, and portable save/reopen. Record the exact commits, commands, results and artifacts in this ticket. Schedule its browser/export work through the global slot; missing evidence keeps 56 open. This pass is owned here and does not wait for final ticket 23. Ticket 23 reuses current Sofle evidence and retains the final cross-board mounted-browser, Standards/Spec and full-check obligations.
- Remaining findings with severity, exact owner/ticket and user-visible limitation. An unsupported hardware claim or failed agreed clearance keeps the gate open.

## Acceptance and verification

- [ ] Fresh Sol verifiers have resolved 47–55 against their own acceptance, including meaningful mutation checks; relevant generated goldens were produced by the assigned owner.
- [ ] The targeted pre-sign-off Sofle mounted pass above has completed independently and its evidence is recorded here; final ticket 23 is not a prerequisite.
- [ ] User reviews the concrete three-variant packet and explicitly signs off proceeding with the remaining boards. Record date, reviewed commits/outputs and any bounded accepted limitation. Agent judgement and elapsed time cannot stand in for this approval.
- [ ] Later-board dispatch remains blocked on this ticket. Any new requirement becomes its own ticket before dispatch; it does not widen a running Sofle implementer.

Docs `repo`; any missing browser/export check is scheduled sequentially with the orchestrator's global slot. Full check remains one batch operation under 23/integration. If a substantive defect remains, keep this gate open and file a bounded fix rather than begin Corne.
