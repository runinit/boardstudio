# F3.3-C02: drag admission and stationary reflow

Paired browser qualification: pinned TypeScript `5a472a9426e6e38993361da402cd4ec730feb369` (`http://127.0.0.1:5175/`, session `f33c02-layout-qual-2318ea1a1376`) and Dioxus build `frontend-functional-controls-20261003` (`http://127.0.0.1:34778/`, session `bs-mig-6280ea4d22a7`, source `ad26b07a7ada49aed1d7ac686fb32a8dbcf2ded6`). Dioxus package proof is `../../frontend-functional-controls-20261003/package-proof.json`; it reports no source/route mismatches.

## Results

- **Alt + 1 CSS px part drag:** U1 moved freely by about 0.43 mm on each surface (world X: Dioxus `273.200012 → 273.630004`; TypeScript `273.20 → 273.626727`). This confirms nonzero client motion admits a drag despite the small world-space delta. One Undo restored the original position and one Redo restored the moved position on both surfaces.
- **Stationary press during Inspector reflow:** with the pointer held at a fixed client coordinate, keyboard activation expanded the Inspector and changed the canvas width from 1013 px to 725 px. The pressed part/key and U1 retained their world positions on both surfaces; the stationary release did not create another history change. This isolates client-coordinate motion from world-coordinate changes caused by panel reflow.
- **Shift/Ctrl selection:** trusted Shift and Ctrl selection sequences with 8 px pointer travel left target part positions unchanged on both surfaces. Selection occurred without a drag or position history change.

Input used trusted browser mouse and keyboard events. No app runtime state was injected. The Inspector was toggled by keyboard while the pointer was held, so this verifies stationary reflow and release behavior, not a selection gesture that itself opens the Inspector.

## Conclusion

F3.3-C02 passes for the tested candidate: 1 CSS px Alt movement starts a free drag; stationary panel reflow does not move the object or create a drag; Shift/Ctrl selection does not start a drag; and the movement is one Undo/Redo step.

## F3.3-C04: nudge keys and snap-guide feedback

Additional paired browser check on the same REVIUNG41 fixture: pinned TypeScript `5a472a9426e6e38993361da402cd4ec730feb369` (`http://127.0.0.1:5175/`, session `f33c04-key-nudge-ts-20261003`) and Dioxus build `frontend-module-attachment-repair-20261003` (`http://127.0.0.1:34782/`, session `f33c04-key-nudge-dx-20261003`, source `733c1da2abede39a617d2eca2e42d9bd437cea41`). Package proof is `../../frontend-module-attachment-repair-20261003/package-proof.json`; it reports no source/route mismatches.

- **Arrow nudge:** with the `main-U1` tree row focused, ArrowRight moved X from `273.2` to `273.3` mm in both builds (`+0.1 mm`). Shift+ArrowRight moved it from `273.3` to `274.3` mm (`+1.0 mm`) in both.
- **Editable field and canvas focus:** with the selected part's `X mm` spinbutton focused, ArrowLeft left the part at `274.3` mm on both builds. Focusing the Layout canvas/application and pressing ArrowRight also left it unchanged on both. The mounted nudge route is the focused tree row, not canvas focus; this matches the reference's tested behavior.
- **Snap guide:** moving U1 toward RST produced a live snapped preview at approximately `(273.2, -43.25)` mm. Dioxus exposed `main-RST · corner / midpoint / center` as a live status label. TypeScript rendered its `.wb-snap-guide` circle/path, with no text in that SVG marker. After pointer-up committed the move, the guide element/status disappeared on both builds.
- **Escape cancellation:** repeating the snapped preview, pressing Escape while the pointer was still held restored U1 to `(273.2, -15)` mm and removed the guide on both builds. Releasing the pointer afterward left the restored position unchanged.

Input was delivered through trusted browser mouse and keyboard actions. DOM reads were limited to visible rendered parts/guide nodes; no app runtime state was injected. This verifies the user-facing Escape cancel path on both paired builds. A later candidate-only native touch-cancel probe is reported separately to the root for C03; paired TypeScript/Dioxus `pointercancel` qualification remains unverified.

The tested C04 legs pass for the mounted tree-item nudge route, editable spinbutton behavior, live snap feedback, and guide removal on commit and Escape cancel. A paired native `pointercancel` event check remains outside this receipt.

## F3.3-C03 supplemental: candidate touch-cancel path

On Dioxus build `frontend-module-attachment-repair-20261003` only (`http://127.0.0.1:34782/`, REVIUNG41), a bounded trusted Chrome DevTools Protocol touch-cancel check exercised the candidate cancellation path. With touch emulation enabled, the input sequence was `Input.dispatchTouchEvent(touchStart, 902,244)`, `touchMove(900,300)`, `touchMove(898,312)`, then `touchCancel`. During the snapped preview the status read `main-RST · corner / midpoint / center` and U1 previewed at `(273.2, -43.25)` mm. After `touchCancel`, the guide was absent and U1 returned to `(273.2, -15)` mm. The TypeScript native `pointercancel` path was not tested.

Input was sent through the browser CDP `Input` domain after enabling `Emulation.setTouchEmulationEnabled`; only rendered DOM was read back. No page script or application runtime state was injected.

## F3.3-C01: position, matrix transforms, and constraints

Paired public-browser checks used the same REVIUNG41 fixture with pinned TypeScript `5a472a9426e6e38993361da402cd4ec730feb369` at `http://127.0.0.1:5175/` (session `f33c01-criteria-ts-20261003`) and Dioxus candidate `frontend-module-attachment-repair-20261003`, source `733c1da2abede39a617d2eca2e42d9bd437cea41`, at `http://127.0.0.1:34782/` (session `f33c01-criteria-dx-20261003`). Package proof: `../../frontend-module-attachment-repair-20261003/package-proof.json`.

Reused paired records: Column Offset X `0 → 2.0 mm` and Key Local X `0 → 0.5 mm`, each with Undo/Redo/reload, are in [the matrix transform receipt](../../../../dioxus-layout-authoring/evidence/f33b-matrix-transform-paired-20261003/README.md). The six Align anchors with Undo each and final Redo/reload are in [the alignment receipt](../../../../dioxus-layout-authoring/evidence/transform-align/layout-align-paired-20261002/RESULTS.md).

New paired accepted edits, each reverted with one Undo:

- Standalone `main-U1` Properties X changed `273.2 → 274.2 mm` and returned to `273.2 mm` in both apps.
- With the `right keys` matrix selected, matrix Origin X changed `140.665 → 141.665 mm`; matrix Rotation changed `0 → 5°`. Both fields accepted the same edit in both apps and each Undo restored its original value.
- With `right keys · Column 1` selected, Stagger changed `0 → 1 mm`, Splay changed `10 → 11°`, and the column Splay Origin X changed `147.285 → 148.285 mm`. Each matched in both apps and each Undo restored baseline.
- The active Stagger pointer handle was also dragged 10 client px vertically. It changed Column Offset Y `0 → -4.7625 mm` in both apps; one Undo restored `0`. This handle edits the column offset; the distinct Stagger field above edits the stagger value.
- An Offset constraint on U1 using the default source `main-right-keys-SW1`, Offset X `2 mm`, Offset Y `30 mm`, Rotation `0°` was accepted in both apps. U1 moved from `(273.2, -15)` to approximately `(149.285, -21.02) mm`; one Undo removed the constraint and restored baseline.
- A Mirror constraint on U1 using source `main-RST`, Vertical axis, coordinate `273.2 mm` was accepted in both apps. U1 moved from `(273.2, -15)` to `(273.2, -45) mm`; one Undo removed the constraint and restored baseline.

Scope note: U1’s direct Properties inspector exposed X/Y but no standalone rotation field in either app. TypeScript’s `Position & rotation` menu item on U1 returned to the same Properties view without exposing a rotation control; Dioxus’s generic transform items were disabled for that Component selection. Selecting the `right keys` matrix enabled the corresponding transform commands in both apps and exposed the matrix Rotation field; selecting Column 1 exposed paired Stagger, Splay, and origin controls. No standalone-part rotation mismatch was reproduced; matrix rotation and the remaining C01 matrix/constraint edits matched in this bounded journey.

No source or build changes were made for this qualification. Browser input used public UI controls and trusted pointer actions; no app runtime state was injected.

Bounded coverage: the reused Align receipt is paired but its Dioxus browser candidate is the earlier `frontend-layout-align-wave-20261002` build (`14d1bfeb...`), not the current `34782` candidate; Align was not replayed here. No standalone-component Rotation field was exposed in the pinned TypeScript inspector, so component rotation itself remains unqualified; only matrix Rotation was edited and paired. Newly exercised values were reverted with Undo, not followed through reload. These limits do not change the paired acceptance results above.

## F3.3-C05: row-scope and driven-target guards

Paired public checks used the REVIUNG41 fixture in pinned TypeScript `5a472a9426e6e38993361da402cd4ec730feb369` at `http://127.0.0.1:5175/` (session `f33c05-c05-ts`) and Dioxus build `frontend-module-attachment-repair-20261003` at `http://127.0.0.1:34782/` (session `f33c05-c05-dx`, source `733c1da2abede39a617d2eca2e42d9bd437cea41`; package proof: `../../frontend-module-attachment-repair-20261003/package-proof.json`).

- **Row crossing differently splayed columns:** in Select: Row mode, selecting Row 1 of `right keys` showed Align disabled on both builds with the exact message “A row can cross differently splayed columns. Align individual keys or a column.” The fixture has Column 1 Splay `10°` and differently splayed neighboring columns, so the selected row crosses those transforms.
- **Driven target:** selected `main-U1`, added the public Offset-from-part layout constraint from `main-right-keys-SW1` with the default zero offsets, and opened Align. Both builds showed all six anchor buttons disabled and the exact feedback “Unlock or remove the driving constraint before aligning.” One Undo on each build removed the temporary constraint and returned the editor to `Layout constraint Optional`.
- **Locked target:** created [a locked U1 fixture](fixtures/reviung41-u1-locked.boardstudio) from the retained [REVIUNG41 archive](../../../../../docs/design/evidence/board-outlines/reviung41-original.boardstudio), baseline SHA-256 `672d5f581e65bf74b9a5df336f734167bba7642865fa7a0dfa12a96347d64a8f`. The only project JSON change is `parts[id=main/U1].locked: true`; all other project JSON values were verified unchanged and every non-`project.json` archive member remains byte-identical. The variant SHA-256 is `10d27bedca865ba8e6d14e4ec07308ad43e07f794a7c25edf60fd2af233c1873`. After importing it with the public `.boardstudio` file input in both apps, U1 showed the `Locked` status, all six Align anchor buttons were disabled, and the UI showed “Unlock or remove the driving constraint before aligning.” No app-state injection was used. The production Align projection checks `part.locked == Some(true)` alongside constrained targets at `web/src/presentation/objects/layout_align_controller.rs:631-637`.
- **Stale revision:** no public stale-callback race was forced. Production `on_align` reprojects and rejects mismatched workspace/scope/context/reference/moving IDs, then rejects a request whose captured snapshot token or revision differs from the currently accepted snapshot (`layout_align_controller.rs:228-270`); it submits at the accepted revision (`:353-363`). The retained `completed_old_scope_is_not_held_by_a_lower_new_revision` regression in `layout_align_geometry.rs:236-258` covers stale-scope settlement, not a public callback race or the action-admission branch itself.

Disposition: paired public guard behavior is confirmed for row scope, a driven target, and a locked target. Stale action rejection is source-backed with adjacent settlement regression evidence; it was not given a public adversarial race here.
