# F1: Reference shell and theming

**Status:** implemented and verified first increment; ready for user review
**Blocked by:** M1 implementation
**Category:** frontend behavior-preserving port; F1 intentionally replaces the M1 demo layout with the reference shell and authorized temporary placeholders.
**Reference:** React 5a472a94; Rust base f0ac0a19.
**Authority:** [parent spec](../spec.md), [roadmap](../../../docs/migration/DIOXUS-FRONTEND-V1.md).

## Work

Project menu, exact six workflow tabs and Export; Objects/canvas/Inspect/footer; light/dark/system; compact navigation. Wire existing Layout/Case/exports, honest placeholders elsewhere.

## Exit evidence

Desktop/compact/theme comparison, keyboard navigation, menu, edit/history retention and existing Case/export paths.

- [x] Port all scoped TSX/UI-hook behavior through existing service contracts.
- [x] Match reference visuals, themes, responsive and interaction states.
- [x] Retain affected checks/browser evidence and resolve independent reviews.
- [x] Update inventory, integrated source, demo and remaining limits.

A visible placeholder does not complete F2–F9. Preserve original projects and
reference source. Backend implementation/format/API changes require separate
scope; never silently remove a frontend behavior when an adapter is missing.

## First-increment requirements

Use exact labels Layout, PCB, Keymap, Keycaps, Case, Parts and separate Export. Project menu retains fixture/saved/import controls; initial no-document state remains usable. Objects contains board/instance and keyboard component navigation; Inspect contains position editing. Preserve stable selection IDs, pointer callbacks, numeric drafts and history. Include Light/Dark/System with reference tokens and browser preference. Layout, Case and existing exports stay functional; other workspaces show honest placeholders and a return path. Desktop uses viewport panels and footer; compact uses reachable navigation/panels without horizontal page overflow. No new public Rust visibility or document/schema changes.

## Reference ownership

The [file ledger](../evidence/tsx-inventory.json) pins exact reference hashes. These
are source responsibilities, not permission to mark whole shared files complete.

- `app/src/main.tsx`
- `app/src/ui/Workbench.tsx`

F1 covers shell composition/navigation/themes only. Project management remains F2;
Workbench tree/canvas/inspector and workspace orchestration continue in F3–F8.

## Result

Executable source `24218450`; [handoff and evidence](../evidence/handoff.md).
All six reviewed shell fixes are resolved. This closes the bounded F1 increment;
full reference panel/canvas/workspace parity and actual AT remain later gates.

## Current shell frontier — 2026-10-03

The first-increment checkboxes above are historical bounded completion, not full frontend-shell acceptance. The current paired [Objects receipt](../evidence/candidate-34768-20261003/objects-pane-receipt.md) removes the ~159 px extra-control tree displacement and qualifies desktop/compact menu dismissal and the compact Objects→Inspect selection handoff. It records remaining compact shell mismatch: separate panel-toggle row, hidden separate Export action/extra Export selector entry, and inline panels instead of the reference topbar toggles/drawers. Preserve these as required parity work under F1; do not silently call compact UI complete. Generic menu button surfaces are corrected in queued source `c56522c4`, not in served34768. Necessary frontend API/design corrections are already authorized in CONSTRAINTS.md; the old bounded-increment restrictions do not introduce a new approval round.

### Compact shared-shell parity continuation

Use the pinned React source `5a472a9426e6e38993361da402cd4ec730feb369` as the visual and interaction oracle. At compact widths, place Objects and Inspect toggles in the topbar beside the workspace selector and standalone Export action; keep the workspace selector limited to the six workspaces except while Export is active. Opening either panel shows a side drawer over the workspace, closes the other drawer, and leaves the opposite panel's unsaved local draft intact. Retain the existing panel mode/width preferences and Session/Core ownership.

- [ ] At 720×640, the topbar exposes Objects, Inspect, the workspace selector, and a separate Export action; no always-present Export selector option or second panel-control row appears.
- [ ] Objects and Inspect open mutually exclusive side drawers above the canvas; closing by toggle, close control, Escape, or drawer scrim returns focus to the corresponding topbar control where applicable.
- [ ] Switching from Objects selection to Inspect preserves accepted selection and any panel-local draft; panel presentation changes do not submit Session/Core edits or change history.
- [ ] Keep desktop pinned/autohide/collapsed behavior and persisted side/width preferences. Match the reference's independent compact thresholds: Objects at 980px and Inspect at 820px; workspace selector at 1050px and six-tab desktop navigation above it.
- [ ] Reuse the existing paired desktop/theme/workspace evidence. Run one changed paired compact Objects→Inspect/export journey after integration; do not expand to a broad viewport/AT matrix absent a failure.
