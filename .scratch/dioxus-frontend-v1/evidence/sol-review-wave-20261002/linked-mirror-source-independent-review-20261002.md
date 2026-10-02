# Linked mirrored-pair source review

Candidate `35cafc77f15fac50575f79e1517d964d64d837ab`, isolated `layout-linked-authoring-20261002`; reviewed full source packet against `fc7a6853dfb5e8b6eafd36644d76e9e847415114`, including earlier private leaf `c321360d` and root merge `de8eb2f9`. Current integration root inspected at `2394b9961a5f7d2dd814f1637cf611185d4449d6`.

Issue 02 SHA256 verified `e74a40895113721ffbcfaf128f135ab8651b5c6700eeda271c0f303f36d77a08`; reviewed its approved source mapping/publication record, new `linked-mirror-canvas-owner-20261002/join-contract.md`, current root CONSTRAINTS, production Session/Runtime settlement, and pinned React form/placement source. Requested reviewer Sol 6.1 High; runtime settings are not exposed here. No app source edits or subagents.

## Spec — HOLD

1. **[P2] Preserve the setup when cancelling the ghost.** `web/src/presentation.rs:3299` sends placement Escape to `on_cancel`; `objects/mirrored_pair_controller.rs:187` clears `open`, `draft`, and placement. The join contract explicitly says “Escape returns to the form without submitting an edit.” Add a distinct ghost-to-form transition retaining entered values; keep full setup Cancel separate. A mounted regression must prove ghost Escape returns to the populated setup and leaves the accepted document untouched.

2. **[P2] Bind settlement to the operation's actual accepted result.** `objects/mirrored_pair_controller.rs:795` passes the currently accepted snapshot as both `result_token/revision` and `accepted_token/revision`. The exact-result check is therefore tautological in production. `PendingPair` retains only the base capture and a terminal-outcome slot, whose value contains no result snapshot. If a subsequent queued edit advances the accepted snapshot before the effect settles (the footer still admits edits), an unchanged pair can satisfy these checks and redirect selection for a stale completion. The contract requires that “the current accepted token/revision exactly match that result.” Capture the result identity at the exact operation settlement through a reviewed private seam, then reject a later accepted snapshot. Test that mounted ordering, rather than only the pure predicate with hand-supplied identities.

3. **[P2] Hide or disable setup edits while its placement is active.** `objects/mirrored_pair_controller.rs:596` continues projecting an editable form during active placement, while `on_preview` at line 209 rejects every request when placement exists. The Objects mount always renders that form. Users can change rows/gap/preset and press an enabled Preview button which silently does nothing; clicking the canvas commits the earlier prepared values. Issue 02 requires preview of the proposed setup before commit, and React's form is mounted only for `pair-setup`, separate from `mirrored-pair`. Make the stage transition explicit and exercise changing setup after cancelling/re-entering preview.

## Standards — HOLD

1. **[P2] Remove the second matrix-geometry implementation.** `web/src/mirrored_pair_geometry.rs:66` independently recomputes cell offsets, stagger accumulation, splay pivots/rotation, mirroring, and matrix rotation already owned by `core/src/matrix.rs:273`. Issue 02 forbids “copied geometry logic”; current CONSTRAINTS Architecture requires one authoritative geometry implementation. The helper's own comment says it mirrors Core's fresh-matrix projection. Its bounded fresh-input claim does not justify copying the general Core algorithm. Consume the existing Core/provider matrix preview projection through an approved private composition seam; preserve public/wire contracts. Test the same prepared matrices against the authoritative projection.

No other documented standard violations found. The private module visibility, reused `CreateMirroredPair` command, exact operation registration, accepted source checks around awaited preparation, and all-state `owns_canvas` predicate are coherent. No tests or thresholds were weakened and no suppressions/stubs were added. The structural geometry duplication should be appended to existing RF-003/RF-006 source/geometry ownership evidence; do not invent a duplicate RF ID. Immediate correctness fixes above cannot be deferred as refactoring.

## Verification and integration boundary

- Independently reran `CARGO_TARGET_DIR=/tmp/layout-linked-target cargo test --manifest-path web/Cargo.toml --bin boardstudio-web --features page mirrored_pair_geometry::tests`: 4 passed; existing unrelated native dead-code warnings remain visible.
- `git diff --check fc7a6853...35cafc77`: passed; candidate worktree clean.
- Read both retained mutation-red logs and green receipt. They establish pure geometry/predicate sensitivity, not the mounted state transitions or production result capture. Author-reported strict WASM Clippy reused, not independently rerun.
- Symmetric PartPlacement ownership/admission/canvas-routing join remains explicitly outstanding. The current packet cannot be accepted as the integrated canvas owner while that join is missing. Review the separate controller API repair, integrate serially, then test both async preparation and pending-save transitions in both directions.
- Mirror-existing all/one, relationship display/unlink/propagation, paired public creation/cancel/history/Undo/Redo/reopen, focus/compact/themes, and parent F3.2 joins remain open. Port 34730's `d320` build excludes this packet and cannot prove it. Rebase/serial integration must preserve root's later PCB07/Parts12 work.

Spec: 3 findings, worst P2. Standards: 1 finding, worst P2. No source clearance or public acceptance claimed.
