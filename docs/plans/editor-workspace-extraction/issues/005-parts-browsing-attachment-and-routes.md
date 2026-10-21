# 005: Preserve Parts browsing, attachment and placement routes

Status: ready-for-agent
Type: build
Blocked by: [004: PCB wiring and plan delivery](004-pcb-wiring-and-plan-ownership.md)
Parent: [001: Editor workspace ownership specification](001-editor-workspace-ownership.md)
Related: [Plan and verification contract](../map.md), [ownership research](../../../research/editor-workspace-extraction.md)

## What to build

Make Parts own its browser, selection/preview and mounted-module action wiring while
preserving controller choosing, attachment and transitions into Layout placement.
Users can complete the same multi-workspace actions without an old Parts selection
redirecting a later result.

## Scope and constraints

- Extend the existing Parts workspace module with panel reveal, module browsing and
  attachment admission, orientation/selection epoch handling and construction of its
  Objects, preview and Inspector surfaces.
- Preserve existing Parts providers and Editor-lifetime state. Keep source ownership
  distinct from the destination Layout transaction and the PCB module destination.
- Consume explicit existing placement and PCB-navigation capabilities. Keep Layout's
  placement transaction implementation in its current owner until ticket 010; do not
  duplicate it inside an active-only Parts Inspector.

## Acceptance criteria

- [ ] Parts owns source selection, admitted browser/module actions and surface-input construction; Editor no longer assembles its feature-specific action internals.
- [ ] Selecting a mounted module from Layout or PCB opens the correct Parts definition and expires the same selection/preview epochs as before.
- [ ] A real attachment action produces the expected accepted module and returns/selects it only after its matching Landed result and current Parts destination checks.
- [ ] Hold attachment/preparation, change Parts selection or destination board, and release; the stale result cannot redirect or select a different module. Existing authoritative work is not canceled merely because its observation retired.
- [ ] PCB controller choosing opens Parts with the current chooser/query, and the actual Parts action reaches Layout placement with its intended definition and orientation.
- [ ] Matrix assembly placement enters Layout 2D, clears/reveals selection and panels according to current policy and does not bypass the shared canvas arbiter.
- [ ] General placement preview and cancellation remain functional through current Layout ports; no accepted edit is created by cancellation or browsing alone.
- [ ] Field drafts, preview activation, provider ancestry, definition loading and one-shot disabling retain their existing owner/binding behavior.

## Verification

Run documented quick checks, page-only WASM compilation, the page-bin browser suite,
relevant Parts browser/module/assembly tests and existing Layout placement tests.
Add focused real-composition success and stale-destination route cases, not a second
placement controller. Apply the plan's verification contract.
