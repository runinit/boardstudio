# Draft F7.4b: Assign and customize mechanical part profiles

**Parent:** F7.4 — Mechanical assembly configuration editor.

**What to build:** A designer can see inherited and missing mechanical fit information for placed part types, assign supported library profiles or custom imported geometry to the selected Case configuration, inspect and edit the resulting profile, and recover from delayed service errors without applying a profile to a different board or instance.

**Blocked by:** INT.1 only, matching F7.4's canonical `start_after`. This child can use accepted saved fixtures and the existing profile/extraction services; no completed F4 feature or shared 3D viewer is required.

**Parent acceptance join:** INT.2 remains required for F7.4 integrated acceptance. This child cannot close F7.4.

**Status:** draft for independent review; not published or counted.

- [ ] Show the selected Case configuration's inherited `PartDefinition` profile and current inferred family by placed part type. Clearly distinguish reusable profiles owned/edited in Parts from Case-specific assignments and overrides; “Edit in Parts” navigates through the current callback and does not duplicate PartDefinition profile editing.
- [ ] Assign only existing library fit choices to an eligible placed definition: MX switch, Choc v1 switch, Choc v2 switch, MX stabilizer 2u and MX stabilizer 6.25u. Show loading, unavailable, empty and service error states; disable assignment until both a supported source and target are selected and no prior assignment request is pending.
- [ ] Assign a custom KiCad geometry profile only to an eligible placed definition with imported KiCad source; preserve stable definition ID and existing profile fields. Read source geometry through the existing extraction service, list the supported source primitives/purpose choices, and apply the selected mappings through the existing extraction result. Expose existing cutout/clearance polygons, access openings and clearance volumes, switch-family choices, derived plate-to-PCB distance and supported plate-thickness range.
- [ ] Preserve unsupported/missing profile states and the current “supplier review needed”/choose-family guidance. Do not create profiles, families, supported geometry, lighting, or library edits absent from existing sources; do not edit F4-owned reusable definitions from this Case surface.
- [ ] Correlate profile request, geometry-read/extraction result and edit feedback to full accepted Scope, component lifetime, accepted source token/revision, configuration board ID, definition ID and source geometry identity. A selection/board/instance/configuration transition or newer request suppresses the late result. Apply successful mappings as a patch against the latest matching saved configuration, not an old whole-configuration copy; preserve unrelated profiles and concurrent accepted fields.
- [ ] Admit persisted changes only under the current instance-selection guard, Runtime Ready, no preview/gesture, Saved accepted revision and single-flight edit guard. Use existing mechanical-profile/extraction services, `SetMechanical` or physical-instance update/replace-document path, and operation outcomes. On failure retain the selection/draft and show an actionable retry state; do not claim extraction success or commit an invalid result.
- [ ] Paired public browser evidence covers each available library profile and target selection, inherited/missing states, custom-source extraction and purpose mapping, polygon/opening editing, family selection and derived outputs, loading/failure/retry, stale source/scope rejection, busy edits, Undo/Redo, native persisted payload/archive save/reopen and selected physical-instance changes. Geometry results remain provisional until existing resolution/generation states say otherwise.
- [ ] **Ownership:** a private profile-assignment/extraction component and its local styles only. Coordinator owns root mount and scope/outcome callbacks; F4 owns reusable PartDefinition profile editing/library content; F7.4a owns root mechanical settings mount; F7.2/F7.3 keep their existing responsibilities.
- [ ] **Profile:** Luna High author and verifier; Astra independent reviewer due to nested profile state, asynchronous service results and physical-instance configuration scope.
- [ ] **Refactoring handoff:** No new refactoring takeaway observed in this source pass. Preserve RF-006 and RF-001 evidence; update only if implementation demonstrates a separate architectural finding.

