# 34: Replace fixed sleeps in the browser tests (audit phase 2, part 2)

Status: resolved
Resolved scope: the shared helper and the keymap `queued_edit_tests` pilot only. The other suites and the audit merges are tickets 36, 37 and 38.
Type: build
Blocked by: [33](33-test-speed-rust.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Audit: [test-audit.md](../audit/test-audit.md)

## Request

The `browser` step is 159 s and the slow wasm suites spend their time in fixed real-time sleeps
(`TimeoutFuture::new(50)` in loops of 15, `rendered()` 60 ms): `keymap::queued_edit_tests` 27.7 s for 15 tests, `case mechanical_settings` 24 s
for 28, the pcb and parts suites ([audit M1](../audit/test-audit.md)). User, 2026-10-10: implement the audit's recommendations.

## What to do

- Replace each fixed settle loop with a wait for quiescence (poll the condition, bounded by a timeout, return as soon as it holds),
  in a shared test helper per crate or in `ui-shared`/test support, so the pattern is one function.
- Merge the keycode/delay twin tests in the keymap suite into table-driven tests and merge the probable duplicate position-draft tests
  in the presentation inspector tests (audit M2, M3): re-read the bodies first and merge only exact duplicates.
- Keep a test per behaviour; list what each merge preserves.

## Acceptance

Met for the delivered scope (helper + keymap `queued_edit_tests`); the rest moves to [36](36-sleeps-case-pcb-parts.md),
[37](37-sleeps-layout-library-ui-page.md) and [38](38-merge-duplicate-browser-tests.md).

- [x] `check.py browser` before/after recorded, with no loss of assertions (59 assert macros before and after in the converted file; no merge was made).
- [x] No test in the converted file depends on elapsed time except the documented 5 s bound; no flakiness over three consecutive runs (the implementer's and the verifier's).
- [ ] Not delivered here: the other suites (36, 37) and audit M2/M3 merges (38).

## Verification

`repo lint typecheck browser` (sandbox lifted for Chrome), run three times.

## Outcome

Partial scope, implemented by Claude Haiku 5.5 (implementer). Time box reached after the keymap
suite; the remaining suites are not converted. Commits: claim `8eac33960`; helper `bdfcb0190`
(`wait_until` with three unit tests), `6bc6cd5cf` (`wait_until_step`, one unit test); keymap
`7e88541e6` (`queued_edit_tests.rs`). Commits carry `Co-Authored-By: Claude Haiku 5.5` (the attribution
reminder), not the Sonnet trailer the brief named.

**Helper.** `web/crates/ui-shared/src/test_wait.rs` (wasm-only, `cfg(test)` or `test-support`): `wait_for_step`
awaits a step on each poll (default poll 5 ms, bound `WAIT_BOUND_MS` = 5 s), `wait_until` and `wait_for` wrap it
with sync conditions, and `wait_until_step` is the panicking form. Unit tests: returns as soon as true, no pause
when already true, step runs once per poll, timeout message names the wait and the bound. 4 passed.

**Keymap (`queued_edit_tests.rs`).** All 25 `settle` sites became `settle_until(runtime, what, predicate)`: it
drains held effects on each poll and returns when the predicate (the assertion's own condition, which stays after
the wait) holds. The 14 `rendered()` sites became a DOM predicate, `rendered_after(probe, mark)` (the host
rendered after a change; `Probe.renders` counts host renders), or `rendered_accepted` (the host rendered the
accepted revision, `Probe.rendered_revision`). The last one was needed: after an accepted edit the next action
carries the render's admission token, so clicking before the re-render silently does nothing and the test hung
(found with a trace). Assertion count unchanged (59 `assert` calls before and after; the diff is one doc comment).
No fixed sleeps remain in the file. Wasm test count 15 before and after.

**Before/after (`check.py browser`, same machine, noisy).** Before = this session's baseline run (`176.1 s`
total); per-suite "before" figures are from the audit, not re-measured (the baseline log's `finished in` times were
truncated by the output filter).

| Suite | Before (audit) | After run 1 | run 2 | run 3 |
| --- | ---: | ---: | ---: | ---: |
| keymap (40) | 37.0 s | 9.66 s | 9.67 s | 9.69 s |
| case (81) | 30.6 s | 30.53 s | 30.47 s | 30.48 s (not converted) |
| pcb (49) | 26.1 s | 26.19 s | 26.16 s | 26.18 s (not converted) |
| parts (68) | 16.1 s | 16.10 s | 16.11 s | 16.11 s (not converted) |
| layout (166 + 1) | 13.7 s | 13.53 s | 13.55 s | 13.56 s (not converted) |
| library (16) | 11.1 s | 10.93 s | 10.92 s | 10.95 s (not converted) |
| ui-shared (30 + 1) | 8.7 s | 8.75 s | 8.76 s | 8.76 s (not converted) |
| step wall | 176.1 s (this session) | 146.0 s | 124.5 s | 124.3 s |

**Test names.** Diff of `test NAME ... ok` lines in the baseline and run-1 logs: +4
(`test_wait::tests::{returns_as_soon_as_the_condition_holds, a_condition_that_already_holds_does_not_pause,
a_step_runs_on_every_poll_until_it_reports_true, times_out_naming_the_wait_and_the_bound}`), none removed; all
keymap names unchanged. Not the `--list` output as the brief asked (not run); the logs print 534 vs 538 names
(not all suites print that form).

**Mutation checks.**
1. Never true, one wait: `two_queued_layer_adds_get_distinct_ids`, "both layer adds are accepted" forced false.
   FAIL in 5.10 s: `wait_until: both layer adds are accepted did not hold within 5000 ms`. Reverted.
2. Never true, two more waits in one full keymap run: "the undo of the first binding removes the keymap" and
   "the older failure is reported inline". Both FAIL with the named message; 38 passed, 2 failed, 19.89 s. Reverted.
3. Production step removed: `binding_controller.rs` on_change's `begin_field` disabled (`if black_box(false)`).
   `two_binding_fields_queue_and_undo_in_commit_order` did NOT fail with the helper message. It hung until the
   runner's 120 s bound (SIGKILL). The blocked await is the test's own `entered.await` gate on a Core call that never
   starts, which is not a new wait and has no bound. Follow-up: bound the gate await in these tests. Reverted.

**Three consecutive `check.py browser` runs:** all passed (`all steps passed: browser`; run-wasm-tests executed 70,
failed 0 each), walls 146.0 s, 124.5 s, 124.3 s. `typecheck` 7.7 s pass, `lint` 8.6 s pass (fmt + clippy),
`repo` 0.1 s pass.

**Not done (scope left open).**
- Not converted: `case` `mechanical_settings` (`rendered()` 60 ms, `settle_fields` 15x40), `pcb` (30 ms x12),
  `parts` (`tick` 30 ms, 22 sites), `layout` (outline and board-inspector sleeps), `ui-shared`
  `pending_edit_helpers` (`settle` 12x30), `keycaps`, the presentation tests, and the 40 ms `rendered`/`settle`
  helpers in `keymap` `macro_controller`, `macro_editor` and `panel`. Their gains are unmeasured.
- Audit M2 (keycode/delay twins in `macro_controller::mounted_step_tests`) and M3 (presentation position-draft
  pairs): no merges made, so no test was merged and the name diff has no merges.
- GitNexus `impact`/`detect_changes` not run: the index is stale (011f39a) and agents do not refresh it. Callers
  were checked with `rg`: `settle`/`rendered` are local to `queued_edit_tests.rs`; `wait_until`, `wait_until_step`
  and `wait_for` are new additive symbols used only by `queued_edit_tests.rs`.
- The 5 s bound (`test_wait::WAIT_BOUND_MS`) was not stressed on a slower machine.
- Three runs is the verification record; no further flakiness runs were done.
- Stale Chromium profiles older than 30 min: 33 removed from `/tmp`.


## Verification

Verified by Claude Sonnet 5.5 (verifier), 2026-10-10, scope as delivered.

**Reviewed and confirmed.** `wait_until` polls every 5 ms with a `gloo` `TimeoutFuture` (a macrotask, so Dioxus effects and
rendering run between polls), returns before the first pause when the condition already holds, and the timeout message names the
wait and the bound. The module is `cfg(all(target_arch = "wasm32", any(test, feature = "test-support")))`: `cargo tree` for the
page build (`-p boardstudio-web --no-default-features --features page`, wasm32) shows no `test-support` feature, and `check.py typecheck` passes.
Native `cargo check -p boardstudio-web-ui-shared` fails in `boardstudio-web-ui-model` (`unresolved import boardstudio_web_runtime::runtime`),
a crate this change does not touch; the web crates are wasm-only (see AGENTS.md), so this is not a regression.
The `--list` of the keymap crate before (`8eac33960^`) and after: 40 tests each, 15 in `queued_edit_tests`, names byte-identical
(`diff` of the sorted lists printed no difference). Assert macros in the file: 59 before, 59 after the Haiku commits.

**Defects found and fixed (small, in the verifier commit).**
1. *Unbounded gate (the Haiku finding).* All 10 `entered.await.unwrap()` gates in `queued_edit_tests.rs` now go through
   `test_wait::wait_until_future(what, future)` (new `bounded` / `wait_until_future`); `wait_for_step` also wraps its poll loop in
   `bounded`, so a step that awaits something that never happens (for example `run_pending` on a gated reply) fails at the bound too.
   Three new unit tests cover the new paths (ui-shared wasm suite 30 to 33 plus panels).
2. *Two waits that passed on stale state.* "the pending macro field is still enabled" (`!input.disabled()`) and "layer fields show their
   enabled state while removal is pending" (`layers.enabled`) were true before the render that applies the pending state, so the
   following `assert!` could not fail. Both now wait for a render after the action (`rendered_after` with a mark taken before it).
3. *Predicate weaker than the next assertion.* In `queued_macro_steps_preserve_eligible_positions_and_retire_shifted_targets` the wait
   was only the accepted steps, but the next assertion reads the step field's failure alert, which renders after Core accepts; the
   ineligible cases now wait for the alert too.

**Mutation checks (all in one keymap build, each in a different test; reverted and re-checked clean).**
- Never true: "both layer adds are accepted" set to `Some(99)`. FAIL with `wait_until: both layer adds are accepted did not hold within 5000 ms`.
- Always true: "the name and tap edits are accepted" replaced by `true`. FAIL at the next assertion (`assert_eq!(item.name, "Renamed")`, line 470),
  so that predicate is load-bearing; `run_pending` alone does not drain work already driven with `drive_pending`.
- Production step removed: `begin_field` in `binding_controller.rs` disabled. `two_binding_fields_queue_and_undo_in_commit_order` now FAILS in
  about 5 s with `wait_until: the held edit reaches the Core executor did not hold within 5000 ms` (was a hang to the runner's 120 s);
  `failed_binding_restores_accepted_field_and_explains_failure` also fails with its named wait. Whole keymap run with the three mutations: 36 passed, 4 failed, 24.7 s.

**Timings (this machine, sandbox lifted, sequential, nothing else running).** `check.py browser`: 149.9 s, 128.0 s, 126.8 s, all
`all steps passed`, run-wasm-tests executed 70, failed 0; keymap suite `finished in` 9.66 s / 9.68 s / 9.66 s (40 passed), keymap launch
wall 20.7 s / 18.4 s / 19.0 s (includes the incremental compile). Other suites unchanged from the Outcome table (case 30.5 s, pcb 26.2 s, parts 16.1 s,
layout 13.6 s, library 11.0 s, ui-shared 8.8 s). `repo` 0.1 s, `typecheck` 1.4 s (warm), `lint` 3.3 s (warm, fmt + clippy) pass.
The first run is slower than the next two (149.9 s against about 127 s); the keymap result is the same in all three, so no flakiness was seen.

**Left unverified or open.**
- Negative assertions that follow a positive-only wait still rely on the wait having been long enough. Example: `leaving_the_keymap_owner_retires_a_pending_layer_name_silently`
  waits for the layer count, then asserts `failure().is_none()`; if the retirement were removed and the failure arrived a moment after Core
  accepts, the assertion could pass. Not mutation-tested (no quiescence signal exists on `Runtime`). The same holds for `feedback.is_none()` in the
  eligible step cases. A follow-up for ticket 36/37: give the test support an idle signal or queue a sentinel edit behind the held ones.
- The new gate bound and unit tests were not run on a slower machine.
- GitNexus not run (stale index; agents do not refresh it); callers checked with `rg`: the new symbols are used only by `queued_edit_tests.rs`.
