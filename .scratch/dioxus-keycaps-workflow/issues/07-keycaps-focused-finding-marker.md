# 07: Show the focused Keycaps finding marker in Layout

**Parent:** F6C.4 — Fit resolution, finding list/navigation, and stale-case state.

**What to build:** After following a Keycaps fit finding to Layout, the current accepted canvas shows the matching Core-provided contour with the same focused identity as the Inspector action. The highlight follows only its accepted board and finding, disappears when its owner or marker is replaced, and adds no project edit or history entry.

**Blocked by:** None as a ticket dependency. Start implementation after the issue05 accepted-navigation owner and delayed-focus/source guards have independent source clearance; the full issue05/F6C.4 journey is not a prerequisite.

**Status:** specification and ticket drafted; independent planning review required before implementation dispatch.

- [ ] Preserve the focused finding identity with its accepted session/document/board, token and revision owner; do not infer focus from the last tree selection or a finding label.
- [ ] Render the active-board `finding_markers` contour matching the focused finding from the current accepted scene. Use Core geometry as supplied, the existing canvas transform and focused marker styling; do not duplicate Core contour calculations.
- [ ] Keep the Inspector action, selected geometry and canvas marker associated with one finding identity through the Layout workbench transition and the delayed destination fit.
- [ ] Suppress the marker when the active board, accepted source owner, finding identity or scene marker no longer matches. A stale/pending/error finding stays readable under the existing fit-state contract but must not expose an old contour as current.
- [ ] Mounted production tests render two findings and boards, focus one, then exercise board change, source replacement, missing marker, same-scope selection replacement and navigation replacement. Verify exact marker ID/contour visibility, no wrong-focus carryover, no document revision/history mutation, and no camera/focus side effect from an obsolete deferred owner.
- [ ] Paired pinned-React/Dioxus browser evidence uses the same byte-identical retained finding fixture and source-stamped candidate. Record finding/target IDs, accepted owner, board, contour/class/DOM identity, camera, revision/history before and after, screenshots and browser errors.
- [ ] Keep Case 3D finding visualization, cross-board pending navigation, the original missing fixture identity, full issue05 acceptance, F6C.4/INT.2 joins, and shared RF ledger ownership explicit; this ticket closes none of them.
- [ ] Carry the accepted-scene marker reuse and the Editor-level owner/refactor observation into the coordinator's existing RF register without modifying shared run/RF ledgers.
