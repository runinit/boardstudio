# 007: Consolidate the complete setup-guide workflow

Status: ready-for-agent
Type: build
Blocked by: [005: Parts routes](005-parts-browsing-attachment-and-routes.md), [006: Case settings and instance navigation](006-case-settings-and-instance-navigation.md)
Parent: [001: Editor workspace ownership specification](001-editor-workspace-ownership.md)
Related: [Plan and verification contract](../map.md), [ownership research](../../../research/editor-workspace-extraction.md)

## What to build

Give the setup guide ownership of its full workflow, from project request and name
through Layout/Wiring/Case/Review stages, while preserving placement hiding, panel
reveals, physical setup, full-settings return, finish and dismiss behavior.

## Scope and constraints

- Move project-name editing, guide admission/status/projections, auto-open/wiring
  effects, stage/actions and drawer composition into the existing setup module.
- Consume the shared PCB plan, established Parts/placement ports, Case settings and
  single physical-setup capability. Do not introduce a guide-owned duplicate.
- Keep guide lifecycle Editor-lifetime when its drawer is hidden. Leave shared
  instance preference shell-owned. Move Layout-only geometry-script visibility to
  Layout presentation without changing its lifetime or sharing.
- Stage changes must retain explicit panel-reveal priority over ordinary workspace
  defaults; project admission must be read live during asynchronous preparation.

## Acceptance criteria

- [ ] The setup module owns request consumption/persistence, name-edit tickets, workflow effects, admitted actions and both guide drawer views; Editor receives only common composition/visibility intent.
- [ ] Requests are consumed once for their accepted matching project. Replacement with reused IDs does not accept an old request, preference update, callback or failure against the new session.
- [ ] Actual stage controls route Project/Layout/Wiring to Layout, Case to Case and Review to Export, preserving selection clearing and requested panel reveals.
- [ ] The guide and Case use one physical-setup busy gate. Leaving the preparation owner prevents an unsubmitted edit; a submitted operation can settle without obsolete feedback/navigation.
- [ ] Successful setup reassignment preserves production scope transition and effective physical-instance reconciliation, not a manually adjusted generation.
- [ ] Wiring consumes the current shared plan and keeps guarded materialization behavior; matrix auto-open occurs only in its existing eligible Layout-stage conditions.
- [ ] Placement/pair setup can hide the drawer and return without recreating its controller or resetting project-name observation/binding policy.
- [ ] Full Case settings and Back return correctly; finish dismisses and returns to Layout, whereas ordinary dismiss retains its different route behavior.
- [ ] Geometry-script visibility and instance preference no longer imply guide ownership; closing the guide does not reset them or mount new authorities.
- [ ] Desktop and compact panel behavior preserve ordinary defaults versus explicit reveal precedence without unintended accepted-document edits.

## Verification

Run documented quick checks, page-only WASM compilation, the page-bin browser suite,
existing setup-guide/panel/name tests and relevant physical-setup/placement tests.
Drive the actual production drawer and reassignment path rather than relying on the
older placeholder-settings guide fixture. Apply the plan's verification contract.
