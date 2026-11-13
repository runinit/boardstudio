# 008: Draft-owner registry and Geometry scripts tracer at both Update Now checkpoints

Status: resolved
Type: build
Assignee: Coordinator (closure)
Parent: [Draft-safe update adoption spec](../spec.md)
Blocked by: none — can start immediately

## Focused Retry reconciliation ownership (2026-11-07; CLAIMED)

Sole implementation/packaged-build/browser owner for the fresh 010 escalation;
base `99eafb6447ef389fc651f8200addf7abcf2171b5`. Exact production/test ownership:
`web/crates/ui-shared/src/geometry_scripts.rs`, existing local
`queued_script_tests` module (native Gortex verified before edits). Also own
`scripts/verify-update-adoption.py` and 008/010 ticket progress. No registry,
Runtime, public API or other owner edits. Preserve unrelated dirty
`CODING_STANDARDS.md` and untracked coordinator research.

Isolated real RED: `successful_save_retry_reconciles_the_rolled_back_clean_script`
accepts intended source at r1 after `Event::RetrySave`, but the still-mounted
source remains empty. Log `008-retry-clean-red.log`: exactly 1 executed failure
at the source-equality assertion, not a compilation failure/cascaded panic.
GREEN `008-retry-clean-green.log`: 1/1. Command for both:
`wasm-pack test --headless --chrome --mode no-install web/crates/ui-shared --lib -- successful_save_retry_reconciles_the_rolled_back_clean_script --nocapture`.
The effect now observes accepted value as well as owner identity, following it
only if this same owner's draft equals its previously observed accepted value.
This reconciles failure rollback after explicit successful retry, without
changing pending helpers, failure restoration or the live registry predicate.
Completed at `be39738b10b985d0529b6ee53a1677f513502b4d`: the additional
`successful_save_retry_preserves_newer_input_while_saving` regression and clean
Retry regression pass. The clean-value guard mutation is killed
(`008-retry-newer-red.log`); no registry or failure-restoration policy changed.
Coordinator independently reran Geometry **6/6**, pending helpers **16/16** and
notifications **30 executed / 0 failed** in `008-coordinator-retry-{geometry,helpers,notifications}.log`.
Fresh Spec and Standards reviews of `21d3e7a..be39738b` report PASS, no required
changes; Standards discloses partial driver coverage, not structural certification.
Parent then read the complete combined source diff and independently passed five
packaged legs, including both literal Retry → direct Update Now paths and Layout
(`010-coordinator-retry.log`). The narrower driver-proof follow-up and outstanding
010 acceptance limits are recorded in [010](010-packaged-a-to-b-proof.md).
That composite checkpoint is now accepted; the final Answer below supersedes
historical CLAIMED/review-pending statements.
Matching Dioxus v0.7.10 docs (not Leptos) verified through Context7:
`use_reactive` supplies non-signal dependencies; `peek` avoids subscriptions.
Native Gortex named read + explore(task) both succeeded; post-edit contract is
WARN (lower-bound behavioral impact), no configured guards, graph test coverage
absent. Later review and coordinator evidence above supersede that pending state;
final integrated verification still belongs to 015.

## Progress (implementation, 2026-11-07; NOT resolved — awaiting coordinator review)

Implemented on branch `fix/ui-integration-recovery` (tracer: registry +
Geometry scripts adapter + both checkpoints). Mounted GREEN at seam S1;
packaged proof belongs to 010. Stopping-point report filed:
`update-draft-safety/008-implementation.md` (notes) and `.evidence/`
`008-red-toast.log`, `008-green-notifications.log` (28/28),
`008-check-quick.log` (exit 0), `008-check-build.log` (exit 0),
`008-check-browser.log` (runtime + ui-shared, exit 0),
`008-check-webbin.log` (28 executed, 0 failed).

- Registry: `web/crates/runtime/src/draft_owners.rs` (new), Runtime-owned
  `RefCell<DraftOwnerRegistry>`, polled synchronously by
  `update_safety_inputs` (both checkpoints + post-takeover reload share it).
  RAII guard; explicit `retire_draft_owner` on unmount/scope change;
poll-prune drops dead entries (one-time `web_sys::console::warn_1`
  diagnostic per entry — the named log/surface) and stale scopes silently.
- Gate: `UpdateSafetyInputs::unresolved_drafts`,
  `UpdateBlocker::UnresolvedDraft { area, remedy }`, message
  `"{area} has an unapplied edit. {remedy}, then choose Update Now again."`
  Geometry copy: area `Geometry scripts`, remedy
  `Apply the script edit, or cancel the edit in the form`.
- Geometry adapter compares the live JSON-encoded draft against the
  live-projected accepted encode (mount-default snapshot when no accepted
  script exists) and the key's in-flight submitted value (new
  `PendingEditSignals::submitted_draft`). No JSON-draft refactor (005).
- Keyboard scope (mounted): tab stops + focus + Space/Enter
  non-interception proven at both checkpoints; synthetic key events do not
  synthesize native clicks (repo-proven button contract), so trusted-key
  activation belongs to packaged runs (010+).

### Escalation RED → GREEN (2026-11-07; awaiting fresh review)

Standing implementation authorization covers GREEN; this is a checkpoint,
not ticket closure. Preserved the three prior RED regressions and their
`.evidence/update-draft-safety/008-sol-red-isolated.log`: each isolated
command executed one test and failed the expected `Some("divergent draft")`
versus `None` assertion after the edit stopped pending, before helper drain.

- **Fixed terminal-record suppression:** `submitted_draft` now returns a
  value only while `is_pending(key)` is true. Completed, failed and refused
  records awaiting `settle` no longer exempt a divergent draft. No signature,
  field-restoration, resolver, key-equality or JSON representation changes.
- RED and GREEN command template (run once per test, serialized):
  `wasm-pack test --headless --chrome --mode no-install web/crates/ui-shared --lib -- <test-name> --nocapture`.
  Tests: `only_in_flight_submissions_exempt_the_current_draft_before_settle`,
  `failed_submission_does_not_exempt_a_divergent_draft_before_settle`,
  `refused_submission_does_not_exempt_a_divergent_draft_before_settle`.
  GREEN: **3/3**, one executed per launch, log
  `.evidence/update-draft-safety/008-sol-green-isolated.log`.
- Focused existing checks using the same command template with module filters:
  `pending_edit_helpers::mounted_tests` **16/16**;
  `geometry_scripts::` **4/4**. Log:
  `.evidence/update-draft-safety/008-sol-focused-ui.log`.
- Added mounted `mounted_geometry_back_retires_the_draft_owner` and
  `mounted_geometry_unrelated_submission_never_clears_the_draft` through the
  existing Update Now seam. The fixture now honors the real editor's Back
  callback by unmounting it. `python3 scripts/run-wasm-tests.py --files web/src/presentation/notifications.rs`:
  **30 executed, 0 failed** (previous coordinator run: 28/28), log
  `.evidence/update-draft-safety/008-sol-notifications.log`.
- `rustfmt --check --edition 2024 web/crates/ui-shared/src/pending_edit_helpers.rs web/src/presentation/notifications.rs`
  passed; log `.evidence/update-draft-safety/008-sol-fmt.log`.
  `git diff --check` passed. No full gate/build rerun: coordinator owns 015.

### Per-finding dispositions (not resolved before review)

1. **Terminal submitted memory:** reproduced RED, fixed and GREEN above.
   The authoritative 004 predicate is live != baseline AND no same-key
   pending submission carries live; stale memory alone is not pending work.
2. **Failure/refusal policy:** intentionally unchanged. `apply_to_field`
   restores accepted text if live still equals submitted; then no divergent
   draft remains. Newer text survives and still blocks (mounted failure and
   older-settlement tests pass). The terminal-window tests cover refusal as
   well as failure without inventing a requirement to retain reverted text.
   Legitimate `Resolution::Unchanged` still lands; no resolver path changed.
3. **ScriptActionKey identity collapse:** no reproduced regression.
   `on_select_script` calls `pending.settle(false, ...)` before switching;
   `held_script_failure_is_retired_when_active_script_changes` passes in the
   4/4 Geometry subset. Do not change equality alone: the editor binds its
   single Apply field/disabled Signals with the empty-id wildcard.
4. **New creation owner:** infeasible as a pre-acceptance editable form.
   `+ New script` immediately submits `new_script_resolver`; editable fields
   render only for an accepted `active_script`. Existing queued-New test
   passes. No imaginary creation-form owner added.
5. **Cancel/Reset coverage:** Geometry has neither control. Its real
   `Back to selection` callback and Escape callback leave the editor.
   Added Back retirement coverage rather than inventing buttons; Escape
   reaches the same callback (trusted activation remains packaged work).
   Existing baseline-revert, genuine-unmount and hidden-mounted tests pass.
6. **Unrelated submission:** added project-rename submission plus observed
   accepted rename, exact source preservation and Geometry-specific Update
   Now deferral; **GREEN** in the 30-test notification run.
7. **Trusted input claim:** corrected the composite checkbox below to
   partial. Mounted code proves tab stops/focus and synthetic Enter/Space
   non-interception, not trusted pointer or native key activation. Trusted
   pointer + Tab/Enter/Space at toast/header remain open in 010.
8. **Ticket metadata:** Type/Parent/Blocked by now precede Progress within
   the first 12 lines. Status remains ready, awaiting the
   coordinator's independent failure-path rerun and fresh combined recheck.

## What to build

The complete draft-safety contract end-to-end for one owner: a Runtime-owned
registry of live draft owners that both Update Now checkpoints (update toast
and header notice) poll synchronously, plus the Geometry scripts owner's
registry adapter. Choosing Update Now with a valid unapplied Geometry scripts
Draft value defers adoption with a cancel-only message naming the Geometry
scripts area and its remedy (Apply the script edit, or cancel the edit in the
form). Adoption proceeds only once no live owner reports an unresolved draft
and no save is pending or failed. The Geometry scripts editor's JSON-encoded
draft stays unchanged; the adapter compares the live encoded draft against the
owner-canonically encoded baseline per admission key.

## Acceptance criteria

- [x] Registration is RAII: an owner registers on mount and retires on genuine unmount or scope change; hidden, keep-mounted and portal-retained owners keep blocking.
- [x] Unresolved state is judged per admission key against the live-projected baseline (or mount-default snapshot where no accepted value exists), using owner-canonical equality; reverting to the baseline clears the key.
- [x] Poll-prune drops dead entries rather than treating them as blockers, with a one-time diagnostic whose log/surface is named in the ticket's completion record.
- [x] RED first: the mounted equivalents of the confirmed draft-loss assertions fail for the expected reason before the fix, then pass (GREEN) — report at the RED and mounted-GREEN stopping points before proceeding.
- [x] Mounted tests cover valid explicit-submit and realistic invalid never-submitted, DOM-unmarked drafts; both entry points; update-dialog Cancel preserves exact value/focus; Apply landing, baseline reversion and actual Back retirement; unrelated submission never clears; failure/older settlement preserve newer input. Helper regressions cover completed/failed/refused terminal memory. Existing accepted-value restoration remains unchanged. Mounted tab stops/focus and synthetic Enter/Space non-interception pass. Coordinator packaged runs verify **trusted pointer and trusted Tab/Enter/Space at both entries**, including actual header Space activation (`010-coordinator-oracles.log`). The composite [010](010-packaged-a-to-b-proof.md#answer) checkpoint, including held-save timing, independent reviews and cleanup, is accepted. Geometry has no Cancel/Reset form controls.
- [x] Deferral copy follows the settled per-key label+remedy rule; no injected invalid marker, save label or worker label is used as evidence.
- [x] Focused quick/build/browser checks for touched crates pass; evidence logs are stored per the plan's evidence conventions.

## Answer

Coordinator acceptance (2026-11-07): registry/tracer `10df36141` plus terminal
record correction `02ec0b636` and Retry reconciliation `be39738b` are reviewed and
independently verified. The earlier complete-diff and rejecting-path checks are
recorded in [execution](../execution.md#verified-evidence-to-date); the latest
Geometry/helper/notification reruns are **6/16/30 passed**, respectively.
The final complete code reviews through `c5a876ec` and packaged trusted-input,
literal Retry, held-save/newer-input and cleanup evidence are in
[010](010-packaged-a-to-b-proof.md#answer). All criteria above are now met.
This resolves the tracer slice, not all-14 coverage, optimized validation,
015 mutations/full gate/final review or the parent adoption umbrella.
