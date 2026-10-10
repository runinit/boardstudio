# 36: Replace fixed sleeps in the case, pcb and parts browser tests

Status: resolved
Resolved scope: the mounted settlement tests in `web/crates/case` (13 of 16 `settle_fields` sites plus the mounted field waits), the pcb `queued_edit_tests` (all 17 sites) and 5 of 11 sites in `pcb_physical_setup/tests.rs`. Everything listed under "Not converted" in the Outcome, including all of `web/crates/parts`, moves to [37](37-sleeps-layout-library-ui-page.md) (section "Carried from 36"). The ticket's original acceptance (suites free of fixed sleeps, `parts` converted) is not met and is not claimed.
Type: build
Blocked by: [34](34-test-speed-browser.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Audit: [test-audit.md](../audit/test-audit.md)

## Request

Continue ticket [34](34-test-speed-browser.md): replace fixed settle loops with `wait_until` (`web/crates/ui-shared/src/test_wait.rs`) in the suites named below. The keymap
`queued_edit_tests` pilot took that suite from 37.0 s to about 9.7 s with the assertion count and test names unchanged. Pattern to follow: each wait checks the precondition the next
assertion needs (including a `rendered_accepted`-style wait for the accepted revision before an action); every wait is bounded; no test depends on elapsed time except that bound; unbounded
`.await` gates in these tests get a bounded variant.

## What to do

- `web/crates/case` (`mechanical_settings`: `rendered()` 60 ms with about 95 calls, `settle_fields` 15x40 ms; baseline suite 30.6 s), `web/crates/pcb` (30 ms x12 settles; 26.1 s), `web/crates/parts` (`tick` 30 ms at about 22 sites; 16.1 s). Largest first.

## Acceptance

Met for the delivered scope only (see Verification); the unconverted sites are carried to 37.

- [x] Per-suite before/after table in the Outcome; `browser` three consecutive passing runs (implementer) plus two more (verifier); executed test names equal the baseline (diff of the crate `--list` outputs, empty for case and pcb; parts source is unchanged).
- [x] Mutation checks: representative waits made never-true and always-true, and a production step removed, by the implementer and again by the verifier on different waits.
- [ ] Not delivered here: `parts` (all), the case `rendered()` sites, the intermediate pcb waits, and the negative-only settle sites. Carried to 37.

## Verification

`repo lint typecheck browser` (sandbox lifted for wasm-pack and Chrome), run three times. Process: [verification-process.md](../../../agents/verification-process.md). Time box about two hours.

## Outcome

Implemented by Claude Haiku 5.5 (implementer; no separate verifier pass has run). Partial scope:
case and pcb are converted in part, parts is not started. Time: about two and a half hours, over
the two-hour box, mostly spent on the browser runs and the mutation checks. Commits: claim
`8068c9801`; case `b322d2505` (mounted settlement tests); pcb `7279e7301` (queued wiring and
physical setup tests). No parts commit. Trailers are `Co-Authored-By: Claude Sonnet 5.5`, as the brief
named, although the implementation was done by Haiku 5.5.

**Converted.**
- case, `mechanical_settings::mounted_settlement_tests`: 13 of 16 `settle_fields` (15 x 40 ms) sites are now
  `settle_until(runtime, what, predicate)`, a `wait_until_step` that drains held effects and then
  checks the state the next assertion reads. Mounted fields wait for their inputs to render. The held
  landing waits for the accepted revision (the verifier added the rendered revision), since its next assertion is a negative. Bare held gates
  (`entered.await`) use `wait_until_future`.
- pcb, `pcb_wiring::queued_edit_tests`: all 17 `settle` (12 x 30 ms) sites; the helper is now `settle_until`.
- pcb, `pcb_physical_setup::tests`: 5 of 11 settle sites.

**Not converted (remaining).**
- case, `mechanical_settings.rs`: 76 `rendered()` (60 ms) sites in `contextual_layer_tests` (pure DOM
  test pages, before `mod mounted_settlement_tests`), about 4.5 s; 17 intermediate `rendered()` (40 ms)
  sites in the mounted module, about 0.7 s; 3 `settle_fields` sites whose next assertions only prove
  absence (`preparation_from_an_unmounted_field_cannot_bind_a_replacement_field`,
  `catalogue_latest_preparation_keeps_identical_newer_text_while_pending`, and the second settle in
  `catalogue_latest_preparation_error_survives_older_failure_arriving_last`), about 1.8 s. Case remains
  above the one-second stopping rule.
- pcb: 12 `rendered()` (30 ms) between-action sites in `queued_edit_tests.rs` and 12 in
  `physical_setup/tests.rs` (about 0.7 s total); 6 physical settle sites at lines 343, 395, 419,
  511, 518 and 560 (about 2 s). 343, 395 and 419 assert only absence or follow an `Event::Open`; 511,
  518 and 560 (layout tests) were not read. Pcb remains above the threshold.
- parts: not started. `tick()` (30 ms) is defined in `component_model_editor`, `module_attachment`,
  `module_profile_editor` and `mechanical_profile_ui`; `parts.rs`, `parts_new_component.rs`,
  `parts_import_footprint.rs`, `generator_settings.rs` and `assembly_editor.rs` also hold 30 ms waits.
  `rg` counts 88 matches for `tick().await` or `TimeoutFuture::new(30)` in `web/crates/parts/src`,
  including four definitions; about 2.6 s. Each site needs its own predicate.

**Before and after.** Before is one baseline `check.py browser` run, made before the edits. Each
after column is one of three consecutive runs. Per-suite times are the `finished in` values from the
wasm-pack log.

| Suite | Before (baseline) | After run 1 | After run 2 | After run 3 |
| --- | ---: | ---: | ---: | ---: |
| case (81) | 30.83 s | 14.81 s | 15.81 s | 14.70 s |
| pcb (49) | 26.46 s | 19.84 s | 20.73 s | 18.34 s |
| parts (68, not converted) | 16.11 s | 16.13 s | 16.63 s | 16.15 s |
| `browser` step wall | 132.7 s | 177.3 s | 148.3 s | 155.5 s |

The step wall did not fall in these runs: 148 to 177 s against 133 s in the single baseline run. The
parts suite, which is unchanged, moved within noise, and the per-suite falls are 16 s (case) and 7 s
(pcb). I did not isolate the step-level cause. The machine is shared, and the step includes launches
and rebuilds that vary between runs. The baseline was one run, so the before column is not a mean.

**Test names.** `wasm-pack test --headless --chrome web/crates/<crate> --locked --lib -- --list`,
names sorted, before against after. Counts are case 81 and 81, pcb 49 and 49, parts 68 and 68. The
`diff` is empty for each crate. No test was merged or renamed.

**Mutation checks.** Each was one edit, then one filtered run, then `git checkout` of the file, and the
tree was clean after each.
1. Never-true, case held landing (`false && input.value() == original`): FAIL with
   `wait_until: the held landing is accepted and the newer original draft stays did not hold within 5000 ms`,
   in 5.22 s.
2. Never-true, case catalogue failure (`false && wall_thickness == 2.0`): FAIL with
   `wait_until: the catalogue failure restores or keeps the draft did not hold within 5000 ms`, in 5.31 s.
3. Never-true, pcb undo of a layout name (`false && ...`): FAIL with the named message in 5.80 s; the other
   five queued-edit tests passed.
4. Always-true, pcb refused edit (`|| true`): the test fails at its next assertion, `the failure stays
   inline` (left None, right Some), so the predicate is load-bearing. The first version of this
   predicate was `is_none_or(boards.is_empty())`, which holds before the refused edit reports. The
   suite caught it on the first run, and it was replaced by the failure's presence. This is the lesson
   3 case. After the failed assertion a RefCell `already borrowed` panic at `queued_edit_tests.rs:80`
   appears as aftermath; I did not investigate it.
5. Production step removed, case: `MechanicalSettingsController::settle` returns at once. The test
   `unchanged_failed_mechanical_fields_restore_accepted_text_and_report_inline` fails with
   `the untouched failed draft restores its accepted text did not hold within 5000 ms`, in 5.15 s.
   It fails at the bound and does not hang.

**Three-run record.** `python3 scripts/check.py browser` ran three times in a row, all sequential:
all steps passed each time, and run-wasm-tests executed 70 with 0 failures each run. Then
`repo` passed in 0.1 s, `typecheck` in 5.3 s, and `lint` (fmt and clippy) in 3.2 s.

**Follow-ups.**
- pcb `queued_edit_tests.rs`, "the second undo clears the board configuration": the implementer suspected its
  predicate `is_none_or(boards.is_empty())` could hold before the undo. The verifier checked: the board
  configuration exists before that undo, so the predicate is false until the undo lands (no change needed; see
  Verification).
- Negative-only sites with no positive sentinel are the remaining settle sites in case (3) and pcb (343,
  395, 419), plus the layout sites in pcb (511, 518, 560) that were not read. The fix is an idle signal, or a
  sentinel edit queued behind the held ones (ticket 34 follow-up).
- The 76 contextual `rendered()` sites have no shared render signal. A per-page render counter, as the
  keymap pilot's `Probe.renders` uses, would give predicates for the between-action waits.

**Unverified.**
- GitNexus `impact` and `detect_changes` were not run: the index is stale (011f39a), and agents do not
  refresh it. Callers of the new helpers were checked with `rg`, and each helper is module-local:
  `settle_until`, `wall_thickness`, `accepted_revision` and `has` in `mounted_settlement_tests`; `settle_until` and
  `configuration_if_present` in `queued_edit_tests.rs`; `settle_until` in `physical_setup/tests.rs`.
- The 5 s bound was not stressed on a slower machine.
- The mutation checks were run once each. Three runs of the full browser step were the only repeat.
- The baseline is a single run.
- Chromium cleanup: stale profiles older than 30 minutes were removed after the sandbox was lifted,
  because the sandboxed `rm` failed with `Read-only file system`. Zero older than 30 minutes remain.

## Verification

Verified by Claude Sonnet 5.5 (verifier), 2026-10-10, in the nested `tce-matrix` worktree, scope as delivered. The machine was shared with a
cold Codex build, so load was 4 to 18 (1-minute average) during the runs and timings are indicative only; conclusions below rest on correctness.

**Site review (predicate against the next assertion).** Every converted site in case and pcb was read against the assertions that follow it.
Weaker predicates found and fixed in commit `d10f6cf60` (small, in the test files only):
- case, held landing (`a_newer_original_mechanical_draft_survives_an_older_held_landing`): waited for the accepted revision in Core state only,
  and the next assertion is the negative `inline_failure(..).is_none()`. The test host now renders the accepted revision in a `.test-revision`
  marker and the wait includes `rendered_accepted` (the `rendered_accepted` pattern of ticket 34). The same barrier is added to the second wait of
  `newer_catalogue_failure_and_draft_survive_an_older_actual_ticket_landing`, whose failure-still-shown and draft-still-shown conditions were
  already true before the held edit landed, so only `wall_thickness == 3.0` (Core state) was doing the waiting.
- pcb, `create_net_disables_while_held...`: waited for the created net only; the next assertions read the queued pad assignment, the re-enabled
  button (DOM) and silence. It now waits for the creation, the assignment, the render of the accepted revision and the enabled button.
- pcb, `part_net_assignment_queued_behind_layout...`: the wait was named "the pad assignment lands" but only proved the layout move; it now proves both.
- pcb, `plan_queued_behind_part_deletion...`: waited for the deletion only; the next assertion reads the refused apply's inline failure, which is
  set after the deletion lands and a render. The wait now includes the failure.
- pcb, `failed_field_ticket_restores_accepted_value...`: the wait "the unrelated edit keeps the field failure" was `failure().is_some()`, true before
  the edit and so a no-op. Investigating showed a pre-existing defect: the injected Core failure leaves the session in save recovery, and the
  "unrelated" `ResolveEdit` ends as `BlockedByRecovery("save recovery is required")` and never lands (name and revision unchanged after 1 s). The
  assertion "an unrelated edit must not clear the field's failure" therefore never exercised a landed edit, before or after ticket 36. The wait now
  names the edit's terminal outcome (`observe_next`) plus the rendered revision, and a comment records the finding. Making the edit land is carried to 37.
- pcb, `physical_topology...`: the "undo removes the instances" wait could only be meaningful if instances exist before the undo; the test now asserts
  they do (it would have been vacuous otherwise; it passes).
Checked and left as is: all other case waits (the predicates equal the assertions and include DOM state that changes only after a render), pcb
`wiring_mode...` (all three waits; the second undo's predicate is false before the undo because the board configuration still exists), the
three-undo and part-net undo waits, and the five physical-setup waits (they read `mount(&probe)`, set only on a host render).
Residual, not fixed (needs a quiescence or idle signal, recorded for 37): negative assertions that follow a wait for a positive effect still rely on
the effect's follow-up render having happened. Case: held landing `inline_failure.is_none()`, presentation return `inline_failure.is_none()`,
profile siblings `failure_at(height).is_none()`. Pcb: `feedback.is_none()` after the net landing, the hidden-stage `take_held_effects` check
(vacuous by construction: `settle_until` drains held effects first). Dioxus effects re-render one idle turn after the render that triggered them,
so the rendered-revision wait narrows but does not close this window.

**Executed test names.** `wasm-pack test --headless --chrome web/crates/<crate> --locked --lib -- --list`, names sorted, `8068c9801^` against
`0d5c8155b`: case 81 and 81, pcb 49 and 49, diff empty for both (the parent files were checked out over the tree for the listing and restored;
tree clean after). Parts: 68, and `git diff 8068c9801^ 0d5c8155b -- web/crates/parts` is empty, so its list is unchanged by construction.

**Mutation checks (verifier; different waits from the implementer's).**
1. Never true, case "the profile volume failure restores its field": FAIL with `wait_until: the profile volume failure restores its field did not
   hold within 5000 ms`.
2. Never true, pcb "the layout move and the pad assignment both land": FAIL with the named message.
3. Always true, case "the held Session edit lands and the returned field shows it": the next assertion fails (`left: 2.0, right: 3.0`,
   "the already submitted Session edit still runs"). Predicate is load-bearing.
4. Always true, pcb "the net creation and the queued pad assignment land and the action re-enables": the next assertion fails (creation count
   `left: 0, right: 1`). Load-bearing.
5. Always true, pcb "the second undo clears the board configuration" (the implementer's flagged site): the test still passes. `run_pending`
   completes the undo inside the first poll, so this predicate cannot decide the outcome; it is not weaker than the assertion either (it is the
   assertion). No change.
6. Production step removed, pcb `apply.rs` (the "A part used by this wiring plan was deleted" retire disabled): FAIL in about 5 s with
   `wait_until: the accepted edit removes the mcu part and the refused apply reports did not hold within 5000 ms`.
7. Production step removed, case `DimensionField` effect (the `!dirty` guard dropped, so an accepted landing overwrites the newer draft): five
   mounted tests fail, including the held landing with `wait_until: the held landing is accepted and the newer original draft stays did not hold
   within 5000 ms`. The same mutation with the old predicate (without `rendered_accepted`) was also caught in 3 of 3 runs, so I could not show the
   rendered-revision barrier is necessary on this machine; it is kept as the lesson-3 barrier and costs nothing.
All mutations were reverted (`git status` clean for `apply.rs`; the test files were restored from copies and re-run green).

**Timings (sandbox lifted, sequential, `check.py browser`; Haiku's table is in the Outcome).** Run 1 (immediately after my edits, so case and pcb
recompiled): step 164.9 s, load average 6.3 at start and 14.1 at end; per-suite `finished in` values were truncated by the output condenser, only
the launch walls survive (case 48.8 s, parts 50.5 s, pcb 51.8 s, compile included). Run 2 (raw output via `rtk proxy`): step 144.1 s, load 13.1
at start and 13.4 at end; suite `finished in`: case 15.11 s, pcb 18.34 s, parts 16.63 s (unchanged suite), keymap 9.72 s, layout 13.99 s,
library 11.02 s, ui-shared 8.93 s; launch walls case 31.0 s, parts 32.8 s, pcb 33.6 s. Both runs: `all steps passed: browser`, run-wasm-tests
executed 70, failed 0; case 81, pcb 49, parts 68 passed. Against Haiku's table (case 30.83 s to about 15 s, pcb 26.46 s to about 19 s, parts about 16 s)
the suite results agree; the `browser` step wall (164.9 s, 144.1 s; Haiku 148 to 177 s, baseline 132.7 s) is dominated by launches and rebuilds and
by the concurrent Codex build, and no conclusion is drawn from it. `typecheck` 6.4 s, `lint` 3.0 s (fmt and clippy), `repo` 0.1 s, all passed.

**Not verified.** GitNexus not run (stale index; agents do not refresh it): the changed symbols are test-module local, checked with `rg`.
The 5 s bound on a slower machine. Flakiness beyond the five full runs. The effect of the new barriers on the residual negative assertions (see above).
