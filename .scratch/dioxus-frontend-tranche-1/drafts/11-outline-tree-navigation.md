# 11: Select outline versions and locate bridges

**What to build:** The Layout tree exposes the active board’s outline, Generated and saved versions, and applicable bridges; users can activate a version and locate a bridge in the real canvas.

**Blocked by:** 10: Browse and select the Layout object hierarchy.

**Status:** draft — awaiting breakdown approval

- [ ] Render reference outline/version labels and active/fixed status with independent disclosure. Show only applicable bridges, including their matrix occurrences without creating duplicate bridge identities.
- [ ] Wire Generated/saved-version activation through the existing authoritative outline edit/history path and show the resulting board shape and selected outline context. Preserve part-selection clearing and supported save/reload/Undo effects from the reference; do not claim version activation is revision-neutral.
- [ ] Bridge selection selects the actual bridge context and fits the visible bridge geometry through the existing camera path. Disclosure alone changes no document state; the camera may intentionally move to locate a bridge.
- [ ] Provide a real usable selection/inspection result, not a dead link to an unfinished editor. Full outline construction/refinement/settings forms remain F3.4; this ticket includes only the minimum context presentation needed for the promised navigation.
- [ ] Reject stale/missing/cross-board targets, use real generated/fixed/bridge fixtures, and verify public keyboard/focus plus active-outline output. If existing private composition cannot deliver this bounded behavior, record the concrete dependency before dispatch rather than claiming an existing Dioxus outline editor.

- [ ] Complete the tranche’s shared acceptance and independent review; record refactoring takeaways or explicitly record none observed.
