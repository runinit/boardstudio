# F7.2 authored Case bounded receipt and criterion reconciliation

This packet records the remaining paired CRUD and cross-board mismatch journey on the served candidate. It is evidence for the open Issue 01 child, not F7.2/INT.2 or parent acceptance.

## Served candidate and inputs

- React reference source: `5a472a9426e6e38993361da402cd4ec730feb369`, `http://127.0.0.1:5173/`.
- Dioxus candidate source: `2107f980e9b45c341bff10548c7156b28de0e388`, `http://127.0.0.1:34761/` (also served under `/boardstudio/`). Root recorded provenance `2d218936e847ba8f4d8680e5ecc4b3f5a05c3ce88c85a2152e2397cd84020ac9`; combined affected strict check passed before serving.
- CRUD fixture: `empty-keyboard-public-export.boardstudio`, SHA-256 `1ed5423b426de67eee93267eb5d699590128334aa8967a029eb8ffa0ac11fcdb`. It was imported through the visible file picker into both task-owned profiles.
- Mismatch fixture: the same empty-keyboard public archive, SHA-256 `1ed5423b426de67eee93267eb5d699590128334aa8967a029eb8ffa0ac11fcdb`. Both profiles used visible controls to configure `Main board`, add `Board 2` in Layout, then open Case on `Board 2`. No storage edits or devtools state changes were used.

## Paired Mount/Gasket CRUD

From the empty keyboard's Case workspace, both apps created a Main board Plate body. The Mounting disclosure began closed; after opening it, `Add mount` created a Hole at X/Y `0`, diameter `2.5`. X was changed to `10`, then that mount was removed. The Gasket channel disclosure began Optional/closed; after opening it, `Add gasket` created a channel with inset `2`, width `2`, depth `1.5`. Width was changed to `2.5`, then the channel was removed.

On Dioxus, accepted project revisions advanced sequentially: body add 1, mount add 2, X edit 3, mount remove 4, gasket add 5, width edit 6, gasket remove 7. Each action settled Saved. The React public UI showed the same add/edit/remove values. IDs are app-generated and intentionally differ. The no-outline fixture reports its existing non-blocking preview error (“Board outline has no resolved contours” / blocked mechanical findings); that did not prevent the Case-body controls or saved edits. This receipt does not qualify body Undo/Redo or body save/reopen.

## Paired cross-board mismatch and return

On each app, the imported empty keyboard was configured through Case → `Configure mechanical stack` while `Main board` was selected. The resolver truthfully reported the missing-outline findings. Through Layout → `New board`, each app added and selected `Board 2`, then returned to Case.

React showed “Mechanical stack belongs to Main board,” kept the `Board 2` authored Case stack and `+ New case body` available, and offered `Show configured board`. Clicking it selected `Main board` and restored its generated stack/settings context.

Dioxus showed the same mismatch and kept the authored editor available. Clicking the `Show configured board` inside the authored Case editor selected `Main board` and restored the generated-stack context. Candidate `34761` had two identical configured-board actions; the changed-source follow-up below verifies the fix on `34763`, where only the authored-editor action remains and returns to Main. This bounded journey does not claim generated-stack disable/retention or a physical-instance handoff.

The final configured-board screenshots are `react-configured-return.png` and `dioxus-configured-return.png`. The paired journey used named task-owned profiles `case-f72-mismatch-react-20261003` and `case-f72-mismatch-dioxus-34761`; both were closed after capture.

## Paired body selection, filtering, dimensions, and history

On a fresh public import of the same no-instance archive, each app added a Main board Plate, changed its kind to Tray, and committed thickness `3.2`, clearance `0.6`, z offset `1.5`, and wall height `16` (wall height defaulted to `14`; wall thickness remained `2`). Each editor then added a second Main board Plate. Selecting the first and second body showed their distinct Tray values and default Plate values respectively. The pinned React body list displays the kind label uppercase (`TRAY`); candidate `34762` displayed lowercase (`tray`). Source follow-up `cb844072` adds the same uppercase CSS transform to the Dioxus body-kind label; candidate `34763` now renders the changed kind as `TRAY`.

Through Layout → `New board`, each app created `Board 2`; in Case, each added a Board 2 Plate. Switching the visible Board selector back to Main restored only the two Main board bodies, with the Tray selected and its accepted values retained. The Board 2 body stayed under its collapsed Board 2 assembly. This is the paired selected-board filtering slice; it does not cover every body kind or more complex instance projections.

On the selected Main Tray, wall height changed from `16` to `17`. Public Undo restored `16`, Redo restored `17`, and a page reload followed by reopening the Case tab retained `17` in both apps. This qualifies one accepted authored-body edit through Undo/Redo and browser reload; it does not qualify portable archive round-trip or every edit type. The React profile was `case-f72-workflow-react-20261003`; the Dioxus profile was `case-f72-workflow-dioxus-34762` on source `900068a0` at `http://127.0.0.1:34762/`. Both profiles were closed. Final post-reload captures are `react-tray-reload.png` and `dioxus-tray-reload.png`.

## F7.2 criterion map

“Proved” means the retained public evidence directly exercises that behavior. “Partial” means only a bounded slice or source seam is evidenced. “Open” means this packet supplies no sufficient qualification. Do not close Issue 01 from this map.

| Issue 01 criterion | Retained evidence and source | Status / boundary |
|---|---|---|
| Body list, add defaults, supported kinds, selection and selected-board filtering | [no-instance public receipt](../default-instance-public/no-instance/README.md) proves public add/default Plate; this packet proves Plate→Tray, switching between two Main bodies, and Main↔Board 2 filtering. `case_bodies.rs` scopes accepted records by board and exposes the supported kinds. | Partial. Lid remains source-only; physical-instance projections and more complex body lists remain open. |
| Thickness, clearance, z, non-Plate wall fields; Mount and Gasket CRUD | This packet pairs Thickness/Clearance/Z/Wall height edits on a Tray and Mount/Gasket add/edit/remove. [Case input receipt](../default-instance-public/case-inputs/RESULTS.md) pairs valid Enter and invalid feedback; source `case_bodies.rs` defines body-specific fields. | Partial. Wall thickness and validation across every field remain unproved; no exhaustive field matrix. |
| Native Mounting/Gasket disclosures and state retention across accepted edits | [34760/34761 disclosure receipt](../issue-12-show-and-f72-disclosure-34760/public-receipt.md) records both details closed by default, user opening both, accepted thickness/clearance edits, and retention on served `2107f980`. This packet exercises those same groups during CRUD. | Proved for default-closed empty groups, opening, and retention over accepted edits; populated-mount default-open predicate is source-backed, not newly replayed. |
| Normal Session save/history; numeric blur/Enter/Escape and invalid feedback | Case input receipt pairs Enter and invalid handling. The [34761 receipt](../issue-12-show-and-f72-disclosure-34760/public-receipt.md) records one accepted thickness edit and absence of duplicate Enter→blur busy feedback after the fix. This packet pairs one body wall-height edit through Undo/Redo and reload. | Partial. Portable archive round-trip and stale-request behavior remain open. Escape is a confirmed React UI difference: with accepted Z=0 and draft 1.5, Dioxus Escape restores `0`, React keeps draft `1.5`; neither saves a revision and both accepted documents stay Z=0. Record as the current intentional/necessary deviation; do not mark React parity or reopen accepted INT.2. |
| Draft/result identity scoped to document, board, session, and Case instance | Source guards in `case_controller.rs:72–99,311–358` bind pending results and submissions to editor instance, active scope, session epoch, document, board, token, revision, saved lifecycle and the target body's board. `case_bodies.rs:303–314` keys the editor to body plus scope; the cross-board public journey above exercises normal navigation. | Source-backed admission/settlement; no stale in-flight board/session/instance transition was injected. Physical-instance authority remains F7.7/F5. |
| Mismatch explanation and configured-board navigation alongside selected authored editor | Paired Main-configured → Board 2 mismatch → Show configured board transition above. Reference composition is in `app/src/ui/useCaseWorkspace.tsx:84–96`; Dioxus surfaces are in `case_bodies.rs` and `mechanical_settings.rs`. Candidate 34763 has exactly one action and its click returns the selected board to Main. | Proved for this selected-board mismatch and return; physical-instance handoff and generated-stack disable/retention remain open. |
| Keyboard focus/order, compact/desktop, light/dark, empty/mismatch/invalid and save/history states | Exact generic Case shell focus and panel behavior is retained in [compact focus/resize evidence](../compact-focus-resize-final-public/README.md): Case at 390×640 and 390×844, desktop resize to 1280×577, public Tab/Shift+Tab and retained workspace focus. Numeric keyboard/invalid evidence is in the Case input receipt. Theme switching screenshots and inspector behavior are in [mechanical UI layout evidence](../mechanical-ui-layout/README.md). Empty authored state is in the no-instance receipt; mismatch in this packet. | Partial, with no additional code needed for the already-qualified native-field/app-shell seams. Those adjacent-shell receipts do not prove every authored editor control at every viewport/theme. |
| RF disposition for the private Case form/edit seam | The [prior scoped receipt](../issue-12-show-and-f72-disclosure-34760/public-receipt.md) records no new takeaway for the same private editor seam. | No new refactoring takeaway observed. Existing RF-001/RF-006 history remains; no new RF item. |

## Scope and unresolved work

This is a bounded receipt and evidence reconciliation only. It does not claim F7.2 parent readiness, F7.4/INT.2 acceptance, authored-body history/reopen, every field's paired validation, every Case editor viewport/theme, or physical-instance scope correctness. The remaining open/partial Issue 01 criteria above stay open for the root's consolidated acceptance decision.

## Changed-source follow-up on 34763

Candidate `99ec041a2895e5ab23880be25501beb9360487db` was served at `http://127.0.0.1:34763/` and `/boardstudio/` with provenance `d3c03d6db107ab7a55753b5798d99444f7d0843540ef96530c00d9f44296fe08`. On a fresh named profile, I imported the same no-instance archive through the visible picker, configured Main board, added Board 2 in Layout, and returned to Case on Board 2. The accessible Case tree/Inspector contained exactly one `Show configured board` action. Clicking that action changed the selected board to Main and restored its current mechanical settings. I then selected Board 2, added an authored Plate, and changed its body type to Tray; the Case tree rendered `01 Board 2 plate TRAY`, matching the pinned React uppercase label. The changed duplicate-action/return route and kind-label deltas are green on this candidate. Screenshots are `dioxus-configured-return-34763.png` and `dioxus-tray-label-34763.png`.

This delta reuses the earlier paired React mismatch and body-kind journeys; it does not repeat the rest of Issue 01 history/reopen coverage or change its remaining open gates.
