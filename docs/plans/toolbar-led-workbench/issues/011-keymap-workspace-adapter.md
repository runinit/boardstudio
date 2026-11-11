# 011: Keymap adapter with layer and input-editor identity intact

Status: ready-for-human
Type: build
Blocked by: [009](009-shared-toolbar-panel-host.md)
Parent: [map.md](../map.md)
Owner: Keymap workspace/navigation/panel and existing layer/binding/macro/encoder owners

## Boundary

Use KeymapNavigation as compact Objects navigation for layers and searchable keys. Move
existing Layer operations (Add, name/rename-on-blur, remove non-base, guards/feedback) into
Inspect, not the compact result list. Keep Keys/Macros/Encoders editing and firmware export.
Relocate DesignViewToolbar contextual/View controls into the shared row; do not add Layout's
five sections. Keep key selection/viewer identity; ordinary key/layer picks never open Inspect.

Audit conditional editor unmounts and scope/key reset effects in `keymap/panel.rs` as well
as keys in `presentation/keymap_workspace.rs`. Retain draft/error/disclosure under layer,
input identity/generation, editor kind/instance and session as applicable. Restoration must
not renew an expired binding/macro/encoder admission token. Existing layer limits, base-layer
protection, pending/save guard and firmware availability remain unchanged.

M2 gate: classify BindingEditor/MacroEditor/EncoderEditor commit modes and actual unresolved
forms before implementation. No DraftOwnerRegistration currently exists in this crate;
that is not evidence that each editor has an explicit-submit draft. Record which forms
need the existing update-safety registry before any such form is retained off screen. Do
not add a second registry or silently reload over newly retained unresolved work.

Layer name is known blur-submit: hoist/retain its text + dirty/error + captured layer ID
as one per-layer owner, not the current component-local dirty flag plus shared name signal.
Retain binding/macro/encoder controller state above conditional sections, using their actual
input/generation keys; use keep-mounted scopes where possible and hoist only keyed resets.
Complete the classification and approve M2 before changing this ticket to ready-for-agent.

## Acceptance criteria

- [ ] Every [Keymap inventory row](../inventory.md#keymap-adapter) reachable, including layer
      operations, macro/encoder editors and export; no duplicated navigation/editor.
- [ ] Drafts survive panel/workspace and Keys→Macros→Encoders/selection changes without
      rebinding to another layer/key; deleted/recreated input remains a different identity.
- [ ] No automatic Inspect opening on normal navigation; explicit editor request works.

## Proof map

- Layer navigation/operations — base plus higher layer, pending/save/limit variants → search,
  pick layer/key, explicitly edit layer name/Add/remove → same accepted layer/feedback,
  base/limit guards and Undo; no implicit Inspect from a pick · mounted · planned
- Dirty layer-name identity — type a new name for A, then hide without blur and pick B before
  a late event → B's text/saved name unchanged; A's full text/dirty/error remains available
  if admission did not settle it; reopening and legitimate blur renames live A exactly once,
  never B. Parameterize genuine blur-before-dismiss separately per the refactoring table · mounted · planned
- Editor lifetime — actual unsubmitted binding/macro/encoder draft A after trigger audit,
  before its real commit event → switch section/key/layer/panel/workspace and return → exact
  A text/error/admission retained, no synthetic commit or B leak; existing real triggers
  still commit exactly once to their captured owner · mounted · planned
- Update-safety classification — real retained unresolved form identified in the M2 audit →
  hide/switch and request Update Now → approved treatment protects that form and identifies
  Keymap/editor/remedy; already accepted or pending values do not become false blockers · mounted · planned
- Input replacement — edit response captured for key/encoder A before removal → recreate at
  same location/ID then release response → replacement stays unassigned, no stale edit
  admitted; existing output-specific readiness unchanged · mounted · planned
- View/export — applicable contextual tools and 2D/3D/Footprints in row → operate and request
  existing firmware export → same view-only state/availability and existing export route · mounted · planned

## Checks and mutations

[Verification](../verification.md): `check.py quick`, `check.py lint`, `check.py browser --only keymap`
and `run-wasm-tests.py --files web/src/presentation/keymap_workspace.rs` for page wiring.
Reuse layer/disclosure/admission fixtures. Named identity-generation proof-gap mutation must
fail replacement-unassigned/no-stale-write assertions. Add the dirty-layer mutant in
verification.md: bind a delayed rename to current B or drop A's dirty state; the assertion
must catch wrong B name or lost A draft/rename, not just zero revisions. Coordinator reruns
those reject paths. No production implementation before its M2 classification/approval.
