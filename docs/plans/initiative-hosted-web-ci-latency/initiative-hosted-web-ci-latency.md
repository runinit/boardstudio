---
type: initiative
status: in-progress
title: "Hosted web CI is predictable in duration and shows useful progress while it runs"
parent: none
sources:
  - docs/plans/research-web-ci-latency/research-web-ci-latency.md
  - docs/plans/research-web-ci-latency/digests/measurements-logs.md
companions: []
after: []
risk: medium
---

# Hosted web CI is predictable in duration and shows useful progress while it runs

## Description

A codebase research into two hosted attempts of the `web` job (run 44018165934, commit `46d758d`) found the passing attempt took 2146.1 s, of which nine of fourteen crate launches each carried 285–322 s of unexplained overhead beyond 148 s of actual tests, and that three buffering layers left the job log silent for up to 454 s at a time. When this is done, the cause of that wait is demonstrated from source, any fix changes only the proven mechanism, every long segment reports useful progress, and per-filter page timings survive the run as an artifact. Source: the research document under References.

## Outcome

Anyone waiting on the hosted `web` job can see what is running and trust that its duration is explained by the work it does, where today roughly half of a 36-minute run is an unexplained wait shown as silence.

## Requirements

- R1 (research section 1; Open questions 1–4): The cause of the 285–322 s overhead carried by nine of fourteen crate launches in both attempts is demonstrated from the exact `wasm-bindgen-test-runner` 0.2.129 and WebDriver source, or one discriminating instrumented experiment is specified with its hypothesis, the observations that settle it, and its stopping condition. It is met when a written finding names the wait, anchored in that source, or the experiment spec is complete.
- R2 (research section 1; measurements digest, Crate launches): Any latency fix changes only the mechanism R1 proves. It is met when representative hosted runs after the fix show the 285–322 s overhead group gone or reduced against the A1/A2 baseline, with all fourteen launches and the 263 page tests still passing.
- R3 (research section 3): The build lanes, the crate launch batch, and the page runner each show useful progress in the hosted log within an interval set at epic refinement, while a command's failure output still prints whole and uninterleaved. It is met when no segment of a hosted run is silent beyond that interval.
- R4 (research sections 1 and 3): The page runner's per-filter `browser.json` durations are retained as a CI artifact on success and on failure. It is met when the artifact is present on both a passing and a failing hosted run.

## Constraints

- Existing checks, behavior, and test counts are preserved; no check is removed or weakened to gain speed.
- No numeric speedup is committed before the spike; the A1/A2 figures in the measurements digest are the only baseline.
- The spike reads source only, external runner and ChromeDriver source included; builds, tests, CI reruns, and experiments need separate user approval.
- Cold-build and cache behavior are unchanged; the cache-on-failure-off rationale in `7f787b9cb` stands unless separately decided.
- A test-coverage claim requires reading the test bodies first; the research read 41 test names and one body, so no missing-test defect is established.

## Done when

- R1's finding or experiment spec is written and accepted, naming the wait from exact source or the one experiment that settles it.
- If a fix is adopted under R2, representative hosted runs show the slow group gone or reduced against the A1/A2 baseline with every launch and page test still passing; if none is adopted, the finding records why.
- A hosted run's log shows progress in each of the build, crate batch, and page runner segments within the interval set at refinement.
- `browser.json` is uploaded on a passing and on a failing hosted run.
- The full project check passes on the delivered branch with unchanged test counts.

## Boundaries

- Out: cold-build and cache optimization; the cold build cost is explained separately in the research and is not this initiative's latency problem.
- Out: fixing the attempt-1 lost-runner event; its cause is unknown and resource starvation is only a hypothesis.
- Out: speculative timeouts, cancellation guards, or concurrency changes not derived from the proven mechanism.
- Out: the cargo lock and scheduling waits and the serial-command shutdown handling the research describes (sections 2 and 4); they are neither the measured ~300 s overhead nor the missing progress.
- Out: broad code cleanup, and the merge of PR 19.
- Must not change: what the `web` job checks — the same commands, crates, filters, and test counts run before and after.
- Touch point: `scripts/build-patched-wasm-bindgen.py` and `scripts/wasm_bindgen_tools.py` — read by the spike, changed only if R1 proves the patched pair is the fix; owner: epic-browser-launch-latency.
- Touch point: `.github/workflows/check.yaml` — artifact retention, and a fix only if the proven mechanism lives there; owners: epic-ci-progress-feedback (artifact), epic-browser-launch-latency (fix).
- Tracer path: the spike demonstrates the wait from source, the proven fix removes it on a hosted run, and the progress epic's thinnest slice retains `browser.json` end to end.

## References

- research — docs/plans/research-web-ci-latency/research-web-ci-latency.md, sections 1–5 and Open questions
- measurements — docs/plans/research-web-ci-latency/digests/measurements-logs.md, the A1/A2 baseline tables
- research digests — docs/plans/research-web-ci-latency/digests/locator-and-analyzer.md; digests/historian.md; digests/verifier.md; brief.md in the same folder

## Notes

- Decision: 2026-11-17 — user approved this initiative and split: two epics over one, because progress feedback is usable even if the wait proves to be upstream; the research folder stays in place as the source; the personal active initiative switches to this slug; commit is asked separately.
- Decision: 2026-11-17 — no platform-baseline epic; this is an existing codebase with its check and CI in place, and the opening work is the spike.
- Decision: 2026-11-17 — the two epics share `scripts/run-wasm-tests.py` and `scripts/check.py`; the spike is read-only so nothing collides now, and fix stories are sequenced against progress stories per file when the latency epic is re-sliced after the spike.
- Open question: the progress interval and mechanism for R3 (heartbeat lines vs streaming captured output); settled at epic-ci-progress-feedback refinement after reading the relevant test bodies. Sofia's pick: a heartbeat at most every ~60 s.
- Unknown: what wait consumes the ~300 s inside a slow launch; spike 1.1 in epic-browser-launch-latency exists to answer it, and that epic's fix entries wait on the answer.
