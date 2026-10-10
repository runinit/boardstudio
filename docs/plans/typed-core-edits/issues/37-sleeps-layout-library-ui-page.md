# 37: Replace fixed sleeps in the layout, library, ui-shared, keycaps, keymap helpers and page browser tests

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
