# 006: Give Case ownership of settings and instance-aware navigation

Status: ready-for-agent
Type: build
Blocked by: [003: Keymap workspace ownership pilot](003-keymap-workspace-ownership.md)
Parent: [001: Editor workspace ownership specification](001-editor-workspace-ownership.md)
Related: [Plan and verification contract](../map.md), [ownership research](../../../research/editor-workspace-extraction.md)

## What to build

Make Case own its settings-controller wiring, current display/edit projections and
workspace surfaces while preserving the intended physical board instance and distinct
assembly, body and layer selection. The guide and Case continue sharing one
physical-setup operation and busy gate.

## Scope and constraints

- Extend the existing Case workspace module with the mechanical-settings mount,
  admission/projection assembly, configured-board/finding routes and surface inputs.
- Keep shared instance preference and board/instance navigation under shell authority,
  independent of whether the setup guide is visible. Preserve one Editor-lifetime
  physical-setup mount and expose only its existing consumer capabilities to guide
  and Case; it must not become an active-Case-only or guide-only mount.
- Preserve mechanical-settings providers above all consumers, including the guide's
  full-settings drawer. Viewer resources and collision cancellation keep their
  existing visible-child lifetime.
- This ticket does not require PCB's plan extraction: existing physical-setup feature
  interfaces can remain unchanged. Its prerequisite is the approved first-slice gate.

## Acceptance criteria

- [ ] Case owns settings mount/projections, admitted selection/navigation and surface construction instead of exporting controller details for Editor to reconstruct.
- [ ] Actual assembly select versus assembly navigate, body, layer, PCB and gasket actions preserve their distinct tree/range/Case selection effects and Inspector priority.
- [ ] Board and explicit physical-instance changes use production scope reconciliation and preserve current instance preference resolution; stale rendered controls cannot edit another instance.
- [ ] Previous same-scope display geometry remains displayable where currently permitted, but cannot authorize stale editable settings or findings.
- [ ] A real physical-setup action from Case shares one busy/settlement workflow with Project guide admission; duplicate submission is refused while pending.
- [ ] A successful reassignment follows only its matching accepted scope transition, and stale preparation/completion after owner departure cannot navigate or report against the new instance.
- [ ] Guide settings consumers still resolve the same mechanical controller; instance authority works with the guide closed. Record the small shared capability used by ticket 007.
- [ ] Viewer/layer-dock/collision lifetimes and settings drafts/binding/drop behavior remain unchanged; no duplicate controller, instance preference or physical-setup mount is introduced.

## Verification

Run documented quick checks, page-only WASM compilation, the page-bin browser suite,
existing Case conditional-Inspector/previous-geometry/settings tests and relevant
physical-setup tests. Add production-composition instance navigation and reassignment
coverage while retaining lower-level mechanical tests. Follow the shared contract.
