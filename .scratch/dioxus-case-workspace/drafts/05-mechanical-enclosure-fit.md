# Draft F7.4c: Configure enclosure, supports, and fit constraints

**Parent:** F7.4 — Mechanical assembly configuration editor.

**What to build:** A designer can configure the supported access openings, battery envelope, mounts, closure hardware, gasket settings, suggested mount adoption and critical-fit dimensions for the selected mechanical assembly, and inspect the current resolved fit information without inventing geometry behavior.

**Blocked by:** INT.1 only, matching F7.4's canonical `start_after`. Use existing services and saved fixtures; F7.3 viewer implementation and F5 completion are not start blockers for these form controls.

**Parent acceptance join:** INT.2 remains required for F7.4 integrated acceptance. Viewer-handle edits remain F7.5 and are not accepted by this form ticket.

**Status:** draft for independent review; not published or counted.

- [ ] Edit and remove document-coordinate access openings and their supported polygon vertices, Z and height; preserve minimum three vertices and source defaults for adding a rectangular contour/volume.
- [ ] For a configured battery envelope, edit size, position, cable exit and cable width; retain the source default envelope when first enabled. Preserve its wireless guidance and the source's represented transport-dependent display; do not infer hardware capability beyond current document state.
- [ ] Add/remove/edit supported suspension holes/bosses and closure screws with stable mount IDs, locations and represented diameters/heights. Honor mount-style conditions (suspension controls hidden for gasket mount; closure mount add restrictions when internal gasket applies). Preserve current suggested-location behavior: candidates come from the accepted mechanical assembly, adoption is explicit, and generated closure screws use the existing defaults/IDs path.
- [ ] Expose current internal-gasket support layout, auto-size/manual lengths, supported presets, width/thickness/compression/travel, advanced clearances/material, closure hardware specifications and critical-fit dimensions. Preserve paired/unlinked support state and service-provided errors/findings; do not implement pointer dragging or change the existing gasket/support algorithms.
- [ ] Keep numeric controls as per-field drafts: source blur/Enter commit, Escape restores accepted value, invalid/out-of-range values do not persist, and one edit produces one normal history operation. Scope drafts and async operation feedback to accepted Scope/token/revision, instance, configuration board, field and component lifetime; never apply a stale complete configuration. Admit only under current instance-selection guard, Ready/no-preview/no-gesture/Saved-revision checks and single-flight edit handling; preserve failed drafts and retry feedback.
- [ ] Persist configuration through existing `SetMechanical` or current physical-instance configuration/replace-document path; patch the latest accepted config after prior acknowledgment so edits to independent controls cannot overwrite each other. Preserve other board/instance settings and authored Case body records.
- [ ] Paired public browser evidence covers opening/battery forms, mount and hardware CRUD, conditional controls, suggested candidate empty/adopt flows, gasket settings and paired/unlinked data preservation, validation/failed/busy drafts, cross-board/configured-instance changes, native accepted payload/archive save, Undo/Redo and save/reopen. Check generated geometry/findings through existing resolution only; do not duplicate resolver assertions as UI logic.
- [ ] **Ownership:** a private enclosure/fit settings component and feature-local styles only. Coordinator owns mount and operation outcomes; F7.4a owns mechanical configuration shell/common commit callback; F7.4b owns profile editor; F7.5 owns direct manipulation; F7.2 owns authored body controls.
- [ ] **Profile:** Luna High author and verifier; Astra independent reviewer because nested configuration patches, mount identity and selected-instance lifecycle must compose with the existing Runtime guard.
- [ ] **Refactoring handoff:** No new refactoring takeaway observed in this source pass. Preserve RF-006 and RF-001 evidence; update only if implementation demonstrates a separate architectural finding.

