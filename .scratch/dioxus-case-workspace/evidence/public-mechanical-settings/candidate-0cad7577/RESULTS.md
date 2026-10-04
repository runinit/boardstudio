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
