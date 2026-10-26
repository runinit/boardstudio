# 006: Rust CAD runtime capture

Status: ready-for-agent
Type: build
Blocked by: [Rust drag smoke end to end](005-rust-drag-smoke.md)
Parent: [Chromiumoxide browser checks](../map.md)
Related: [Real-browser checks drive Chrome from Rust](../../cad-deepening/issues/016-rust-browser-harness.md)

## What to build

The existing CAD measurement workflow invokes the Rust harness instead of Playwright and
produces equivalent measurement artifacts and verdicts. Extend the shared harness only where
this consumer requires it; keep the old capture available until comparison passes.

## Acceptance

- [ ] Preserve launcher inputs, capture arguments, timeouts, exit codes, CSV columns/events
  log fields, source/build binding, readiness/currency checks and failed-capture semantics.
- [ ] Old/new capture against the same packaged build and inputs proves equivalent events,
  statuses and output schema. Compare nondeterministic timing fields by their documented
  meaning, not byte identity; do not claim a speed improvement from this port alone.
- [ ] Cold/warm HTTP-cache and service-worker conditions match the old runner. Prove bypass
  versus blocking explicitly, including a controlled worker fixture; retain response status
  and service-worker-origin metadata without dropped required events.
- [ ] Preserve protocol self-check coverage without a browser. Keep any Node-based tooling
  tests that still serve that contract; removing all Node is not this migration's goal.
- [ ] Shared isolated browser launch/error handling is reused. Negative controls prove
  malformed replies, page failures, capture identity drift and incomplete observations fail.
- [ ] Run affected tooling tests and real old/new capture parity before switching the default;
  retain the old runner if any required proof is blocked. No production CAD behaviour changes.

## Verification and scope

Use TDD at the existing public launcher/output seams: demonstrate the expected regression
before the fix, then run affected tests and typechecking during development. At closure run
the full supported check and combined Spec + Standards review; record terminal verdicts,
source/build identity and any blocked gates honestly. No weakened assertions, masked page
errors, baseline deletions or retry-until-green. Preserve the archived prototypes and unrelated
work. Builds follow repository concurrency/lock rules; evidence uses supported receipts and
logs, not custom hash manifests. All checks must be runnable by humans and CI without agent tools.
