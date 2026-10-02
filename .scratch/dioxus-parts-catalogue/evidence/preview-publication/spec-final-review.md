# F4.4 preview draft — final independent Spec review

Decision: ready for publication from the Spec axis. All three prior findings are resolved; no additional material finding in the revised ticket/README.

The ticket now preserves definition-ID visibility state, including effective empty visibility for another definition and restoration of the remembered pair; same-definition recipe/orientation changes do not reset it. Explicit catalogue/preset activation, including reselect, resets the view to 2D separately from recipe-sensitive async invalidation.

The 2D path now covers no selection, pending compilation without geometry, pending recompilation with geometry, compile-error alerts and compiled diagnostics. It requires real service inputs, preserves the reference Ergogen fallback and forbids invented non-Ergogen geometry. The reference compile-error alert remains applicable whether geometry exists or not under the stated full-reference requirement.

The ownership wording now permits the required ephemeral sample ProjectDoc while prohibiting a second authoritative active-project document/store/session. F7 still owns the sole viewer/renderer, integration owns shared mount/Runtime/CSS/manifests, and the no-public-API/schema/visibility boundary remains intact.

Canonical graph unchanged: start after F4.1 and F7.1; acceptance joins F7.3 and INT.2. Publication does not clear those prerequisites, authorize implementation before the concrete dispatch contract, or close any parent acceptance. The README can replace its draft/awaiting-review status during publication; that is metadata, not an additional Spec blocker.

Reviewed SHA-256:
- 03-isolated-library-preview.md: ecb2c4fe0a8f3b869f421d652206e13b9389786267774adf439b4f0be5d6bb0b
- README.md: 5889c8d3b9279eed01cfbdfc6ab09a4de66223a0624594779f8a5ee193ab072b

Read-only review; no source/Cargo/build/browser work. No new RF.
