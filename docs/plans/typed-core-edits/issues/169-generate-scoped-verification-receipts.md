# 169: Generate scoped verification receipts for reviews

Status: resolved
Type: build
Priority: P3
Blocked by: None (can start immediately)
Related: [Verification receipt research](../../../research/retro-review-verification-receipt.md)

## What to build

Let an existing check invocation optionally write a compact machine-readable
verification receipt and a concise reviewer summary from the outputs it actually
captured. Reviewers should see which test scopes ran, their observed results and
whether any baseline comparison is supported, without reconstructing counts by hand.

The preceding review misstated two focused group deltas; the corrected historical
values were Case 3 to 4 and Runtime 12 to 14. Current page-suite JSON covers only
one part of browser verification. Reuse the native name guard and existing command
capture; do not build a replacement runner or infer a whole-browser result from
the page suite's counts.

## Acceptance criteria

- [x] An opt-in receipt records invocation identity, source revision/local-change
  status, selected/planned commands, each command's scope, exit status and observed
  results. Current default output and check exit behavior remain compatible.
- [x] Native binaries, direct WASM crate launches and filtered page suites retain
  separate counts, completeness and full observed test names with outcomes, not
  bounded samples. Derive focused group counts from explicit module prefixes within
  each launch's observed names. Provider/product smoke checks remain a separately
  labeled evidence scope, never added to Rust test counts.
- [x] Native baseline reporting reuses full-versus-partial name-guard semantics.
  Show matched, missing and new names against the checked-in guard list, identifying
  its recording revision and local changes. That list is a retention guard, never
  a before-change test count. Before/after deltas require explicit base-revision run
  evidence; without it the comparison is unknown. Do not rewrite the guard list.
- [x] Failures, missing terminal results, malformed output, interrupted commands
  and planned-but-unrun commands remain visible in a partial receipt. A complete
  page run with unexpected failing tests is not reported as passing, and a focused
  run cannot claim full verification. Preserve runner policy for known exclusions:
  show raw failures and named exclusions separately, let gate acceptance follow
  the runner's exit status, and never describe excluded failures as all tests passing.
- [x] Fresh results come from this invocation's captured output. Cached artifacts
  cannot silently fill gaps; any explicitly supplied comparison evidence is marked
  reused with its original run/source identity.
- [x] Fixture-based tests cover multiple native binaries, separate WASM crates,
  page JSON with failures despite terminal completeness, partial/early-stop runs,
  malformed output, known exclusions and known versus unknown baseline evidence.
  Use whole-crate output fixtures whose full names yield Case collision-group 4 and
  Runtime collision-group 14; require explicitly identified base-run evidence for
  group counts 3 and 12. Verify unknown comparisons when only current output or the
  native retention list is supplied. No new filtered-launch CLI is required.
- [x] The reviewer summary is derived from the receipt, keeps scopes and limitations
  visible, and is documented as the evidence to provide at review handoff. Run the
  affected tooling tests and a bounded invocation smoke check; do not rerun full
  browser/provider builds solely to exercise formatting.

## Comments

Created at the user's request from retro item 4 after primary-source research.
Independent of the other three retro follow-ups; no new source-manifest ceremony
or external reporting service is required.

Revised after Claude Opus audit: retain full names for reproducible group counts,
separate retention baselines from base-run evidence, and preserve known-failure policy.

## Answer — 2026-10-18

Implemented in `b4a4595b`, with review corrections in `4a49af46`. The opt-in receipt
and derived summary keep native binaries, WASM launches, page results and product
smoke scopes separate. They retain full names/outcomes and module-prefix groups,
record source/run identity, and distinguish known exclusions from raw failures and
runner acceptance. Native retention identities include their binary; the retention
list is never presented as the reviewed change's before-count.

Parent behavioral RED/GREEN covered failed native evidence loss, stale page results,
incomplete/differently filtered base comparisons, preflight/interrupt partial receipts,
malformed payloads and colliding names across binaries. Both reviewers also found
stale summary wording and rejected invocations writing empty receipts; those were
reproduced and fixed. All **26 receipt**, **31 runner** and **15 quick-runner** tests
pass. The child's initial missing-module RED is not behavioral regression evidence.

The real full check at clean code revision `4a49af46` passed all six gates in
**526.6 s**. Its receipt records **56/56 commands**, no unrun commands, a clean source
identity, 2,064 native passes and 11 existing ignores, all 1,992 retention names
matched with none missing, and 72 pre-existing names outside that list. Every direct
WASM launch was complete/passing; page results were fresh and complete with
**103 passed, zero failed**. Runtime/Case collision-group counts derive from the
whole-crate names; before/after stays unknown because no base-run receipt was supplied.

Receipt and summary: `.evidence/retro-implementation/full-receipt.json` and its `.md`
companion; logs and RED/GREEN: `.evidence/retro-implementation/169/`. No second test
execution is used to collect results. Remote CI, optimized release and benchmarks
were not run or claimed.
