# Macro root mount — Standards review

Reviewed actual three-file integration delta against `96874be6b5187eace0fbd65e8ba20cc76938133c`, matching corrected patch SHA-256 `4400a4a31810168e7ef031fa74a4ec8c001ed31886787b2764aa8e4f1faf8556`.

Final source hashes:

- presentation.rs: `0cc0f32121d9336d6d9d0528e708b9de41949e89af95c0e791c3a9b8c150525e`
- keymap.rs: `3a8bd893e4b456ee55139dc309576adde648f43456336b444b92f6457f8eab38`
- keymap/panel.rs: `6f3a39a0b0551042984e9e37297b03c6ec8b762e4d5edc9f93600c143ba274e8`

No remaining material Standards finding in this mount delta. The macro hook is unconditional in Editor before scope/snapshot early returns and workspace branching. It receives actual accepted LayerSource, live workspace/generation signals and the existing physical-instance admission callback. Its Editor sequence/outcome state therefore survives panel hiding. MacroEditor uses full Scope/editor keying and shared source/sequence handles, with token/revision excluded from its outer key.

The corrected shared children slot stays inside KeymapPanel but outside the selected-key conditional. Macros remain available with no selected key or no matching search results. BindingEditor retains its existing projection/selected-key guard, props, callback and stable draft key. Both editors remain in the existing styled/scrolling panel and inherit its theme, focus and compact 44px controls. This clears the earlier provisional outside-panel placement finding.

Only private module declarations/reexports are added; no public API or provider visibility is widened. No new mutable domain owner or document clone is introduced by wiring. Existing RF009/private composition observations remain; no new structural refactoring takeaway established.

Patch applicability and source whitespace checks passed. Root compiler run is separate and not claimed here. Public no-selection/search/Binding preservation, macro failure/retry, workspace/scope lifecycle, Undo/Redo and persistence verification remain required. No source edits, Cargo or browser work performed.
