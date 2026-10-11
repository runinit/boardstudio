# 128: Bound waits in the keycaps queued settings tests

Status: resolved

Root commit gate: complete all/staged GitNexus analysis found 45 changed symbols across the four leased files,
two dimensions-test flows and medium reported risk, with no partial/truncated result. Source capture succeeded
in 38 s; the wrapper reported dirty-index metadata. Independent verification passed as recorded below.
Post-merge integration batch remains pending; parent 37 remains partial.
Type: build
Blocked by: none
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Audit: [test-audit.md](../audit/test-audit.md)
Parent: [37](37-sleeps-layout-library-ui-page.md) (partial; this child does not resolve the parent)

## Request

Replace fixed settling delays and unbounded gate awaits in the five keycaps queued settings browser tests with bounded waits whose predicates establish the next assertion's precondition. Preserve test names, assertions and behavior. This is one isolated remaining-37 slice; [38](38-merge-duplicate-browser-tests.md) remains blocked on complete parent acceptance.

## Scope and dispatch gates

- Worktree: `/home/chris/01_Projects/ts-boardstudio2/.worktrees/lane-web-sleeps`; branch: `codex/lane-web-sleeps`.
- Source lease: only `web/crates/keycaps/src/keycaps_settings.rs`, within the five named tests, local `mounted_legend` helper, and (solely for retirement causality/observation) `queued_settings_tests::Probe`, `host`, and their initializer sites in the same module. This ticket is the only documentation edit lease.
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

Luna 6 high implemented bounded waits in the mount helper and the three legend tests. The dimensions test conversion is not verified: in both browser executions, the later held-Core legend request did not enter the gate and `wait_until_future` failed at its 5-second bound. The other three converted legend tests and the unchanged retirement test passed in both executions. The retirement body remains unchanged because no test-local negative-idle proof of owner-effect completion has been established; this acceptance item is open.

Before source edits, the full keycaps list contained the expected 30 tests; the sorted list is `/home/chris/.cache/boardstudio-baselines/ticket128/keycaps-before-tests.sorted` and raw output is `/home/chris/.cache/boardstudio-baselines/ticket128/keycaps-before-list.log`. The pre-edit five-test module baseline passed (5 passed, 25 filtered, 1.99 seconds); log: `/home/chris/.cache/boardstudio-baselines/ticket128/keycaps-before-queued-settings.log`.

The focused command was `wasm-pack test --headless --chrome web/crates/keycaps --lib --locked -- keycaps_settings::queued_settings_tests::`. Its first attempt failed compilation because the test imported `ui_shared`; this was corrected to the already-linked `boardstudio_web_ui_shared::test_wait` path. Two subsequent executions each ran five tests, with 4 passed and the dimensions test failing at `the first legend edit reaches the held Core executor`; the browser test time was 5.42 seconds. Those run outputs were returned by the command tool and were not saved to durable log files. The action revision/token wait added after the later legend Undo was after the failing gate, so it did not test the stale-action hypothesis.

Source comparison places the earlier 50 ms delay immediately after the second dimensions Undo and before the first legend `B` edit. At that point the test now waits for the existing legend input, then checks that its text and `Probe.actions` scope/token/revision/selected key/accepted settings match the current accepted snapshot and Keycaps workspace. In the one-test diagnostic run this gate was reached, so the readiness hypothesis explains the earlier gate timeout. The test then failed at the existing post-Undo assertion (`None` instead of `Some("B")`): after releasing the B and empty edits, the new settlement wait checked only accepted legend `None`, which is also the initial state and can hold before either request settles. The original repeated drain hid this ordering. The wait still needs a causal completion predicate for both edits before Undo. Durable output: `/home/chris/.cache/boardstudio-baselines/ticket128/keycap-dimensions-diagnosis.log`; command exit code 1, one test ran (29 filtered), browser test time 0.10 seconds. The B gate precedes failure injection, and `B` differs from the accepted empty legend. No mutation, affected repo/lint/typecheck checks, post-edit full-list comparison, commit, or fresh verification has occurred. Parent 37 remains partial and 38 remains blocked.


Follow-up source correction is written but not yet executed or independently verified. The dimensions test now observes the next Runtime operation before committing `B`, then observes the next operation again before committing the newer empty legend. It checks that the two outcome slots are distinct, captures and requires the exact successive request-sequence values, and waits for both terminal outcomes plus accepted-empty/input-empty/current-actions readiness before the original Undo. It asserts both actual outcomes are `Completed` and includes their request sequence and observed outcome in failure diagnostics. The falsifiable prediction is that this wait cannot pass on the initial accepted `None`: if both distinct operations complete as `Completed` and readiness holds, the unchanged Undo assertion should restore `Some("B")`; any other terminal outcome or a repeated `None` result falsifies that prediction. Requested next command: `wasm-pack test --headless --chrome web/crates/keycaps --lib --locked -- keycaps_settings::queued_settings_tests::keycap_dimensions_queue_and_preserve_the_accepted_sibling`; durable log: `/home/chris/.cache/boardstudio-baselines/ticket128/keycap-dimensions-operation-completion.log`. The retirement acceptance remains open and unchanged.


The first operation-completion command did not compile the test: two E0599 errors found that request_sequence is a Signal field and must be called with field-call syntax. No test ran, so this provided no evidence for or against the operation-completion hypothesis. The two exact invocation sites are corrected without changing the predicate or assertions. The next requested single command is the same focused dimensions test, with full output at /home/chris/.cache/boardstudio-baselines/ticket128/keycap-dimensions-operation-completion-v2.log; if it reaches the test and fails on terminal outcomes or Undo, stop and record that evidence before any further edit or rerun.


The syntax-corrected operation-completion dimensions run passed: one test passed, 29 filtered, browser test time 0.10 seconds, actual command exit 0 (session elapsed 20.3 seconds). It observed distinct B and empty Runtime outcome slots with successive request-sequence IDs; both terminal outcome assertions were Completed. The wait also observed accepted empty legend, empty input, and current action readiness before the preserved Undo assertion restored Some("B"). Full stdout/stderr and exit status: /home/chris/.cache/boardstudio-baselines/ticket128/keycap-dimensions-operation-completion-v2.log. This is one focused result, not full ticket acceptance: retirement proof, full 30-test post-edit list comparison, remaining acceptance checks, affected checks, and fresh verification remain open.


A bounded test-only retirement revision captures the exact expected Gone request, original owner identity and operation outcome slot before submission; keeps the Core reply gated; and observes a render-captured checkpoint showing the request pending under the original Keycaps owner. While the reply remains held it waits for the Layout shared owner, changed key-owner generation, request pruning and still-empty terminal slot. It then releases the reply, waits for that exact operation’s terminal result and a host-published render checkpoint carrying the result, remounts Keycaps and retains the original negative status assertion. The injected failure is now checked as the exact `ExecutorFailed` reason, alongside the existing non-Completed assertion. The focused retirement test passed once (1 passed, 29 filtered, test time 0.08 seconds, exit 0); full output: `/home/chris/.cache/boardstudio-baselines/ticket128/keycaps-retirement-compound-failure.log`.

The post-edit keycaps inventory contains 30 names and matches the captured pre-edit inventory after normalizing the baseline’s trailing-colon delimiter. The queued-settings module passed three sequential focused browser runs, each 5 passed and 25 filtered; logs are `/home/chris/.cache/boardstudio-baselines/ticket128/keycaps-queued-module-run-1.log`, `keycaps-queued-module-run-2.log`, and `keycaps-queued-module-run-3.log`. Runtime fixture validation passed the focused held-failure identity test, the 9-test in-process adapter module and the existing fail-only restart test; see the corresponding `runtime-held-failure-identity-green-v3.log`, `runtime-adapter-module-green.log`, and `runtime-restart-fail-only.log` logs under `/home/chris/.cache/boardstudio-baselines/ticket128/`.

Independent fresh verification is complete for ticket 128's bounded source behavior, scope, and affected checks. The verifier reviewed the full original ticket and four-file diff. All 19 original assertion blocks and five original test names remain; the complete 30-name inventory matches after removing only the baseline's trailing-colon delimiter. The raw baseline is retained and this normalization is disclosed. Original acceptance item 48's causal sentinel and item 68's representative sensitivity passed. No semantic defects remain. The focused retirement run passed (1 passed, 29 filtered); the full keycaps crate passed 30/30. Three sequential final module runs each passed 5 tests (25 filtered), and final `repo`, `lint`, and `typecheck` checks passed. An initial affected-check run failed formatting; the subsequent scoped rustfmt changed layout only, after which the verifier independently reran and passed all three module runs and affected checks. Earlier runtime held-failure identity, adapter-module (9/9), and fail-only restart gates also passed.

The negative-idle proof uses separate futures/gates and the same valid payload and transport ID; its identity evidence is adapter-local, not distinct Session operations. The existing `ExecutorFailed` mapping is retained. The exact retirement reason assertion was added alongside the original generic assertion; waits and the checkpoint remain. The causal ordering was independently reviewed: while the exact reply is held, the private edit is live and the outcome is empty; before release, the Layout owner/generation changes and the exact request is pruned while the outcome remains empty; after release, the exact typed failure reaches the host render checkpoint before the original negative DOM assertion. A false checkpoint and omitted sender release each fail at the named bound. A mismatched reason is rejected by the new exact assertion while the original generic non-Completed assertion still passes. No DOM-only, global-idle, or render-count proxy is used.

The nine actual independent mutation runs and their restored-green checks are recorded in these durable bundles under `/home/chris/.cache/boardstudio-baselines/ticket128/`:

| Mutation | Bundle | Result |
|---|---|---|
| Dimensions predicate false | `fresh-verifier-mutant-v1-dimensions-false-20261011T001858.058812Z/` | Named landing assertion failed at 5 s; restored green passed. |
| Legend restoration predicate false | `fresh-verifier-mutant-v4-legend-restoration-false-20261011T002014.074724Z/` | Named restoration assertion failed at 5 s; restored green passed. |
| Retirement checkpoint false | `fresh-verifier-mutant-v6-retirement-checkpoint-false-20261011T002126.256935Z/` | Named checkpoint assertion failed at 5 s; restored green passed. |
| Retirement release omitted | `fresh-verifier-mutant-v7-retirement-release-omitted-20261011T002237.089228Z/` | Sender remained alive/channel open at the exact terminal bound; restored green passed. |
| Retirement reason mismatched | `fresh-verifier-mutant-v8-retirement-reason-mismatch-20261011T002352.165757Z/` | Exact typed reason assertion rejected the mismatch; restored green passed. |
| Depth predicate omitted | `fresh-verifier-mutant-v2-depth-omitted-20261011T003515.759412Z/` | Named landing assertion failed at 5 s; restored green passed. |
| Legend blur omitted | `fresh-verifier-mutant-v5-legend-blur-omitted-unique-20261011T004047.290188Z/` | Named restoration assertion failed at 5 s; restored green passed. |
| Valid wrong depth (2.25) | `fresh-verifier-mutant-v9-depth-2p25-normal-landing-20261011T004200.890683Z/` | Named landing assertion failed at 5 s; restored green passed. |
| Vacuous landing predicate at depth 2.25 | `fresh-verifier-mutant-v3-depth-2p25-vacuous-landing-20261011T004312.643423Z/` | Preserved next-units assertion detected `None` instead of `(2, 1.5)`; restored green passed. |

Each bundle includes the four dirty preimages and hash manifest, bounded impact/source confirmation, configuration and diff, mutant test output, restored-green output, and restoration proof. Each mutant exited 1 for its intended named assertion; each exact keycaps-directory preimage was restored and touched, all four tracked-file hashes matched before/after, and the same named test passed green (1 passed, 29 filtered). A non-unique blur selector guard stopped before any source write or browser test; it is not mutation evidence or an implementation finding. The subsequent retry used the unique function and mounted-legend selectors. GitNexus reported UNKNOWN with zero callers/processes; this was treated as unresolved, and the exact WASM test entrypoint/anchor was confirmed from source.

The original literal 68 acceptance requirement is preserved: the next assertion catches the missing behavior under the vacuous-predicate mutation. The stronger proposed settled-state comparison (actual 2.25 versus expected 1.5) was not achieved. No settled-state or completion claim is made. Supplemental isolated `follow_owner`/prune producer-sensitivity for the original negative DOM assertion was not performed; it is not a standalone literal 128 mandate or blocker. No paired-departure scenario, production mutation, retry, speed claim, or broader browser acceptance is claimed. Parent 37 remains partial and parent 38 remains blocked.

Independent run logs (all under `/home/chris/.cache/boardstudio-baselines/ticket128/`):

- Focused retirement: `fresh-verifier-retirement-exact-reason-20261011T000531Z.log` (1 pass, 29 filtered; exit 0).
- Complete keycaps inventory run: `fresh-verifier-keycaps-complete-20261011T000605Z.log` (30/30; exit 0).
- Initial affected-check run, which failed formatting before scoped rustfmt: `fresh-verifier-affected-checks-20261011T000645Z.log`.
- Final sequential module runs: `fresh-verifier-final-module-run-1-20261011T000920Z.log`, `fresh-verifier-final-module-run-2-20261011T000951Z.log`, and `fresh-verifier-final-module-run-3-20261011T001015Z.log` (5 pass, 25 filtered each; exits 0).
- Final affected checks: `fresh-verifier-final-affected-checks-20261011T001042Z.log` (`repo`, `lint`, `typecheck`; exit 0).

The four implementation-source paths remain frozen with independently reviewed hashes: `web/crates/keycaps/src/keycaps_settings.rs` `db5629bcdfac00099a197712acee1d24355cf1bcbdb0aacd14f3f88a5335b5e9`; `web/crates/runtime/src/in_process_support.rs` `8bae73c632c9e4f3e2c9108592fe1ef9c2bcd612d224ae26cd7d4acae256140e`; `web/crates/runtime/src/runtime.rs` `27100292df8e27f88341cbc0a3dd21690d4ada6278003f94951a4745f4ab1ccc`; verifier-reviewed pre-tracking ticket hash `ea9eee4c32bebd6fa9caffe84986478f655f85e91a90441629d984daec1ad58b`.

Status is implemented and independently verified for the bounded ticket scope, awaiting root's complete `detect_changes` gate and commit. Do not mark resolved until root completes that graph gate. No claim is made that parents 37 or 38 are resolved.
