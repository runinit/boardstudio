# PCB wiring mode source review

Independent Sol 6.1 High review, 2026-10-02. No source edits or heavy test jobs were performed by this reviewer.

## Exact packet

- Worktree: `/home/chris/.local/share/boardstudio/worktrees/pcb-wiring-mode-apply-20261002`.
- Frozen source: `1e5ef6ac561136101f13cb5438a4f3e5d52cb6ba`.
- Fixed point: `3f5453e14b9610257c635de975d12c8ddeb505bc`.
- Reviewed diff: `git diff 3f5453e14b9610257c635de975d12c8ddeb505bc...1e5ef6ac561136101f13cb5438a4f3e5d52cb6ba`.
- Contract commits: `d5e527a150ea354efc68091084bf29e2d24f0da9`, then refinement `882b64f96bccf70a2b41650ae89bc16432eeda15`.
- Contract: `.scratch/dioxus-pcb-view/drafts/F5.2c-board-wiring-mode.md`, Issue08 `.scratch/dioxus-pcb-view/issues/08-board-wiring-mode.md`.

Applicable standards: supplied AGENTS instructions, CONSTRAINTS.md, current architecture/ownership documentation and issue-tracker/domain guidance. Standards and Spec were assessed separately in the bounded independent assignment.

## Standards

Clear in the inspected source; no separate documented-standard or smell finding.

The proposal is a narrow crate-local operation, changes only the target electrical configuration's mode, and creates the existing default configuration when absent. Other board configurations and unrelated document fields are preserved by cloning the accepted document. Runtime remains in the Editor-lifetime owner; the Inspector receives read-only accepted projection and a typed event. No public API, wire schema, provider or persisted field changes were found. The selector stays within existing board/controller context and uses the reference labels and values.

## Spec

Changes required: two P2 findings.

### SP1 — Saved feedback is suppressed by the pre-save action identity

Location: `web/src/presentation/pcb_wiring/mode.rs:214`, in conjunction with lines 110 and 128–130.

After the exact operation completes, the owner stores `Saved` with the original request identity. The final projection exposes feedback only if that entire identity equals the current source identity. `BoardWiringModeIdentity.plan` includes token and revision. `application/src/session.rs` SaveCommitted handling advances both accepted token and document revision before emitting the operation's Completed outcome. Thus a normal successful edit creates a new source identity, immediately suppressing the Saved feedback; the Inspector's implemented “Wiring mode saved.” state cannot appear for that successful edit.

Keep strict request identity for admission. Give post-operation feedback a target/currentness comparison that survives the edit's own accepted token/revision advance while still rejecting unrelated project/session/board/instance/selection/generation changes. Verify the production owner shows Saved after its exact durable operation and suppresses that feedback after a different UI target takes ownership. Do not relax stale-action guards.

### SP2 — Required mounted owner and settlement coverage is absent

Location: `web/src/presentation/pcb_wiring/mode.rs` (production owner); contract Testing Decisions and Issue08 exact operation/accepted display criteria.

The contract explicitly requires: “Exercise the production mounted editor action, not only a proposal helper,” and mounted verification of SaveCommitted plus rejected, cancelled, executor-failed and persistence-failed settlement. The frozen packet adds only two native proposal tests. They establish configuration preservation/default/no-op behavior but never invoke the mounted typed request, stale identity rejection, exact operation observer, accepted selector projection or resolver invalidation. Existing general Runtime/resolver tests do not execute this new owner, and no specific reused evidence for it was supplied.

Add bounded mounted production-owner tests for a valid request through one accepted edit; stale project/session/board/revision/workspace/selection/generation rejection; exact operation settlement and prior accepted mode on failure; and old-plan invalidation/new accepted mode resolution. Include the SP1 regression. Keep packaged paired mode/Undo/Redo/save/reopen qualification separately open. This is the approved child gate, not a demand for a full suite or unrelated parent closure.

## Verified behavior and checks

The source separates normalized selection-independent plan identity from UI scope, selected part and generation. Admission checks PCB workspace, current instance preference, saved Ready accepted source, no preview/gesture, exact UI/plan/token/revision/executor identity and a matching Current plan. The exact operation observer is allocated before one normal committed ReplaceDocument edit; the Editor-lifetime mount is registered in production. Existing resolver currentness is keyed by the newly accepted token/revision and continues to discard stale asynchronous responses. Accepted projection reads the source document, not the requested mode.

Independent formatting and full fixed-point diff checks passed on the frozen clean worktree. The author reports wasm check, strict target Clippy and 2/2 native proposal tests; output was not originally preserved, and the author is preparing a docs-only immutable receipt with fresh logs. No browser or heavy test rerun was attempted by this reviewer.

Preserve the feedback identity observation under existing RF-006, alongside strict request ownership and exact outcome settlement; no API/owner redesign is authorized. Public packaged mode/history/reopen evidence and all F5.1/F5.2/F5.3/full parent acceptance remain open.

Standards: clear, zero findings. Spec: changes required, SP1 and SP2 above.
