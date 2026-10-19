# 169: Generate scoped verification receipts for reviews

Status: claimed
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

- [ ] An opt-in receipt records invocation identity, source revision/local-change
  status, selected/planned commands, each command's scope, exit status and observed
  results. Current default output and check exit behavior remain compatible.
- [ ] Native binaries, direct WASM crate launches and filtered page suites retain
  separate counts, completeness and full observed test names with outcomes, not
  bounded samples. Derive focused group counts from explicit module prefixes within
  each launch's observed names. Provider/product smoke checks remain a separately
  labeled evidence scope, never added to Rust test counts.
- [ ] Native baseline reporting reuses full-versus-partial name-guard semantics.
  Show matched, missing and new names against the checked-in guard list, identifying
  its recording revision and local changes. That list is a retention guard, never
  a before-change test count. Before/after deltas require explicit base-revision run
  evidence; without it the comparison is unknown. Do not rewrite the guard list.
- [ ] Failures, missing terminal results, malformed output, interrupted commands
  and planned-but-unrun commands remain visible in a partial receipt. A complete
  page run with unexpected failing tests is not reported as passing, and a focused
  run cannot claim full verification. Preserve runner policy for known exclusions:
  show raw failures and named exclusions separately, let gate acceptance follow
  the runner's exit status, and never describe excluded failures as all tests passing.
- [ ] Fresh results come from this invocation's captured output. Cached artifacts
  cannot silently fill gaps; any explicitly supplied comparison evidence is marked
  reused with its original run/source identity.
- [ ] Fixture-based tests cover multiple native binaries, separate WASM crates,
  page JSON with failures despite terminal completeness, partial/early-stop runs,
  malformed output, known exclusions and known versus unknown baseline evidence.
  Use whole-crate output fixtures whose full names yield Case collision-group 4 and
  Runtime collision-group 14; require explicitly identified base-run evidence for
  group counts 3 and 12. Verify unknown comparisons when only current output or the
  native retention list is supplied. No new filtered-launch CLI is required.
- [ ] The reviewer summary is derived from the receipt, keeps scopes and limitations
  visible, and is documented as the evidence to provide at review handoff. Run the
  affected tooling tests and a bounded invocation smoke check; do not rerun full
  browser/provider builds solely to exercise formatting.

## Comments

Created at the user's request from retro item 4 after primary-source research.
Independent of the other three retro follow-ups; no new source-manifest ceremony
or external reporting service is required.

Revised after Claude Opus audit: retain full names for reproducible group counts,
separate retention baselines from base-run evidence, and preserve known-failure policy.
