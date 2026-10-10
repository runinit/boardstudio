# 128: Bound waits in the keycaps queued settings tests

Status: claimed
Type: build
Blocked by: none
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Audit: [test-audit.md](../audit/test-audit.md)
Parent: [37](37-sleeps-layout-library-ui-page.md) (partial; this child does not resolve the parent)

## Request

Replace fixed settling delays and unbounded gate awaits in the five keycaps queued settings browser tests with bounded waits whose predicates establish the next assertion's precondition. Preserve test names, assertions and behavior. This is one isolated remaining-37 slice; [38](38-merge-duplicate-browser-tests.md) remains blocked on complete parent acceptance.

## Scope and dispatch gates

- Worktree: `/home/chris/01_Projects/ts-boardstudio2/.worktrees/lane-web-sleeps`; branch: `codex/lane-web-sleeps`.
- Source lease: only `web/crates/keycaps/src/keycaps_settings.rs`, within `queued_settings_tests` and its local `mounted_legend` helper. This ticket is the only documentation edit lease.
- No production, Core, model, catalogue, recipe, shared helper, golden, map or configuration changes. No retry/recovery expansion.
- Claimed by the Luna 6 high implementer on 2026-10-10. Implementation follows the source, baseline, impact and verification gates below.
- Before code edits, confirm absolute checkout, HEAD/base, branch and status; inspect overlapping changes and latest integration changes. Read the exact central registry and run complete, bound impact analysis for all changed symbols. Root alone refreshes GitNexus; escalate stale prerequisites or contested scope.
- Luna high implements; a fresh Sol 6.1 xhigh session independently verifies after the implementer stops. Implementer ends at `implemented (awaiting verification)`; only the verifier resolves acceptance-complete scope.

## What to do

Retain these five test names:

1. `keycap_dimensions_queue_and_preserve_the_accepted_sibling`
2. `a_newer_legend_draft_survives_an_older_failure_that_reports_inline`
3. `a_failed_unchanged_legend_restores_the_accepted_text_with_an_inline_failure`
4. `a_second_legend_edit_replaces_the_observation_and_both_edits_undo`
5. `leaving_the_keycaps_workspace_retires_a_pending_edit_silently`

Reuse `ui_shared::test_wait::{wait_until, wait_until_step, wait_until_future}` as appropriate; do not add shared helper APIs. Existing test support includes `run_pending`, `drive_pending`, `gate_next_core_reply` and `fail_next_core_reply`. Confirm actual signatures before use. Bound gate observation as well as settlement; if driving pending work is necessary, keep it inside the bounded step.

| Site | Required precondition before the next action/assertion |
| --- | --- |
| Mount / `mounted_legend` | Required input (`Keycap width for SW1` or `Legend for SW1`) and the required action/probe are available. |
| Queued width then depth | Width draft is `2`; the accepted/render observation required by the depth action is current. Observe the held reply before proceeding. |
| Dimensions landing / undo / failure | Accepted units `(2, 1.5)`, then `(2, 1)`, then `None`; failure observation includes inline error and restored input before checking accepted units. |
| Newer legend draft and older failure | Input is `AB` and inline failure is visible before checking that the accepted legend remains `None`. |
| Unchanged legend failure | Input is restored to the empty string and inline failure is visible before checking accepted state. |
| Second legend edit and undo | Accepted legend and rendered input are `B`; each undo reaches its existing expected accepted value before its assertions. |
| Workspace departure | Observe held work, retirement and a causally later test-local completion sentinel before asserting absence of late failure/retry draft. |

Retain every original assertion, including sibling preservation and both undo results. Predicates must account for the accepted revision/render where the next action reads a field or hook; an input value alone is insufficient when it can already equal the draft.

### Negative-idle proof

The workspace-departure test asserts absence of a late status/retry draft. A positive DOM transition or completed `run_pending` alone does not prove Dioxus effects are idle. Establish a bounded, observable test-local sentinel causally after the released held work and the relevant owner-retirement effects, then assert the original negative conditions. Explain that ordering in the Outcome and mutation-check it.

The audit did not establish that such a sentinel is available. Confirm feasibility inside the lease before converting this site. If it needs a shared idle API or production change, stop that contested work and escalate; record the exact residual and leave acceptance open. A timed drain may be documented as a residual, but does not satisfy this child's negative-idle acceptance. Do not weaken the test to make the conversion pass.

The failure tests currently assert failure handling, not a subsequent successful retry. Keep that boundary. Ticket 36's save-recovery finding means an added edit after an injected Core failure would need separate recovery reasoning and is outside this child.

## Audit evidence and limitations

The read-only Luna medium audit found a one-file candidate with roughly 1.6 seconds of fixed waiting by source accounting; this is an estimate, not measured speed. Parent 37 records a historical keycaps baseline of 30 tests / 2.54 seconds. Re-establish the current baseline.

Against `boardstudio-lane-web-sleeps`, `mounted_legend` impact was LOW with four direct test callers and zero processes. The dimension test returned UNKNOWN with an empty caller set; source inspection confirmed a WASM test entrypoint. Helper context extraction did not provide a complete consumer census. These are index limitations, not proof of independence. Reconfirm unresolved answers with source searches in the next command window.

Production dependencies exercised include `use_keycaps_settings_actions`, `KeycapsSettingsEditor`, accepted-document updates, undo, field failure reporting and workspace retirement. They are dependencies to inspect, not production edit leases. The audit confirmed the expected branch and clean tree but did not independently check HEAD against root's assigned `f2d180c6`.

## Acceptance

- [ ] All five tests keep their names and existing assertions; the full keycaps crate list remains 30 tests, with an empty sorted before/after name diff. Escalate any baseline discrepancy before changing the expected count.
- [ ] Converted waits and gate observations are bounded and establish the next assertion/action precondition; any residual site and its reason are recorded.
- [ ] Workspace retirement has the explicit bounded negative-idle proof above; no retry/recovery behavior is added.
- [ ] Three sequential targeted keycaps browser runs pass. Record executed names/counts, command, load and duration; do not infer speed from noisy targeted runs.
- [ ] Three representative waits are independently mutated: dimension landing, legend failure/restoration, and retirement sentinel. For each, a never-true predicate fails at the shared bound with its diagnostic; omitting the relevant test action/held-reply release makes the named test fail. Also make one positive predicate vacuous and confirm the preserved next assertion detects the missing behavior.
- [ ] Mutations stay inside the test-module lease, one at a time, in the tested checkout. Restore exact bytes, touch restored source, compare with the pre-mutation snapshot and rerun the clean affected test. If omission cannot exercise a required behavior without production edits, request a temporary mutation lease from root; do not silently widen scope.
- [ ] Affected `repo`, `lint` and `typecheck` checks pass, complete bound `detect_changes` matches the lease before each commit, and the fresh verifier records independent acceptance evidence.

## Verification

Confirm the actual targeted WASM test and full-crate `--list` commands from current Cargo/runner documentation in the next command window; no command was verified during drafting. Capture the complete baseline list before source edits and the complete after list, even if execution is filtered to this module. Keep logs outside committed source and record their paths in the Outcome.

Root explicitly narrows this child's browser gate to three sequential targeted keycaps runs. Parent 37 retains its final full-browser and quiet timing gates; root runs the full batch check once per batch. Do not repeat the full browser/full check for this child without new evidence requiring it. All shell commands, checks, mutations and commits require root's serialized foreground window; Cargo uses 12 jobs. Release the window explicitly when finished.

Follow [verification-process.md](../../../agents/verification-process.md) and [agent-workflow.md](../../../agents/agent-workflow.md). Preserve sufficient baseline, mutation and restoration evidence for the fresh verifier; no prior delivered-slice verification is repeated or claimed for this child.

## Outcome

Drafted by the Lane A coordinator from the Luna medium read-only audit. No source implementation, checks, mutation runs, stage or commit has occurred. Implementation awaits root's dispatch gates. Parent 37 remains partial; 38 remains blocked.
