# 012: Complete Layout canvas and gesture ownership

Status: ready-for-agent
Type: build
Blocked by: [010: Layout creation and placement](010-layout-creation-and-placement-ownership.md), [011: Shared camera and surface coordination](011-shared-camera-and-surface-coordination.md)
Parent: [001: Editor workspace ownership specification](001-editor-workspace-ownership.md)
Related: [Plan and verification contract](../map.md), [ownership research](../../../research/editor-workspace-extraction.md)

## What to build

Give Layout ownership of its complete transaction canvas, overlays and gesture
dispatch so pointer and keyboard editing work as before without Editor reconstructing
the workflow. Shared navigation and PCB-specific hit behavior remain separate.

## Scope and constraints

- Move Layout transaction rendering, event-priority routing, preview/selection
  overlays, threshold/sample/commit/cancel handling and related admitted callbacks
  together into existing Layout ownership.
- Consume established selection/Inspector, creation/placement and shared surface/
  navigation capabilities. Retain one arbiter and existing Session preview/commit
  routes; do not add generic events or rewrite geometry.
- Preserve DOM nesting/keys/IDs, accessible canvas instructions, callback freshness,
  event bubbling, pointer capture and the cleanup registration's owning lifetime.
- Keep PCB's different hit/readiness/modified-selection rules in PCB while sharing
  only pan/surface/drag mechanics that were actually common before.

## Acceptance criteria

- [ ] Layout owns its transaction canvas, overlays and feature gesture admission; Editor composes the resulting workspace rather than reconstructing handlers and transaction markup.
- [ ] Actual pointer down/move/up preserves selection modifiers, movement threshold, sample/preview and exact commit behavior. Preview cancellation creates no accepted edit.
- [ ] Space-pan priority versus placement/outline/transform ownership remains unchanged; conflicting interactions cannot own the canvas simultaneously.
- [ ] Escape, pointercancel and lost capture clean up the owning transaction and release capture without committing, clearing another owner or leaving a stale preview.
- [ ] Enter/arrow placement and other existing canvas shortcuts work, while Ctrl/Cmd+Z and Redo are dispatched once rather than duplicated through bubbling or listeners.
- [ ] A real accepted move has the same geometry and Undo boundary; use existing feature geometry tests rather than reimplementing expected math in the page fixture.
- [ ] Board/project navigation during a gesture invokes production cleanup before stale callbacks can act; actual Editor-only and App teardown release owned resources safely.
- [ ] PCB remains usable with its distinct readiness, locked-part and modified-hit rules; shared navigation changes do not route PCB actions through Layout admission.
- [ ] Callbacks remain fresh while memoized scene geometry is reused: selection, preview and layer changes reach the rendered canvas without stale input handlers.
- [ ] Controls, focus, roles, labels, CSS ancestry and 2D/3D return behavior remain equivalent; no active-only controller mount, extra arbiter or replacement camera is introduced.

## Verification

Run documented quick checks, page-only WASM compilation, the page-bin browser suite,
relevant Layout interaction/canvas/placement tests and affected PCB tests. Run the
documented packaged-page drag smoke against freshly built assets to exercise trusted
pointer movement and exact Undo, not only synthetic DOM callbacks. Apply the shared
verification contract and retain final broad integration checks for ticket 013.
