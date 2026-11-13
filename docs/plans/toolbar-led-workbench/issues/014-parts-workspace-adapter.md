# 014: Parts adapter with catalog, authoring and placement continuity

Status: awaiting-decision
Type: build
Blocked by: [009](009-shared-toolbar-panel-host.md), [005](005-retain-objects-capabilities.md)
Parent: [map.md](../map.md)
Owner: Parts workspace/catalog/Inspector/task presentation, existing Parts controllers

## Boundary

Parts Objects is a compact catalog navigator, not a PCB tree. Preserve searchable name/
category groups, key-assembly presets/orientation, query and selected definition/module/
assembly identity, loading/failure/empty feedback and preview activation. Move Create key
assembly / Import KiCad / new custom component into named larger catalog task entry points
in Project/Add or existing explicit commands; retain native file picker/import semantics.

Inspect retains saved-assembly editing, definition/custom fields, generator and model editors,
module/variant/attachment details and existing placement/apply actions. Larger authoring uses
existing editor/task surfaces; do not truncate a full editor into compact Objects. Preserve
2D/3D model/footprint previews, Layers/legend visibility, import/retry/errors and Back to PCB
controller-browse context. Share utility row without fabricated Layout tools/second action row.

Keep PartsWorkspaceState query, generator_draft, selection_generation, preview_activation,
assembly selection/orientation and controller placement owners. Ordinary catalog selection
changes preview, not placement or automatic Inspect opening. Explicit edit/place is separate.
M2 approval is required. Keep existing PartsWorkspaceState above selection/panel switches;
retain assembly/generator/module/model editor scopes and their actual registered draft
signals/guards together. Hoist reset-owned state only under the real selection generation.
Any newly found unregistered form needs trigger classification before update-safety changes;
off-screen blockers identify Parts and the real editor/remedy.

## Acceptance criteria

- [ ] Every [Parts inventory row](../inventory.md#parts-adapter) remains reachable, including
      custom/model/module/assembly and controller return routes, not just catalog search.
- [ ] Draft/error survives A→B→A, panel/workspace round trip; retired/foreign selection cannot
      receive a late import/generator/model/placement result.
- [ ] No preview/body remount from opening a panel; browsing cannot create an accepted edit.

## Proof map

- Catalog navigation — query/category/preset/orientation fixture with error/empty variants →
  browse/select/reopen Objects → correct identity/query/preview, no placement or Inspect auto-open · mounted · planned
- Authoring continuity — selected custom/assembly/module fixture → explicitly open retained
  task/editor, change generator/model/assembly or variant/attachment through existing action →
  correct accepted definition/result and existing Undo where supported; file-picker/import
  and errors still reachable · mounted · planned
- Draft lifetime — unresolved generator/custom draft A before selecting B → switch panels/
  workspace then return A → exact draft/error restored only as A, no visibility-induced edit · mounted · planned
- Hidden registered owner — unresolved registered assembly/generator/module draft before hide →
  switch workspace and request Update Now → Parts/editor/remedy defers reload until existing
  submit/Cancel resolves that owner, no lost text or false blocker after settlement · mounted · planned
- Captured placement — controller browse from PCB or selected-key apply with response held →
  change selection/generation/owner then deliver → no foreign placement/draft write; current
  explicit place/apply still succeeds once and Back to PCB retains context · mounted · planned

## Checks and mutations

[Verification](../verification.md): `check.py quick`, `check.py lint`, `check.py browser --only parts`
and `run-wasm-tests.py --files web/src/presentation/parts_workspace.rs` for page wiring.
Reuse existing import/preview/placement fixtures and native accepted-state evidence. Named
selection-generation proof-gap mutation must fail no-foreign-write assertion; coordinator
independently reruns reject path. Any newly found live workflow blocks retirement until mapped.
