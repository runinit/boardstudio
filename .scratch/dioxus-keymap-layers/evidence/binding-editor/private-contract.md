# F6K.2 private binding editor contract

This contract defines the author-owned private component seam for the supported
binding editor. It is scoped to the new `web/src/presentation/keymap/binding_editor.rs`;
the Keymap panel mount, accepted projection, operation admission, history, and
shared styling remain coordinator-owned.

## Component inputs and edit request

`BindingEditor` receives one typed accepted `KeyBinding`, selected key label and
ID, active layer ID, full application `Scope`, accepted `SnapshotToken` and
revision, editor-lifetime ID, immutable `Rc<[BindingLayerChoice]>` and
`Rc<[BindingMacroChoice]>`, enabled state, optional correlated feedback, and an
`EventHandler<BindingEditRequest>`. It does not receive a document or writable
Keymap state.

Each edit request contains the captured full Scope, admission token/revision,
active layer ID, key ID, field identity (`Behavior`, `Keycode`, `Tap`,
`HoldModifier`, `Layer`, or `Macro`), editor-lifetime ID, a monotonically
increasing component request ID, and the complete typed `KeyBinding` value.
The component does not parse or validate ZMK expressions and has no Runtime or
Core access. The root callback must re-read current accepted state and validate
all captured identity before submitting the existing `EditKeymap` /
`KeymapChange::Binding` path. It correlates Pending/Saved/Failed feedback using
the same request identity and existing private operation outcome handling.

Feedback carries the same Scope, admission token/revision, target layer/key,
field, editor ID and request ID, plus Pending/Saved/Failed status. The view
requires exact target/editor/request correlation. Pending feedback additionally
requires the original accepted token/revision; Saved and Failed can outlive the
admission token only while the root controller still considers that feedback
relevant to the current accepted field.

## Draft and interaction behavior

Keycode drafts are owned by a small keyed child control. Its key includes
Scope, editor lifetime, active layer, selected key, field, and accepted field
value. A target or accepted field-value change remounts that control and drops
its draft; an unrelated token/revision advance with the same accepted value
keeps a dirty rejected draft. Each emitted edit still captures the latest
token/revision from current props, and the root callback revalidates that
admission. A rejected blur leaves the same draft editable while the root
supplies its validation message. Blur trims and emits only a changed keycode;
no Enter handler is added. Curated datalist values match the pinned React choices while
the text input remains free-form for Core-accepted expressions.

The behavior selector exposes Key press, Mod tap, Layer tap, Momentary layer,
Toggle layer, Go to layer, Sticky layer, Sticky key, Macro, Transparent, and
Unassigned. Defaults match React: A; LSHIFT/A; first non-Base layer (or the
first layer) with SPACE tap; first non-Base layer (or first layer) for layer
behaviors; first macro or empty ID; and distinct Transparent/None values.
Macro selection is disabled when no macros exist. Layer and macro controls
render names and submit stable IDs. Modifier, tap, keycode, and reference edits
preserve all other fields of the current typed binding.

The component uses `.m1-keymap-binding-editor`, `.m1-keymap-binding-field`,
`.m1-keymap-binding-pending`, `.m1-keymap-binding-saved`, and
`.m1-keymap-binding-error` class names. Shared/global CSS is root-owned.

## Evidence boundary

Behavior was transcribed from pinned React source
`5a472a9426e6e38993361da402cd4ec730feb369`: `KeyBindingEditor.tsx` and
`keyBindingChoices.ts`. The existing Core `KeyBinding` enum is the only binding
family used. No public API, schema, parser, history path, or writable duplicate
state is introduced.

Formatting and diff checks are author-local. No Cargo/compiler, native test,
WASM test, or browser verification is claimed here. The parent acceptance stays
open pending root integration, independent review, and the public/native checks
listed by F6K.2. RF-009 remains the applicable parity/source-accounting item;
this implementation pass identified no new refactoring finding.
