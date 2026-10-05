# Parts renderer initialization failure review — 2026-10-04

Verdict: **CLEAR on the final settled source; no unresolved concrete defects.** This is the independent diff review for the two-file test-only batch, not parent acceptance. The retained final strict preview-module run executed 10 tests with zero failures; coordinator integration gates remain separately required.

## Reviewed identity

Worktree: `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001`.
Branch: `codex/rust-v1-ui-parity-20261001`.
Base HEAD: `e2ec1c1e511f82fb9aaaa10a0406da7dafa4f76c`.

Final working-tree SHA-256:

| File | SHA-256 |
| --- | --- |
| `web/src/presentation/shared_viewer.rs` | `6d1b6bac802e9b9e8885505a9a4c685212ebecec1612b841e2a52e4bb4c6c56e` |
| `web/src/presentation/parts/preview.rs` | `3c57d81238b1ac228e9c01e131b7fa6259869e9432c9c6fb3bdf044f0fb5fb1f` |

Only these two source files were reviewed for implementation. Unrelated source/evidence/records and earlier reports were preserved. This reviewer ran no tests, builds, packages or commits and wrote only this report.

Final fixture delta was rereviewed after the earlier CLEAR: the accepted document now contains a board whose ID exactly matches the fixture scope, and the `KicadSource` contains a minimal footprint string instead of an empty marker. Both changes improve fixture validity without altering admission, mounting, assertions, or production behavior. The preview hash above supersedes the earlier reviewed `9d649c96…` hash; shared_viewer is unchanged.

## Resolved findings and standards

Two concrete issues in the initial test were corrected before the final verdict:

- Nonmutation checks originally compared the fixed accepted snapshot returned by `set_definition_name_test_state`, whose Runtime submit path records events instead of applying them. Such comparisons alone would pass despite an accidental submitted edit. Final source retains token/revision/document/Arc comparisons **and asserts `take_definition_name_test_event().is_none()`**, detecting any submitted event.
- The host initially named an absent `presentation::parts::mechanical_profile` module. It now uses the actual `crate::parts_mechanical_profile::ProfileDefinitionSource` path.

No remaining documented-standard violation or actionable baseline smell found. Existing test bodies/assertions were not removed. New seams are one-shot, guarded by `cfg(test)`, reset by scoped guards, and use existing capture/error machinery rather than introducing a production renderer/provider API. The non-test renderer branch still invokes the same `RendererPageHost::mount` with the same arguments and handles its result in the unchanged match. The Parts seed branch is absent outside test builds.

## Mounted behavior and exact fixture boundary

The test mounts the real `PartsMechanicalProfileWorkspace`, retaining its Mechanical fit/Define profile controls around `PartsPreviewPanel`. It waits for the actual 3D control before clicking. The prepared `PcbPreview` seed is consumed only after the existing accepted scope/token/document check, real `PartsPreviewCapture::capture`, and installation of its owner lease. `capture.accept_preview` retains the sample source/revision/request-target validation. Downstream preview owner matching, Runtime snapshot currentness, `PartsSampleViewer` signal admission, and SharedViewer scope/token/owner guards remain unchanged.

The second seed produces one renderer mount error at the real SharedViewer mount call boundary. The test requires the actual mounted `role=alert` to contain that error; it does not fabricate an alert or directly set component failure state. It verifies retained fit/form controls and both existing view toggles, then clicks 2D and requires a rendered footprint SVG with no canvas. It also requires no project edit submission.

The 2D fixture uses a definition with `generator: None` and a `KicadSource` containing a minimal footprint string, exercising the real source-backed footprint rendering path. Its provided pads/courtyard are fixture data; it does not prove KiCad parsing, generator execution, worker transport, model decoding, real GPU initialization failure, host resource cleanup, or retry completion. The geometry seed is likewise prepared fixture data, not Core-worker integration. Those boundaries are appropriate for this mounted failure/form/2D join; retained worker/model/currentness and scripted renderer cleanup evidence remain separate. The test shows the neighboring form action stays mounted, not editing/saving a profile through a renderer failure.

The exact mounted test passed in the author's retained `parts-renderer-init-debug-source-backed-20261004.log` (one executed, zero failed). After the final fixture delta, `parts-renderer-init-final-20261004.log` records strict `presentation::parts::preview::tests::` execution: 10 executed, zero failed. The coordinator confirmed attribution to the final hashes above. Earlier failures are preserved: invalid enum path, clicking before mount, missing worker asset, missing generator asset, and absent source-backed fixture marker. Coordinator integration gates are not inferred from either focused run.

## F7.3-C01/C09 evidence reconciliation

The current task wording is decisive:

- C01 requires one common viewer/private adapter for five consumers, scoped scenes and supported interactions without cloned viewer behavior; its finish condition requires the five public routes to use the same implementation and accepted scope projection.
- C09 requires F3/F4/F6 wiring to the single viewer, private renderer DTOs and no assumed API widening; its finish condition requires accepted workflow sources and no duplicated viewer/unapproved widening.

The current consumer map identifies Layout, Parts, Keymap, Keycaps and Case producers/wrappers, scoped identities and the shared `CaseSharedViewer`. Named retained public receipts establish their route/scope/control outcomes, including the isolated Parts mount/currentness/nonmutation/layer journey. The inspected source keeps renderer-private DTOs internal and contains no separate Parts viewer implementation. **No additional Parts model-pick observation is required by either C01 or C09, and no concrete missing clause was found in those two criteria.** Their evidence may be reconciled from the existing attributable map/receipts without another public pick session. C05 already owns project part/module/Case/finding picking and is verified. C10 owns isolated sample safety and renderer failure/form/2D recovery; this test supplies its distinct mounted initialization-failure join, subject to execution gates. A sample-pick observation could add evidence, but is not a new C01/C09 acceptance requirement. This does not accept the F7.3 parent or waive its other criteria/joins.

## PCB evidence scope

Read-only inspection of `pcb-host-themes-20261004/RECEIPT.md` and the F5.1-C01/C03 record diff found no scope overclaim. C01's paired Left/Right host geometry/defaults and selected-board route are supported by the receipt's same-archive identities/transforms, host polygon and default-layer observations. The compared PCB runtime files are unchanged between served `4e716156` and HEAD. The receipt explicitly excludes pixel/primitive equivalence and exhaustive clipping; C03 retains remaining visual assessment instead of treating functional theme/toggle proof as complete visual qualification. No parent acceptance or mobile qualification is claimed.
