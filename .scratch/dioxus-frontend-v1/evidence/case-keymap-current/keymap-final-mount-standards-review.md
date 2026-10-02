# Independent Standards: final Keymap mount and compact tree handoff

Reviewed dirty integration against `e6f75902`. Final blobs: presentation `e865d2cb8102a7b35ad4ce708a8b79d7307b70df`; CSS `8ab08119409a0bd296b8eab1521f0d85b3dc5156`; Keymap module `17c59d399baed270eef9a310bf193141a9a9edbe`; view `b2bbd5b75c0142a7a115fcfff54c59d0d71e48f8`; canvas `2dc6d72e5893a5cd51e7b474ff830e314ff66c20`; panel `adf50f15b2d9ebe1d03bddd667d5fb5830c590de`. No compiler/browser/source edits. Diff whitespace check passed. Unrelated Case controller and viewer documents excluded.

**No remaining material Standards finding in these final blobs.** Initial presentation blob `c3ca2130` excluded Keymap from the outer InspectorPanel condition, making its nested panel unreachable. Root corrected that condition; the final source consistently includes Keymap in Inspector mounting, right track and compact Inspect control. Public confirmation remains required.

The new active-layer Signal and projection memo are called before optional-scope/accepted-snapshot returns. Memo dependencies cover accepted token, full Scope, board and layer; it rechecks the current accepted token and shares an immutable Rc view across both surfaces. Projection data stays accepted-snapshot based. Callback-time projection rebuilding checks live membership without cloning document/definition payloads; no performance-budget claim follows.

Selection and layer callbacks check active workspace, full Scope, adapter generation, and fresh accepted document/board/instance. Real IDs resolve through existing context/selection helpers. Empty selection submits empty Replace and clears private context/anchor stamp without inventing an ID or changing Session's retained anchor-ID behavior. Layer browsing changes only view state.

The shared SVG uses the existing scoped camera/pointer lifetime; Keymap adds no part-drag start. Enter/Space activation stops propagation before reaching pan keyboard handling. CSS uses existing theme tokens and local selectors with visible focus. Existing Layout/Parts owners remain in place. Scope/mount transitions, compact focus, camera and keyboard behavior still need public evidence.

The compact tree hunk matches the reviewed bounded patch: current context/Scope/generation checks precede selection, then fresh checks precede panel updates. Board preserves panels; LayoutGroup closes Objects only; Matrix/Row/Column/Key/Component close Objects/open Inspect, including empty contexts. No extra document command or panel-preference write appears.

RF heuristic: the repeated Inspector-workspace condition caused the now-corrected omission; a single private computed predicate could keep these three gates aligned. This is nonblocking, not a new architecture requirement. T1-10/F3.1/Keymap public and accessibility acceptance remain open.
