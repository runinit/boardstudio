# 013: Verify integrated ownership and remove obsolete plumbing

Status: ready-for-agent
Type: build
Blocked by: [007: Setup-guide workflow](007-complete-setup-guide-workflow.md), [012: Layout canvas and gestures](012-layout-canvas-and-gesture-ownership.md)
Parent: [001: Editor workspace ownership specification](001-editor-workspace-ownership.md)
Related: [Plan and verification contract](../map.md), [ownership research](../../../research/editor-workspace-extraction.md)

## What to build

Finish the ownership refactor as one behavior-preserving application: remove obsolete
forwarding/input plumbing, verify every branch together and provide evidence that
editing, navigation, guide, canvas and teardown still obey the parent specification.

## Scope and constraints

- The direct blockers include every extraction transitively. Verify all predecessor
  outcomes and the current checkout rather than relying on the prototype's base.
- Remove only plumbing made obsolete by completed extractions: unused inputs,
  forwarding, re-exports, temporary adapters and test-host duplication. Keep real
  shared authorities and existing independently valuable feature tests.
- Review Editor by caller knowledge and responsibility, not a target size. It must
  compose workspaces and shared shell behavior, not hide the old monolith behind
  another name or rebuild large workspace controller bags.
- No parent-issue closure or status change is part of this ticket. Record completion
  evidence and remaining findings without silently waiving parent acceptance.

## Acceptance criteria

- [ ] All six workspace modules own their coherent controller/source/projection/action/surface wiring; guide and camera responsibilities are separated. Editor retains only justified common composition, routing and shared authorities.
- [ ] No obsolete input/forwarding/re-export or alternate test host remains solely to preserve the former monolithic wiring. No generic registry, mega-context, duplicate plan/setup/instance/camera authority or new public cross-crate contract was introduced.
- [ ] Map every parent user story and required behavior-matrix row to named passing tests or explained existing lower-seam evidence. Fill composition gaps rather than labeling manual prototype observations as regression coverage.
- [ ] Verify the complete six-workspace cycle, pending success/failure with newer drafts, production board/instance reconciliation, reused-ID replacement and stale callback rejection on the integrated composition.
- [ ] Verify cross-workspace Parts/module/controller routes, guide setup/finish/dismiss, Keycaps/Layout finding routes, measured camera/focus, desktop/compact panels and 2D/3D transitions together.
- [ ] Verify Export Back/Escape/header return and distinct wiring/Case review selection, including actual unmount while Export is mounted; its removed listener cannot navigate afterward.
- [ ] Verify Library hide versus Editor-only and App teardown, including late allowed work, observer/listener/pointer cleanup and no stale DOM/focus/navigation or dropped-Signal writes. No prototype cross-scope-warning suppression is accepted.
- [ ] Authoritative work, accepted publication, Undo boundaries, silent retirement, one-shot disabling, captured gesture/export ownership and accessible controls preserve the parent contracts.
- [ ] No prototype feature, runner, controls, fixtures, diagnostics or explanatory HTML is included in production. The throwaway branch remains evidence, not a merge source to apply wholesale.
- [ ] All relevant native tests, workspace mounted suites, the page-bin browser suite, page-only WASM compilation and the documented full pre-PR checks pass against the integrated result, including checked packaged build and browser/provider/drag smoke.
- [ ] Complete both spec-conformance and repository-standards review, resolving blocking findings. Record receipt scope, base/current revisions, any pre-existing failures and ownership/interface outcomes; unrelated work is preserved.

## Verification

Use the documented full pre-PR check and verification receipts, not a partial crate
browser selection labeled as full acceptance. Respect one build per physical target
and the supported scheduler. Update the plan's evidence/coverage notes with links to
ticket outcomes; do not close or modify the parent specification.
