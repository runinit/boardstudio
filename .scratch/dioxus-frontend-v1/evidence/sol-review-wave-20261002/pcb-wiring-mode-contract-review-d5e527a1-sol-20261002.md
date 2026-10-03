# PCB F5.2c wiring-mode contract review — 2026-10-02

Reviewer: independent Sol 6.1 High. Planning/source-capability review only; no source edits or implementation acceptance.

- Frozen planning commit: `d5e527a150ea354efc68091084bf29e2d24f0da9`.
- Base: `3f5453e14b9610257c635de975d12c8ddeb505bc`.
- Worktree: `/home/chris/.local/share/boardstudio/worktrees/pcb-wiring-mode-apply-20261002`, clean at review.
- Spec `.scratch/dioxus-pcb-view/drafts/F5.2c-board-wiring-mode.md`: independently matched SHA-256 `d949f6de1b22d0d961521e7fca9d38f0a254ac5c25b6d312748f777f04212c0f`.
- Issue `.scratch/dioxus-pcb-view/issues/08-board-wiring-mode.md`: independently matched SHA-256 `b34a48ba414f1c6ff5401a66289cf25fd44fd8be319097e29682958e8069ab3e`.

| Axis | Verdict | Bound |
| --- | --- | --- |
| Standards | CLEAR | Private accepted-source feature ownership and explicit inherited gates. |
| Spec | CLEAR | Capability gate is present; the bounded Matrix/Direct edit can start. |

## Standards

The exact diff adds only two planning files, with no API, configuration, source or schema mutation. Reviewed applicable supplied instructions and current CONSTRAINTS execution authority. Capability-level implementation starts are explicitly allowed while broader parents remain incomplete; this packet neither closes those parents nor relabels unperformed acceptance as successful.

The Inspector receives projected read-only values plus a typed callback; Runtime and authoritative writable state stay with the existing Editor-lifetime owner. Existing ElectricalMode/ElectricalBoardConfiguration and Event::Edit/ReplaceDocument are used without new public visibility, contracts or persistent fields. A narrowly scoped private/crate-local registration and root mount may be authored in isolation and independently reviewed for serial integration under current authority.

No speculative abstraction or new state owner is required. No new refactoring takeaway was observed in this bounded contract inventory; preserve RF-001/RF-006/RF-009.

## Spec and verified start capability

Pinned React `5a472a9426e6e38993361da402cd4ec730feb369`, `app/src/ui/WiringPanel.tsx:40`, exposes accessible Wiring mode with values matrix/direct and labels Matrix/Direct GPIO. Its usePcbWorkspace contextual routing distinguishes switch/generic-part views from the board/controller Wiring panel. The contract accurately restricts the new selector to that existing board-level context and keeps Apply as a separate follow-up.

Current candidate source supports the claimed capability gate:

- `web/src/presentation/pcb_wiring.rs:80–146` PcbWiringSource carries accepted document, normalized board-plan identity, UI scope, scope generation and active part selection. Its constructor rejects mismatching epoch/document/board/scene revision and normalizes only physical instance out of the plan scope.
- `web/src/presentation.rs:3832–3858` projects that source from current scope/accepted snapshot, active board and current instance preference, retaining UI scope/selection/generation for guarded edits.
- `core/src/electrical.rs:25–33` already defines Matrix and Direct. `core/src/model.rs:1082–1091` already defines a Default ElectricalBoardConfiguration with board ID, controller, mode, locks, assignments, key bindings, jumper states and protected handoff. Existing target configuration can be cloned and only mode changed; absence can use the same default with the requested board ID. Other boards/document fields remain untouched.
- `web/src/presentation/pcb_wiring/controller.rs:769–917` owns automatic resolution at Editor lifetime. Current board identity includes scope, accepted token/revision and executor epoch; old async results are rejected after identity/request generation changes. A newly accepted mode edit advances token/revision and naturally triggers fresh resolution.
- `web/src/runtime.rs:412` allocates exact operation IDs; the existing part-net owner in `pcb_wiring/controller.rs:499–540` demonstrates private guarded Event::Edit/ReplaceDocument submission through that allocator and normal operation outcome observation.
- `application/src/session.rs:1294–1370` keeps the previous accepted snapshot on persistence failure and publishes a new accepted snapshot/token only on SaveResult::Committed. This supports the specified accepted-only selector value and prevents requested mode from appearing saved on rejection/failure. It does not require an optimistic local mode draft or rollback store.
- `web/src/runtime.rs:769–802` resolves the existing board configuration's ElectricalMode using the current accepted source. No new Core resolver request/algorithm is needed.

The requested request identity guards cover active PCB workspace, session/document, selected board and UI/normalized scope, accepted token/revision, selection context and generation. Implementation must capture and compare the rendered selection identity, rather than treating the plan's intentionally selection-independent identity as sufficient. It must also preserve ready/saved state, no-preview/no-gesture admission and current instance checks. These are explicit contract requirements and existing owner data can support them privately.

No-op mode choice must submit nothing; a real accepted mode change is one normal committed edit/history entry. Existing configuration fields, other boards, nets/pins/bindings/extensions and protected handoff are preserved. The contract requires production mounted action tests, stale-context tests, accepted-save/rejection behavior, current resolver refresh, and paired Direct-mode/Undo/Redo/save-reopen/second-board checks. Required failure cases are not delegated to a pure proposal helper.

## Verification and remaining gates

Exact commit/base resolved; the diff is nonempty and limited to the spec/ticket. Both declared digests and exact diff check passed. Relevant current source contracts and pinned React control/routing were read directly. No executable suite was needed to review these documentation-only start decisions; implementation checks remain required and unexecuted for this new slice.

The author may proceed with this bounded private implementation in isolation. Source/registration/root join review, affected supported checks and packaged paired public mode/history/reopen acceptance remain OPEN. Applying the plan, controller choice, locks/pins, protected-remap flow and full F5.1/F5.2/F5.3 and canonical parent closure are not accepted by this review.

Standards: zero findings. Spec: zero findings; bounded capability start is clear.
