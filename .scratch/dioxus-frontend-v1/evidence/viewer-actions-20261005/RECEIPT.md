# Required shared viewer actions — desktop, 2026-10-05

Published candidate `frontend-routed-case-cancellation-20261005`, source `38641dd1353bcfb6230d52c1d5b84c7ad7908e7a`, port34823; read-only TypeScript reference source5a472a9 on5175. Input `/home/chris/.local/share/boardstudio/retained-tmp/20261002/keycaps-findings-f6c4-c6.boardstudio`, SHA256 `9027125846d2878176f127ec39b60c9ab605a0eb2a4b9019e10dbba846fe87a6`. Both imported unchanged through public Open project. Desktop1280×720, Left PCB, Keymap3D.

## Completed actions

Both Fit/Top and Wireframe→Hybrid executed; wireframe screenshots retained. Candidate controls are inside View controls; placement is cosmetic, not missing functionality. Both actual individual SW19 cap Hide→Show executed; hidden screenshots show the underlying switch and other caps retained. Candidate camera/geometry scales differ; exact cosmetic parity deferred.

## Confirmed product gap: generated cap picks

Two candidate visible-cap clicks at(554,224), in freshly observed Top views, left Selected key empty. Corresponding reference cap click(535,380) selects `matrix/left-keys/r3c0`, label `left-keys-SW19 · Unassigned`. Repeat selected-option JSON and before-pick screenshots are retained. The initial generic pick JSON is not authoritative for selection because its option filter could hide the relevant row; use selected-option and repeat-selected JSON.

Falsifier: candidate Hide left-keys-SW19 removes exactly the cap at the clicked position. With that cap hidden, the SAME(554,224) click selects `matrix/left-keys/r3c0`; see keymap-candidate-underlying-model-pick.json. Thus this is cap ownership routing, not a missed canvas click. Cap restored afterward in both apps. No edit/persist proof is inferred from viewer selection.

Source diagnosis: renderer/src/wasm.rs::pick returns object.id. shared_viewer::append_keycap_bodies uses `keycap:<spec.id>`/`keycap-legend:<spec.id>`. LayoutCanonicalViewer routes Picked only through module_for_current_pick or part_for_current_pick, which calls case_preview::part_for_native_preview_reference requiring a matching PCB preview model reference. Generated cap IDs lack this mapping. Owning native RED/fix and packaged Keymap/Keycaps replay remain required; no passing cap-pick claim.

F7.8-C02 remains open. C03/C04 evidence joins received independent conditional CLEAR in the existing consolidated review; changed Case keycap gate and public replay remain required. No mobile checks or repeated fault matrix.

## Keycaps and Layout continuation

Keycaps: both Fit/Top and Wireframe→Hybrid executed with actual geometry screenshots. Clicking the next top-row cap at candidate(577,224)/reference(570,381) leaves candidate previously selectedSW19/r3c0, but reference selectsSW20/r3c1; keycaps-pick.json. Same shared generated-cap mapping defect, not a new independent repair.

Layout: candidate Wireframe→Hybrid and both PCB Hide→Show executed; actual hidden screenshots retain models. Candidate Layout does not request the generated keycap consumer in this route; its captured assembly shows switches. This receipt claims render/layer actions only, not identical cap contents. Reference first mouse/Space attempts did not change Hybrid because the overlapping Layout toolbar obscures the control. Enter on the observed Wireframe button successfully sets pressed state and changes geometry to lines; only layout-reference-wireframe-confirmed.png is its passing render capture. Earlier filenames ending wireframe.png and wireframe-keyboard.png show unsuccessful attempts, not pass. Enter on Hybrid restores the mode. This pre-existing reference placement does not block candidate functional qualification.

Parts: public MX switch 3D sample click at C(600,366)/R(595,477) retained both MX headings, 3D mode and Sofle project identity; parts-readonly-pick.json and before-pick screenshots. This is the isolated read-only sample action; no project-part selection or persisted document edit is claimed. Initial candidate Back-form/Front-geometry mismatch is the already-open generator repair, not a successful orientation comparison.

## F6 desktop Light/Dark focus comparison

On the same unchanged C6 input, set Light through Project → Workspace settings → Color theme in both apps. Keymap2D Selected key is SW19/r3c0 in both (using public selection control, no document edit). Find a key → Tab focuses Selected key; both actual captures show a visible focus ring, readable behavior/selection labels and accessible scroll to the lower controls. Keycaps2D clearance → Tab focuses Matrix profiles summary with visible focus ring; board colors, clearance and profile labels remain legible. Captures keymap-*-light-focus.png/keycaps-*-light-focus.png. Existing same-input dark before-pick and Keycaps hybrid captures cover ready dark panels. Canvas framing/font/spacing differences are cosmetic and retained; no unusable control found in these comparisons. Loading/error/empty clauses still require the independently assessed retained evidence, not a claim inferred from ready screenshots.

## Remaining visual states and pending-feedback repair

For ordinary validation, both public SW19 behavior controls changed Unassigned→Key press, accepting the default A binding. Both then entered `A)`; Enter alone left the draft visible, and blur produced the actual rejected-expression alert while the accepted key/option still showed A. Only keymap-*-invalid-after-blur.png proves visible error; earlier invalid-visible.png files are before the alert and do not prove it. Candidate inline/footer error and reference top-level error are readable; styling differs cosmetically. Corrected to A and blurred before reload.

After that action the fixture is **C6 plus accepted SW19=A**, not the original unchanged archive. Both fresh reload→Keycaps→3D captures show ordinary pending work without interception/fault injection. Reference keycaps-reference-pending.png shows Loading assembly viewer. Candidate keycaps-candidate-pending.png has real Generating keycap CAD text almost covered by the absolute view toolbar, with the independent PCB preview already ready. Independent review confirms a functional readability gap; this image is RED, not passing loading evidence.

Read-only DOM bounds: candidate design view group absolute, x247/y94/height39.33; canonical viewer starts y86. CSS also positions Layout group at top68 relative to the workspace, plus approximately40px height. Root m1.css reserves112px before the first viewer status, with8px/12px spacing for subsequent messages; both initial fallback and current-PCB/keycap pending/error markup use this status class. Ready viewers with no status remain unchanged. Exact file SHA a945604db386a58fc6cfdd2d0a2903f8f2642923ee8acb9619f6bfd9040b28a9. Independent source review and actual packaged pending GREEN remain required; no synthetic native CSS test.

## PCB desktop theme controls — F5.1-C03

On the same C6+acceptedSW19A input used for the preceding state captures, enter PCB in each app. Both show Left PCB,70parts, Matrix wiring,18assignments, readable used/free pins and existing Resolve automatically/Apply wiring controls. `pcb-*-light.png` and `pcb-*-dark-focus.png` show current1280×720 panels after changing theme only through Project→Workspace settings. Wiring mode→Tab focuses Resolve automatically with a visible ring in both. No wiring edit, resolver action or pin apply was submitted. This is theme/control visibility evidence, not a new initial-preferences claim or an exact footprint-layer geometry comparison; initial defaults/host-layer behavior retain their existing F5.1-C01/C02 evidence. Canvas zoom/framing, glyph/panel spacing and retained layer strokes differ; cosmetic matching stays deferred. No missing or unreadable PCB control found in these captures.

## F2.4-C04 bounded desktop continuation, 2026-10-05

Candidate38641/34823 and pinned reference5a472a9/5175, desktop1280x720. Fixture is retained C6 plus accepted SW19 Keypress A and the publicly saved long project name `Migration desktop long-name qualification — Sofle split keyboard with shared Keymap, Keycaps, PCB and Case contexts 20261005`; candidate r14 Saved. New tabs76/77 reopened that persisted active project automatically. Not unchanged original C6 and not a new archive equivalence claim.

Paired `shell-*-long-name.png` show header ellipsis leaves workflow/Export reachable, full multiline current library card title, current indicator and usable name input/delete affordance. Input's full DOM value matches the saved name; both document scroll widths1280 equal viewport. Menu, settings and guide opened normally with that name. Exact card/menu spacing differs; no clipped inaccessible action was found.

Explicit Color theme System was selected publicly in both. Paired `shell-*-system.png` show both resolve to Light, with readable menu/canvas/inspector. `shell-system-proof.json` records actual selected System values and desktop bounds. Body computed colors differ because reference paints nested surfaces; body color alone is not the theme verdict. No forced OS change or dark-System claim.

Setup guide opens with heading focused in both. Actual Tab sequence: heading → Back to objects → Project & hardware → Layout & assemblies. Both buttons show2px blue visible focus. `shell-guide-focus.json` and paired guide-focus images record targets/appearance. Back to objects returns the Objects tree. Objects options → Tab Tree grouping → Tab Auto-hide objects → Tab Collapse objects works in both, and Escape dismisses options and restores Objects options focus with expanded=false. `shell-panel-focus.json` plus paired panel-focus images record the actual sequence. No auto-hide/collapse edit or repeated five-stage history run was performed. Mobile omitted.

Visible settings content differs: reference also exposes Restore default panels and Reset local projects; candidate settings only exposes theme. This observation is not silently labeled exact settings parity; reviewer must check existing scope/ownership before whole-parent acceptance. This bounded proof supplies the previously named long-name, guide/panel focus and System clauses only.
