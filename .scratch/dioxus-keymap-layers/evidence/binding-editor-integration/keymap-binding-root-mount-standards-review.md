# Binding root mount Standards review

Reviewed the six-file source/CSS working-tree delta against integration HEAD `86a83ddfee611899695de0952c9996794dd12e41`. SHA-256 identities:

- presentation.rs: `19dfad60bc12ee64bcf2fbdc125b1ed1c8af628cdc94eca7a3c93efc2942b51f`
- keymap.rs: `64da4483797efbafc661aea6d8eb7e6309f5723d4701c36be287e943f9ea4955`
- keymap/panel.rs: `6c22e23fdd32016de29a69090acb4c37cfe10c69aad8f840c7c1e1d0c2ede5b8`
- keymap/binding_editor.rs: `3a32f90dc3b434fd9d5bbb76558c977aacacac7eeffcbe8b90232b3ddcccbf86`
- keymap/binding_controller.rs: `127d8305e68630a6c2dbde17e99ecfec13835b57164bbff77e9b16034a3a3d6b`
- m1.css: `fd743eb98d48a13535d7891f3401155389c85240a99a34785867711f8445a9a4`

No material Standards finding. The binding hook runs unconditionally in Editor before scope/snapshot returns and workspace branching. It receives the actual optional full Scope/token/revision source, accepted Rc Keymap projection, stable active-layer signal, workspace, generation, and live physical-instance admission callback. Its monotonic request sequence and exact outcome observer therefore outlive the visible child. The child key includes full Scope, effective layer, key and editor identity, excluding acceptance token/revision; ordinary saving cannot reset the outer draft owner.

The compiler-driven borrowed layer argument, unused-mut removal, map simplification and equivalent let-chains do not change request semantics. Private module registration/reexports do not widen public APIs. Existing memoized projection and bounded field feedback remain intact. Native controls retain labels, disabled behavior and status/alert feedback; the new input aria-invalid follows the displayed field error. CSS uses existing theme tokens and inherits existing narrow-screen 44px controls without new layout authority.

RF takeaway: existing private outcome/view ownership seams remain appropriate; no new refactoring issue established. Source diff whitespace check passed. Root-reported native/strict checks are not independently rerun here. Final WASM check after aria-invalid and public save/failure/retry/Undo/remount/accessibility verification remain open. No Cargo, browser or source edits performed.
