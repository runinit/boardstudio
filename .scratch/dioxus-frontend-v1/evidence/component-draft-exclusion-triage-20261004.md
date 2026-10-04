# Mounted component draft exclusion triage (2026-10-04)

Read-only review at `418dcb41411b` on `codex/rust-v1-ui-parity-20261001`; followed root `AGENTS.md`, frontend handoff, `CONSTRAINTS.md`, and `AGENT-ROUTING.md`. No source edits, test/build/browser execution, or commits. The exclusion and expected assertion remain unchanged.

## Corrected finding

The `4.00` versus `7.25` result does **not establish a product defect**. The wasm fixture sets `layout_component_inspector_test_state`, so `Runtime::submit` records `Event::Edit` and returns before Core/Session (`web/src/runtime.rs:1736-1747`). Focusing Y after X can trigger X's blur and emit the `7.25` move, but the recorder never accepts it. The fixture's `accept_unrelated_revision` then changes only revision, Y, and a parameter, leaving accepted X at `4.00` (`layout_component_inspector_tests.rs:390-435`). Returning to Properties calls `reset_properties_drafts` using the current projection's accepted position; that is why the stale fixture resets X to `4.00`.

The reference does not support the current all-fields-survive assertion: React `Coordinate` commits on blur (`app/src/ui/InspectorControls.tsx`), and switching to Relations replaces/unmounts the property subtree (`app/src/ui/Workbench.tsx:1400-1401`); accepted X should repopulate on return if its commit has been applied. React `ConstraintNumber` only updates local state, so unsaved Offset X `8.5` is lost when that subtree unmounts. The test also expects Y to reset to `4.00` although focusing the margin field after Y should blur/commit Y as well. These expectations are internally inconsistent with ordinary focus/blur plus accepted-edit behavior.

## Coverage and next seam

The existing component module's local unit coverage only checks position parsing/finite/locked gating; it cannot validate accepted draft refresh. `mounted_component_drafts_survive_unrelated_acceptance_and_blur_uses_latest_owner` has a fake event sink, not a Core/Session acceptance loop, so its latest-owner assertion only proves command construction. Keep the exclusion until a correct mounted seam applies emitted position/outline edits through Core/Session, accepts them, then checks post-acceptance values across Relations/Properties and latest-owner submission. Compare only behavior React actually preserves; unsaved ConstraintNumber state should not be required to survive a subtree unmount. A public mismatch is not established by this source-only review.
