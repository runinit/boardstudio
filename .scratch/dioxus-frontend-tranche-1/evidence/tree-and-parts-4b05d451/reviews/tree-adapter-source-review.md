# T1-10 concrete tree adapter source review

Review scope: worker `/home/chris/.local/share/boardstudio/worktrees/frontend-tree-20261002`, committed hierarchy `890fd54f`, owned nudge `27ad564c`, and initially uncommitted `presentation.rs` / `presentation/selection.rs` adapter. Governing contracts: `/tmp/frontend-run/tree-contract-review.md` and `/tmp/frontend-run/tree-adapter-design-review.md`. Reviewed retained React `app/src/ui/Workbench.tsx`, `useWorkbenchTree.ts`, `workbenchGeometry.ts`; application Session and Runtime public interactions; installed dioxus-signals 0.7.10 source. No builds, test execution, repository edits, or public bug reproduction by this reviewer. Findings below are source-grounded implementation risks, not reproduced defects.

## Initial decision: changes required before integration

1. **Per-ID selection eligibility and anchor maintenance.** Initial observer used only board membership + live document existence, retained disabled matrix members whenever semantic context remained nonempty, and cleared *all* selection when last context became invalid. Instead clear invalid semantic context separately, filter each selected/anchor ID through current enabled matrix membership or standalone eligibility, and preserve unrelated eligible selections. Invalidate private anchor stamp before *any* maintenance Replace, including nonempty survivors: Session rewrites its anchor from incoming first ID. Post-submit helpers must not blindly restamp an anchor altered by maintenance. Validate explicit incoming IDs against current eligibility, and stamp only the explicitly established eligible anchor. Author has acknowledged and is revising.

2. **Do not adopt an older gesture on pointer-ID reuse.** Initial `move_pointer` submitted GestureBegin then accepted any `model.gesture` with matching pointer ID, copying its generation. Session ignores Begin while a gesture exists. During an older unsettled gesture, a fresh press with reused pointer ID can therefore acquire old gesture ownership and submit the new positions to it. Require no existing gesture before Begin; afterward verify a genuinely new matching generation/targets plus current Scope and adapter generation. Never cancel/adopt merely on pointer ID. Author acknowledged and is revising.

3. **Capture follows successful validation.** Initial real-part pointerdown captured the SVG before `context_for_part`, selection result and accepted snapshot checks. Several subsequent returns left no owned Drag for cleanup. Validate semantic/real eligibility and selection first, then capture; release consistently on failed coordinate or gesture acquisition. Author acknowledged and is revising.

4. **Nudge must match retained reference multi-selection/locking.** `Workbench.tsx:689–711` ignores a locked focused part; if focused ID is selected, moves all unlocked selected parts, otherwise that unlocked focused part. It updates selection to actual moving IDs. Initial adapter always submitted MoveParts for only requested ID and had no locked checks. Resolve fresh eligible IDs, respect this selection rule, preserve exact target IDs/positions and one committed command. Existing 0.1/1 mm step sizes match workbenchGeometry.ts. Author notified. Tree primary/standalone handler restriction matches the retained useWorkbenchTree handlers; empty contexts must not edit.

5. **Signal mutation must compile against installed API.** `selection.rs` accepts `&SelectionAdapter` then directly invokes `.set()` on fields. Installed dioxus-signals 0.7.10 `WritableExt::set` requires `&mut self`; use local mutable copies of Signal handles and mutable closure-local handles as needed. No compiler run was permitted here; root's compile/strict checks remain mandatory.

## Sound source-level choices

- One App observer, Weak Runtime in callback, synchronous maintenance before version bump; observed Scope updated before nested submits and reentrancy guard suppresses nested render bump. No Session borrow or selected-context read guard spans maintenance submit.
- Full application Scope plus adapter generation protects project/board/instance transitions, including same-ID reopen. Weak Editor cleanup drops pending drag/capture and space state before exposure; Session cancellation requires matching scope and recorded gesture generation.
- Navigation validates board/instance before cancelling owned old drag and submitting Navigate. Cleanup deregistration compares registration token.
- Canvas real-part/ghost selection routes through private helpers; no new public Session/Runtime API. Empty semantic context has visible Inspector context and suppresses stale numeric inspector controls.
- Existing MoveParts/Edit command is sufficient for tree nudge; no new domain operation required.

## Remaining verification gates

Review revised exact source, then root-owned compile/strict checks and public regressions: live/empty/disabled and removed targets; multi-selection pruning; board/instance and same-ID reopen; asynchronous adoption cleanup; rejected navigation; stale callbacks and reused pointer IDs, especially unsettled prior gestures; anchor fallback; tree/canvas context synchronization; locked/multi-select nudge with Undo; selection/history/camera neutrality; disclosure and grouping. Existing Runtime CapturePointer/ReleasePointer effects run asynchronously and carry pointer ID only: reused-pointer browser evidence remains necessary, and source guard changes alone do not prove that lifetime gate.

RF: existing RF-001/007/010 remain the relevant shared-composition, observation and cancellation-ownership seams. No public facade widening or broad refactor proposed.

## Revised-source decision

Re-reviewed exact current adapter after author corrections. **The actionable adapter source blockers above are resolved; cleared for root-owned compilation and public verification. This is not a parity/acceptance pass.**

Reviewed SHA-256:

- `web/src/presentation.rs`: `a2b79b5e8ee5275e50b4b6547d54d5cd1b0716301c3ab395c033d4cf3970db6d`
- `web/src/presentation/selection.rs`: `aa35e991351aae6960337a78d2530bc900cb35e723060157b0f3a258feab2924`

Corrections checked:

- Observer now independently filters IDs by fresh `context_for_part` eligibility; removed contexts no longer discard unrelated selections. Every maintenance Replace invalidates anchor scope. Explicit selection filters both requested/range IDs and stamps only its expected eligible anchor. An ordinary click on an already-selected real part uses Add when needed to establish its anchor without collapsing the selected drag group.
- GestureBegin requires no existing Session gesture, then validates matching pointer/target IDs and current full Scope/generation. Every cancel checks recorded Session gesture generation. Pan returns before part GestureSample.
- Blank-canvas pan and real-part pan/move reject acquisition when a local Drag or unsettled Session gesture already exists, before capture or selection, preserving the current pointer owner's up/cancel path. Capture occurs after successful real target/selection validation.
- Nudge ignores locked focus, resolves current eligible unlocked moving IDs according to reference selected-group behavior, updates real selection, opens Inspect, submits one existing committed MoveParts operation, and announces the resulting focused position. Steps remain 0.1/1 mm.
- Mutating Signal operations now use mutable copied handles. Reviewer `git diff --check` passes. Author reports compiler currently blocked by Objects/tree diagnostics assigned to their owner; this review does not supersede that gate.

Additional source risks discovered during review (pan sampling a lingering gesture, replacing a Drag on a second pointerdown) were corrected before this decision. They were not reproduced in a browser. Required public evidence includes second-pointer/reused-pointer acquisition and asynchronous Runtime release effects; source review alone cannot certify those lifetimes. RF remains existing RF-001/007/010, with no API expansion or new broad refactor proposal.

## Final Clippy-only condition cleanup

Reviewed the final `selection.rs` condition cleanup: computing `invalid_range_anchor` then using `ids.is_empty() || invalid_range_anchor` preserves the reviewed Replace/Range decision. The extra anchor read for an empty Range is read-only. No new source blocker. `presentation.rs` hash remains unchanged; final `selection.rs` SHA-256 is `42aad1d0a17db0dfbf11719f016ad2af2a6698f63a9dbc042edc237b001e9c2d`. Reviewer diff check passes. Author reports final strict WASM Clippy passed; reviewer did not rerun compiler. Public verification gates above remain.
