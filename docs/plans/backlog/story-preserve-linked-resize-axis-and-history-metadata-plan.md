---
title: 'Preserve linked-resize axis and history metadata'
type: chore
ticket: 3
created: '2026-11-16'
status: built
blocked_reason: ''
baseline_revision: '564cf69fd996e149bcaacbcdafd93702cdf00b71'
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

**Problem:** The accepted linked-resize test derives reflection from the post-edit axis and omits layout ownership/link metadata from relevant-state history snapshots. These are coverage gaps, not established product defects.

**Approach:** Capture both accepted layouts' ownership and full mirror links before planning. Use the independently captured accepted axis for reflected poses. Before the act, clone accepted metadata and explicitly expect the right link to adopt `split_gap: Some(142.0)` with `spacing: None`; all ownership, axis, correspondence and exceptions remain unchanged and the left link stays absent. Do not invoke production adoption helpers or pre-adopt the fixture. Compare full committed metadata to that independent expectation; retain full accepted and validated committed metadata in both-halves Undo/Redo snapshots without revision counters. Preserve widths 18→56, displacements −19/0/+19, partner sizes, six unique identities, document/scene agreement, the legacy zero-angle fixture and real planner/typed Core seam. Change only the existing test/local closures and this plan. Unexpected failures against unchanged production block completion for separate investigation. No A2, A3/A4 redo, pilot acceptance, epic closure, audit, mapping change, cleanup, branch switch, stash/reset/rebase, push or PR. The three pre-existing untracked files remain unread, untouched and excluded.

</frozen-after-approval>

## Implementation Notes

Oneshot: fewer than 100 source lines expected, one local test. Low business risk: no production behavior change. Reuse the local snapshot/member closures in `web/crates/layout/src/objects/keycap_resize.rs`; `LayoutMirrorLink` supports Clone/PartialEq. Snapshot layout id, board id, matrix ids, part ids and full optional mirror link, not names or document revisions. Gortex file reads work; indexed reads are unrouted, so scoped native fallback is authorized. User approved proceeding with the three untracked files preserved.

- Ticket validation by an independent inspection-only agent returned no findings before test execution.
- Gortex writes remained `view_building` on the retry after 30 seconds for both the plan and source; used the repository native-edit exception. Source diff is confined to the existing linked-resize test: accepted metadata capture, independent accepted axis, metadata-inclusive snapshots and committed-state invariance.
- Historical block (resolved by approved criteria in Plan Change Log): the first strengthened named test fails against unchanged production. Accepted partner link has `split_gap: None`; committed partner link has `split_gap: Some(142.0)`. Other displayed ownership/link fields match. `core/src/model.rs` documents absent split gap as legacy board-coordinate placement until the first committed geometry edit adopts the half frame. This suggested a requirement/fixture-semantic question, not a confirmed production bug. At that point Build stopped for separate investigation; no assertion weakening, fixture repair or production fix was authorized.

## Plan Change Log

- 2026-11-16 — User explicitly approved aligning this frozen intent with the adoption-aware ticket committed as `c956d5d3409a96d9ced1b0957d13bc38a83608ba`, clearing the resolved block and resuming A1 Build. Original baseline and failure evidence below are preserved. Whole-link invariance across commit is superseded by an independently expected complete adopted link, not metadata exclusion; no production defect established. Independent Ticket validation returned no findings. Existing A1 test edits remain authorized continuation work.
- Gortex plan writes were rejected twice as stale after HEAD changed; used the repository's native-edit fallback. The historical blocked note and original named log below remain evidence of the prior criteria conflict, not a current unresolved product bug.

## Review Triage Log

- Resumed A1 quick inspection review: no findings; no patches, deferred findings or unresolved review questions. Thorough lenses were skipped for the oneshot route.
- Parent `git diff --check` passes. Production portion before `#[cfg(test)]` is byte-identical to original baseline `564cf69fd996e149bcaacbcdafd93702cdf00b71`; all temporary control edits are absent. Current source diff is confined to the existing linked-resize test.

## Verification

Serialize every Cargo/check invocation using the repository FD9 build lock; preserve configured concurrency/debug flags. Long logs go under `.evidence/a1-linked-metadata/` only.

- `cargo test -p boardstudio-web-layout --lib objects::keycap_resize::tests::replace_document_reflows_source_and_core_syncs_linked_partner_through_history -- --exact`: execute one test, zero ignored.
- Temporary test-act controls, assertions unchanged: shift committed axis plus partner document/scene X poses together; separately corrupt ownership and correspondence with geometry intact; corrupt adopted split_gap and spacing independently; corrupt metadata at Undo and Redo reply boundaries. Each must fail a relevant assertion, not compilation/setup. Restore each exact edit and rerun the named test to one pass/zero ignored. Record quoted failures and commands here; no controls committed.
- `cargo test -p boardstudio-web-layout --lib objects::keycap_resize::tests::`: all module tests pass, zero ignored.
- `python3 scripts/check.py`: full normal project check passes; record any failures accurately.
- Inspect final diff: only test code and this plan, existing numeric/identity oracles retained, no production changes. Review per Build workflow. Ticket validation precedes execution.

### Execution result — 2026-11-16

- Ran the exact named Cargo command above under FD9 lock with `BOARDSTUDIO_BUILD_LOCK_FD=9`; log `.evidence/a1-linked-metadata/named.log`.
- Compiled successfully; executed one test: **0 passed, 1 failed, 0 ignored**, exit 101.
- Failure: `assertion `left == right` failed: resize preserves accepted layout ownership and mirror links`. Committed `split_gap: Some(142.0)` differs from accepted `split_gap: None`.
- No controls, module tests, full project check or Build review ran after this failure. No production edits, commits, pushes or PRs. Temporary controls were never introduced. Three protected pre-existing untracked files remain unread and untouched.

### Resumed execution — adoption-aware criteria

- Historical execution result above describes the original halted run; the following supersedes its pending-execution state, not its failure evidence. Original `.evidence/a1-linked-metadata/named.log` remains unchanged.
- Test-only implementation retains complete metadata in snapshots, captures accepted axis before planning and constructs independently expected full committed metadata before the act: accepted right link is legacy `None`/`None`; only expected `split_gap` becomes `Some(142.0)`. Source/partner geometry or fixture construction are unchanged.
- Named command from Verification under FD9 lock: **1 passed, 0 failed, 0 ignored** (`adoption-named.log`). Module command: **10 passed, 0 failed, 0 ignored** (`module.log`).
- Seven temporary reply-boundary controls, assertions unchanged, each compiled and executed **0 passed, 1 failed, 0 ignored**, exit 101. Each exact control was removed before its restored named run, each **1 passed, 0 failed, 0 ignored** (`control-{name}.log`, `restored-{name}.log`):
  - `axis`: shift right-link axis +1 and both partner document/scene X positions +2 together.
  - `ownership`: replace right-layout board ownership with `wrong-board`, geometry intact.
  - `correspondence`: replace matrix correspondence mirror ID with `wrong-matrix`, geometry intact.
  - `gap`: clear adopted right-link `split_gap`, geometry intact.
  - `spacing`: supply `spacing: Some(142.0)` instead of absent, geometry intact.
  - These five fail at `assertion left == right failed: resize preserves ownership and mirror links with expected gap adoption`.
  - `undo`: leave `split_gap: Some(142.0)` in the Undo reply with restored geometry; fails `assertion left == right failed: Undo restores both halves`.
  - `redo`: clear `split_gap` in the Redo reply with correct geometry; fails `assertion left == right failed: Redo restores both halves`.
- Commands are the exact named command above, with log redirected to each named log; expected failure status was checked as 101. All logs are under `.evidence/a1-linked-metadata/`. No controls remain.
- Gortex detect identifies only the test module/function; guards have no rules, contract allows the change, and graph discovery has no covering-test edges. Explicit execution above supplies the named/module evidence; no production extraction/refactor was performed.
- Full `python3 scripts/check.py` under FD9 lock completed with exit 0: **all six steps passed — repo, tooling, build, lint, test, browser**, total 727.4s (`full-check.log`). Browser execution: **263 executed, 0 failed**. No full-check failure remains; intermediate expected error diagnostics are not the terminal result.
- Final disposition: built, not human-accepted/done. Only the existing test and this A1 Build plan are included in the implementation commit; prior ticket/refinement commits remain intact. Protected pre-existing untracked files stay unread, untouched and excluded. No production changes, A2 work, A3/A4 redo, epic acceptance/closure, cleanup, push or PR.
