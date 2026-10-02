# 08: PCB part input and generator connection controls

**Parent:** F5.3 — Manual pin review, nets, and protected handoff. Related acceptance join: F5.2 current accepted board/selection projection.

**What to build:** Selecting a reachable generic PCB part exposes the reference's contextual press-scan controls when the part definition has `inputProfile.press` or its generator source is exactly `ceoloide/rotary_encoder_ec11_ec12`; supported Ergogen net/anchor controls are shown only when the packaged parameter schema provides them. Changes update that part through the ordinary edit/history path. Preserve the reference routing: controller selection returns to board Wiring, and every `definition.kind === 'switch'` uses the inherited/read-only switch Inspector before press-scan controls are considered.

**Blocked by:** None for private implementation once accepted PCB part selection and the normal edit callback are proven. Public paired browser acceptance joins the F5.1 canvas selection route. This ticket is independent of ticket 07's terminal/pad assignment UI.

**Status:** implementation in progress; isolated source checkpoint only. Paired browser and parent acceptance remain open.

- [ ] Show the reference's press-input scan mode only for a non-switch/non-controller generic part with `inputProfile.press` or the exact EC11/EC12 generator source above. Preserve the reference's branch order so builtin switches always show inherited wiring/named terminals and controllers always show board Wiring; neither gets scan controls through this ticket. Matrix is disabled if no accepted matrix contains the part, or if the press profile forbids independent input. It is enabled only when a containing matrix exists and independent press input is not forbidden, matching the current TypeScript predicate.
- [ ] Allow the same Direct GPIO, Unassigned and eligible Matrix choices as the reference, and persist the supported per-part value through normal project validation/history.
- [ ] Show only currently supported Ergogen net and anchor parameters, preserving default, empty, finite-number, and existing-value behavior from the reference.
- [ ] Keep net parameter choices scoped to the selected board's accepted nets; keep anchor axes independently editable and omit empty coordinates as the reference does.
- [ ] Display the existing guidance that a changed press connection requires applying the board wiring plan. Do not present the per-part scan mode as a wiring-plan edit or silently auto-apply it.
- [ ] Verify each supported edit in paired React/Dioxus browser journeys, including visible selection, saved document property, Undo/Redo, save/reopen, stale selection rejection and no unrelated board/instance changes.
- [ ] Preserve the existing project schema and Core service behavior; do not add generator, firmware or input-scan APIs.
- [ ] Retain F5.3/F5.2 parent criteria and RF-001/RF-006/RF-009 source/build provenance.
