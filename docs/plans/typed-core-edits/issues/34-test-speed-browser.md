# 34: Replace fixed sleeps in the browser tests (audit phase 2, part 2)

Status: claimed
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

- [ ] `check.py browser` before/after recorded, with no loss of assertions (counts of assertions per merged test).
- [ ] No test depends on elapsed time except a documented timeout bound; no flakiness over three consecutive runs.

## Verification

`repo lint typecheck browser` (sandbox lifted for Chrome), run three times.
