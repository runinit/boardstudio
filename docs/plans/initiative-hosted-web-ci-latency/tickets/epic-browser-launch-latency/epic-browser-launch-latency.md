---
type: epic
status: in-progress
title: "Browser launch latency is explained and the proven wait removed"
parent: initiative-hosted-web-ci-latency
covers: [R1, R2]
sources:
  - docs/plans/research-web-ci-latency/research-web-ci-latency.md
  - docs/plans/research-web-ci-latency/digests/measurements-logs.md
companions: []
after: []
refined: true
risk: medium
---

# Browser launch latency is explained and the proven wait removed

## Description

Nine of fourteen crate launches in each of two hosted attempts carried 285–322 s of overhead beyond their tests, the slow set changed between attempts, and four crates were fast in both. The code names a 300 s navigation allowance and sets `pageLoadStrategy: eager`, yet the waits remain, and an opt-in patched `tiny_http` runner pair exists unexplained and unselected by CI. When this epic is done, the wait is demonstrated from the exact runner and WebDriver source — or one experiment that settles it is specified — and any fix the proof supports has removed the wait on hosted runs. Source: the research under References.

## Outcome

Whoever next waits on the hosted `web` job, or decides whether to adopt the patched runner pair, acts on a demonstrated cause instead of a 300 s coincidence.

## Requirements

- R1 (parent R1): The ~300 s launch wait is demonstrated from the exact `wasm-bindgen-test-runner` 0.2.129 source and the ChromeDriver shipped with Chrome for Testing 156.0.8078.73 (the pin in the measurements digest), including whether `pageLoadStrategy: eager` applies to it, what the `tiny_http` task-pool patch changes and why it was written, and what the four always-fast crates' test pages lack that the others load; or one discriminating instrumented experiment is specified with hypothesis, observations, and stopping condition.
- R2 (parent R2): The adopted fix, if any, changes only the proven mechanism, and representative hosted runs show the 285–322 s overhead group gone or reduced against the A1/A2 baseline in the measurements digest, with all fourteen launches and the 263 page tests still passing.

## Constraints

- The spike reads source only, external runner and ChromeDriver source included; no builds, tests, CI reruns, or experiments without separate user approval.
- No speculative guards: timeout, concurrency, or runner changes land only from the mechanism R1 proves.
- The patched pair is neither adopted nor deleted on suspicion; R1 decides its relevance.
- Existing checks and test counts are preserved by any fix.

## Done when

- The spike's finding, written to `finding-launch-wait.md` in this epic's folder, names the wait from exact source with anchors, or specifies the one experiment that settles it, and the user accepts it.
- If a fix follows, representative hosted runs show the slow group gone or reduced against baseline with every launch and page test passing, delivered through the normal CI path; if no fix follows, the finding records why.
- The full project check passes on the delivered branch.

## Boundaries

- Out: progress reporting and artifact retention — epic-ci-progress-feedback owns those.
- Out: cold-build and cache optimization, and the lost-runner event.
- Must not change: which crates, filters, and tests run, or their pass criteria.

## References

- parent — docs/plans/initiative-hosted-web-ci-latency/initiative-hosted-web-ci-latency.md, section Requirements
- research — docs/plans/research-web-ci-latency/research-web-ci-latency.md, section 1 (Browser startup and execution) and Open questions 1–4
- measurements — docs/plans/research-web-ci-latency/digests/measurements-logs.md, Crate launches table

## Notes

- Decision: 2026-11-17 — tracer bullet is spike 1.1, the riskiest unknown first. Fix entries are planned by re-slicing after the spike because their shape depends on its answer, so R2's coverage is deferred with the user's knowledge; sequencing against epic-ci-progress-feedback's stories is settled per shared file at that re-slice.
- Decision: 2026-11-17 — no "Refactor sweep" entry: the breakdown opens with a single spike; revisit if re-slicing grows it past three entries.
- Decision: 2026-11-17 — spike 1.1 accepted as done (`85fe869aa`): verdict "experiment specified" — phase bounds, `eager`'s application and limits, the `tiny_http` lost-queue mechanism, and the fast-four `dioxus-web` difference are demonstrated from source; which request stalls and what releases it are not. User approved the finding's §7 experiment (logging-only instrumented runner, up to three hosted batches, no fix, patched pair unused) as entry 1.2, opening with an independent follow-up review of the finding's patched §6–§7 passages. R2's fix entries remain deferred until 1.2 reports; re-slice then.
- Decision: 2026-11-17 — replan on new evidence (fastest-path review, Opus): experiment batches 1 and 2 both reproduced slow launches, then lost their tri-logs when the runner died at page-runner start (uploads skipped), so entry 1.2 is paused after B2 with no B3 on the unchanged capture design. User approved entry 1.3, a patched-pair A/B whose verdict reads off streamed log timestamps and therefore survives a runner death, run in parallel with a resource sampler. Requirements R1/R2 are unchanged — the A/B is a trial on a throwaway branch, not adoption on suspicion; a pass is intervention evidence for R1 and the shape R2's acceptance needs, and adoption still lands as its own fix entry after user acceptance. The lost-runner scope question and main-branch cache seeding are deferred until the A/B and sampler report.
- Unknown: which request stalls in a hosted slow launch and which event releases it at ~240–300 s; entry 1.3 tests the one-condition mechanism by intervention, entry 1.2 (paused) would name the request and release, and this epic's fix entries wait on the A/B verdict.
