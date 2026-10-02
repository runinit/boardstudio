# Draft F7.4d: Configure per-part manufacturing and stabilizer overrides

**Parent:** F7.4 — Mechanical assembly configuration editor.

**What to build:** A designer can add/remove per-part process overrides and select represented stabilizer-fit choices for the current mechanical configuration, while the existing resolver remains responsible for supported methods, materials, constraints and resulting fit diagnostics.

**Blocked by:** INT.1 only, matching F7.4's canonical `start_after`. Saved-fixture editor work does not wait for F7.3 or other F7 workflows.

**Parent acceptance join:** INT.2 remains required for F7.4 integrated acceptance.

**Status:** draft for independent review; not published or counted.

- [ ] For represented plate, plate-foam, bottom-foam and bottom targets, show whether an override is absent or configured; add/remove the override and expose only current method/material/thickness options. Foam remains cut-sheet; plate/bottom/foam thickness updates that map to shared configuration fields keep the standard process entry synchronized as the source does.
- [ ] Keep material choices conditional on process and target, preserve the existing material fallback when a method changes, and show the supported constraint-set version. An unsupported saved constraint version remains visible and reports that existing diagnostics block export; do not silently replace it.
- [ ] Show stabilizer choices only for current eligible wide-key candidates; offer the represented PCB-mount, none, and custom plate-mount option only when a custom profile is present. Size and orientation remain derived from existing key layout/profile data; do not add new stabilizer geometry or profiles.
- [ ] Correlate field drafts and operation feedback to accepted Scope, physical instance, mechanical configuration board, source token/revision, part ID and component lifetime. Admit only under the Runtime current-selection/Ready/no-preview/no-gesture/Saved-revision gate and single-flight operation policy; apply a field patch to the latest accepted configuration after acknowledgment. Failure retains the draft and reports current validation/error state.
- [ ] Use existing mechanical configuration edit/history and resolution services only; preserve unrelated config fields, selected-instance values, save/reopen and Undo/Redo. No new process/material/constraint schema, public API or geometry behavior.
- [ ] Paired public browser evidence covers override CRUD, conditional material/method/thickness choices, synchronized standard thickness, unsupported constraint feedback, stabilizer candidate and custom-profile states, stale/busy/failed field recovery, actual native accepted payload/archive, selected-instance transitions, Undo/Redo and save/reopen.
- [ ] **Ownership:** a private process/stabilizer settings component and local styles only. Coordinator owns root mount and operation outcomes; F7.4a owns config shell/common patch admission; F4 retains reusable profile authoring; F7.4b retains profile assignment; no edit to shared root files.
- [ ] **Profile:** Luna High author and verifier; Astra independent reviewer for scoped full-config edits and conditional resolver-backed controls.
- [ ] **Refactoring handoff:** No new refactoring takeaway observed in this source pass. Preserve RF-006 and RF-001 evidence; update only if implementation demonstrates a separate architectural finding.

