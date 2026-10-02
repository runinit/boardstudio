# F6K.2 private layer operations contract

**Status:** source-validated implementation contract only. No implementation or F6K.2 acceptance is claimed. The bounded F6K.1 Base/read-only-selection browser evidence is green; layered-fixture public QA is in progress, so wait for root's explicit gate record before starting F6K.2 source work. F6K.2 authoring starts after the bounded F6K.1 projection and stable-layer-ID selection behavior is proven. F3.1 remains an integrated acceptance join; it is not an F6K.2 start prerequisite.

## Bounded feature

Add, rename, and remove persisted Keymap layers from the existing Keymap Inspector. Use only the existing `EditKeymap` operation and ordinary `Event::Edit`/history/save path. Keep edits private to the page: no new application/core command, document field, generated contract, public facade, or configuration authority. Keep binding editing, macros, encoder controls, firmware export, Keycaps, and canvas behavior out of this slice.

The source boundary is split as follows:

- The private Keymap panel author may extend `web/src/presentation/keymap/panel.rs` and the small read projection in `view.rs`. The panel gets layer labels in document order, the currently resolved layer ID, the existing view-selection callback, and typed layer-operation callback(s). It may add local rename draft state and operation feedback rendering. It does not own `Runtime`, `SelectionAdapter`, accepted-document authority, layer-ID allocation, or `EditCommand` construction.
- The root coordinator owns `web/src/presentation.rs`, including callbacks and state tied to `Runtime`, full `Scope`, accepted `SnapshotToken`, adapter generation, root Editor lifetime, edit IDs, command construction, and callback-time validation. Existing root callback logic must be reused; do not add a parallel Session/Edit path in the panel.
- Existing `Runtime::submit(Event::Edit { .. })`, `Runtime::operation`, and the page-private operation-outcome observer are sufficient. No `Runtime`, Session, core, CSS, or public API changes are authorized by this contract.

## Pinned React behavior

`app/src/ui/KeymapPanel.tsx:16-24` is the direct behavior oracle:

- Add is disabled at 32 layers. Clicking Add creates an ID, submits `AddLayer` named exactly `Layer {map.layers.length}`, then selects that stable ID immediately.
- The layer-name input is native/uncontrolled, has `maxLength={32}`, is keyed by `${layer.id}:${layer.name}`, and submits `RenameLayer` on blur only when the text differs from the accepted name. Base is not special for rename.
- Remove is rendered only when the resolved active layer ID differs from `map.layers[0].id`; therefore the first persisted layer is protected regardless of ID or displayed name. Removing another selected layer does not explicitly rewrite the stored layer ID.
- The active layer is resolved by stable ID and falls back to the first layer if the saved/current ID is absent (`createKeymapWorkspace.tsx:66-67`). Therefore after removing the active non-base layer, the first layer becomes visibly active through fallback while the stale stable ID remains as view state. If Undo restores that same ID, React resolves to it again. Preserve this behavior; do not replace it with index-based selection or eagerly rewrite the ID to Base.
- Add selects its newly generated ID before the asynchronous edit has been accepted. During that short interval the layer projection falls back to the first layer. If the edit is rejected, the local layer ID still has no match and display continues to use the first-layer fallback.

`createKeymapWorkspace.tsx:66-67, 82-83` creates a virtual Base map for absent/empty layer data and routes these edits through the existing `emit({ kind: 'edit-keymap', change }, [boardId])`. `Workbench.tsx:1003-1006` keeps `keymapLayerId` in workspace-level view state; do not reset it just because the Keymap panel unmounts. The React callback is `onChange`, which does not perform a second write after the edit.

For rename failure, React does not control the input value. The `key` remains the same while the accepted layer name remains unchanged, so a rejected rename leaves the attempted text in the input. `createProjectActions.ts:40-44`/`useProjectSession.ts:55-58,105-109` surface core errors through the existing app-level alert. Dioxus should preserve the attempted draft on rejection and display the existing core error through the page's existing status/feedback mechanism; it must not silently replace the draft or normalize it. The input resets from accepted data when its layer ID/name key changes, and when the user changes selected layer.

## Core and command contract

Use existing Rust `boardstudio_core::model::KeymapChange::{AddLayer, RenameLayer, RemoveLayer}` and `EditOperation::EditKeymap { change }` (`core/src/model.rs:1159-1160`; `core/src/keymap.rs:106-130`). Commit through:

```rust
runtime.submit(Event::Edit {
    operation_id,
    command: EditCommand {
        base_revision: captured_snapshot.document.revision,
        transaction_id: unique_transaction_id,
        phase: EditPhase::Commit,
        target_ids: vec![captured_scope.board_id.clone()],
        operation: EditOperation::EditKeymap { change },
    },
});
```

The target list matches React's `[boardId]` metadata. Core's `apply_edit` returns the document ID as its modified target for all three operations. The operation ID/transaction ID are for Session/history identity; neither is a layer ID.

Core behavior in `core/src/keymap.rs:163-175,235-243,302-307,374-396` is authoritative:

- A missing saved map starts from `KeymapConfiguration::default()` (one `base` / `Base` layer) before Add or Rename. Thus the virtual Base on a legacy document with `keymap == None` is renameable by its stable `base` ID; the explicit rename causes core's existing `unwrap_or_default()` path to persist a default map with the renamed first layer. Viewing the synthetic Base alone remains read-only. The projector defensively treats an empty saved layer array as virtual Base, while core validation forbids persisting a zero-layer map; if an accepted `Some(empty)` map is nevertheless encountered, Add would append without a Base and Rename(`base`) would fail `Unknown layer`. Report that condition as a source/fixture discrepancy rather than repairing it in the page.
- The map must have 1–32 layers. Layer IDs must be unique, 1–64 bytes, and ASCII alphanumeric/underscore/hyphen. A layer name must have non-whitespace content, at most 32 UTF-8 bytes, and consist of printable ASCII excluding `"` and `\`, plus spaces. Keep the input's 32-character browser limit, but pass the exact value to core: do not trim, case-fold, or otherwise “repair” it before validation. Core error text is `Keymap needs 1–32 layers` or `Layers need unique ids and valid names of 1–32 characters`.
- Rename looks up by stable ID and accepts the first layer. Unknown IDs yield `Unknown layer`.
- Remove rejects the first layer with `The base layer cannot be removed`; otherwise it removes by ID. Root still checks that the target exists and is not index zero before enqueueing, because removing an unknown ID is otherwise a no-op followed by a successful validation.
- Final `validate(&map)` preserves all bindings, sensors, macros, order, and untouched layer fields, and only then stores `doc.keymap = Some(map)`. Do not reconstruct/rewrite the whole configuration in presentation code.
- Normal committed edits remain individually undoable/redoable and save/reopen through Session. Do not coalesce unrelated layer operations into a new command or transaction protocol.

## Private panel seam

Suggested private-only interface in `presentation/keymap/panel.rs`:

```rust
#[derive(Clone, PartialEq)]
pub(super) enum KeymapLayerOperation {
    Add,
    Rename { layer_id: String, name: String },
    Remove { layer_id: String },
}
```

Add a private `on_layer_operation: EventHandler<KeymapLayerOperation>` prop and, if needed for actionable errors, a read-only private feedback prop keyed to the root Editor mount and request ID. Keep existing `on_layer(String)` browsing callback separate. The panel determines the resolved selected layer by `active_layer_id` lookup with first-row fallback, disables Add when `view.layers.len() >= 32`, shows Rename for the resolved layer including index zero, and shows Remove only for a resolved layer whose stable ID differs from the first row's ID. Emit only on blur and only when the raw draft differs from the accepted name. Do not submit on every keystroke.

Keep a raw draft keyed by `(Scope, active_layer_id, accepted_layer_name)` (or equivalent keyed child state). Scope/layer changes reset the draft; ordinary renders and an error do not. On successful accepted rename, new accepted name causes the draft to reset. Escape should follow existing native input behavior unless a later source-backed reference explicitly requires another contract; F6K.2 only specifies blur and React does not install a custom Escape handler here. Preserve native keyboard, focus, and max-length behavior.

## Root callback/freshness design

The root callback captures the full render `Scope`, accepted `SnapshotToken`, accepted revision, current adapter generation, and the current Keymap workspace. On every callback, before allocating or submitting, re-read the live Runtime model and require:

1. Keymap remains active and `runtime.scope() == captured_scope`, comparing session epoch, document ID, board ID, and optional instance ID.
2. `SelectionAdapter` generation is unchanged; the accepted snapshot still has the captured token, document ID, and session epoch; active board/instance still match the captured scope.
3. The requested operation is valid against that fresh accepted keymap: Add has fewer than 32 effective layers; Rename target ID exists (first layer is allowed, including virtual `base` when `document.keymap` is absent); Remove target exists and is not first.

The child callback carries only `KeymapLayerOperation`; this captured root closure supplies freshness, so the panel cannot forge a scope/token. Keep a root Editor-instance ID and a non-reused monotonically increasing request counter for asynchronous feedback correlation; the panel may be conditionally unmounted, so a counter local to `KeymapPanel` can be reused and is insufficient. Store full pending identity `(editor_instance_id, request_id, Scope, SnapshotToken, revision, operation_id)` and drop feedback whose identity no longer matches the live editor/scope. Observe an operation **before** `Runtime::submit`, then display pending/completed/core failure through existing private feedback/status. Never use a global error string as a freshness guard.

Use synchronous single-flight admission: admit one Keymap layer operation only when the fresh model is `Lifecycle::Ready`, has no `display_preview`, has no active gesture, has `Durability::Saved { revision: accepted.document.revision }`, and this Editor has no unsettled layer-operation request. Recompute that full gate inside the root callback; disabled controls are presentation only. Record the request/outcome slot before submitting so Runtime's synchronous notify cannot race ahead of tracking. While pending, disable all three layer-operation controls and retain the rename draft. This policy is a narrow page safety gate, not React parity; paired public acceptance must confirm it does not strand edits and that the user can retry after the gate returns. Do not queue a stale child request for later.

All terminal outcomes clear the single-flight slot exactly once. `Completed` acknowledges success only when the same root Editor instance, full captured Scope, and operation ID still match and the current accepted map reflects the requested layer result. The accepted `SnapshotToken` is expected to advance after success; do **not** discard feedback merely because the old admission token is no longer current. A rejection, persistence failure, recovery block, executor failure, superseded/cancelled operation, or closed session clears pending and displays its existing reason/state; it never auto-replays. Keep a failed rename draft until its layer/name/scope key changes or the user edits it, so a fresh blur is an explicit retry with a new request ID. Repeated identical rejections must receive distinct request identities and must not leave the editor permanently busy.

If Keymap unmounts while an operation is outstanding, keep the operation tracker owned by the still-mounted Editor until its terminal outcome so it is not orphaned. Suppress feedback while another workspace is active; clear stale feedback on a full Scope/editor-instance change. When the Keymap panel is remounted, the draft follows the React key identity (the child-local draft resets), but an outstanding root request remains single-flight until it settles. If the scope changes before admission, reject the callback; if it changes after the Event is submitted, do not replay or re-target that already-owned Session operation, and do not expose its feedback in the new scope. Once its terminal outcome clears, a user action in the new scope is a fresh request.

For Add, allocate the entity ID in root from a fresh `runtime.operation().0` with a private prefix (for example `layer-{n}`), checking against current accepted layer IDs and retrying on collision. The prefix plus decimal operation counter meets core's identifier grammar and is not a bare operation number. Keep the generated ID distinct from the separately allocated Session operation ID and transaction ID. Use the accepted effective layer count for the exact `Layer {count}` default. After the callback's freshness/policy checks, set the root-owned `keymap_layer_id` to the allocated stable ID (matching React's immediate `onLayer(id)`), then submit `AddLayer` with the new ID/name. A rejected add retains the view ID and falls back visually to the first accepted layer, as React does.

For Rename, submit exactly `RenameLayer { id, name }` after re-reading that current accepted layer. For Remove, submit exactly `RemoveLayer { id }`; keep root's `keymap_layer_id` unchanged so the view projection fallback selects the first remaining row and Undo can resolve the old ID again. The UI's hidden Remove control is not the validation boundary: the root callback and core both protect index zero.

Use a separate fresh `OperationId` for the `Event::Edit` and a unique transaction ID, observe that Session operation before submit using the already-private runtime observer, and only surface the terminal result while its full request identity still matches. The existing `Event::Edit` path is non-strict; submit the captured revision, but do not assume a stale captured revision itself is a scope guard. Root freshness checks are mandatory even though Session may rebase queued edits. On a core rejection, show the existing reason, leave the accepted map authoritative, preserve raw rename text for correction, and permit a fresh retry after the user changes/commits again. No automatic replay after scope change, rejection, or storage failure.

## Acceptance and handoff joins

Do not begin source implementation until root records the bounded F6K.1 public gate: read-only projection and stable-layer-ID browsing/selection through its accepted-snapshot path. The parent F3.1 shared-selection acceptance is still required for integrated acceptance, but it is not a start block for F6K.2. Then independently review the implementation and verify through public Dioxus behavior; React source alone is not acceptance.

Required paired scenarios:

- Legacy/no-saved-map virtual Base: Add creates `Layer 1`, selects it, and preserves/undoes/redoes the legacy first-layer bindings. A saved first layer with a non-`base` ID and renamed name still remains the protected/removable-policy Base by index.
- Rename Base and a later layer by blur; no write occurs before blur, same accepted value is a no-op, accepted rename remains selected by stable ID, and failed invalid names leave draft visible with core feedback. Exercise valid 32-byte and invalid blank, over-byte-limit, quote/backslash, control, and non-ASCII names; verify the native maxlength and core validation boundary separately.
- Add up to 32; Add is disabled at 32 and a stale/racing 33rd callback cannot corrupt the accepted map. Remove non-base, preserve all remaining bindings/sensors/order, show the first layer as fallback, Undo restores and reselects the removed stable ID, Redo removes it again. Publicly verify the UI omits protected removal; independently review the root first-index callback guard and verify core's `The base layer cannot be removed` policy with the existing focused core test or equivalent direct test.
- Scope/token/generation changes between render and callback—including same-ID reopen and optional instance change—drop the stale request. No layer operation may cross a board/document/session boundary or write from a stale panel.
- Each accepted operation has normal Undo/Redo and save/reopen behavior. Compare exact accepted map/layer IDs/order/names/bindings/sensors and verify rejected edits do not mutate revision/history/storage.
- Public desktop/compact, light/dark, keyboard/focus, and axe checks for layer list, name input, Add/Remove disabled state, error and pending feedback. First-layer protected state must remain perceivable to keyboard and assistive technology.

No dedicated React `KeymapPanel` layer-operation test file was found in the targeted `app/src/ui` search during source validation; use the real React UI as the behavioral oracle and add no claim of test coverage until located or created by the authorized feature author. Keep inherited page compiler/build/browser gates and the F3.1/parent joins open until evidence is recorded.

## Source validation and RF

Validated against the current checkout's issue 02/dispatch, existing F6K.1 private contract and reviewer clearance, pinned React `KeymapPanel.tsx`, `createKeymapWorkspace.tsx`, `Workbench.tsx`, React edit/scheduler path, Rust `KeymapChange`/`apply_edit`/validation, and the current private Dioxus `KeymapPanel`/`KeymapView`/`Runtime`/`EditCommand` seams. Key source references are included inline above. The current VIK public read-only projection check exercises the exact layered Keymap structure, but the importer's `properties.variant` delta is underclassified; do not treat that fixture as broad import/round-trip Keymap acceptance. Base/read-only-selection evidence and the layered-fixture verifier remain separately scoped as stated above. No Context7 lookup was needed: relevant runtime/core API behavior is present in the installed repository source. No compiler, build, or browser test was run for this contract.

**RF:** No new refactoring takeaway observed in this design pass. Preserve/reconcile the existing RF-009 parity and source-accounting observations when the implementation/public evidence is handed off; this contract closes no RF or acceptance item.
