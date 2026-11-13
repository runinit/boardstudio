# 023: Truthful and quiet save feedback specification

Status: resolved
Type: planning
Owner: fix/ui-integration-recovery
Blocked by: none
Parent: [021: Repeated save Toasts](021-repeated-save-toasts.md)
Related: [Plan map](../map.md), [Save ownership](../../../adr/0013-save-before-publication-and-explicit-recovery.md), [Testing boundary](../../../adr/0021-test-real-workflows-natively-and-browser-behavior-mounted.md)

## Problem Statement

Routine interaction floods the interface with “Saved locally” Toasts. The reported
zoom symptom was reproduced: 20 alternating Zoom out/in clicks produced 20 new
save-success Toasts. A separate zoom produced the same message while the stored
project was unchanged. This makes save feedback both disruptive and misleading.

The inspected camera operation updates view state and completes without a
document persistence effect. A generic Runtime completion path classifies that
completion as a transient save success, and the production notification bridge
renders every queued event. Camera completion and document save success need
distinct meanings.

## Solution

Reserve save feedback for actual persistence outcomes. Camera and navigation
completion must not announce a save or start document persistence merely to
justify a message. Routine successful autosaves use the existing header status
instead of success Toasts. Preserve meaningful failure/recovery feedback and
explicit action outcomes, including export results and waiting-update notices.

Keep the existing save-before-publication contract, retry behavior and Undo
semantics. This is a correction to result classification and presentation, not a
change to the persistence policy.

## User Stories

1. As a keyboard designer, I want to zoom repeatedly without save Toasts, so that I can inspect my design without notification clutter.
2. As a keyboard designer, I want camera changes to remain view operations, so that moving around is not presented as a document edit.
3. As a keyboard designer, I want navigation completion to avoid announcing a save, so that feedback describes what actually happened.
4. As a keyboard designer, I want the saved indicator to reflect actual persistence, so that I can trust it.
5. As a keyboard designer, I want ordinary autosave success in the header, so that repeated edits do not stack success Toasts.
6. As a keyboard designer, I want document edits to keep saving normally, so that quieter feedback does not reduce durability.
7. As a keyboard designer, I want the current saving and failure states to remain accurate, so that I know when work is pending or needs attention.
8. As a keyboard designer, I want failed saves and recovery choices to remain visible, so that important problems are not hidden by notification suppression.
9. As a keyboard designer, I want retry to persist the retained result without replaying the edit, so that feedback changes preserve my design and Undo history.
10. As a keyboard designer, I want Undo results to save and reload correctly, so that quieter feedback does not weaken restoration.
11. As a keyboard designer exporting a file, I want the explicit export outcome reported, so that removing automatic save Toasts does not hide action results.
12. As a keyboard designer, I want separate meaningful repeated actions to retain their results, so that generic text deduplication does not swallow notifications.
13. As a keyboard designer, I want the waiting-update notice preserved, so that save-feedback changes do not interfere with adopting a newer build.
14. As a keyboard user, I want failure and explicit-result feedback to retain its accessible presentation, so that important outcomes remain perceivable.
15. As a maintainer, I want tests to distinguish camera completion from actual save settlement, so that another generic completion handler cannot recreate this symptom.
16. As a maintainer, I want the real zoom sequence covered in a packaged browser, so that source-only checks do not substitute for observed behavior.
17. As a product owner, I want reproduction, regression proof and broader acceptance reported separately, so that completion claims match the evidence.

## Implementation Decisions

- The user confirmed the distinction between zoom completion and autosaving.
  The reproduced camera path already avoids document persistence; preserve it.
- Classify results from their actual operation and persistence outcome, rather
  than treating every ordinary Completed settlement as a save success.
- Routine successful autosaves use the existing header state; they do not enqueue
  a success Toast. Camera/navigation completion must not fabricate a saved state.
- Preserve actual pending, committed, failed and recovery outcomes and the
  accepted-document publication boundary. Do not disable saving or change retry,
  history or storage behavior to reduce notifications.
- Preserve explicit action result notifications and blocking inline reports.
  Do not infer permission to redesign failure/recovery presentation.
- Retain the one-consumer notification bridge and meaningful repeated events.
  Broad message-string filtering, arbitrary throttling and global deduplication
  are not the agreed policy.
- Keep the shared Toast controls, Themes, accessibility and waiting-update
  behavior. Make documentation describe the narrow ordinary-autosave exception
  to transient action-result Toasts.

## Testing Decisions

The user confirmed these existing seams on 2026-11-06. Tests drive observable
behavior through real application transitions, with expected outcomes independent
of the implementation's classification logic.

- Drive real Runtime/Session workflows using existing controlled persistence
  adapters. A camera operation must update its view without a persistence request,
  accepted-document revision change or save-success notification. An actual edit
  must still persist before accepted publication and expose accurate save state.
- Mount the production notification bridge and shared controls. Verify repeated
  camera/navigation completion and ordinary autosave successes do not stack save
  Toasts, while meaningful explicit results and existing failure/recovery feedback
  remain available. Do not replace the bridge with a lookalike or assert strings
  in source code instead of observable notification behavior.
- Reuse existing delay/failure/retry controls to distinguish pending work,
  committed persistence and failed persistence. Preserve the retained-result retry
  contract without duplicating the whole persistence test suite.
- Reproduce the actual packaged-browser zoom sequence with an identified build,
  isolated project/profile and dedicated origin. Establish RED on the reported
  flood before repair; demonstrate GREEN using the same behavior assertion after
  correction. Observe camera movement, new Toast count, header state and unchanged
  stored document for zoom; also verify a real edit still saves.
- Keep the existing save/Undo/reload proofs meaningful and rerun relevant controls
  for the affected application boundary. Do not weaken document/scene comparisons
  or replace durable evidence with the absence of a Toast.
- Record commands, terminal outcomes, source/package/browser identity and limits.
  Follow the implementation workflow's applicable checks and independent review;
  ticket completion does not replace the combined acceptance gate.

## Out of Scope

- Changing which document edits autosave, persisting camera state as a document
  edit, or redesigning storage, revision, retry or Undo ownership.
- A generic notification rate limiter, global text deduplication, new notification
  framework, user preference, or broad shell redesign.
- Collision/provider harness repair, update adoption, or the cancelled UI recovery
  tickets. Those retain their own scopes and statuses.
- Product implementation during this specification step, additional worktrees,
  branch changes, pushes, merges or replacement of the user's live preview.

## Further Notes

The reproduction and source trace are recorded in [021](021-repeated-save-toasts.md).
It used the existing packaged artifact retained by 009; no new build was made
during status, and it was not the user's original browser project. The broader
pattern across all navigation actions is a regression requirement, not a claim
that every such action was reproduced during status.

The accepted [save decision](../../../adr/0013-save-before-publication-and-explicit-recovery.md)
and [native/mounted boundary](../../../adr/0021-test-real-workflows-natively-and-browser-behavior-mounted.md)
remain authoritative. At specification publication,
[DESIGN](../../../../DESIGN.md)'s transient-result guidance still needed the
ordinary-autosave distinction; 021 subsequently added that exception alongside
implementation. The specification step itself added no product repair or new
execution evidence.

## Completion record (coordinator, 2026-11-08)

- Candidate `HEAD`: `0c1d1a6f`; policy implemented by
  [021](021-repeated-save-toasts.md#completion-record-coordinator-2026-11-08) at
  `542313f4` + `0c1d1a6f`. Implementation Decisions and Testing Decisions above
  are unchanged; this pass changes no policy or code.
- Reused receipts, not new executions in this docs pass: 021's Runtime/mounted
  RED/GREEN, R1 and cadence RED/GREEN, packaged GREEN (20 zooms, zero save Toasts,
  stable header and durable rename), and 17-check drag smoke. Packaged RED is
  **REUSED status reproduction on retained 009 package**, **NOT freshly
  re-established RED** (`021-status.log`). The initial packaged harness failure
  and initial full-gate browser failure remain in 021's record. The full-gate
  rerun `021-coordinator-full-gate2.log` passed all steps (581.0s; browser 223
  executed, 0 failed); the original gate failure's cause remains unproven.
- Combined review after R1: Spec **PASS**, Standards **PASS** carried,
  acceptance **READY**; checkpoints `reviews/021-spec-standards` and
  `reviews/021-r1-delta-recheck`. Cadence reviewed during the gate cycle.
  `DESIGN.md` now documents the ordinary-autosave exception. No unmet policy
  gates or new retained risks; 021 records cleanup. Next: coordinator integration,
  not closure of 010/011.

## Answer

Resolved: 021 implements the agreed truthful, quiet save-feedback policy at
`542313f4` + `0c1d1a6f`, with the receipts and limits recorded there. Routine
success is header-only; explicit outcomes and meaningful failure/recovery remain.
No Implementation or Testing Decisions changed in this closure pass.
