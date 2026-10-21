# Map: Lifetime-preserving Editor workspace extraction

## Destination

Give each workspace ownership of its page-shell wiring without changing editing,
navigation, canvas, guide or persistence behavior. Keep the common shell small by
responsibility, not by imposing a line-count target.

## Specification

- [001: Separate Editor workspace ownership without changing lifetimes](issues/001-editor-workspace-ownership.md) — `ready-for-agent`.

001 is the parent specification, not a separate implementation slice. Leave its body
and status unchanged while working the children below. The user approved this
12-ticket breakdown on 2026-10-20. All children are `ready-for-agent`; blocking edges
determine which can be claimed, not a different readiness label.

## Implementation tickets

| Ticket | Direct blockers | Delivers |
| --- | --- | --- |
| [002: Establish the real App composition test seam](issues/002-production-composition-test-seam.md) | None | Scope-correct real composition, gated work, reconciliation, Export return and actual teardown coverage. |
| [003: Give Keymap complete workspace ownership](issues/003-keymap-workspace-ownership.md) | 002 | First complete ownership slice: layers, bindings, encoders, macros and export admission. |
| [004: Give PCB ownership of wiring and plan delivery](issues/004-pcb-wiring-and-plan-ownership.md) | 003 | PCB wiring and actions, with one plan serving the other workspaces. |
| [005: Preserve Parts browsing, attachment and placement routes](issues/005-parts-browsing-attachment-and-routes.md) | 004 | Parts source ownership and checked module/controller/placement transitions. |
| [006: Give Case ownership of settings and instance-aware navigation](issues/006-case-settings-and-instance-navigation.md) | 003 | Case surfaces/settings with shared instance authority and one physical-setup workflow. |
| [007: Consolidate the complete setup-guide workflow](issues/007-complete-setup-guide-workflow.md) | 005, 006 | Project-through-Review orchestration, drawer and stage/panel behavior. |
| [008: Consolidate Keycaps assessment and finding navigation](issues/008-keycaps-assessment-and-finding-navigation.md) | 006 | Current fit/settings and complete deferred Layout/Case finding navigation. |
| [009: Give Layout ownership of selection and Inspector orchestration](issues/009-layout-selection-and-inspector-ownership.md) | 003 | Layout admission, projections, Inspector choice and finding-selection routes. |
| [010: Give Layout ownership of creation and placement workflows](issues/010-layout-creation-and-placement-ownership.md) | 005, 009 | Shared-arbiter creation, preview, commit/cancel and post-Landed selection. |
| [011: Consolidate shared camera and surface coordination](issues/011-shared-camera-and-surface-coordination.md) | 008, 009 | Measured surface, camera and shared view/resource lifecycle coordination. |
| [012: Complete Layout canvas and gesture ownership](issues/012-layout-canvas-and-gesture-ownership.md) | 010, 011 | Layout transaction canvas, pointer/keyboard dispatch and cleanup. |
| [013: Verify integrated ownership and remove obsolete plumbing](issues/013-integrated-ownership-verification.md) | 007, 012 | All branches integrated, obsolete plumbing removed and full acceptance evidence. |

### Frontier and blocking rationale

- **Initial implementation frontier: 002 only.** Do not claim 001 as a second build task.
- 002 must establish safe, scope-correct composition characterization before any
  workspace moves. 003 is the specification's deliberate first-workspace pilot gate;
  later independent extractions use its interface/lifetime outcome, not its controllers.
- After 003, PCB (004), Case (006) and Layout selection/Inspector (009) can proceed
  independently. Sharing Editor edits requires integration coordination, not fabricated
  dependency edges or concurrent builds against the same physical target.
- Parts consumes PCB's checked destination/plan capabilities. Guide consumes the
  established Parts/placement and Case/settings/setup capabilities. Keycaps consumes
  Case navigation; camera coordination consumes stable Keycaps and Layout projections.
- Layout creation consumes Parts ports and Layout selection; its guide consumer can
  remain in place until 007, so 010 is not blocked by the guide extraction. Layout
  gestures consume creation plus shared camera/surface capabilities.
- 013 waits on 007 and 012; these two edges include every earlier implementation
  ticket transitively. There is no all-at-once integration branch: each slice must
  leave production composition and its relevant checks green.

## Shared implementation and verification contract

- Read the parent spec, ownership research, current architecture/ADRs and predecessor
  outcomes before editing. Refresh source/callers against the changing checkout;
  preserve unrelated changes. Each ticket owns its broken callers, tests and fixtures.
- Containers stay in the bin. Keep workspace hooks unconditional in Editor scope,
  provider ancestry and existing visible-child lifetimes. Preserve one Session/Core
  authority, Runtime subscriber, canvas arbiter, plan query and physical-setup mount.
- Keep operation-specific admission, accepted/save publication, pending-edit/draft/
  binding semantics, cancellation and cleanup intact. No UI redesign, formula changes,
  broad draft cache, new registry, mega-context or public cross-crate interface.
- A ticket's tests exercise user action through real composition and assert visible
  output plus accepted/saved or view-state results. Keep useful native/feature tests;
  add page tests for composition gaps, not duplicate every field or assert source shape.
- Run the [documented checks](../../../README.md#checks) appropriate to each
  slice: quick changed-scope checks, page-only WASM compilation, page-bin mounted tests
  and the relevant workspace/Runtime suites named in each ticket. A subset is not full
  acceptance. Use the supported scheduler and never overlap builds against one target.
- Record exact commands, baseline/current scope, test outcomes and diagnostics in the
  ticket's outcome. Long logs and verification receipts belong under the gitignored
  evidence directory, not committed source copies. The final ticket runs full checks,
  checked packaged build, provider/browser/drag smoke and both review axes.
- Never use explanatory HTML, DOM-root removal, manually advanced generation or a
  serialized Open behind a held edit as proof of real teardown/reconciliation or a
  reply delivered after accepted replacement. Capture evidence at the seam that can
  actually exercise that behavior and explain any lower-seam coverage.

## Evidence and decisions

- [Ownership research](../../research/editor-workspace-extraction.md) identifies the
  existing modules, three distinct lifetimes, cross-workspace flows and coverage gaps.
- The throwaway prototype is retained on branch
  `t3/editor-keymap-lifecycle-prototype`, source commit
  `c9e60307feeec9bdb0d2dc6d21a30a4af7440f59`, verdict commit
  `10cb410a1104fbe28970eac6e7a97b755a08ecb6`. Do not merge the experiment wholesale.
- Real mounted interactions demonstrated bounded App Runtime injection and Keymap
  extraction feasibility. Cross-scope Signal warnings and untested stale-reply,
  routing, focus and resource teardown paths remain implementation verification gates.
- Shared identity and controller lifetime are preserved independently of source-module
  location. Containers stay in the page-shell binary; no new workspace framework.

## Related work

- [Cleanup audit follow-ups](../cleanup/map.md) remain independent of this spec.
- [Architecture](../../architecture.md) and [domain vocabulary](../../../GLOSSARY.md)
  remain authoritative.
