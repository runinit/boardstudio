# 006: Rust CAD runtime capture

Status: claimed
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

## 2026-10-26: implementation inventory and baseline

Ticket 005 was resolved for local scope at `6bcadff`; hosted CI remains separate.
The current launcher in `cad/scripts/measure-cad.py` checks the build and source
binding, staged package, artifact currency and post-capture drift; it still invokes
`measure-cad-runtime.cjs`. That driver owns the CSV and event output, worker reply
identity and completeness checks, response status and service-worker metadata,
cold-cache and warm-start behavior. The U1 path in `measure-cad-page.cjs` also
requires real page interaction, file upload and downloads. The shared Rust harness
provides isolated launch, page errors, evaluation and input; the CAD response and
cache/service-worker observation and U1 paths still need implementation and proof.

On `6bcadff`, `python3 -B cad/scripts/test-measure-cad.py` passed 16 tests and
`node cad/scripts/test-measure-cad-runtime.cjs` passed five controlled protocol
checks plus STL controls. These are baseline tooling checks, not browser or
old/new parity evidence. No Rust CAD capture, regression RED/GREEN, cache fixture,
negative controls, real capture parity, full check or closure review has been
completed for ticket 006. The existing runner remains the default and tickets 007
and 008 have not been advanced.
