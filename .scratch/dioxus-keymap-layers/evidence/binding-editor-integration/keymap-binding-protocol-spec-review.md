# Binding protocol Spec review

Source-clear at the reviewed private module boundary; root integration and runtime acceptance remain open. Reviewed worker HEAD `b27f3176` plus released dirty files:

- `binding_editor.rs`: `49cd76a7ca38807ce8ec8ed14d1027843d946f55eca5381a937cbc2a58c9f149`
- `binding_controller.rs`: `bd3d3cb336c0f542c4f5019e4282f3ada1f57686be93c95c5aee298da8944a8f`
- private contract: `4509483a763416c2e4d1b01d6314291a389764ec76fcaf3426135940ae93e2e5`

No remaining material Spec finding in this delta. The controller now separates accepted display projection from Ready/Saved edit admission, preserving the editor and Pending feedback through Busy/Unsaved and accepted-prop advancement. Draft identity excludes token/revision; submitted operations retain fresh full Scope, token/revision, generation, selected-key membership, effective layer, editor identity and field checks. Field-specific changes merge into freshly accepted binding values without overwriting sibling fields.

The controller-owned monotonic request sequence survives child remounts. Feedback authority is the exact admitted request plus observed operation outcome. An ignored later blur may allocate an ID but cannot replace or hide the already admitted Pending feedback. Restoring the old latest-locally-allocated filter would reintroduce the defect. Failed text reverted to its accepted value hides the obsolete code-field failure without submitting a fake no-op. Actual failure relevance and successful settlement are checked against the affected field and accepted state.

The eleven behavior defaults, typed-first/base legacy fallback, stable layer/macro IDs, trimmed changed blur, and no bespoke keycode parser agree with issue03 and pinned React `5a472a9426e6e38993361da402cd4ec730feb369`. The author applied the reported Option-return correction (`then_some(state)`); this reviewer did not modify source.

Root must still mount the hook unconditionally for the Editor lifetime, pass its current accepted KeymapView/source, wire the shared sequence, and preserve the documented child draft key. Compiler checks, native operation/feedback regression proofs, and actual public Pending/failure/retry/Undo/scope/remount behavior remain required. No Cargo or browser acceptance of BindingEditor was performed here; full parent acceptance is not implied.
