# 173: Mark interrupted native test binaries incomplete in receipts

Status: resolved
Type: build
Priority: P2
Blocked by: None (can start immediately)
Related: [169: Verification receipts](169-generate-scoped-verification-receipts.md); [Research](../../../research/audit-input-boundaries-2026-10-18.md)

## What to build

Reviewers see incomplete native test evidence when a launched test binary never
finishes, even if every test name observed so far has passed. Preserve all actual
observations and the check's existing failure status; do not invent outcomes for
tests whose names were never printed.

The audit supplied output announcing 100 tests, one passing result, no terminal
binary summary and exit 130. The receipt marked the command complete with no
problems. The overall invocation correctly remained failed; the defect is its
misleading completeness metadata.

## Acceptance criteria

- [x] First reproduce that interrupted-output case yielding complete evidence.
  After the fix it is incomplete and failed, retains the one observed pass and
  exit 130, and explains the missing summary/count mismatch.
- [x] Track each launched native binary independently, require its terminal
  result summary, and reconcile announced executed-test count, named terminal
  outcomes and summary totals. Filtered-out tests are not executed tests.
- [x] Missing, truncated, inconsistent or malformed binary output cannot become
  complete through another binary's summary or the command exit code. A launch
  interrupted before announcing tests also remains incomplete.
- [x] Valid zero-test, filtered, ignored and multi-binary cases stay supported.
  A fully observed failing suite may be complete while remaining failed; keep
  terminal completeness distinct from pass/fail and baseline retention policy.
- [x] Keep observed incomplete test names separate from any unnamed/unobserved
  shortfall. JSON and its derived reviewer summary agree, without fabricating
  missing test identities or changing known-exclusion and reused-evidence rules.
- [x] Run affected receipt, native-output parser and check-runner tests plus a
  bounded invocation smoke check. Preserve default check output/exit behavior and
  the checked-in native name guard. Full browser/CAD rebuilds are unnecessary.

## Comments

Created from the user's audit findings after primary-source research. A focused
follow-up to resolved 169, which remains unchanged. Independent of the other four active
tickets; no replacement runner, external receipt service or new manifest scheme.

## Answer — 2026-10-18

Implemented on `dev`. `check_native_tests.launches` now returns one record per launched
native test binary (announced `running N tests` count, named outcomes, terminal
`test result:` summary), and `verification_receipt.native_record` reconciles those three
per launch: a launch is complete only with its own terminal summary, announced count and
named terminal outcomes. A missing summary, a launch that never announced a count, and a
disagreement between announcement, names and summary each add a problem, without inventing
identities for never-printed tests. `render_summary` prints the same problems and marks the
native section `INCOMPLETE`.

Unchanged: the name guard (`collect`/`verify`), the retention list's matched/missing/new
semantics, known-exclusion and reused-evidence policy, and terminal completeness as distinct
from pass/fail. Only a named test observed starting without a verdict is counted `incomplete`.

RED/GREEN at the real seam (`.evidence/ticket-173/repro.py` calling the public
`build_receipt`/`native_record`): the audit case — announced 100 tests, one observed pass, no
summary, exit 130 — returns `complete: true`, `problems: []` against the pre-fix `HEAD`
(`5f8d3ac39`) modules, and `complete: false`, `passed: false` after the fix with the one
observed pass and exit 130 retained and both problems reported: `no terminal test-result
summary` and `announced 100 test(s) but 1 named outcome(s) were observed`. Logs:
`.evidence/ticket-173/red.json`, `green.json`.

Tests: 33 native-output parser, 35 receipt, 31 check-runner, 15 quick-runner, all passing
(`.evidence/ticket-173/green-tests.log`). Bounded invocation smoke: `check.py test --receipt`
with a stand-in `cargo` printing the interrupted output exits 1 (default behavior preserved)
and writes a partial receipt whose native command is `complete: false`, exit 130,
`passed: false`, one named pass, zero fabricated names, and both problems in JSON and in its
`.md` summary (`.evidence/ticket-173/smoke.json`, `smoke.log`, `smoke.json.md`). No browser or
CAD rebuild was run; the parent owns the integrated full six-gate check.

### Follow-up: per-outcome reconciliation and truncated summaries

A diff read of the first implementation found `_launch_problems` compared only the summary
*total* (`passed + failed + ignored`) with the named terminal count. A summary that mislabelled
an outcome (`0 passed; 1 failed; 0 ignored` against one observed `ok`) therefore passed as
complete, and the `FAILED`/`ok` status token was never checked against `failed`. The summary
regex also accepted a truncated line (`… filtered out` with no `; finished in N.NNs`).

`check_native_tests.SUMMARY` now requires libtest's own `; finished in N.NNs` tail, so a launch
killed while printing its summary has no terminal summary, and `Launch.summary` retains the
status token. `_launch_problems` reconciles each named outcome (passed/failed/ignored) with the
summary's own count and requires the status token to agree with the failed count; the announced
count check is unchanged, and the total check is subsumed by the per-outcome one. Filtered-out
and measured tests are still not executed tests. The retention list, known-exclusion and
reused-evidence rules are untouched.

RED/GREEN at the real seam (`.evidence/ticket-173/followup-repro.py`, calling the public
`build_receipt`/`native_record`): against the pre-fix `HEAD` (`f209af58b`) all three cases —
mislabelled outcome, `FAILED` with zero failed, and a truncated summary — return
`complete: true, passed: true`; after the fix each returns `complete: false, passed: false` with
its own problem, while a complete consistent summary stays `complete: true, passed: true`
(`followup-red.json`, `followup-green.json`).

Tests: 34 native-output parser, 37 receipt, 31 check-runner, 15 quick-runner, all passing
(`.evidence/ticket-173/followup-green-tests.log`). Bounded invocation smoke with a stand-in
`cargo` printing a truncated summary exits 1 (default behavior preserved) and writes a partial
receipt whose native command is `complete: false`, exit 101, `passed: false`, one named pass,
zero fabricated names, the truncated-summary problem in JSON and `INCOMPLETE` in its `.md`
(`smoke-truncated.json`, `smoke-truncated.log`, `smoke-truncated.json.md`). Not covered: the
analogous `WASM_SUMMARY` in `verification_receipt.crate_record` still accepts a `filtered out`
line without `finished in` (wasm-pack launches, out of this ticket's native scope); no browser or
CAD rebuild was run.
