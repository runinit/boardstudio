---
title: BMad-led project tooling cleanup
type: chore
ticket: ''
created: '2026-11-15'
status: built
baseline_revision: 'fac38bc86f4211ee2eb26fe1eddc14e8b7828c6f'
route: oneshot
route_source: auto
risk: low
review: quick
review_source: pinned
lenses_ran: [quick]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Routine tooling still mixes product/build checks with optional agent integrations and diagnostics; obsolete model-lookup files remain after their workflow retirement.

**Approach:** Add an opt-in `agent-tools` step for existing GitNexus refresh/stop-hook, mutation-runner and verification-receipt fixture suites. Keep product tooling in default `tooling` and preserve CI commands. Changed-file checks include optional suites when their utility/test sources change. Remove the model-lookup helper and fixture, repair historical links/registration, and describe commands without imposing a full-check sequence.

Preserve product regressions, native/WASM execution guards, build locks/scheduling, security policies, CI permissions/cache/deploy gates and optional mutation/receipt APIs. Do not change active initiative selection or product behavior. The user authorized committing all earlier work first and continuing on this branch; leave the new cleanup unstaged and uncommitted.

Acceptance:
- Given default or CI product checks, when command plans are inspected, then optional utility suites are absent and existing product/security/release commands remain.
- Given an explicit `agent-tools` invocation or changed optional utility/test source, when check selection runs, then its suites remain executable and failure propagates.
- Given the retired helper and fixture, when references and registered test scripts are checked, then both files are absent without broken links or stale registrations.
- Given runner help and README, when consulted, then they describe optional commands without prescribing an extra review/verification sequence.

</frozen-after-approval>

## Implementation Notes

Oneshot: approximately 80 added/modified code-and-test lines plus mechanical docs edits and two approved deletions. Reuse STEPS, changed-file Selection and existing fixtures. Low risk: no product implementation changes; selection is hermetically testable.

Files: `scripts/check.py` (registry/help/quick plan); `scripts/check_quick.py` (optional-source selection); `scripts/test-check.py` and `scripts/test-check-quick.py` (registration/default/explicit/changed-source assertions); `scripts/model-lookup.js` and `scripts/test-model-lookup.cjs` (delete); `docs/agents/model-lookup.md` (historical paths); `README.md` and `.github/workflows/check.yaml` (optional-suite documentation, unchanged CI commands).

- Implemented the opt-in suite and changed-source routing; appended the Selection field to preserve existing positional constructor arguments. The retired lookup test's registration assertion is replaced by explicit opt-in/default/failure-propagation coverage.
- Gortex's worktree write view rejected the checkpoint HEAD twice; used the documented local-patch fallback. Codegraph's index belongs to the main checkout, so source reads used Gortex's verified current disk content.
- Verification: the new opt-in test first failed with missing `agent-tools`; after implementation, runner fixtures passed 81/81 and quick-selection fixtures 22/22. `check.py repo tooling agent-tools` passed under the shared build lock. Logs: `.evidence/bmad-tooling-cleanup/`.
- A direct checkpoint/current command comparison confirmed unchanged DEFAULT and every non-tooling product step. Exactly four original tooling commands moved to the optional step, with all remaining tooling commands/order retained. Both CI workflows are unchanged after removing comments; security policy, permissions, cache and deployment gates are preserved.
- `check.py --list`, deleted-file absence, branch/HEAD/staging checks and `git diff --check` passed. No Rust product or browser suite was run because no implementation or product command changed.
- Graph detection sees the Git diff but cannot publish an exact post-checkpoint worktree graph yet; coverage/contract fallback warnings are not current-checkout certification. Local runner/selection tests and command comparisons are the verification evidence.

## Plan Change Log

## Review Triage Log

- Quick review: no findings. The context-free reviewer inspected the plan, diff and surrounding code; all stated acceptance criteria were met, with no introduced bugs or instruction violations identified. Nothing deferred.

## Verification

- `python3 -B scripts/test-check.py` — scheduler, registry and CI-contract fixtures pass.
- `python3 -B scripts/test-check-quick.py` — changed-source routing and existing product selections pass.
- `python3 scripts/check.py repo tooling agent-tools` — repository, routine tooling and optional suites pass under the shared build lock.
- `python3 scripts/check.py --list` — defaults exclude agent-tools; explicit step lists four retained fixture suites.
- `git diff --check` — whitespace clean.
- Compare CI commands/security workflow and product/native/browser plans against the checkpoint: unchanged except optional-suite separation.
