# Proposed P2-r1-R1: runnable lifecycle prototype with Luna Fast

Status: proposed, one human execution decision pending. This refines the existing
[P2-r1-R1 renewal](../prototypes/p2-lifecycle/evidence/scope-extension.json), not
an alternative architecture or a migration backlog. Base is exact reviewed
integration **689f8962f77e04b1150a9541128fec7aca28addc**. The original P2 run stays
blocked and preserved; this proposal does not reinterpret it as complete.

## One bounded destination

Build the already specified [P2 lifecycle prototype](../prototypes/p2-lifecycle/PLAN.md):
one SVG editing gesture backed by the public engine/accepted worker, one canvas
backed by the existing renderer, final-sample commit and one-step Undo/Redo,
capture/cancel/focus, resize/DPR and visible frame, remount/teardown and late-result
rejection. Provide the runnable local release and browser evidence at root and
/boardstudio/. A working interim build may be shown during development, labelled
as incomplete until the original acceptance gates pass.

## Proposed execution authority

- One fresh task-owned worktree/branch from the recorded integration base;
  preserve the exhausted P2 worktree, accepted P1 artifacts and unrelated work.
- Apply only the already prepared formatting patch to renderer/src/geometry.rs,
  math.rs, mechanical.rs and wasm.rs. Its raw SHA-256 is
  `1fdbebc0f24270b1bba341c19cadb88c6d6f2aa24d5e195143c4f39ad52fa024`.
  No renderer behavior/API/visibility/backend/version change is authorized.
- Original prototype and coordinator-document ownership remains. Root owns
  specification/status/architecture and integration; one coding worker owns
  the task worktree, the exact formatter prerequisite and isolated prototype
  subtree. Serialize all state/interface/manifest/lock changes.
- Root remains coordinator. Request **gpt-6-luna/high with Fast (priority)** for
  coding and review; Luna/medium for narrow read-only inventory. At most one
  coding writer initially and three open children total. Archive the writer
  after preserving results, then use two fresh independent reviewers. No
  self-delegation, Sol/Astra switch, alternate paid provider or global config edit.
- One approval covers this exact run, task commits, local migration-branch
  integration, routine in-scope corrections and frozen dependency/build/browser
  operations. No routine progress approvals. A behavior/API/pin/scope change or
  unavailable required capability remains a blocker and needs a new decision.

## Reduce friction while keeping the quality bar

1. Reuse the accepted specs, exact-version Dioxus source verification, P1 proofs
   and unchanged fixture/provider/artifact evidence. Refresh only stale facts;
   perform the required read-only runtime smoke before a writer starts. Record
   fixture identity before executable edits. Do not rerun settled discovery.
2. Fix the known formatter prerequisite first; run its required affected checks.
   Develop the existing P2 behavior in small tested increments. Focused tests and
   regular native/WASM checks give quick feedback; use eligible shared caches.
3. Check evidence paths/status consistency before review. Keep a compact task
   ledger with commands/exits/logs and a final owned handoff rather than asking
   for approval or reviewing every administrative checkpoint. All material
   findings still require correction and affected exact-candidate re-review.
4. Run every original P2 regression/build/contract/boundary/real-browser/paired
   lifecycle/frame/resource/accessibility gate before acceptance. Preserve frozen
   budgets, failures and unperformed checks; no baseline regeneration or waiver.
5. Fresh Spec and Standards review the exact candidate against the current
   integration base. Root alone integrates it locally and validates that exact
   resulting revision before marking P2 complete. Changed candidates require
   affected review/verification again.

## Explicit finite repair policy proposed for this renewal

The old two-attempt budget remains exhausted. The new run starts only on approval
with a finite limit of **two diagnosed repairs per failed implementation task**.
Administrative evidence/link corrections use their own finite two-repair budget;
that budget cannot silently waive a failed implementation gate or reset its
attempts. This separation is a proposed policy clarification for human review,
not a retroactive reset. No unlimited retry loops. Systemic regressions, data
compatibility uncertainty, missing authority or exhausted relevant budget stop
dependent work; independent approved work continues where safe.

## Preserved scope and handoff

All original P2 acceptance/spec/ADR/constraints requirements remain. No P3,
production editor-session/workspace port, schema/provider behavior/CAD-u64 change,
new framework pin, automatic promotion, push, deployment or main merge. Preserve
failed worktrees and uncommitted work. Record requested versus observed model/
effort/tier; [Luna Fast evidence](research/luna-fast.md) explains current limits.
The [human execution decision](issues/08-choose-p2-execution-policy.md) stays open
until the user answers. This document alone authorizes no implementation.
