# Case mount-handle mismatch diagnosis — 2026-10-05

## Finding

The candidate failure is before settings-controller submission. In `case_viewer.rs`, the exact text `Blocked move was not saved.` is emitted when the End sample is passed to `move_case_mount` and it returns `None` (lines 923–946). The only route to the later `SetMountPosition` patch is after that check (lines 977–1004). This matches the observed unchanged revision 18 and unchanged mount list. It is not evidence of a persistence rejection or a stale-request guard.

For the observed Bottom / closure handle, Start captures the accepted closure mount collection and bottom-body move constraints (`case_viewer.rs:740–776`). `mount_constraints` derives the bottom body's outer regions, solid holes, and height-intersecting openings (`:1353–1390`). The move predicate rejects points outside the region with the boss-radius-plus-0.5 mm edge margin, inside/too near a hole or opening, or too close to a sibling mount (`:1393–1431`). The handler discards Z and validates XY.

The pointer point is owned by the shared 3D renderer path: pointer down, move, and up call `point_on_plane_at_client` at the captured handle Z (`shared_viewer.rs:1987, 2156, 2260`). The host converts client coordinates through the canvas bounding box and backing-buffer scale (`renderer_host_page_extensions.rs:242–257`); the renderer then constructs a ray from its current camera and intersects the requested plane (`renderer/src/wasm.rs:794, 1063–1079`). The SVG helper `presentation.rs::pointer_location` is not used by this handle path. Therefore a 4-pixel gesture is not a portable world displacement across the two viewports.

The reference archive supplies a useful world-space acceptance target: proposal 47 starts at `(208.2962435897436, -2.7308064516129065)` and the accepted endpoint is `(205.6639709472656, -2.2012462615966797)`, a delta of `(-2.632272642478, +0.529560190016)`. The candidate gesture has no recorded pointer-up world coordinate, so the saved evidence cannot tell which candidate predicate rejected its point or show whether this exact reference endpoint passes the candidate's generated constraints.

The reference uses the same edge/opening/sibling-clearance rules (`caseEditing.ts:40–76`), captures the same bottom target for closure mounts (`AssemblyScene.tsx:239–253`), and persists only a valid pending drag (`:262–273, 284–305`). The candidate additionally recomputes and revalidates the pointer-up world point at End (`case_viewer.rs:908–947`), whereas the reference commits its last valid pending Move sample. This is a behavior difference worth testing, but the present archive alone does not prove it caused these two failures.

## Ranked hypotheses and falsifiers

1. **Candidate pointer-up world endpoint violates a mount clearance.** If true, capturing the candidate End XY and applying the accepted reference endpoint to the same candidate constraints will show the actual candidate point fails while the reference endpoint passes. Report the exact failed predicate (outer margin, opening/hole margin, or sibling spacing) before changing any threshold.
2. **Different candidate and reference camera/canvas mapping puts the same 4-client-pixel drag at different world points.** If true, the candidate's measured endpoint will differ from the reference delta, while the reference endpoint passes candidate `move_case_mount` against its own captured Bottom constraints.
3. **Move/End sample divergence causes End-only rejection.** If true, candidate Move and End report different XY for the same pointer coordinates, or the last valid Move endpoint passes while the End endpoint fails. Compare the sample coordinates and canvas rect/backing dimensions at both phases.

## Minimal next regression

When the frozen gate permits testing, use the exact saved revision-18 fixture and a mounted WASM case-handle test (the actual pointer/renderer path is required to diagnose mapping). Capture `WorldPoint` for Start, Move, and End on `case-mount:closure/auto-closure/mount-proposal-47`; assert that the exact reference world endpoint is accepted by candidate Bottom constraints and results in one `SetMountPosition` request with that endpoint. Also record the candidate endpoint from the two reported 4-pixel gestures and the individual clearance predicate. This distinguishes world mapping from validator geometry without loosening constraints. If only a native predicate test is available, seed `move_case_mount` with the constraints extracted from the real candidate revision-18 `CadScene` and the exact reference endpoint; do not substitute hand-authored polygon bounds.

## Evidence boundary

Source was inspected read-only. No production files, browser state, saved project, or fixture were changed, and no tests/builds were run during the frozen gate. Inputs and outputs remain those in `input.boardstudio`, `candidate-after-attempt.boardstudio`, `reference-after-move.boardstudio`, and `mount-comparison.json`. Since candidate End XY and candidate prepared Bottom polygons were not captured in those artifacts, the underlying clearance or mapping cause remains unresolved; this report makes no repair claim.

## Astra follow-up: exact provider and coordinate chain

The earlier boundary-data limitation is now resolved without compilation or changes to the application. `inspect-prepared.mjs` executes the **published candidate's own** `assets/core-worker/m1_core_worker_bg.wasm` (`30844349a4f6ab1d74c5a5189bd1b1fd04b5075f97128920a2729d74d1a15c60`, matching the package provenance) through its exported `CoreEngine`. It opens the actual `input.boardstudio`, reproduces the selected Right instance's `cad_jobs::effective_case_inputs` configuration, resolves mechanical geometry, and prepares that assembly. The script asserts this fixture is unflipped/wired, has no shared gasket layout, and requires no automatic stabilizer defaults; the remaining fixture-specific defaults are therefore explicit rather than guessed. The complete returned Bottom body and derived constraints are preserved in `prepared-constraint-analysis.json`.

The actual Bottom constraints contain one outer polygon and **zero holes or intersecting openings**. The selected boss radius is 3 mm, so the required margin is 3.5 mm. Evaluation of the source-equivalent Rust/TypeScript predicate gives:

| Point | Nearest outer edge | Margin remaining | Result |
| --- | ---: | ---: | --- |
| Original proposal 47 `(208.2962435897436, -2.7308064516129065)` | 4.0828064516 mm | 0.5828064516 mm | accepted |
| Actual reference endpoint `(205.6639709472656, -2.2012462615966797)` | 3.5532462616 mm | 0.0532462616 mm | accepted |

All sibling distances exceed 111 mm against a 6.5 mm requirement. This rules out a disagreement over this actual endpoint's boundary/hole/sibling validity. It is not a captured candidate pointer sample and does not assert the failed pixel gestures reached that endpoint. The script is a provider reproduction plus explicit predicate analysis, not a newly compiled Rust unit test.

The coordinate chain also has no found units discrepancy: `pointer_point` uses client pixels; the host converts each axis using the canvas rect and its corresponding backing size; the renderer ray consumes those backing pixels. `three_d::radians` in the pinned local `three-d-asset 0.10.0` source is `cgmath::Rad(v)`, a wrapper rather than a conversion, so `radians(34.0_f32.to_radians())` correctly supplies 34 degrees. Candidate and pinned reference use the same ray/plane mathematics and the same Bottom handle plane (`bottom stack z + 0.8`, **-6.2 mm** in this fixture).

Accepted-scene identity is captured separately from disposable preview identity (`shared_viewer.rs:404–420, 1404–1422, 2952–3001`). Admitted preview updates transfer gesture ownership while retaining the accepted scene. They still use the live renderer camera for each world sample. Scene projections request `keepCamera:true`; `Renderer::accept_scene` preserves target/distance/yaw/pitch, although it refreshes bounds and the camera's near/far planes. Handle updates do not orbit or fit the camera. No evidence presently implicates preview identity or camera reset, and changing End validation is not justified.

## Narrow layout hypothesis awaiting public confirmation

There is a concrete overlooked layout difference. `shared_viewer.rs:2842–2844` inserts the gesture feedback as a `<p class="m1-case-edit-hint">` **inside** `.m1-case-view-canvas-shell`, next to the canvas. The shell is a row flex container (`web/assets/m1.css:1108`), and its canvas is `flex:1` (`:1109`). **No CSS definition for `m1-case-edit-hint` exists.** Thus the newly inserted paragraph participates in flex sizing. Other canvas overlays are explicitly positioned absolutely. In pinned TypeScript reference `app/src/ui/assembly-preview.css:238,270,275–276`, `.wb-gasket-message` is an absolute overlay with pointer events disabled.

Start adds “Drag the mount; release to save its case position.”; Move and End replace it with messages of different intrinsic widths. An in-flow paragraph can therefore resize the canvas between pointer samples. The host reads the current bounding rectangle for each sample, and its ResizeObserver resizes the renderer's viewport, so a fixed client coordinate need not map to the same world point. This supplies a specific falsifiable explanation for otherwise valid-looking short gestures. Root has been asked to compare the visible canvas/scene before and after the public drag; no internal browser state inspection is needed.

If confirmed, the smallest coherent repair is a **canvas-shell-scoped** overlay style for its direct gesture-message child, preserving ordinary edit hints elsewhere. Retain the margin rules, accepted-owner checks, and End validation. The owning regression should mount the real canvas-shell/feedback markup with the maintained stylesheet and assert canvas rectangle/mapping stability through Start/Move/End feedback, followed by one packaged public inward mount gesture proving exactly one persisted edit. A fake polygon or a test that only checks the new CSS string would not establish this failure.

No live source, browser, index, or compiler state was changed during this follow-up. Evidence-only script/results and this appended analysis are the only writes.

## Public reflow confirmed; repair prepared, not applied

Root then reimported the exact revision-18 input and performed a visibly inward/down drag `(680,304) → (680,307) → (680,312)`. It again reported “Blocked move was not saved.” `canvas-feedback-reflow.json` records the public DOM geometry before and after:

| Measurement | Before feedback | After feedback |
| --- | ---: | ---: |
| Canvas CSS width | 701 px | 542.09375 px |
| Canvas backing width | 1050 px | 812 px |
| Canvas CSS height | 509.8020935 px | 509.8020935 px |
| Canvas left/top | 247 / 164.1979218 px | unchanged |
| Feedback width / positioning | absent | 158.90625 px / `static` |

The lost canvas width is **exactly the inserted feedback paragraph's width**. This confirms the missing overlay style causes live viewport reflow during a captured gesture; it is no longer a hypothetical camera reset. The live point-on-plane mapping depends on that rectangle/viewport. The saved public evidence still does not contain the intermediate world samples, so it does not identify which End XY or precise edge predicate was reached. A passing repaired gesture remains required before claiming the interaction fixed.

Two separate, apply-check-clean patches are prepared in this directory:

- `feedback-layout-test.patch` adds `shared_viewer::tests::mounted_case_gesture_feedback_preserves_canvas_geometry`. It mounts **the actual `SharedViewer` component** with `web/assets/m1.css`, changes its feedback input through the actual Start/valid-Move/blocked-Move/blocked-End/cleared messages, waits for each message to render, and compares all four canvas rectangle values. Its renderer owner is deliberately inactive: the owning defect is DOM layout, so this regression neither boots unrelated GPU providers nor claims to test persistence. It uses an existing nonrestoring Runtime fixture and supplies its complete context (Runtime only; the layer child has no context dependencies).
- `feedback-layout-fix.patch` adds one direct-child-scoped overlay rule to `web/assets/m1.css`. It removes feedback from flex sizing, constrains its width, and disables pointer interception. Ordinary hints outside the canvas shell keep their existing layout. It changes no mount constraint, gesture owner, preview admission, or End behavior.

Source-freeze execution plan: after release, apply **only the test patch**, run the one fully qualified mounted test with the strict runner, and require a width-change assertion RED. Only then apply the CSS patch and rerun the same test GREEN. Retain the existing gesture guards and complete one packaged public inward gesture proving one accepted revision and saved-coordinate change. Neither patch has been applied to live source or compiled; `git apply --check` is the only patch validation so far. No test result is claimed.

## Executed repair result

After coordinator released source freeze at commit `113d76fd43d2c2a25660af4e3ff51032c7082f08`, the test-only patch compiled and failed exactly one listed test. Its standard report truncated the assertion behind compiler warnings, so one unchanged-input diagnostic repeat retained the raw output: **Start feedback changed canvas width from 701 to 416.28125 px**. See `feedback-layout-red-raw.log` and `feedback-layout-red-confirm.json`; there were no inventory/harness problems or exclusions.

Then the one CSS rule was applied and only the new Rust test block was formatted. The same strict mounted test passed **1/1**, complete=true, with no problems or exclusions (`feedback-layout-green.json`; raw output in `feedback-layout-green-raw.log`). The exact filter took 43.983 seconds including runner startup. The test verifies all four canvas-rectangle values remain stable across Start, valid Move, blocked Move, blocked End and message removal. `git diff --check` passes. Live production changes are only the CSS overlay rule; the Rust change is test-only.

`feedback-layout-result.json` pins both final source hashes and complete structured results; patch artifacts were refreshed to the frozen final diff. Heavy slot 2 is released. No additional tests, source edits, commit or package are started. The coordinator still owns final review/integration and one necessary packaged public drag/save replay; this result alone does not claim a persisted gesture or acceptance criterion.
