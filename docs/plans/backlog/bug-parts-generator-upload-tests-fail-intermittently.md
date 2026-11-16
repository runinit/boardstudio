---
id: 1
type: bug
title: "Parts generator upload tests fail intermittently in the full project check"
parent: none
covers: []
after: []
assignee: ""
refined: false
hitl: false
risk: low
severity: P2
---

# Parts generator upload tests fail intermittently in the full project check

## Description

The browser step of the full project check sometimes fails in the Parts generator-settings tests and then passes on an unchanged retry. A failed run costs a full re-run of the check, and its follow-on failures hide whatever else the run would have shown. The check should give the same result on unchanged code.

## Reproduction

Run `python3 scripts/check.py` repeatedly on unchanged code. Seen in two of the seven full checks recorded for stories 1.1–1.5 of `initiative-layout-test-strength` on 2026-11-15 and 2026-11-16; none of those stories touches Parts code:

- Story 1.2, first full check: `check: first failure [Rust panic/assertion]: parts::generator_settings::tests::mounted_generator_upload_landing_preserves_apply_draft_012 at web/crates/parts/src/parts/generator_settings.rs:3485:24 — no entry found for key; follow-on symptom: 33 later test(s) hit 'cannot modify the panic hook from a panicking thread' in shared WASM state`. Unchanged retry passed.
- Story 1.5, first coordinator full check: `parts::generator_settings::tests::upload_preview_refresh_keeps_a_later_failed_apply_draft at web/crates/parts/src/parts/generator_settings.rs:2461:24 — no entry found for key`, with the same 33 follow-on failures. Unchanged retry passed.

Expected: both tests pass on every run. Actual: each has failed once and passed on retry. The failure has not been reproduced on demand.

An earlier full check on the base commit `14c276248bb8064c8f5d27760b5c33ac7ce63b53` was reported to fail intermittently with "Parts model-upload preview missing parameter"; no log of that run is in this checkout, so it is unconfirmed as the same failure.

## Cause Hypothesis

Both tests run their effects, wait a fixed 30 ms (`gloo_timers::future::TimeoutFuture::new(30)`), then read the generator parameter `switch_3dmodel_filename` from the accepted document. The panic is that read finding no such key. The likely cause is the fixed wait: under load the accepted document has not yet taken the uploaded model's parameter when the test reads it. Not established: whether the parameter arrives later in a failing run, or is lost.

## Acceptance Criteria

1. **The two tests give the same result on every run**
   **Given** unchanged code
   **When** the Parts generator-settings browser tests run repeatedly, including under load
   **Then** `mounted_generator_upload_landing_preserves_apply_draft_012` and `upload_preview_refresh_keeps_a_later_failed_apply_draft` pass every time
2. **Tests cover the condition found and fixed**
   **Given** the test suite
   **When** it runs
   **Then** a test that fails on the established cause and passes on the fix covers it, and one covers each related case the fix touched
3. **Or: no change is needed, with proof**
   **Given** the reproduction
   **When** it is run on the current code
   **Then** the failure cannot be produced and the cause of the two recorded failures is explained, with the evidence recorded in Notes — this supersedes 1 and 2

## References

- parent — none
- logs — .evidence/bmad-build-1-2/full-check.log, line 7402
- logs — .evidence/story-1-5-coordinator-full-check.log, line 7476
- plan — docs/plans/initiative-layout-test-strength/tickets/epic-repair-weak-tests/tickets/story-rotated-envelope-snap-test-fails-when-rotation-is-ignored-plan.md, section Implementation Notes
- plan — docs/plans/initiative-layout-test-strength/tickets/epic-repair-weak-tests/tickets/story-refactor-sweep-plan.md, section Review Triage Log

## Notes

- Open question: is the missing parameter late or lost? The first step of the work is to tell these apart.
- Assumption: severity P2 (the check is impaired and a retry is the workaround); risk low (test gate only, no user-facing behaviour known to be wrong).
