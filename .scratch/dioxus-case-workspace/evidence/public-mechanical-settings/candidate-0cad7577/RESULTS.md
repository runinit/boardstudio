# Public mechanical-settings candidate verification

Date: 2026-10-02. Candidate: `frontend-mechanical-settings-20261002`, source commit `0cad75775e84aba13dc2f88d513b9f662e037e5e`. Root route: `http://127.0.0.1:34675/`. The retained `build-provenance.json` records all 965/965 source hashes verified; `served-page-assets.json` records exact root/subpath page asset comparisons. No source, build, or production data was modified for this pass.

Browser automation used `agent-browser 0.38.1` in named sessions and task-owned disk profiles:

- `mech-semantic-0cad7577-7dd31abc1fc4`, profile `/var/tmp/frontend-run/mechanical-settings-candidate-0cad7577/profile`.
- `mech-sofle-0cad7577-public-import`, profile `/var/tmp/frontend-run/mechanical-sofle-candidate-0cad7577/profile`.

All accepted-document inspections below were readonly IndexedDB reads from `boardstudio-m1-root`; user-facing edits/import/export were performed through the public UI. No DOM, provider, storage, or project-document writes were injected. No Cargo/native test was run in this pass.

## Reviung41 public Configure, process mapping, and history

The first session opened the candidate's built-in `REVIUNG41 copy`, whose initial actual accepted record was revision 3 with an existing gasket configuration. Because that built-in state differs from the React fixture, I made no parity claim from it. Through the Case UI I used **Disable mechanical stack**, then **Configure mechanical stack**. Configure saved revision 5 with the visible defaults (printed, tray, shell; plate 1.5 mm, plate foam 3 mm, PCB 1.6 mm, bottom foam 2 mm, bottom 3 mm, wall 2 mm, clearance 0.3 mm), four real `auto-closure/mount-proposal-*` mounts, and four generated NPTH parts/definitions. The board grew from 85 to 89 members.

Publicly selecting `CNC machined` saved revision 6. Plate and bottom process materials changed to Aluminium; plate/bottom foam stayed cut-sheet EVA. Undo saved revision 7 and restored printed/PLA for plate and bottom. Redo saved revision 8 and restored CNC/Aluminium. All four mounts, generated part IDs, definitions, and 89 board memberships were retained through method Undo/Redo.

I exported `reviung-candidate-cnc.boardstudio` at revision 8 using **Export → Export archive**. The archive's `project.json` canonical sorted-key JSON SHA-256 is `010ead5307d381cc11f359163834786578b99b7cbf759c07568cb470cc75beac`; the readonly active IndexedDB ProjectDoc at export time produced the same SHA-256. This compares the full accepted document, not only the mechanical summary.

Public Disable then saved revision 9 with `hardware.sharedConstruction=null`, the main instance mechanical value null and `constructionLinked=false`, zero generated closure parts/definitions, and board part count 85. Undo saved revision 10 and restored CNC plus four mounts/parts/definitions and 89 members. The public UI showed `Saved` after each completed action.

## Imported Sofle split-instance scope

The second session publicly imported the actual React UI archive `sofle-left-configured-react-reference.boardstudio` (source SHA-256 `889c89a177b3398aa4adc8687a331a8128481eb3d0f10c3d1b38a4a825bc36bd`) using the visible **Import .boardstudio** control. Its accepted record opened at revision 3, ID `6d011120-0f9c-4070-8ed9-259241cb460e`, topology split: Left was printed/PLA with four mounts and four generated parts; Right was unconfigured with 70 parts.

I selected `Right PCB` in the public Board combobox; the public physical-instance control then read `right half`. In Case settings I selected CNC. At revision 4, `sharedConstruction.boardId` was `right`; Left remained printed/PLA with exactly its original four mount IDs/coordinates. Right became CNC/Aluminium with four generated mounts and four generated parts/definitions. The left/right board counts were 74/74; there were eight generated closure parts and eight generated definitions total.

The full Left-scope projection `{left.mechanical, left generated board memberships, left generated parts, left generated definitions}` has the same canonical SHA-256 in the original React archive and candidate archive: `1401672af345f654d7d9a0019782485fca5c9ae27c2fc4a5253a637a899c06a0`. This directly verifies preservation of the actual imported Left configuration, mounts, generated definitions, and membership; the whole document is expected to differ after configuring Right.

I exported the candidate Right-CNC state at revision 4 (`sofle-candidate-right-cnc.boardstudio`, archive SHA-256 `30fd3d6517c6c28b47bcd623255c6604c43b1e652063cbd4f3e6c974f7764106`). Public Disable while Right was selected saved revision 5: both instance mechanical values null, both links false, shared construction null, zero generated closure parts/definitions, and both boards returned to 70 parts. Undo saved revision 6 and restored Left printed/PLA and Right CNC/Aluminium, four mounts each, eight closure parts/definitions, and `sharedConstruction.boardId=right`. Redo saved revision 7 and cleared those values again. One more public Undo saved revision 8; the exported full ProjectDoc SHA-256 and readonly IndexedDB ProjectDoc SHA-256 matched exactly at `0b0150ed794351155c7474a4e93749520ceb687cbe19bd1978c35426f36a38b0`.

Reloading the public page retained the imported project at revision 8, the Left/Right methods and materials, right-scoped shared construction, all eight closure parts/definitions, and 74 members per board. The visible selection returned to Left after reload; selecting Right afterward was a view operation and did not alter the accepted document.

## Evidence files and hashes

- `build-provenance.json` — candidate source/build provenance, 965/965 hashes verified.
- `served-page-assets.json`, `offline-manifest-root.json`, `offline-manifest-subpath.json` — previously captured exact served packaging evidence.
- `reviung-pre-disable.png`, `mechanical-settings-cnc-0cad7577.png`, `mechanical-sofle-right-cnc-0cad7577.png` — public UI captures.
- `reviung-candidate-cnc.boardstudio` — full public Reviung accepted archive, revision 8.
- `sofle-left-configured-react-reference.boardstudio` — actual imported React public archive.
- `sofle-candidate-right-cnc.boardstudio` and `sofle-candidate-right-cnc.json` — candidate public archive and extracted project at revision 4.
- `sofle-candidate-right-cnc-after-undo.boardstudio` and `.json` — same accepted Right-CNC document restored at revision 8; full hash matches IDB.

## Limits

This verifies the public mechanical settings, generated closure document data, accepted archives, Undo/Redo, reload persistence, and split physical-instance scope. It does not verify generation of authored case-body meshes: the UI reported `Resolved stack · 0 layers` and the imported record contained no case bodies. It does not establish STEP/CAD export behavior, numeric draft/error behavior, viewport layout, or complete mechanical fidelity. Numeric draft/validation and layout checks are owned by the other verifier. No errors appeared in the browser error output during this pass.

## F7.4-C02 supplemental derived-gap check (2026-10-04)

A paired public UI pass loaded the retained Sofle split-instance fixture (`sofle-left-configured-react-reference.boardstudio`, SHA-256 `889c89a177b3398aa4adc8687a331a8128481eb3d0f10c3d1b38a4a825bc36bd`) in the React reference at `:5175` and Dioxus candidate at `:34803`. I changed Plate thickness from 1.5 mm to 2.0 mm and committed the field with Enter in each UI. Both displayed Plate foam 2.8 mm afterward. React also displayed “Plate underside to PCB top: 3.00 mm, derived from the switch mounting dimensions”; Dioxus accepted the edit but, with no explicit fit profile assigned, omitted the current configured gap. Thus the demonstrated mismatch was the supported configured gap not being presented in the empty-profile state; this pass does not claim exhaustive edit coverage for every represented dimension.

The source fix in `web/src/presentation/mechanical_settings.rs` passes the resolved `plate_to_pcb` value into `ProfileGuidance` and renders that configured gap even when no explicit profile is assigned, while retaining the unavailable-profile-range guidance. The focused wasm regression `presentation::mechanical_settings::contextual_layer_tests::unassigned_profile_guidance_shows_saved_plate_to_pcb_gap` was observed RED before the rendering fix and GREEN on rerun after the concurrent layout compile issue was resolved (1 passed, 0 failed). Scoped verification also passed: `rustfmt --edition 2024 --check web/src/presentation/mechanical_settings.rs` and `git diff --check -- web/src/presentation/mechanical_settings.rs`.

The paired browser observation predates the source fix; the candidate was not rebuilt for this supplemental pass. The wasm regression verifies the source-rendered empty-profile state, not a rebuilt browser candidate or archive persistence. Numeric field validation and broader dimension coverage remain outside this specific gap check.

## F7.4-C02 published Dioxus retest (2026-10-04)

Retested Dioxus candidate `frontend-context-gap-20261004` at `http://127.0.0.1:34804/`, build source `58145c178156ee0a88ccaadc33dcae194c0f1639`, using the same Sofle split-instance fixture and no React retest. In the public Case tab, Plate thickness began at 1.5 mm and Plate foam at 3.0 mm. After changing Plate thickness to 2.0 mm and pressing Enter, the accepted settings showed Plate thickness 2.00 mm and Plate foam 2.8 mm. The Switch fit profiles region stated that the supported plate thickness range is unavailable until a profile is assigned, and showed “Plate underside to PCB top: 3.00 mm · current configured gap.” This confirms the fix on the published candidate for the previously observed paired React/Dioxus gap omission. The viewport showed previous generated geometry while current settings were saved; this retest establishes the visible settings and gap, not fresh geometry generation or archive persistence.

## F7.4-C03 bounded paired closure-hardware journey (2026-10-04)

Using the same saved Case archive in TypeScript `http://127.0.0.1:5175/` and frozen Dioxus candidate `http://127.0.0.1:34804/` (build `frontend-context-gap-20261004`, source `58145c178156ee0a88ccaadc33dcae194c0f1639`), I tested the existing first closure boss Position X. Fixture: `react-mounted-actions.boardstudio`, SHA-256 `0f41c012d17a36ca97a6de525f2bd677fe9b28d440a09c673fcf840cfad4623b`; its initial value was 128 mm in both apps. In each app, I changed Position X to 129 mm and committed with Enter; both showed 129 mm. I invoked Undo in each and both returned the field to 128 mm. After reloading each app, reopening Case and expanding Closure screws showed Position X still at 128 mm.

No functional mismatch appeared in this single closure-hardware edit/Undo/reopen path. This is bounded evidence for that existing boss-position field only; it does not qualify battery/cable, opening, hardware/critical-fit, process override, stabilizer-fit or other closure controls, nor generated geometry/export behavior. No source change was needed.

## F7.4-C03 bounded paired battery-envelope journey (2026-10-04)

Continued on the same `react-mounted-actions.boardstudio` fixture (SHA-256 `0f41c012d17a36ca97a6de525f2bd677fe9b28d440a09c673fcf840cfad4623b`) in TypeScript `http://127.0.0.1:5175/` and frozen Dioxus candidate `http://127.0.0.1:34804/`. The Battery control existed in both with the envelope initially disabled. Enabling “Include a battery envelope” in each app exposed the same represented values: width 30 mm, depth 20 mm, height 6 mm, cable width 2 mm, position X/Y 0 mm, and cable exit X/Y 0 mm. Undo returned the checkbox to disabled in both. Reloading and reopening Case retained the disabled state in both.

No mismatch appeared in this single battery-envelope toggle/Undo/reopen path. This does not qualify independent edits to the battery dimensions or cable exit, case openings, hardware/critical fits, process overrides, stabilizer fit, or generated geometry/export behavior.

## F7.4-C04 authored-body disable and resolved-stack projection (2026-10-04)

I imported the same existing authored-body archive, `f55-5-c03-profile-ts-5175.boardstudio` (SHA-256 `a61bc827aec668d86a195446cb1f08b37423255475fdc14ca95b93c7bd935a9e`), into pinned TypeScript `http://127.0.0.1:5175/` and Dioxus candidate `http://127.0.0.1:34805/`. It contains an editable Main-board Switch plate (thickness 3 mm, clearance 0.5 mm) and a separate Board 2 case assembly. After activating Configure mechanical stack in both, TypeScript showed five generated layer rows; the frozen Dioxus candidate showed `Resolved stack · 0 layers` and `The stack appears after the current revision resolves.` Dioxus still showed explicit no-profile/range guidance and the configured 3.50 mm plate-to-PCB gap. Its tree retained the authored body row while the generated stack was active; TypeScript instead stated that one authored Case body remained saved.

Disable mechanical stack in both removed the generated-layer rows and returned the Switch plate editor with its saved 3 mm thickness and 0.5 mm clearance; the Board 2 assembly remained. Undo and reload reactivated each stack. The TypeScript five-row stack and authored-body-saved message returned; Dioxus returned to the same zero-row pending-resolution state. No authored-body value was edited.

The source fix in `web/src/presentation/mechanical_settings_mount.rs` makes exact-current MechanicalResolution/current-scene rows the settings projection when available, falling back to displayed geometry only when current rows are unavailable; fallback rows keep the previous-result marker. Existing upstream scope/token/revision admission is unchanged. Focused wasm regression `presentation::mechanical_settings_mount::tests::current_resolution_layers_precede_previous_display_layers` was RED before the fix (projected prior thickness 1.5 mm instead of current 2.0 mm) and GREEN after (1 passed, 0 failed); it also checks the previous-row fallback. Scoped `rustfmt --check` and `git diff --check` passed.

The paired public run was against frozen candidate `34805` before this source change; no rebuild/replay was requested. It verifies authored-body preservation on Disable/Undo/reload in that candidate and records the pre-fix row mismatch. The wasm regression verifies current-versus-previous layer projection in source, not the rebuilt browser candidate or export behavior.

## F7.4-C04 qualified-candidate retest (2026-10-04) — rows GREEN, layer action FAIL

Verified candidate `frontend-functional-refresh-20261004` from package proof (source `6d3e89fc854d43f0943e5964890f22c773bcaa6f`) at `http://127.0.0.1:34806/boardstudio/`. The live page loaded the candidate's content-addressed JS/WASM asset names from the matching built subpath artifact. Reusing the prior fixture (`f55-5-c03-profile-ts-5175.boardstudio`, SHA-256 `a61bc827aec668d86a195446cb1f08b37423255475fdc14ca95b93c7bd935a9e`), Configure displayed five rows: Plate, Plate foam, PCB, Bottom foam, and Bottom. The heading was `Resolved stack · 5 layers`; rows had no `Previous revision` marker, and no previous-stack label was present. This closes the zero-row/current-versus-previous presentation mismatch for the qualified candidate.

The visible Plate row action did not select the row: after clicking it twice, its `aria-pressed` state remained `false` and no selected-layer state appeared. The settings pane shows rows from current MechanicalResolution, while the callback may admit selection only if that layer exists in the runtime's completed display CadScene; this is the suspected mismatch seam, not yet source-confirmed. Stopped at this first action failure; the requested Board 2 scope switch was not run. The TypeScript baseline remains the prior paired F7.4-C04 observation (five generated rows); it was not repeated. No source or test change was made in this retest.

## F7.4-C04 qualified-candidate row-selection repair (2026-10-04)

Source tracing confirmed the failure: `MechanicalSettingsMount` projected Plate from the identity-admitted current `MechanicalResolution`, but its click callback searched only the completed `Runtime::cad_scene()`. The candidate could therefore show five current rows while rejecting every selection until viewport scene publication. The Case callback now accepts layers present in the exact identity-matched resolved projection before scene completion. Its scene fallback requires exact current scope, snapshot token, and revision; a displayed previous-scene row cannot select against a stale owner. This remains a Case settings projection change and does not modify shared viewer/runtime behavior.

The focused wasm regression `presentation::mechanical_settings_mount::tests::mounted_current_resolution_layer_is_selectable_before_scene_completion` was RED against the old scene-only admission (1 failed at the expected selection assertion), then GREEN after the fix (1 passed). It also rejects the same retained projection when the requested identity token is stale. `rustfmt --edition 2024 --check web/src/presentation/mechanical_settings_mount.rs` and `git diff --check` passed. This verifies the source callback admission seam; the published browser candidate was not rebuilt or retested, and Board 2 scope switching remains untested in this bounded packet.

## F7.4-C04 qualified-candidate row-selection retest (2026-10-04) — fixture setup BLOCKED

Verified candidate `frontend-current-actions-20261004` at `http://127.0.0.1:34807/boardstudio/`: package proof and build provenance both identify source `3b5bb1b3651e2bbd35911e15f698995f02c1abed`; the live page loaded `boardstudio-web-dxh13baa3f233276cc.js` and `boardstudio-web_bg-dxh6b63a924c749c7.wasm`, whose SHA-256 values match the candidate's `site-subpath/boardstudio/assets` files. I attempted to import the same authored-body fixture (`f55-5-c03-profile-ts-5175.boardstudio`, SHA-256 `a61bc827aec668d86a195446cb1f08b37423255475fdc14ca95b93c7bd935a9e`) from both the landing-page Import control and the editor's Open project control. The browser dispatched the file input/change events and reset the input, but remained on the existing Sofle project; no fixture/editor transition or visible import error appeared. The browser console was empty. I closed the owned browser session without running the row click or Board 2 switch.

The requested five-row/Plate-selection/Board-2 journey is therefore **unverified on this candidate**; this is a fixture-import setup blocker, not a product FAIL or a GREEN result. The earlier `34806` observation and source regression remain the available evidence. No source change was made.

## F7.4-C04 absolute-path qualified-candidate retest (2026-10-04) — GREEN

Retried candidate `frontend-current-actions-20261004` (`3b5bb1b3651e2bbd35911e15f698995f02c1abed`) at `http://127.0.0.1:34807/boardstudio/` in a fresh agent-browser session. The exact archive was readable at its absolute worktree path (2,467,102 bytes; SHA-256 `a61bc827aec668d86a195446cb1f08b37423255475fdc14ca95b93c7bd935a9e`). The first post-upload snapshot still showed the landing page, but the app then transitioned to the editor; accepted-state markers distinguished the import from the prior attempt: Main board with 33 parts, authored `Switch plate PLATE` at 3 mm thickness/0.5 mm clearance, and separate Board 2 case assembly.

After Configure, the Main-board Case pane reached `Resolved stack · 5 layers` with Plate, Plate foam, PCB, Bottom foam and Bottom, and no `Previous revision` labels. The Plate row began with `aria-pressed=false`; clicking it opened the visible Plate-specific inspector (heading `Plate`, thickness 1.5 mm, fit-issues section), confirming selection was accepted. I then switched the Layout board selector to Board 2 and returned to Case. The Board 2 case assembly was selected; the Plate inspector/stack rows were absent, and the pane showed `Mechanical stack belongs to Main board` with the configured-board affordance. No stale Main-board layer remained selected in the Board 2 Case context.

Result: the requested current-row display, Plate selection and Board 2 owner-switch path are GREEN on this candidate. This verifies the one authored-body fixture path only; it does not qualify other layer actions, scope transitions, or generated geometry/export behavior. Browser session closed; no source change was made.

## F7.4-C05 bounded paired numeric-draft journey (2026-10-04)

Using `react-mounted-actions.boardstudio` (SHA-256 `0f41c012d17a36ca97a6de525f2bd677fe9b28d440a09c673fcf840cfad4623b`) in TypeScript `http://127.0.0.1:5175/` (source `5a472a9426e6e38993361da402cd4ec730feb369`) and Dioxus `http://127.0.0.1:34807/boardstudio/` (candidate `frontend-current-actions-20261004`, source `3b5bb1b3651e2bbd35911e15f698995f02c1abed`), I used the selected Left case assembly's Plate thickness field, initially 1.5 mm. In each app, typing 2.3 and pressing Escape restored 1.5 without accepting an edit. Typing 1.8 and pressing Enter accepted 1.8 mm. One Undo restored 1.5; one Redo restored 1.8. Reloading and reopening Case showed 1.8 in both apps.

No mismatch appeared in this bounded Escape/commit/history/reopen path. Source inspection additionally confirms the Dioxus field owner key includes editor, scope/presentation generations, session, document, board and instance; controller admission compares the request against current identity, accepted snapshot token, document revision/session/document and active board, and resolving requests recheck ownership. This pass did not inject an in-flight scope/revision transition or qualify invalid numeric drafts and other fields. No source change was needed.

## F7.4-C04 profile assignment affordance (2026-10-04) — source partial

On candidate `34807`, the authored-body fixture's Case pane showed “No explicit switch fit profile is assigned” and the current 3.50 mm gap, but had no assignment action. After Configure, public Disable returned the Main-board Switch plate editor with its saved 3 mm thickness and 0.5 mm clearance; the separate Board 2 case assembly remained present. The paired TypeScript Disable/preservation result is recorded above; no layer, plate-selection or board-switch path was repeated.

The TypeScript Case panel offers Library fit profile and Custom KiCad geometry selectors for unassigned placed definitions. Dioxus now projects target choices from the exact accepted active-board snapshot, exposes those two assignment actions, validates board membership and duplicate assignments in the existing Case controller, loads switch fit geometry through the existing Runtime profile service, rechecks request ownership after the asynchronous result, and commits through the Case document edit path. A focused owner-level regression covers cross-board and duplicate-target rejection. The current Rust Runtime profile loader accepts MX, Choc v1 and Choc v2 switch families; it has no Parts/Core request adapter for TypeScript's MX stabilizer 2u/6.25u library choices. Those two choices remain unsupported here, so F7.4-C04 is partial.

This source change was not included in a rebuilt candidate. `rustfmt --edition 2024` and `git diff --check` passed; the combined page compiler/package check remains with the coordinator. The browser observations above verify the prior `34807` candidate's no-profile feedback and authored-body Disable behavior, not the new selectors or accepted profile edit.
