# F4.2 custom definition and field affordance journey — 2026-10-03

## Scope and provenance

This is a bounded public UI qualification for F4.2-C03/C04/C05. The integrated Dioxus candidate is `http://127.0.0.1:34784/`, build `frontend-parts-search-feedback-20261003`, source `b2bb813cdb75340a2725ac4488dfde6752104484` (provenance and bundle checks in the adjacent `package-proof.json`). The pinned TypeScript reference is `http://127.0.0.1:5175/`, source `5a472a9426e6e38993361da402cd4ec730feb369`.

The paired fixture was fresh Sofle v2. Custom creation produced one project-owned component, named `F42 Custom Passive`, with kind `Passive`, 12 × 6 mm courtyard, one circular pad (number `2`, 2.5 × 2 mm at the origin). The input KiCad source used for the affordance comparison was `app/src/parts/sources/thqwgd001/THQWGD001C [4pin] [Reversible].kicad_mod`, SHA-256 `eaa7eccad4b5467dbaa4e36298f4e39b1c90a496ec98eb6d5db310623ea4b4e5`.

Owned browser sessions were `f42-custom-dioxus-7dd31abc1fc4`, `f42-custom-react-7dd31abc1fc4`, and the clean settled control `f42-slow-react-7dd31abc1fc4`.

## C03 — custom create/edit, history, and portable reopen

Both applications created the custom definition through Parts and selected it in the current project. The representative accepted edits covered kind, courtyard width, pad number, and pad size. In Dioxus, changing pad number `1 → 2`, Undo (`2 → 1`), and Redo (`1 → 2`) settled visibly. The accepted `.boardstudio` export was reopened through the public project import control; the selected project definition retained name, `Passive` kind, 12 × 6 mm courtyard, pad number `2`, and 2.5 × 2 mm size. Evidence: `candidate-custom-edited.png`, `candidate-custom-redo.png`, `candidate-custom-reopened.png`.

The settled TypeScript replay performed each field edit with a 600 ms wait for the accepted state before moving to the next field. Undo restored the pad width to 2 mm; Redo restored 2.5 mm. Its portable export reopened through Project > Open project, and the selected definition retained the same named values. Evidence: `reference-custom-slow-edited.png`, `reference-custom-reopened.png`.

The Dioxus archive `candidate-custom-edited.boardstudio` has SHA-256 `8b67d82ade85e2b2a04d72abd23ac96f64d855423af287ea881872e783c752da`; its definition ID is `ui-a28543ed-5e61-4410-afef-5cd957d2ae6d`. The settled TypeScript archive `reference-custom-slow-edited.boardstudio` has SHA-256 `50cec449ab78181c59ec71e6950054c38a7d0055d54200a609d56d6f27c3033e`; its definition ID is `ui-60f81d99-dc59-46d5-80a9-3937c0694298`.

Both archives contain 15 definitions and 6 assets. Compared with the retained fresh Sofle baseline at `/home/chris/.local/share/boardstudio/retained-tmp/20261003/functional-exports/sofle.boardstudio` (14 definitions, 6 assets, 140 parts, 4 matrices, 4 layouts, 100 nets, 2 boards), each archive preserves the exact `parameters`, `parts`, `matrices`, `layouts`, `nets`, `outline`, `boards`, `caseBodies`, `materials`, `assets`, `scripts`, and `constraints` values. The single extra definition is the authored custom part.

### Supported mechanical-fit profile field

For the project-owned custom Passive, both Parts UIs expose `Define profile` and the mechanical-fit editor field `Plate underside to PCB top (mm)` (initially 0) with `Save fit profile`. This is a supported profile field, separate from the custom definition geometry fields. The Dioxus editor is in `web/src/presentation/parts/mechanical_profile_editor.rs`, routed through `mechanical_profile_ui.rs`; TypeScript exposes the paired editor in `app/src/ui/PartMechanicalProfileEditor.tsx`.

In each app, the field was set to 0.45 mm and saved. Undo changed the card back to `Define profile`; Redo restored `Edit profile`. The reopened editor displayed the saved 0.45 mm (as `0.449999988...` in the numeric input). Each portable archive was then reopened through the public project controls and the profile editor again showed 0.45 mm. Screenshots: `candidate-profile-editor.png`, `candidate-profile-reopened.png`, `reference-profile-editor.png`, `reference-profile-redo.png`, and `reference-profile-reopened.png`.

The Dioxus archive `candidate-custom-profile-edited.boardstudio` has SHA-256 `651d415af4661ee48c9e0491379f4ac1ba0db92b1f6db52edc1552b4243a691e`; the TypeScript archive `reference-custom-profile-edited.boardstudio` has SHA-256 `f30bea74a5f3d3baedb7677df71b29fac38809bfaa6b93077708d5cc3920035b`. Both archive profile payloads record `plateToPcb: 0.45`. Each retains 15 definitions, 6 assets, 140 parts, 4 matrices, 4 layouts, 100 nets, and 2 boards; within each app, the custom definition's ID, name, kind, courtyard, and pads match its pre-profile archive. The baseline unrelated document fields listed above remain preserved.

## Rapid TypeScript burst and settled control

A first TypeScript pass issued name/kind/courtyard/pad-number/pad-size field changes consecutively without waiting between accepted operations, followed by Undo/Redo and export. The UI showed a `Stale base revision` alert. Its exported archive, `reference-custom-edited.boardstudio`, SHA-256 `6cd2b734882a10bc649004ec851da85afb890db6b45bd46688f569bcbd579a32`, retained the name, kind and pad number, but still contained the original 10 × 6 mm courtyard and 2 × 2 mm pad; the width and pad-size values visible in the editor were not committed. Screenshot: `reference-rapid-stale-error.png`.

A clean TypeScript session repeated the same custom fields one at a time with 600 ms accepted-state waits. No stale alert appeared; all edited values were present in the exported archive and remained after reopen, with Undo/Redo working. This is evidence of a fast-edit reference race; its source cause is unproven. The fast archive is retained as an excluded attempt, not used as the settled comparator.

## C04 — project ownership, selection, and persistence

In Dioxus, `New custom component` created the selected `F42 Custom Passive` entry in the current project. The portable archive contained it; reopening the archive and selecting the row retained that same definition and values. The paired TypeScript journey did the same. This qualifies the public custom-create/select/save/reopen clause on this fixture.

## C05 — editable custom fields versus imported source fields

Custom definitions in both applications expose the same field family: kind, courtyard width/height, pad ID/number/position/size/shape/drill, Add pad, and Remove pad. The separate mechanical-fit profile editor adds `Plate underside to PCB top (mm)` for the selected custom component; 0.45 mm was accepted, Undo/Redo worked, and the value survived archive reopen in both apps.

For the imported 20-pad KiCad fixture, both current public UIs expose editable courtyard dimensions while locking the source-owned pad ID/number/position/size/shape/drill fields and disabling Add/Remove pad. Screenshots: `candidate-imported-locked-fields.png` and `reference-imported-locked-fields.png`. The retained import receipt `.scratch/dioxus-parts-catalogue/evidence/05-import-kicad-footprint-paired-journey-20261003.md` records the repeated source numbers, unique physical pad IDs, source preservation, editable courtyard, and malformed retry behavior. No imported geometry was modified in this journey. Unrelated project fields remain equal in both custom exports as listed above.

## C06 — delayed import ownership

The public delayed-read check used the same supported KiCad fixture described above (SHA-256 `eaa7eccad4b5467dbaa4e36298f4e39b1c90a496ec98eb6d5db310623ea4b4e5`). In each named browser session, `File.prototype.text()` was wrapped to defer reading by 7 seconds. While that delay was pending, the selected Parts definition was changed from `MX switch` to `mcu nice nano`; after the read completed, both UIs still showed `mcu nice nano` selected. The Dioxus Parts list had no imported fixture row, and the exported document remained revision 3 with 14 definitions. Its archive `candidate-c06-stale-selection.boardstudio` has SHA-256 `aad5a3cef6f005a9a8cca3290730a8b10d2cd9323e2db2122973c4d9dbfdc94a`. Screenshot: `candidate-c06-selection-stale-rejected.png`.

The paired React run kept `mcu nice nano` selected but appended `THQWGD001C [4pin] [Reversible]` after the same delay. Its exported archive `reference-c06-stale-selection.boardstudio` has SHA-256 `aa9fd09442b436b5079f0c7f9eec3b7b3f61a1874b48aff858de569e3876029e`; it is revision 3 with 15 definitions, including the fixture. Screenshot: `reference-c06-selection-stale-accepted.png`. The archive proves the selection-scoped stale result was committed despite the selection remaining unchanged. The mismatch is in the TypeScript reference behavior; this receipt does not infer an implementation cause.

The Dioxus project-owner check repeated the delayed upload, then used Project > New project before the delay elapsed. After the read completed, the new `Untitled keyboard` remained revision 0 with zero definitions and no imported fixture. Archive `candidate-c06-after-project-switch.boardstudio` has SHA-256 `1e2a41d816b6f2f596989e248add229636740cc1cf9c747c2e14ce07443a90ea`.

The delay controlled the public file-reading phase (`File.text()`); parser/provider latency was not separately injected. Existing source-backed unit coverage in `parts_import_footprint.rs` checks owner matching for a replacement selection and retains feedback across revision-only advancement. The public route evidence exercises actual delayed reads plus selection and project changes; no source edits or additional regression test were made because the candidate did not reproduce a gap. It does not qualify provider-stage races beyond the source's current-token/owner rechecks.

## Exact coverage and limits

- **F4.2-C03:** verified for representative custom create/edit including kind, courtyard, pad, and supported mechanical-fit profile fields; accepted edits with Undo/Redo; portable export/reopen; and asset/unrelated-document preservation on fresh Sofle.
- **F4.2-C04:** verified for project-owned creation, selected definition, export, and reopen on both applications.
- **F4.2-C05:** verified for representative custom kind/courtyard/pad and mechanical-fit profile edits and paired custom-versus-imported affordances; profile and definition geometry remain separate, and the imported pad lock and repeated-pad preservation are additionally backed by the retained import receipt.
- **F4.2-C06:** the Dioxus public route rejected delayed results after both selection change and project change; the reference route failed the selection-change case by adding the stale import while keeping the new selection. Project-change parity on React and parser/provider-phase injection were not separately tested. The candidate covers the criterion's required stale-owner outcomes for this fixture; see bounded runtime/source-backed limits above.
- Existing F4.2-C01/C02 remain covered only by the linked import receipt. No malformed-field matrix, multiple-pad editing matrix, or imported courtyard mutation was attempted. This receipt does not close F4.2 while its required joins remain open.
