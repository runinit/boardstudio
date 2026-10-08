# Second app: Case, Keymap, Keycaps and Library settlement

Assignment: [Case, Keymap, Keycaps and Library settle through PendingEdits](issues/09-case-keymap-keycaps-library-onto-pending-edits.md).

Worktree: `/home/chris/.codex/worktrees/boardstudio-module-deepening/09-case-keymap-keycaps-library-pending-edits`  
Branch: `deepening/09-case-keymap-keycaps-library-pending-edits`  
Starting dev commit: `8ce860a0950304ddfae34f12d5891b8c6f267f2f`

## Work

1. Confirm the worktree branch, HEAD and clean starting state. Read the current root `AGENTS.md`, the worktree `AGENTS.md`, [parallel-run rules](handoff.md#rules-every-agent-follows), your ticket, and its decision links. Your branch starts after the PendingEdits tracer gate; the gate is the prerequisite for source changes.
2. Inventory every settlement site in the ticket’s owned paths. Use `tdd`, `codebase-design` and the applicable Rust skills. Replace settlement policy through the shared module and remove the shallow layer it replaces. Keep domain behavior and authoritative Session work intact. Account for every site in the final report.
3. Case owns only settlement and begin-edit wiring in mechanical_settings_controller.rs: preserve patch logic, flush_prepared catalogue order and native tests. The typed Core mechanical patch waits for human approval and follows this branch. Keymap and Library must gain genuine native Runtime coverage. The remaining two crates have independent ownership, so your app can use subagents on disjoint crate paths and integrate/review once.
4. Run the ticket’s native checks, lint, typecheck and full test step. Request an exclusive Chrome lease from the orchestrator before browser checks; continue source/native work while another app holds it. Release the lease when the runner exits, and preserve unrelated browser processes. If a failing native step short-circuits check.py, run the browser step separately under the lease. Report native and executed browser coverage separately, plus baseline failures and unexecuted gates.
5. Pin your absolute worktree, base and final HEAD for parallel Standards and Spec reviews using `code-review`. Address findings, rerun checks affected by fixes, and report final verdicts. Read-only reviews can run in parallel with checks.
6. Report your commits, a clean working-tree status, every migrated site, executed checks, review verdicts and an Outcome draft. The orchestrator handles integration with current dev, tracker changes and any history operation requiring approval.

## Shared contract

Reuse the merged [keyed collection contract](issues/15-keyed-pending-edits.md#outcome), [Signal helper contract](issues/16-pending-edit-ui-helpers.md#outcome), and [complete Matrix tracer](issues/18-matrix-field-pending-edits.md#outcome). Read the source example before choosing your panel key type. The helper retains bindings, so use bounded logical keys and keep bound Signals alive for the observation lifetime. A real panel unmount must retire its observation even when editor/selection remain; accepted revisions alone keep it live. Field helpers own submitted-draft memory and settlement writes to bound draft/failure Signals; callers own admission, accepted projection, owner liveness, error placement and precise Landed follow-ups.

For text-field migrations, bind the actual stable draft/failure Signals before submission and pass the exact typed text before parsing. Remove the panel's submitted-request tracking and settlement restoration once the helper owns them. Accepted-projection effects must preserve a newer draft after the helper drains an older result. Prove this in a mounted held-result test, including an older failure that still appears inline beside the newer text.

## Ownership and coordination

This app owns this assignment’s source and tests in its isolated worktree. The orchestrator owns `dev`, ticket claims/resolutions and the canonical GitNexus index. Follow the current [GitNexus rules](handoff.md#rules-every-agent-follows), including impact before function edits and complete change analysis before commits. Reindexing belongs to the orchestrator. Report a shared Runtime/helper contract gap before changing another ticket’s files. Stage explicit paths, preserve unrelated edits, and use the repo’s Git safety rules.

## Known verification limits

The documented native CAD failure is `core_internal_gasket_fixtures_export_connected_positive_regions` (rotated-concave/bottom volume). Treat that gate as failing if reproduced; list the actual result. AppImage-hosted KiCad failures reported by the helper app are a separate environment issue, not the CAD failure. Run the current tree and report what actually happens. Browser presentation compiles only for WASM; native tests alone do not verify it.

## Review follow-up after the lint fix

Resume the existing branch at its actual HEAD; retain the lint fix `8a5f167bc` and
all existing work. The remaining blocking Spec finding is that Case, Keymap and
Keycaps still retain local observation/settlement layers instead of using
`PendingEditSignals`. The requirement is explicit in the parent ticket's migration
rules and acceptance criteria; passing browser tests does not close that finding.

Start with Keycaps, then Keymap, then Case. Preserve request-specific owner semantics,
domain payloads and follow-ups. Metadata can remain in bounded request-bearing keys
with equality by logical field/action, or bounded caller projection memory; operation
ids must not create an unbounded key or binding history. Preserve the latest-per-key
observation rule and authoritative queued Session work.

A single liveness boolean applies to one helper's owner domain. Use independently
owned helper instances where fields have genuinely independent owner lifetimes; do
not collapse per-request selection checks into one global true flag. If preserving
existing ownership cannot be expressed with the published contract, report one
concrete failing scenario and the smallest proposed shared extension. Root owns any
shared implementation/commit; this stream remains inside its four crates. A small
extension may be considered, but no shared API change is approved by this note.

Bind actual field Signals at the component that owns them, or lift stable Signals to
the appropriate lifetime and pass them through component props. Retire observations
before bound Signals are disposed. Child ownership is a lifecycle design constraint,
not a reason to keep local submitted-text/restoration policy. Preserve domain retry
intent without reintroducing a second copy of helper draft bookkeeping.

Prove unchanged-submitted failure restore, newer-draft protection with inline older
failure, latest-per-key replacement and real owner departure in mounted Runtime tests.
Then rerun affected checks and both reviews pinned to final HEAD. Record the reported
Keymap ChromeDriver SIGKILL on branch and clean base as a reproduced baseline;
remaining browser suites are still unexecuted until run under an exclusive lease.
