# F6K.4a private binding-target contract

**Purpose:** Dispatch the encoder-rotation slice from `drafts/06-encoder-rotation-editor.md` without inventing physical key IDs, duplicating KeyBinding controls, or racing the current Keymap owner. This is a private page contract only. It changes no canonical task edges, Core/public types, schema, persistence, or operation family.

**Pinned React source:** `app/src/ui/KeymapPanel.tsx` at `5a472a9426e6e38993361da402cd4ec730feb369`, with selected-board input production in `app/src/ui/createKeymapWorkspace.tsx` and `app/src/main.tsx`. **Current Rust seams:** `web/src/presentation/keymap/binding_editor.rs`, `binding_controller.rs`, `view.rs`, `panel.rs`, and the Keymap mount in `web/src/presentation.rs`. See `.scratch/dioxus-keymap-layers/evidence/binding-editor/private-contract.md` and `evidence/encoder-firmware-ticket-split/source-contract.md` for the accepted behavior and broader ticket ownership.

## Private target and request identity

Extend the existing private binding-editor request target with a discriminant instead of overloading `key_id`:

```rust
enum BindingTarget {
    Key { key_id: String },
    EncoderRotation {
        encoder_id: String,
        direction: boardstudio_core::model::EncoderDirection,
    },
    EncoderPush { encoder_id: String, key_id: String },
}
```

The enum is `Clone + Debug + Eq + PartialEq` and remains scoped to `crate::presentation`. `Key` preserves all current selected-switch behavior. `EncoderRotation` carries the stable encoder ID and exact direction. `EncoderPush` carries both the stable encoder ID and the exact reported push-key ID so the controller can prove that the key belongs to that encoder in the current accepted input projection. Push still commits through `KeymapChange::Binding`; it is never represented as a rotation or a fabricated ordinary key.

Give every encoder-input projection a mandatory opaque private identity, even when no F5 plan exists:

```rust
struct EncoderInputIdentity {
    scope: Scope,
    token: SnapshotToken,
    revision: u64,
    projection_generation: u64,
    electrical_fingerprint: Option<String>,
}
```

The root allocates a monotonic, non-reused `projection_generation` for each changed visible input lineage and memoizes it while that lineage is current. Advance the generation when encoder membership, push-key association, or the F5 electrical-plan fingerprint changes; do not advance it for a Keymap binding save that leaves encoder input lineage unchanged. The module-only case still has full Scope/token/revision and its own generation; it does not invent an F5 fingerprint. A present F5 fingerprint is an additional freshness guard. Encoder targets carry `Some(EncoderInputIdentity)` in request and feedback; `Key` carries none. Exact equality of the complete identity with the current projection is an **admission** check. Never treat the optional fingerprint as the projection identity itself.

Replace the `key_id` target field in private `BindingEditRequest`, `BindingEditFeedback`, and `BindingEditorProps`/`EditContext` with this target (the `Key` branch preserves the old contract). Keep the current captured full `Scope`, admission `SnapshotToken` and document revision, effective stable layer ID, `BindingField`, editor-lifetime ID, monotonic request ID, and complete requested `KeyBinding`. Encoder targets require a captured `EncoderInputIdentity`; that identity always includes scope/token/revision/projection generation, while its F5 electrical fingerprint may be absent. `None` input identity is permitted only for ordinary `Key` targets. Feedback copies the full target and captured identities. This is internal event data only and is not serialized or added to a Core request type.

## Immutable accepted projections

Keep the current `BindingEditorProjection` narrow: effective layer ID, label, one typed binding, and shared `Rc` layer/macro choices. Add a separate immutable encoder projection, memoized by the accepted `LayerSource` identity, active/effective layer ID, and the selected-board input-projection identity. Suggested private values:

```rust
struct EncoderInputProjection {
    identity: EncoderInputIdentity,
    encoders: Rc<[EncoderInputChoice]>,
}
struct EncoderInputChoice {
    id: Rc<str>,
    label: Rc<str>,
    push_key_id: Option<Rc<str>>,
}
struct EncoderBindingRow {
    id: Rc<str>,
    label: Rc<str>,
    clockwise: KeyBinding,
    counterclockwise: KeyBinding,
    push_key_id: Option<Rc<str>>,
    push_binding: Option<KeyBinding>,
}
struct EncoderEditorProjection {
    source: LayerSource,
    input_identity: EncoderInputIdentity,
    effective_layer_id: String,
    rows: Rc<[EncoderBindingRow]>,
    layers: Rc<[BindingLayerChoice]>,
    macros: Rc<[BindingMacroChoice]>,
}
```

The exact type names can follow existing naming, but keep these ownership and content limits. The root supplies the immutable visible encoder projection from the accepted selected-board F5 handoff plus attached-module rows that React adds locally. Its mandatory `EncoderInputIdentity` carries full `Scope` and accepted token/revision plus the memoized projection generation; when an F5 plan is present its fingerprint is an additional guard. It contains only IDs, labels and nullable reported push IDs; no `ProjectDoc`, complete `ElectricalPlan`, cloned Keymap, or writable state crosses into the component. F5 encoder rows may be absent while accepted attached-module rows remain visible, matching React. A push editor is present only when a row has `Some(push_key_id)`; no `/push` fallback or inference is permitted.

Build visible rows with the current React source rule: preserve F5 encoder order and records, then append attached modules for this host board whose `hostBoardId` matches, are not detached, whose definition has catalogue row `ec11-evqwgd001`, and has an electrical rotary profile; use the stable module ID and definition name with no push ID. Suppress an appended row only when the F5 encoder list already contains that ID. Do not filter visible rows through `electrical_peripherals::describe` or predict whether Core will accept a rotation.

**Visible does not guarantee writable:** React can display an attached-module encoder even when Core's current `KeymapChange::Encoder` validator cannot resolve that module ID through `electrical_peripherals::describe`. Preserve that row and submit the existing typed edit. If Core rejects it, show the correlated Core failure under that encoder and direction; leave the accepted binding unchanged. Do not silently hide the row, synthesize a physical part, pre-authorize it, or change Core/domain behavior in this UI slice. Do not claim every visible module row can save until the existing Core acceptance path supports it; that is a separate evidence/ownership decision.

For each current effective layer, derive clockwise/counterclockwise from `KeymapLayer.sensors[encoder_id]`, with absent sensor entry showing `KeyBinding::None`. Derive push binding using existing key-binding projection semantics for the actual reported key ID, including existing Base legacy fallback where applicable; non-Base absence remains transparent. Reuse the already-projected `Rc` choices for layers/macros. Project only visible rows and their available bindings; do not clone the map, document, or all per-key state on render. Keep source and row handles immutable and memoized; root/compiler review owns actual allocation checks.

## Shared controls and operation controller

`BindingEditor` remains the single writable field component for ordinary key bindings, both encoder directions, and a reported push key. Encoder rows mount it once for each direction with `BindingTarget::EncoderRotation`; an optional push mounts it with `BindingTarget::EncoderPush`. Each instance receives the row label, one current `KeyBinding`, existing `BindingField` choices, shared `Rc` layer/macro choices, `BindingActions`' editor-lifetime ID and monotonic request-sequence signal, current enabled state, exact correlated feedback, and existing `on_change` event handler. No encoder-specific copy of behavior/keycode/tap/modifier/layer/macro controls is introduced.

These simultaneous editors share an editor-lifetime ID, so that ID alone is not a unique control identity. Include the full typed target in the root Dioxus component key and keycode-draft child key, including encoder ID and direction for rotations and both encoder/key IDs for push. Keep draft keys independent of token/revision so an unrelated accepted update does not discard a dirty rejected draft. The current shared datalist ID uses only the editor ID and would collide across these controls; include a deterministic, collision-free encoding of the complete `BindingTarget` in each datalist ID (variant tag plus length-delimited or hex-encoded ID bytes and direction). Preserve a distinct ID for the same key rendered in multiple contexts. The root/editor request sequence remains shared and monotonic; target-qualified DOM/draft IDs do not allocate request identities.

Extend the existing `binding_controller` target projection and callback rather than adding a second writable controller or operation observer. Keep controller admission separate from the ordinary selected-key projection: `BindingActions.enabled` represents current scope/source/Ready-Saved admission and the single-flight state, not `projection.is_some()`. The encoder projection is resolved independently, so an encoder-only view with no selected canvas key can still be enabled. The ordinary key editor remains absent unless its selected-key projection exists.

- For `Key`, preserve current selected-key membership, legacy Base fallback, target checks, field merge, and `KeymapChange::Binding` behavior.
- For `EncoderRotation`, fresh-read the current accepted effective layer sensor field, default to `None`, validate the exact `(encoder_id, direction)` against the current visible encoder projection and its captured accepted input identity, then submit existing `EditCommand::EditKeymap { change: KeymapChange::Encoder { layer_id, encoder_id, direction, binding } }`. Visible membership is not a Core-eligibility prediction; preserve an exact terminal Core rejection as correlated feedback.
- For `EncoderPush`, verify both encoder membership and exact `push_key_id` match in the fresh current input projection; then fresh-read that real key's active-layer binding and submit existing `KeymapChange::Binding` with that ID. Do not require the push key to be the current canvas-selected key.
- Every request rechecks full `Scope` (including session epoch and optional physical-instance identity), current `Runtime.scope`, scope generation, current workspace/root admission closure, current accepted token and revision, Ready/Saved/no-preview/no-gesture admission, active/effective layer, target membership, exact mandatory `EncoderInputIdentity` for encoder targets, optional F5 fingerprint, editor lifetime and monotonically increasing request ID. It rejects stale or mismatched requests without altering current state.
- Preserve existing field-level merge: apply only the requested `BindingField` to the newly read current target binding. This prevents an old complete binding value from overwriting a sibling field. No-op values submit nothing.
- Keep one editor-lifetime pending slot/single-flight sequence and the existing `Runtime.observe_operation` terminal observer. Store the exact `OperationId`/`OutcomeSlot` with Pending and keep the original full admission identity on Pending/feedback. Once submitted, do not require full input identity equality to settle it: a successful own edit advances accepted token/revision. A terminal settles/releases only the exact pending operation under stable full Scope, scope generation, editor lifetime, and target/request identity. At terminal time re-read the newer accepted snapshot and current input projection, then validate current target membership and compare the requested field for acknowledgement using the existing binding-controller relevance rules. A binding-only token/revision advance does not change `projection_generation`, so matching Pending remains visible while persistence completes. If a genuine input-lineage change made the target stale, suppress its stale display feedback but still release the matching terminal slot. Never replay the request. A terminal for a different operation or owner cannot settle the slot.
- Feedback projection is target-aware. Ordinary-key feedback keeps the current selected-key test; encoder rotation/push feedback is filtered by current Scope/layer/editor/request and matching target plus stable `projection_generation`, never by `selected_part_ids` or exact old token/revision equality. Pending remains tied to the exact `OperationId` in the root controller. The BindingEditor compares its complete current target to feedback target. Thus an editor with no selected canvas key can receive Pending/Saved/Failed for its encoder control.
- Existing stale-feedback relevance/retry rules remain unchanged for `Key` and apply by target/field to rotations and push.

## Root mount callback and ownership

Call the binding hook unconditionally in `Editor` as today, preserving its lifetime outside the conditional panel. Add the current F5-plus-module `EncoderInputProjection` to the hook/source seam and expose the memoized `EncoderEditorProjection` through `BindingActions` (or an equivalent private action value); keep the request handler and editor sequence shared with ordinary keys. The root owns the mount in `presentation.rs` and any integration-only `keymap.rs` export. The Keymap panel mount owner adds the Encoders section/panel as a child beside current binding/macro content and passes the shared projection, active layer, source-level `enabled`, target-aware feedback lookup, and `on_change`. The child is display-only except for sending typed `BindingEditRequest` events to the same controller. Keep `panel.rs`/`presentation.rs`/global CSS with the coordinator; do not edit those files during the isolated component author turn.

**File ownership for one collision-free author turn:**

- Encoder author: new `web/src/presentation/keymap/encoder_editor.rs` plus any new encoder-only projection helper module; no root mount files.
- Shared binding component owner, with explicit handoff: `binding_editor.rs` target/request/feedback/props changes.
- Coordinator/controller owner, with explicit handoff: `binding_controller.rs`, `view.rs` only if the projection seam is placed there, `keymap.rs` exports, `panel.rs`, `presentation.rs`, and CSS.
- F5 owner: the accepted selected-board encoder/push projection and Wiring/electrical source. Its private value should be adapted to `EncoderInputProjection` by the root; root also adds attached-module rows from the current accepted document using the pinned React rule above. The encoder author does not edit F5 files or synthesize fixture hardware in production.

Do not dispatch the encoder UI author against `binding_editor.rs` or `binding_controller.rs` while their owners are editing them. The private target types and exact request/event identity in this document are the interface to settle first. A single coordinated vertical integration turn may intentionally combine the shared component/controller changes, but its file ownership must be assigned to one writer.

## Required evidence before acceptance

Fixture-backed checks cover both rotation directions, absent sensor `None`, real and absent push IDs, every existing binding field and its validation/recovery, unrelated field preservation, encoder save/feedback when no canvas key is selected, unique datalist IDs and target-qualified draft remount keys for simultaneous directions, keyboard/focus, one-step Undo/Redo, saved payload/reload, and stale requests across layer, board, physical instance, session/document, accepted token/revision, projection generation and changed electrical fingerprint. Pair against the real F5.2 supported input projection before claiming selected-board hardware parity. Firmware generated-output checks belong to the separate F6K.4c/F8.2 acceptance; this editor slice does not require or implement an artifact provider. Keep F6K.4, F5.2, F8.2 and F6.6 open.

**Checks:** author-local rustfmt for owned Rust and diff checks; root owns serial Cargo/WASM compilation and public builds. Use paired React behavior as the oracle, not as Dioxus verification. Report fixture evidence separately from F5.2/F8.2 evidence.

**Refactoring handoff:** Preserve RF-009 source accounting/parity reconciliation. This seam planning identified two source-backed boundaries to reconcile after port: React-visible attached-module encoders may be rejected by the current Core rotation validator, and existing legacy `SetKeyBinding` updates the board map and synchronizes a supported value into the first typed layer when a Keymap exists. Neither rule changes in this private encoder slice; the latter is not a promise that legacy edits leave typed Base values untouched.

## Proposed next-frontier entry

Add a bounded Keymap row under the next encoder frontier (planning only; do not modify the canonical task graph):

> **F6K.4a encoder rotations and reported push:** dispatch a Luna High private component/controller author after the private `BindingTarget`/accepted input projection seam is approved. It may author the new encoder child module independently; the binding component target change, operation controller and root mount are coordinated serial ownership joins. F6K.1/F6K.2 are satisfied in the integrated candidate. Fixture implementation can proceed without firmware output; F5.2 is the actual supported-encoder/push acceptance join and F8.2 remains the output-delivery join. Do not add a sibling canonical dependency or claim whole F6K.4 closure.

> **Source note:** the visible encoder projection must preserve React's attached-module rows, even though current Core encoder-edit validation may reject a module ID. Preserve exact operation feedback and record the limitation; do not silently pre-filter. Legacy `SetKeyBinding` currently synchronizes supported values into typed Base when a Keymap exists, so do not claim legacy key-position edits are isolated from typed Base.
