# 008: Consolidate Keycaps assessment and finding navigation

Status: ready-for-agent
Type: build
Blocked by: [006: Case settings and instance navigation](006-case-settings-and-instance-navigation.md)
Parent: [001: Editor workspace ownership specification](001-editor-workspace-ownership.md)
Related: [Plan and verification contract](../map.md), [ownership research](../../../research/editor-workspace-extraction.md)

## What to build

Make Keycaps own settings, fit assessment and the complete finding-to-Layout/Case
workflow, so a current finding selects, frames and focuses its intended destination
while obsolete deferred work cannot interrupt a newer selection.

## Scope and constraints

- Move accepted edit/fit source construction, controllers, projections, admitted
  selection/settings actions and surface inputs into the existing Keycaps module.
- Move finding admission, source-camera capture, route dispatch, pending destination
  fit and owner-checked deferred focus together, not just the source click handler.
- Keep controllers and navigation alive for Editor lifetime across workspace switches.
  Expose only genuinely shared fit display/focused finding/projection capabilities.
- Consume shared tree/Case navigation and measured-surface capabilities; retain Case
  selection priorities and operation-specific identities. Do not copy camera or
  instance state or rewrite destination fitting formulas.

## Acceptance criteria

- [ ] Keycaps owns controller/source/projection assembly, admitted actions and surface construction; Editor does not reconstruct its controller inputs or deferred finding workflow.
- [ ] Fit and legend-related assessment refresh after relevant edits made outside Keycaps, using current accepted inputs rather than visibility of the Keycaps view.
- [ ] Real mounted settings and key-selection actions retain accepted values, pending/failure policy and physical-instance eligibility checks.
- [ ] An actual current finding routes through production shell navigation to the intended Layout selection, preserves its source camera basis, then frames the measured destination and focuses its current Inspector control.
- [ ] A Case finding selects its intended body/layer and applies the existing conflicting-selection clearing and panel reveal/pinning rules.
- [ ] Replace selection, board or project before deferred fit/focus completes; obsolete work cannot move the camera, focus a new control, restore a stale marker or write dropped Signals.
- [ ] Finding admission rejects obsolete fit/target/source identities. Leaving/returning does not revive a retired navigation observation.
- [ ] Hook lifetime, provider ancestry, shared 3D consumers and route-slot refresh semantics remain unchanged; no active-Keycaps-only navigation mount is added.
- [ ] Document the small fit/projection/surface capabilities consumed by later camera coordination without exporting individual controllers.

## Verification

Run documented quick checks, page-only WASM compilation, the page-bin browser suite
and existing Keycaps settings/fit/navigation tests. Add real destination canvas and
Inspector coverage; recorded route effects alone do not establish DOM focus or
measured fit. Fixture CAD evidence is not packaged provider/GPU qualification.
