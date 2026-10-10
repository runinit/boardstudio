# 37: Replace fixed sleeps in the remaining browser suites (layout, library, ui-shared, keycaps, keymap helpers, page; plus what 36 left in case, pcb and parts)

Status: ready-for-agent
Type: build
Blocked by: [36](36-sleeps-case-pcb-parts.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Audit: [test-audit.md](../audit/test-audit.md)

## Request

Continue ticket [34](34-test-speed-browser.md): replace fixed settle loops with `wait_until` (`web/crates/ui-shared/src/test_wait.rs`) in the suites named below. The keymap
`queued_edit_tests` pilot took that suite from 37.0 s to about 9.7 s with the assertion count and test names unchanged. Pattern to follow: each wait checks the precondition the next
assertion needs (including a `rendered_accepted`-style wait for the accepted revision before an action); every wait is bounded; no test depends on elapsed time except that bound; unbounded
`.await` gates in these tests get a bounded variant.

## What to do

- `web/crates/layout` (35-80 ms settles, outline settle up to 100x10 ms; 13.7 s), `web/crates/library` (80 ms and 40x20 ms polls; 11.1 s), `web/crates/ui-shared` (`pending_edit_helpers.rs` `settle` 12x30 ms; `geometry_scripts` up to 100x10 ms), `web/crates/keycaps` (40 ms), the 40 ms helpers in keymap `macro_controller`, `macro_editor` and `panel`, the page binary tests under `web/src/presentation` (40 ms x27; `case_workspace` settle 100 ms). Stop where the gain per suite is under about 1 s.

## Acceptance

- [ ] Per-suite before/after table in the Outcome; `browser` three consecutive passing runs; executed test names equal the baseline except deliberate merges (diff the crate `--list` outputs before/after).
- [ ] Mutation checks: three representative waits per ticket made never-true fail with the helper's message within the bound; the production step each depends on removed makes the test fail fast.

## Verification

`repo lint typecheck browser` (sandbox lifted for wasm-pack and Chrome), run three times. Process: [verification-process.md](../../../agents/verification-process.md). Time box about two hours.

## Carried from 36

Ticket [36](36-sleeps-case-pcb-parts.md) was resolved for its delivered scope only (verified 2026-10-10; Outcome and Verification there). The work below was not
done and belongs to this ticket, with the same pattern (each wait is the precondition of the next assertion, bounded, plus the render of the accepted revision where
the next assertion reads a field or hook) and the same stopping rule (stop where a suite gains under about 1 s). Counts are from 36's Outcome; re-count with `rg` first.

- **case** (`web/crates/case/src/mechanical_settings.rs`): 76 `rendered()` (60 ms) sites in `contextual_layer_tests` (pure DOM test pages, before `mod
  mounted_settlement_tests`; about 4.5 s; no shared render signal, so give each page a render counter as the keymap pilot's `Probe.renders` does);
  17 intermediate `rendered()` (40 ms) sites in the mounted module (about 0.7 s); 3 negative-only `settle_fields` sites (about 1.8 s):
  `preparation_from_an_unmounted_field_cannot_bind_a_replacement_field`, `catalogue_latest_preparation_keeps_identical_newer_text_while_pending`, and the second
  settle in `catalogue_latest_preparation_error_survives_older_failure_arriving_last`. They assert only absence; they need an idle signal or a sentinel edit
  queued behind the held ones, and `settle_fields` stays only until then.
- **pcb**: about 24 intermediate `rendered()` (30 ms) waits (12 in `pcb_wiring/queued_edit_tests.rs`, 12 in `pcb_physical_setup/tests.rs`; about 0.7 s) and 6
  `pcb_physical_setup/tests.rs` settle sites still on the fixed `settle` (lines near 343, 395, 419, 511, 518, 560; about 2 s). 343, 395 and 419 assert only
  absence or follow an `Event::Open`; 511, 518 and 560 (layout tests) were not read.
- **parts** (`web/crates/parts`): not started. About 84 `tick()` (30 ms) sites (`component_model_editor`, `module_attachment`, `module_profile_editor`,
  `mechanical_profile_ui` define `tick`; `parts.rs`, `parts_new_component.rs`, `parts_import_footprint.rs`, `generator_settings.rs` and `assembly_editor.rs`
  also hold 30 ms waits); suite 16.1 s, about 2.6 s of waits. Each site needs its own predicate.
- **Found by the 36 verifier, not yet fixed**: in pcb `failed_field_ticket_restores_accepted_value_and_keeps_feedback_across_another_edit`
  (`pcb_wiring/queued_edit_tests.rs`) the "unrelated" `ResolveEdit` ends as `BlockedByRecovery("save recovery is required")` because the injected Core failure
  leaves the session in save recovery; it never lands, so "an unrelated edit must not clear the field's failure" is untested. Plan: fail with a refusal that
  does not enter recovery (or recover first), then wait for `name == "Later name"` and `rendered_accepted(&probe)` before the failure assertion, and
  mutate (clear the failure on any revision change) to see it fail.
- **Residual negatives**: negative assertions that follow a positive wait (case held landing, presentation return, profile siblings; pcb `feedback.is_none()`
  after the net landing) still rely on effect-driven re-renders having run. A test-support idle signal (no dirty scopes, no held effects, no in-flight
  Core work) would close this; consider adding it to `test_wait.rs` before converting more suites.

Acceptance for this section: the list above is converted or each remaining site is recorded with its reason, per-suite before/after table for case, pcb
and parts, name diff empty, mutation checks as in the section above.
