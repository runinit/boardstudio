# Macro root mount Spec review

**Source clear for the bounded mount.** Reviewed the actual integration working-tree diff against `96874be6b5187eace0fbd65e8ba20cc76938133c`, matching corrected patch SHA-256 `4400a4a31810168e7ef031fa74a4ec8c001ed31886787b2764aa8e4f1faf8556`. No remaining material Spec finding in these three files.

Exact reviewed source SHA-256:

- `web/src/presentation.rs`: `0cc0f32121d9336d6d9d0528e708b9de41949e89af95c0e791c3a9b8c150525e`
- `web/src/presentation/keymap.rs`: `3a8bd893e4b456ee55139dc309576adde648f43456336b444b92f6457f8eab38`
- `web/src/presentation/keymap/panel.rs`: `6f3a39a0b0551042984e9e37297b03c6ec8b762e4d5edc9f93600c143ba274e8`

The approved contract requires `use_macro_operations` to run “unconditionally for the Editor lifetime, even when the Keymap panel is hidden.” Its call precedes the Scope/snapshot early returns and workspace branches. It receives the existing accepted LayerSource, live workspace and adapter generation, and the same fresh instance-selection guard as layer/binding operations. The shared request sequence and exact controller feedback are passed through unchanged.

The MacroEditor key contains full render Scope and the controller’s Editor identity, excluding token/revision. Its accepted read source and shared sequence stamps come directly from the controller. Missing/mismatched source gets an explicit unavailable status.

The corrected placement is inside KeymapPanel children, therefore inside the existing panel styling/scroll container. Moving that children slot outside the Selected key section makes macros available with no selected key, no matching search results or a zero-key board. The BindingEditor remains conditional on a valid binding projection and selected ID; its existing key, callbacks and feedback are unchanged. Search no longer controls either child’s placement. Private module registration/reexports stay within presentation visibility.

`git diff --check` passed for the reviewed files. No source edits or Cargo. Compiler-driven private-module deltas, actual responsive scrolling, accessible focus, macro operations/validation/history/persistence, workspace lifecycle, and provider output still require parent verification. This mount review does not close F6K.3 or its parent acceptance joins.
