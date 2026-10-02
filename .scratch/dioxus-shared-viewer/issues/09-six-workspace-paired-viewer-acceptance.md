# 09: Paired viewer acceptance across six workbench streams

**Parent:** F7.8 cross-workflow viewer adoption and paired acceptance.

**What to build:** A reproducible public journey compares the same real saved project in React and Dioxus while entering Layout, PCB, Keymap, Keycaps, Case and Parts. It proves the existing viewer behaviors where each workflow uses them, while preserving the distinct canonical-board, physical-instance and isolated-sample authorities. This is an acceptance slice; it does not build a second viewer or replace each workflow’s own implementation ticket.

**Blocked by:** F7.8’s implementation prerequisites F7.3, F7.5, F7.6 and F7.7 and named workflow acceptance joins F3.6, F4.4, F6C.5 and F2.3. Six-stream acceptance additionally requires the existing PCB cross-workspace evidence F5.8; coordinate this as a F7.8 acceptance join with the task-graph owner. No capability-level implementation may treat this final paired suite as a start blocker.

**Status:** draft for independent Spec/Standards review and canonical graph reconciliation; this ticket does not alter the 62-task graph itself.

- [ ] Use a genuine saved project/archive in both public applications and record matching document/archive identity before actions. Similarly named demo copies without payload equality are exploratory only.
- [ ] Visit Layout, PCB, Keymap, Keycaps, Case and Parts via the public UI and complete each workflow’s current paired action journey. Include a viewer action for each actual viewer consumer and do not invent 3D behavior for a screen that has none in the pinned reference.
- [ ] Verify scene authority: Layout/Keymap/Keycaps use the canonical selected board; Case follows selected physical board/instance; Parts uses an isolated sample and cannot select or edit a live project part; PCB retains its own accepted board/workspace semantics.
- [ ] Across actual viewer consumers, compare available camera fit/presets/orbit/zoom, render modes, exploded/section views, layer/model visibility and colors, theme, mapped/empty picks, and loading/missing/error/retry states. Where a behavior is not represented in the React workflow, record “not applicable” with source evidence rather than creating a parity claim.
- [ ] Verify real mesh delivery from the actual producer and asset-byte provider. Capture exact model-row IDs and transforms; select only existing current-scope part/module/body identities. Record no-mesh, missing-byte and unmapped-pick conditions separately; an unregistered helper or decoder-only unit test is not end-to-end evidence.
- [ ] Change board, project, physical instance, Parts definition/companion/placement and document revision through supported UI actions. No prior scope’s selection, preference, draft, model result, error or camera owner may overwrite the new projection.
- [ ] Verify renderer failure/retry, cancellation, resize/DPR, focus, context loss/unmount and clean return to 2D while surrounding workflow controls remain available. Record compact/desktop and relevant theme/focus/accessibility evidence.
- [ ] Preserve the Case-specific generation/readiness/STEP checks owned by F7.6, direct manipulation owned by F7.5, physical-instance join owned by F7.7, and export-to-project joins owned by F8. Do not replace them with a generic multi-tab smoke test.
- [ ] Retain exact source/build/fixture, paired action trail, screenshots, accepted-document/history evidence, model/provider traces and all failures. Keep unperformed gates explicitly open.
- [ ] Update mandatory RF evidence for the slice (existing RF-001/RF-006/RF-009/RF-012 as applicable) or state “No new refactoring takeaway observed” for the reviewed surface.

**Canonical graph note:** F7.3 retains INT.2 and BND.1 acceptance joins, F7.7 retains F5.6, F7.8’s current joins remain unchanged pending coordinator reconciliation. The six-stream acceptance implies PCB evidence through F5.8; adding the F5.8 edge to F7.8 is a coordinator-owned graph decision. This child cannot close F7.3/F7.8 or any upstream workflow.
