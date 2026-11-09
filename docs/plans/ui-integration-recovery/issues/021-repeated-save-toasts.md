# 021: Stop repeated save Toasts during routine interactions

Status: resolved
Type: bug
Blocked by: none
Parent: [001: Recovery specification](001-ui-integration-recovery-spec.md)
Related: [Plan map](../map.md), [023: Save feedback specification](023-truthful-save-feedback-spec.md)

## Report

The user reports that the save notification appears far too often: even zooming
can produce dozens of save Toasts. Repeated success notifications flood the
interface during ordinary use and obscure useful feedback.

At filing this was a user-observed symptom, not an agent-reproduced failure;
the affected build, sequence and cause were not yet established. The triage
reproduction and source trace below supersede that initial limitation without
claiming the user's original build was identified or durable saving was faulty.

## Desired outcome

Routine interaction and automatic saving provide calm, accurate save feedback
without repeatedly stacking save-success Toasts. Keep meaningful failure and
recovery feedback visible and preserve the intended persistence behavior.

## Acceptance criteria

- [x] Reproduce the reported Toast flood on an identified build, including
  repeated zooming. Trace whether it comes from distinct saves, replayed
  notifications, or presentation lifecycle, and record the demonstrated cause.
- [x] Repeated zooming and representative routine interactions do not generate
  a flood of save-success Toasts. Record the intended success-notification policy
  before implementing its regression coverage.
- [x] Save-status feedback remains accurate and durable saves still complete;
  suppressing Toasts must not hide failed saves or disable required persistence.
- [x] Preserve meaningful failure/retry/recovery feedback and unrelated result
  notifications, including exports and the waiting-update notice.
- [x] Demonstrate RED/GREEN at the real notification boundary and verify the
  reported interaction in the packaged application. Update relevant records
  with actual tested source and scope.

## Verification

Use `diagnosing-bugs` to establish the actual reproduction before choosing a
repair. Prefer existing mounted production notification and real Runtime/Session
workflow seams; use a packaged browser scenario for the reported zoom behavior.
Record the notification count and relevant save outcomes without substituting
notification suppression for persistence proof.

## Comments

The progress notes below retain their original dates and slice-local scope.
The coordinator completion record supersedes their outstanding acceptance and
execution limitations; the date ordering is retained, not silently rewritten.

### R1 review-fix progress — 2026-11-08

Base remains `2c30c93b8c163c035689f624ef466d636c9f18ff` plus the existing 021
delta; no commit. R1 changes only Runtime and this progress note.

| Requirement | Precondition → action → independent oracle | Tier | Result |
| --- | --- | --- | --- |
| Retire obsolete failure, preserve it during view changes | Real TestPersistence failure → camera/navigation/selection and stale committed ack → failure/durability unchanged; successful RetrySave → actual saved document, Saved durability, retired report, empty notification queue | wasm Runtime, existing view-completion case extended | RED: `021-r1-red.log` (retry leaves injected failure); GREEN: `021-r1-green.log` |
| Retire initial prompt after Open | Existing real opened fixture → status is the neutral saved report, no Toast | wasm Runtime, existing autosave case extended | RED: `021-r1-red.log` (initial prompt remains); GREEN: `021-r1-green.log` |
| Preserve export failure and update safety | Export-owned persistence failure → unrelated real landed edit → failure remains; mounted notifications/header suite → existing update gates/results unchanged | wasm Runtime + mounted | GREEN: `021-r1-green.log` (6 Runtime and 33 mounted tests) |

Signal chosen: `Runtime::complete` sees a committed Persist ack **and** an accepted
snapshot change **and** `Durability::Saved`. Session ignores mismatched save-attempt
acks; accepted publication only occurs when the pending save commits. Checking
`Settled.landing` alone would be wrong: `Resolution::Unchanged` also calls
`settle_landed`. Retire at this accepted ack rather than adding operation-ID
bookkeeping or clearing on later view/session-only Completed effects.

A private report lifetime flag tags the initial prompt and non-export
PersistenceFailed reports (also save-recovery BlockedByRecovery reports when
Durability is Failed). All other reports, including export failures and update
notices, remain untagged. No message matching chooses retirement. `status()`
becomes non-alert, non-transient `Saved locally.` without queueing a notification;
RuntimeReportBanner already suppresses that neutral report. Consumer inventory
found no production dependency on the old failure/prompt text. No presentation
production or test changes were needed; header/durability/Session contracts remain
untouched.

Logs are under `.evidence/ui-integration-recovery/`. RED: 4 passed, 2 intended
assertion failures. GREEN: 6/6 Runtime plus 33/33 mounted notification/header tests,
serialized with the existing build lock. The first combined command reached the
120s harness timeout before a mounted verdict; confirmed no build remained, then
reran mounted successfully with a longer timeout (recorded in the same GREEN log).

Post-edit Gortex detect/tests/guards/contract used the exact worktree overlay:
behavioral/high-risk lower-bound warning; no configured guard rules. Its test graph
points at the native stub, not the executed wasm tests, so is not behavioral proof.
Optional LSP diagnostics are unavailable (rust-analyzer exited/config-disabled).
`rustfmt --check` and `git diff --check` pass; full diff read. Existing unowned edits
were preserved. Packaged acceptance and the coordinator's full gate were outside
this R1 slice; their subsequent receipts and acceptance closure are recorded below.

### Implementation progress — 2026-11-08

Policy: 023's ordinary autosave success is header-only; view completions are
silent. Explicit results and persistent failures keep their existing treatment.
Implementation base: `2c30c93b8c163c035689f624ef466d636c9f18ff`.

Proof map (planned before repair; results updated after execution). Logs below
are in `.evidence/ui-integration-recovery/`:

| Requirement | Precondition → action → independent oracle | Tier | Status |
| --- | --- | --- | --- |
| Quiet, truthful completion | Real opened Session → camera/navigation/selection → applied view, no Persist, unchanged accepted revision/durability/status, empty notification queue; real edit → persisted new revision, Saved, empty queue | wasm Runtime | RED: `021-red-wasm.log` (camera Toast/status and autosave Toast assertions); GREEN: `021-green-wasm.log` (6/6 notification tests) |
| Production bridge classification | Mounted production shell → repeated camera/navigation plus autosave → no save Toast; explicit export → result Toasts | mounted | RED: `021-red-mounted.log` (unexpected save Toast); GREEN: `021-green-mounted.log` |
| Update/failure safety | Existing mounted update and geometry draft blockers plus header notices → unchanged gates and accessible controls | mounted | GREEN: `021-green-mounted.log` (31 notification tests including all update-now and geometry blockers, plus 2 header notice tests; 33/33) |
| Packaged behavior | Identified fresh package → repeated real zoom → moved camera, same stored bytes, Saved header, no Toast; real edit → changed stored bytes | packaged | Deferred in this slice; subsequently GREEN: `021-packaged2-toasts.log` (see completion record); packaged RED is REUSED triage reproduction on retained 009 package, NOT freshly re-established RED |

Scope guards: notification production code (019), exports (including firmware's
pinned result), failure reports, Cancelled/Closed, queue retention, persistence,
retry and header mapping were not changed in this slice. Acceptance closure below
also includes R1's provenance-based retirement of obsolete save-failure reports.

Verified Completed producers losing the generic save Toast (source:
`application/src/session.rs`, unchanged):

- Edits (`ResolveEdit`/commit and gesture commit), Undo/Redo, Open (including
  recovery reopen), ReviewElectricalRemap and export-wiring commits save through
  the shared `CoreReply::Scene` → Persist → `settle_landed` path
  (1888–1936, 2008–2063). Successful RetrySave settles twice: the original
  retained operation and the retry operation at the same landing (2057–2060).
  Saving/Saved and accepted publication remain owned by Session.
- SetCamera (1117–1124), Navigate (1075–1099), SelectParts (849–912),
  SelectMatrixCell (921–1073), GenerationFinished (1302–1310),
  ClearPreview (1571–1591), CancelGeneration (1229–1231), and non-gesture
  PreviewEdit completion (1867–1879) all use the ordinary completion branch.
  Session close also completes there (2580–2584); the separate Closed and
  Cancelled outcome reports remain untouched.
- GenerationFinished first sets `GenerationStatus::Ready`. The production Case
  panel derives its title from that state and geometry via
  `case_generation_title` and renders it in a polite status paragraph
  (`web/crates/case/src/cad_presentation.rs:192,228–230`;
  `web/crates/runtime/src/case_generation_lifecycle.rs:267–294`). Generation
  therefore retains panel-visible feedback without a fabricated save Toast.

Execution notes:

- The first test compile exposed two fixture/type mistakes; corrected before
  actual RED (`021-test-compile.log`, not counted as regression evidence).
  Runtime RED: 4 passed, 2 failed for the intended Toast/status assertions.
  Mounted RED's first failure is the new Toast assertion; later failures are
  shared-WASM panic-hook contamination, not separate diagnosed regressions.
- Both GREEN commands serialized under
  `flock ~/.cache/tmp/boardstudio-build.lock`: direct runtime `wasm-pack test
  --headless --chrome --chromedriver <pinned-driver> web/crates/runtime --locked
  --lib -- notification_tests` with `CARGO_PROFILE_DEV_DEBUG=false` and pinned
  `CHROME_BIN`; then `python3 scripts/run-wasm-tests.py --files
  web/src/presentation/notifications.rs web/src/presentation/header.rs`.
- `scripts/test-toasts.py` now asserts 20 real alternating zoom changes, stable
  Saved header and stored project bytes, a real rename saved at a newer revision,
  no routine save Toasts, and preserved explicit export/error accessibility and
  dismissal. `python3 -m py_compile scripts/test-toasts.py` passes. This slice
  deferred packaged execution and drag smoke until a fresh page build; the
  coordinator subsequently executed both (receipts below). No old package was
  presented as GREEN.
- `runtime_test_stub.rs` only settles operation observers; it has no status or
  notification queue/reporting behavior, so no mirror change is needed.
- Gortex initially routed to the main checkout/pending overlay. Retried at
  60-second intervals without tracking/reindexing; explicit checkout
  `01a1b3cf-a8df-75da-ab9e-d3fe3b2d3378` then served exact worktree state at
  base 2c30c93b. Pre-edit impact succeeded (11 direct dependents; critical
  lower-bound blast-radius warning, contract attachment rebuilding).
  Post-edit detect/tests/guards/contract completed: behavioral/high-risk warning,
  no configured guard rules, graph test coverage incomplete (it points at the
  native stub rather than recognizing wasm tests). This does not replace the
  executed Runtime/mounted evidence. Runtime's repaired source was re-read via
  Gortex to verify worktree content.
- `rustfmt --check` on the two Rust files and `git diff --check` pass; full owned
  diff read. Only the five assigned files changed by this implementation;
  pre-existing instruction/research/Gortex files were left alone. No commit.

> *This was generated by AI during triage.*

### Agent brief — 2026-11-06

**Category:** bug. **Summary:** distinguish camera/navigation completion from
actual persistence and stop routine automatic save-success Toasts.

The user confirmed the existing Runtime/Session, mounted production notification
and packaged-browser seams. The agreed behavior, scope and verification contract
are in [023](023-truthful-save-feedback-spec.md). Keep normal document persistence,
header save status, failure/recovery and explicit action outcomes intact. Start
from the reproduced zoom flood, demonstrate RED/GREEN, and do not alter saving
merely to silence its feedback. This issue is ready for implementation planning;
no product code was changed by publishing the specification.

> *This was generated by AI during triage.*

### Triage notes — 2026-11-06

- Confirmed in a dedicated incognito browser at localhost:4725/boardstudio/
  against the existing packaged site used by 009. No new build was made; current
  source is 17fdfbc38e222a650d92bd515fbaf425847d0dd8, and the retained site's
  build provenance is recorded in 009. This is not the user's live preview.
- Opening an editable demo and alternating the real Zoom out/in controls 20
  times, 100 ms apart, created 20 new “Saved locally.” Toasts. A browser assertion
  requiring no flood failed with `021: zoom floods save Toasts` and count 20.
  A separate 20-click Zoom in burst produced eight Toasts before reaching the
  zoom limit. A further zoom produced a Toast while the complete stored project
  record remained unchanged (13,372 serialized bytes).
- Source trace: the production footer zoom submits `Event::SetCamera` through
  `CanvasNavigationState::apply_zoom_target`; Session updates camera state and
  settles Completed without persisting a document. Runtime's ordinary Completed
  handler emits a transient “Saved locally.” report. The notification bridge
  renders each queued event as a success Toast. The queue drains once and is
  explicitly designed to retain distinct repeated events; broad text-based
  deduplication would also suppress legitimate separate action results.
- Recommended category: bug. Keep `needs-triage` until the success-feedback
  policy is settled. Proposed policy: ordinary automatic save success uses the
  existing header status; camera/navigation completion produces no save-success
  Toast. Preserve save-failure/recovery and explicit action outcomes, exports
  and the waiting-update notice. Do not alter persistence to silence the UI.
- 008 and 009 are resolved; their harness observes the header and durable
  document, not Toast text. Implement this separately and rerun relevant
  save/Undo/reload controls when changing the Runtime notification boundary.
- Then-open decision, subsequently settled in 023: header-only feedback for
  routine successful autosaves, retaining meaningful failure/recovery and
  explicit outcomes.

- 2026-11-06: Filed at the user's request: “the save notification pops up way to
  many times. even when doing things like zooming I will see it dozens of times
  via the toast”. No investigation or implementation performed by filing it.

## Completion record (coordinator, 2026-11-08)

- Candidate `HEAD`: `0c1d1a6f`. Implementation `542313f4` silences generic
  Completed reports and uses `retire_on_save` provenance to retire obsolete
  reports only at committed Persist acknowledgement + changed accepted snapshot
  + `Durability::Saved`. Cadence correction `0c1d1a6f` calls `changed()`
  unconditionally for Completed: silent is not unnotified; `observed` describes
  the operation outcome slot, not subscribers. Session save/publication ownership
  is unchanged. [023](023-truthful-save-feedback-spec.md)'s policy is implemented,
  including the ordinary-autosave exception added to `DESIGN.md`.
- Executed implementation receipts (all logs below under
  `.evidence/ui-integration-recovery/`): Runtime/mounted RED in
  `021-red-wasm.log` and `021-red-mounted.log`; GREEN 6/6 Runtime and 33/33
  mounted in `021-green-wasm.log` and `021-green-mounted.log`. R1 RED in
  `021-r1-red.log` has two intended failures (stale failure after retry and initial
  prompt after Open); `021-r1-green.log` passes both and the preserved export,
  failure and update controls. These executions are reused by this docs pass,
  not newly run here.
- Cadence's own RED/GREEN: `021-r2-cadence-red.log` demonstrates the missing
  subscriber callback (calls 0, expected 1); `021-r2-runtime-green.log` passes
  7/7, `021-r2-parts-green.log` passes 106, and
  `021-r2-mounted-header-green.log` passes 33. This is a semantics correction
  with independent regression evidence, not a proven explanation of the first
  full-gate failure.
- Packaged RED = **REUSED triage reproduction on retained 009 package**
  (`021-triage.log`), **NOT freshly re-established RED**. The first fresh-package
  run (`021-packaged-toasts.log`) failed on a `test-toasts.py` harness arity
  error, then corrected. Clean `021-packaged2-toasts.log`: 20 real zoom changes,
  zero save Toasts, stable Saved header, stored bytes unchanged by zoom and changed
  by rename. `021-drag-smoke.log`: 17 checks passed. The retained package's
  provenance remains the 009/triage record; it is not relabelled as the new build.
- Full gate: `021-coordinator-full-gate.log` **FAILED** its browser step at
  `mounted_generator_upload_landing_preserves_apply_draft_012`, panic
  `no entry found for key`, followed by shared-WASM cascade failures. The cadence
  correction landed on the hypothesis that silent != unnotified. Focused and
  full parts suites **also passed without that correction**, so the original
  failure's cause is **not proven**. Timing/load under parallel browser launches
  is plausible, unproven. Rerun `021-coordinator-full-gate2.log`: **ALL STEPS
  PASSED**, 581.0s; browser executed 223, failed 0. Both receipts stand; the first
  failure is not erased or causally attributed to the cadence correction.
- Combined Sol review: Spec **CHANGES REQUIRED** (R1: stale failure report),
  Standards **PASS**, checkpoint `reviews/021-spec-standards`. After R1 correction,
  delta recheck Spec **PASS**, Standards **PASS** carried from the unchanged
  reviewed scope, acceptance **READY**, checkpoint `reviews/021-r1-delta-recheck`.
  The cadence correction was reviewed during the gate cycle and has the separate
  RED/GREEN above.
- Completed-producer closure (detailed source enumeration retained in progress
  above): edits/gesture commits, Undo/Redo, Open/recovery reopen,
  ReviewElectricalRemap and export-wiring commits keep the shared Scene → Persist
  → settle_landed save path; RetrySave settles the retained and retry operations.
  SetCamera, Navigate, SelectParts, SelectMatrixCell, GenerationFinished,
  ClearPreview, CancelGeneration, non-gesture PreviewEdit and Session close lose
  only the generic save Toast. Closed/Cancelled reports are unchanged.
  GenerationFinished retains verified Case-panel feedback via
  `GenerationStatus::Ready` / `case_generation_title` and its polite status
  paragraph. Firmware export's explicit `Saved locally.` result is intentionally
  unchanged (023-preserved); an optional wording rename is out of scope.
  `runtime_test_stub.rs` has no report/queue behavior, so needs no mirroring.
- Cleanup: packaged server stopped, port free, no browser sessions remain.
  Standard `web/target/site` output is retained; `021-packaged/` contains only
  `export-results.png`. No new retained risks or unmet ticket gates;
  update-safety guards are all green. Next: coordinator integration; this closure
  does not close final technical/human acceptance in 010/011.

## Answer

Resolved: routine autosaves use the header, and view completions no longer
fabricate save-success Toasts. Durable saves, meaningful failure/retry/recovery,
explicit exports and waiting-update feedback are preserved. Packaged zoom and
rename checks pass; the full-gate rerun is green. The initial gate failure and
its unproven cause remain explicitly recorded above.
