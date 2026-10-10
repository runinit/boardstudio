# 36: Replace fixed sleeps in the case, pcb and parts browser tests

Status: implemented (awaiting verification)
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

- [ ] Per-suite before/after table in the Outcome; `browser` three consecutive passing runs; executed test names equal the baseline except deliberate merges (diff the crate `--list` outputs before/after).
- [ ] Mutation checks: three representative waits per ticket made never-true fail with the helper's message within the bound; the production step each depends on removed makes the test fail fast.

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
  landing waits for the accepted revision, since its next assertion is a negative. Bare held gates
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
- pcb `queued_edit_tests.rs`, "the second undo clears the board configuration": its predicate is
  `is_none_or(boards.is_empty())`, which is true when the hardware is absent. It can hold before the undo
  lands. The suite passes, but this predicate is weaker than it should be (lesson 3). It was not
  mutation-checked. Follow-up: name the removed board's id.
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
