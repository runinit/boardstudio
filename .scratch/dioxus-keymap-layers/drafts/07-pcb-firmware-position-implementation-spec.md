# F6K.4b private implementation spec — PCB firmware positions

**Status:** source-grounded private draft; not an issue/status update and not an implementation authorization by itself. Do not dispatch the mounted integration until the F5 selected-board projection and coordinator callback seam below are agreed. The legacy Core edit exists; the missing piece is its Dioxus composition/accepted-plan adapter, not a Core port. F5.2 and F8.2 remain acceptance joins, and F6K.4 remains open.

## Behavior and ownership

Port the existing PCB Wiring **Firmware keymap** control as one page-private presentation module. It receives an immutable, already-current selected-board projection from F5 and a narrow accepted-edit callback from the coordinator. It does not resolve electrical state, construct IDs, own document state/history, or change the typed Keymap editor. Its control behavior is: render plan-ordered IDs and labels, read the selected board's `hardware.boards[].key_bindings`, show absent entries as `&none`, count every choice except `&none` as assigned, offer the reference choices (including `&trans`), show the same value preview, and request one normal edit when a choice changes. Accepted document state remains the displayed value; a failed/stale edit must surface the existing operation error and leave the accepted value visible for retry.

**F6-owned files:** add `web/src/presentation/firmware_positions.rs` for the private projection view types and control, and `web/assets/firmware-keymap-panel.css` for feature-local styles (class names namespaced to this control). Keep the view module private to `presentation`; no contract/export changes. The page presentation root remains the mount and stylesheet-link owner, so F6 does not edit `web/src/presentation.rs` or `web/assets/m1.css`. If the asset pipeline cannot serve a feature-local stylesheet without those root edits, stop at that seam and ask the coordinator to mount/link the owned file; do not expand CSS ownership implicitly.

The private interface should be a small immutable value plus one callback, conceptually:

```text
FirmwarePositionView {
  state: Loading | Unavailable(reason) | Ready { board_id, plan_revision, keys, bindings },
  editable: bool,
  feedback: Option<operation-scoped error/status>
}
on_change(key_id, encoded_binding)
```

`keys` carries stable IDs and display labels only. `bindings` is the F5 selected-board map. F5 owns the conversion from the accepted electrical plan/document to those values and the loading/error/readiness state. The coordinator owns the selected-workspace mount and supplies `on_change` backed by the existing Runtime edit/history path. Avoid forwarding Core internals or passing a writable `ProjectDoc` through this module. This is intentionally one seam: a pair of F5/coordinator adapters must agree on freshness and error semantics before it becomes an integration contract.

## Projection and operation contract

- Matrix entries are the accepted plan's `assignments` in plan order, with labels `part.reference` or the stable key ID fallback. The current React producer uses `project.parts.find(part.id === assignment.keyId)?.reference ?? assignment.keyId`.
- Append only plan-reported auxiliary press entries in the same order as the plan's peripheral rows. Require the actual supported push role and a non-null real `pressKeyId`; preserve the reference's `encoder-push` / `input-push` GPIO-role recognition and its `S1` input-terminal case. Never synthesize `${partId}/push` in this Dioxus control. Keep any matrix/peripheral overlap behavior explicit in the F5 projection and paired fixture; do not silently deduplicate or invent an identifier without comparing the actual plan/reference case.
- Scope `Ready` to the accepted snapshot/document and current `Scope` (session epoch, document ID, selected board and physical instance), plus matching plan board/instance/revision. Render no stale plan as current. The control's callback carries the captured stable `board_id` and key ID, then the coordinator rechecks the current Runtime scope and current plan/document before dispatch; a late callback cannot retarget a new board/session/instance. F6 must not independently query Core or infer electrical eligibility.
- Dispatch the existing `EditOperation::SetKeyBinding { board_id, key_id, binding }` once per changed select through the normal accepted `Edit`/history path. Use a fresh operation/transaction identity, current base revision and target IDs `[board_id, key_id]`. Keep choice controls disabled during unavailable/loading or when the current edit admission guard is false. Error handling is operation-specific and truthful; do not optimistically treat a submitted select value as committed.
- Core stores the raw value in `hardware.boards[board_id].key_bindings[key_id]`. When `ProjectDoc.keymap` exists, Core additionally maps only valid `&kp CODE`, `&trans`, and `&none` values into the first persisted typed layer. This synchronization belongs exclusively to Core: do not duplicate it, mirror legacy values into other layers, or claim legacy edits are independent of typed layer zero. Values outside that supported mapping are rejected as “Use the structured Keymap editor for this behavior.”
- Preserve other board entries, unrelated key bindings, wiring assignments/locks/protected handoff, all other typed layers, and document data. Undo/Redo and save/reload are the existing document/session behavior, not local control state.

## Required checks for implementation

Use actual React-produced/retained projects and the real Core electrical plan/Core edit path. In native/application tests, assert key order/labels and reported-ID membership from an actual plan; `&none` default, `&trans`, choices/count/preview; one real `SetKeyBinding`; Core synchronization into typed layer zero when present and no typed-map creation when absent; validation/error; preservation of other board/wiring/typed-layer values; Undo/Redo and saved archive reload. In a paired public UI workflow, test plan loading/error/no-controller, stale plan/scope rejection, keyboard/focus/compact styling, matrix plus actual reported press ID, accepted value/error/retry, and that legacy editing does not affect typed layers beyond Core's first-layer compatibility mapping. Fixture-only tests do not satisfy the real F5.2 selected-board acceptance join. Firmware package output remains F8.2/F6K.4 evidence, outside this child.

## Dispatch gate and unresolved seams

The React side is source-complete, but the Dioxus integration side is not mounted: `web/src/presentation.rs` currently declares/mounts Keymap, Parts and Case inspector modules and has no PCB/Wiring inspector projection; F5 issue 05 explicitly says its Dioxus electrical workflow adapter and presentation are missing. Thus this draft resolves F6-local behavior/file ownership, but does **not** make issue 07 ready to implement against an integrated callback yet.

Before mount work, coordinator + F5 + F6 must settle and review:

1. The immutable plan/context type F5 supplies (accepted document identity/revision, board and optional instance, actual electrical plan, board key-binding map, loading/error/readiness/editability). Decide whether F5 hands over these existing Rust objects or a narrow value projection; no new public contract is needed.
2. The single callback owner's admission and outcome contract: who creates/registers the operation ID, where stale scope is rechecked, how the exact terminal Core/Session outcome and retryable error reach this control, and how controls become enabled after durability/operation settlement.
3. The React producer includes a fallback `${peripheral.partId}/push` when `pressKeyId` is absent. The published issue correctly forbids synthetic IDs and requires F5-reported IDs. Pair this deliberate constraint against real encoder/press fixtures; F5's current `PeripheralRequirement.press_key_id` is the candidate source of truth. Also determine overlap behavior where a real press ID is already a matrix assignment.
4. Style loading ownership for a feature-local CSS asset while the coordinator owns `presentation.rs` and global stylesheet composition.

Do not widen public visibility, add a Core command/schema field, copy the firmware generator, or edit F5/coordinator-owned files to bypass these questions. F6 may prepare its isolated private UI implementation after the data/callback interface is reviewed, but no mounted parity claim is allowed before F5's actual plan and Wiring composition are present.

## Source evidence and provenance

Inspection used direct source reads because the integration worktree has no `.codegraph/` index (`codegraph explore ...` reports that the project owner has not enabled one). No CodeGraph initialization, Cargo command, production source edit, or issue-status change was made. Pinned checkout HEAD: `e82c039b5486d934de96931238e65622ad0ffb1d`.

| Source | Evidence | SHA-256 at inspection |
| --- | --- | --- |
| `app/src/ui/FirmwareKeymapPanel.tsx` | React values/default/count/preview and choice callback | `bb3db7db1705364c28b3f98ae3fa4c491161e5210daf89112fc2d370a609936b` |
| `app/src/ui/keyBindingChoices.ts` | Exact legacy choice values and labels | `dae3361256f508006ac2d1361f037d9fba18ac82aa303f9f34b6561697af2db2` |
| `app/src/main.tsx` | Mount input order/labels and current `set-key-binding` callback; includes the React `/push` fallback called out above | `a71e3e64bf301737a3c1e701542889f784ad12a165b108a45414006877d2695d` |
| `app/src/useElectricalPlanning.ts` | Selected-board configuration, accepted electrical plan currentness and existing plan resolver owner | `9e4612a79feb4c3712b170a93190bed3f4e02df0d34a450d56d3909188ef1537` |
| `app/src/electricalPlanContext.ts` | Current plan keyed to exact accepted document object/revision/board | `115c58c296bd52d1707557f8c3becee2aa36372b9dcaf3cef3675a0ba2651325` |
| `app/src/ui/WiringPanel.tsx` / `usePcbWorkspace.tsx` | Existing `firmwareControls` composition slot and Wiring ownership | `96c2c3cd6b6d740b44e53e2aa8a1ac041bbd4937d8bcb2e832b7e44e98dce488` / `f66a97c030c2e747de454ab5ba1b520765b6d9259f09e556a78d026e4dd1bf23` |
| `core/src/model.rs` / `core/src/keycaps/edits.rs` | Existing `SetKeyBinding` operation and accepted mutation semantics | `e2ea75daa133195a25a877ef33e03e9a971f49ead3d70ef10a8f1d51dcb2efea` / `0e9873c4eeb3aa32d96569568539c5a38c08a422acb0ce1728a03147c9c0861c` |
| `core/src/electrical.rs` / `core/src/electrical_peripherals.rs` | Real plan shape/order and source-supported `press_key_id`/GPIO role projection | `51c733b17d38d58a4d473d880fecc80ab76b9ce9c395007c6c3aff46973c1404` / `2c1a15492d0ec9d02e2a86fffc2f94160d3d431062f1fdba5ece493954bf34eb` |
| `core/tests/keymap_edits.rs` / `core/tests/keymap_workflow.rs` | Existing board-membership/history/merge tests and legacy-to-first-typed-layer compatibility test | `5fc33da568130c0d6eb80589aa31bc804a6ec78d0aa7e90e801e0dacc99661fe` / `15500056cd47738d7e56b47834bd679b4f8518358b8fc2b1b718a6d2ffd10f7b` |
| `web/src/presentation.rs` | Root owns module declaration, selected workspace/Inspector mount, accepted Scope and Runtime callbacks; currently no PCB Wiring mount | `f52d368f790a7f644a969610b8a24d5c866d09585bc3ee0066c4eb75e87d59d5` |
| `web/src/presentation/keymap/binding_controller.rs` | Existing private accepted-edit/outcome pattern; do not duplicate it or reuse the typed binding controller for legacy edits | `e5e96a4b35e2b7590d8072b08822fec0e2a8f17440bcb810f67dfb9cb6c36898` |
| `.scratch/dioxus-keymap-layers/issues/07-pcb-firmware-position-editor.md` | Published child requirements and unchanged parent/acceptance joins | `d86689bf058b1fb8dd6db4bbda476569c7d759aa19ab69a4ec4a25e690ace857` |
| `.scratch/dioxus-frontend-v1/issues/05-pcb.md` | F5.2/F5.3 current scope and explicit missing Dioxus adapter/presentation | `9a379c54072e3a8932a318aa15baef2153616dab39bc939c6943115c9eacd3bd` |
| `.scratch/dioxus-frontend-v1/issues/06-keymap-keycaps.md` | F5-owned selected-board electrical/legacy key handoff; F6K.4 joins | `71934f02d27b68c3766e4e258d4a46d5e16e491eb16ac8fb66a35b3c9b48e6f6` |
| `.scratch/dioxus-keymap-layers/evidence/private-contract/keymap-private-contract.md` | Typed-layer/legacy separation and first-layer fallback rule | `c8277af420f45fb2ac94f9f5fb71f0a87cd6f3aaf29ab825a2e64325da3655ac` |

Relevant domain constraint: `docs/architecture.md` states firmware-position controls use the existing `set-key-binding` edit and Core merges field edits while preserving other bindings, pin locks, and protected handoffs. `docs/keycaps-and-keymap.md` documents typed Keymap authoring/export separately. No new RF finding was identified; retain RF-009 source-accounting/parity reconciliation from the published ticket.
