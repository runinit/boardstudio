---
title: 'Stabilize Parts generator-upload browser tests'
type: bugfix
ticket: 1
created: '2026-11-16'
status: done
baseline_revision: '143e1d47ef01d3150daab3b087bb3224838849f9'
route: full
route_source: auto
risk: low
review: quick
review_source: pinned
lenses_ran: [quick]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Two mounted Parts generator-upload tests intermittently index a missing `switch_3dmodel_filename` in accepted state, poisoning subsequent shared-WASM tests. Unchanged retries do not establish correctness or explain the failures.

**Approach:** Establish whether the upload is late or its binding is lost using the real mounted upload path and controlled asynchronous boundaries. Repair only the demonstrated test synchronization defect, retaining all existing behavior assertions and adding deterministic regression coverage.

## Boundaries & Constraints

**Always:** Preserve the approved standalone bug, severity P2 and risk low. Use the existing worktree and branch; preserve unpublished commits and unrelated edits. Leave `necessist.db` unread, untouched and unstaged. Stage only this build's deliverables. Use supported browser runners and the shared build lock; retain Cargo configuration. Keep long logs under `.evidence/`.

**Never:** Treat the 30 ms wait as established causality, call a longer sleep a fix, weaken assertions, ignore/delete tests, or claim stability from a retry. No branches, checkouts, stash, reset, push, PR, tooling installation/repair, or Layout entries 2.1/2.2. Production or shared-harness changes require reconsidering scope and verification with the human.

**Decisions:** User authorized native tools because exact-checkout Gortex routing is unavailable, and authorized proceeding with the unrelated retrospective and database untracked. User approved targeted-only verification: five normal and five four-worker CPU-load generator-settings suite runs, deterministic before/after evidence and changed-file formatting; no full project check for a Parts-test-only repair.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|---|---|---|---|
| Normal upload | Real file input and completed upload | Accepted asset and matching generator binding, rendered filename, and retained Apply draft | Existing assertions remain |
| Controlled late upload | File completion withheld beyond the old observation boundary | Prove missing state while withheld; release and drive real effects to distinguish late from lost | Bounded failure with useful state diagnostics |
| Queued failed Apply | Upload precedes a later failing Apply | Upload binding survives; width draft and retry preview remain | Preserve explicit failure feedback |
| Upload never completes | Completion remains withheld | Synchronization must not report success or reach binding assertions | Bounded diagnostic failure, not a missing-key panic |

</frozen-after-approval>

## Code Map

- `web/crates/parts/src/parts/generator_settings.rs`: `uploaded_model_projection` backs the first failing test; the second repeats its 100-poll asset wait, queue drains and 30 ms sleep. Neither asserts loop success. Keep queued-Apply ordering and all assertions. Reuse mounted fixture and controlled preview helpers. Production `generator_model_upload_resolver` inserts binding and asset atomically: do not assume separate eventual binding acceptance.
- `web/crates/ui-shared/src/model_asset_import.rs`: file `array_buffer` and storage complete before submission. Control the File locally in tests; no production hook.
- `web/crates/runtime/src/runtime.rs`: reuse support `take_held_effects`, `run_effect` and Core gates. Held effects and returned continuations are separate queues; draining once does not await future callbacks.
- `README.md`, `scripts/run-wasm-tests.py`: browser runner environment and crate launches.
- Historical logs named in the ticket confirm unchecked binding reads and cascading panic-hook failures, not late-versus-lost causality.

## Tasks & Acceptance

**Execution:**
- [x] `web/crates/parts/src/parts/generator_settings.rs` — control upload completion against the existing observation sequence; record missing state, held effects and release/recovery or loss before choosing a repair.
- [x] Same test module — repair the demonstrated synchronization defect with explicit completion, bounded failure and diagnostics. Preserve Apply ordering and assertions. Stop if production changes are needed.
- [x] Same test module — cover delayed completion in both affected scenarios and reject never-completed uploads.
- [x] This plan — record cause, controls, runs, review and limitations.

**Acceptance Criteria:**
- Given unchanged repaired code, when the generator-settings browser suite runs five times normally and five times under four CPU-load workers, then both named tests execute and pass in all ten runs.
- Given the established failure condition, when deterministic regression coverage runs against the old observation sequence and the repair, then the old sequence fails and the repair passes, including each related case touched.
- Given an upload that never completes, when the completion check reaches its bound, then it fails diagnostically instead of accepting incomplete state.
- Given non-reproduction alone, when evaluating AC3, then do not claim completion without explaining both historical failures.

## Implementation Notes

- Acceptance: 2026-11-16 — user explicitly requested “Accept and close the Parts bug.” Marked this standalone ticket done based on repair commit `1e047c164aba4d217e371181aa38f434ad5a997e`, deterministic counter-examples, the approved five normal/five loaded runs and independent quick review with no findings. Full project check remains intentionally omitted; exact historical scheduling remains unknown. This acceptance does not close the repair epic or resolve its retrospective.
- Completed coordinator verification and independent quick review with no findings or deferrals. Marked built after all four matrix scenarios executed successfully. Local commit is limited to this plan and the Parts test module; no remote operation.
- Test-local `ControlledUpload` retains a real File and its original byte-read promise, overriding only that instance's `arrayBuffer` completion. Both affected mounted scenarios hold it beyond the old 100 × 20 ms observation plus drains/30 ms; controls check read entry, absent asset/binding, empty held effects, and active reading feedback before release.
- The proposed completion helper drives returned continuations and newly held effects on every turn, requires matching accepted asset/binding and rendered filename, and returns bounded state diagnostics. It leaves queued Apply Core effects deferred after Upload acceptance, retaining the original failure injection and retry-preview assertions.
- Added an immediate-release normal scenario and a never-released-within-bound scenario; the latter asserts the diagnostic error then releases and finishes the upload for shared-WASM hygiene.
- First repaired browser run passed all 22 generator-settings tests. The delayed controls observed file-read entry with neither asset nor binding and no held effects, then release recovered the real upload and all retained assertions passed. This demonstrates an unchecked observation-boundary defect, not a binding lost after atomic acceptance; it does not attribute the historical scheduling delay to the 30 ms sleep itself.

## Plan Change Log

## Review Triage Log

- Quick review: 0 findings (high 0, medium 0, low 0, false 0, maybe-false 0). Independent inspection checked acceptance criteria, repository rules and bugs against the diff and surrounding source; no patches or deferrals.
## Verification

- Coordinator independently read the implementation diff, checked matrix coverage against all 22 observed test outcomes, and reran the module: `.evidence/parts-generator-upload/parent-verification.log`, 22 passed, 0 failed. Coordinator formatting and `git diff --check` also passed; production prefix comparison matched HEAD.
- Deterministic counter-examples: temporarily restored the original unchecked `parameters["switch_3dmodel_filename"]` read at the controlled old boundary. Each named scenario ran separately and failed, after logging `File read pending, asset/binding absent, held effects empty`, with `no entry found for key`. Logs: `.evidence/parts-generator-upload/old-failed-apply.log` and `old-upload-landing.log`. Temporary reads were removed; permanent absence controls remain.
- Repaired control suite: `.evidence/parts-generator-upload/repaired-control.log`, 22 passed, 0 failed. Includes both delayed/released scenarios, immediate release, and diagnostic rejection of the withheld upload followed by cleanup recovery.
- Matrix coverage: normal upload → `uploaded_model_refresh_preserves_apply_draft`; delayed upload and queued failing Apply → `upload_preview_refresh_keeps_a_later_failed_apply_draft`; delayed upload and independent Apply/update blocker → `mounted_generator_upload_landing_preserves_apply_draft_012`; withheld completion → `never_completed_model_upload_fails_diagnostically`.
- Changed-file `rustfmt --check --edition 2024` and `git diff --check` passed. Direct comparison with HEAD confirmed all source preceding the test module is byte-for-byte unchanged. No production/shared-harness change; full project check intentionally omitted under the approved targeted-only protocol. Review remains the parent build workflow's next step.
- Validation-launcher issue (not a test failure): five initial normal runs passed, but Python 3.14 forkserver re-imported the temporary launcher when starting load workers (`Bad file descriptor` / `Connection reset by peer`); no loaded run was counted. Replaced only the temporary launcher with four standalone Python subprocess CPU loops, with ready handshakes, alive checks before/after each suite, and terminate/wait cleanup; reran the complete ten-run protocol.
- Final unchanged-code protocol: normal runs 1–5 and four-worker CPU-loaded runs 1–5 all exited 0, each executing the same 22-test generator-settings suite: 220 passing outcomes, 0 failures. Both originally failing tests and both new tests passed in every launch. All four load workers terminated and were waited for. Summary `.evidence/parts-generator-upload/repeats.log`; individual logs `normal-1.log`–`normal-5.log` and `loaded-1.log`–`loaded-5.log` in the same directory. Every launch used the shared build lock and the installed runner environment, with `--mode no-install`.
- Implementation complete; independent review/staging/commit remain with the parent build workflow. Limitation: controls establish the late-read mechanism and reproduce each historical panic shape, not the exact machine scheduling of the historical runs. No post-acceptance binding loss was observed or repaired.
- Implementation baseline confirmed directly with `git rev-parse HEAD`: `143e1d47ef01d3150daab3b087bb3224838849f9`. Initial compile exposed an ambiguous `unchecked_ref` type in the new test fixture; fixed to `js_sys::Object` before browser execution. Log: `.evidence/parts-generator-upload/initial-repaired.log`.
- Independent BMad Ticket check completed; its sole finding (undefined repeats/load) is resolved by user approval above. The earlier quota-limited attempt was not a review.
- Installed runner environment plus `wasm-pack test --headless --chrome web/crates/parts --lib -- parts::generator_settings::tests::`; verify actual test execution, installing nothing.
- Capture old-sequence failure and release/recovery under `.evidence/parts-generator-upload/`; run the same control after repair.
- Five normal and five four-Python-CPU-worker launches; reliably terminate/join load workers and record every outcome.
- Every build/test launch: `exec 9>~/.cache/tmp/boardstudio-build.lock; flock 9; export BOARDSTUDIO_BUILD_LOCK_FD=9`. No overlapping builds.
- `rustfmt --check --edition 2024 web/crates/parts/src/parts/generator_settings.rs`.
- Inspect diffs before tests and after controls; production unchanged, unrelated files unstaged. Full check intentionally omitted per approval.
