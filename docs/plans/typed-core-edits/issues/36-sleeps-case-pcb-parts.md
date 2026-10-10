# 36: Replace fixed sleeps in the case, pcb and parts browser tests

Status: ready-for-agent
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
