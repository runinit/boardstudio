# F3.2b: Create and link mirrored layouts

**Parent:** F3.2 — Matrix and component authoring.

**What to build:** In Layout, a designer can create two linked mirrored halves from a setup form or mirror eligible existing layouts about an X axis, preview the result before committing, and later understand or deliberately unlink the relationship.

**Blocked by:** Canonical F3.2 start gate F3.1 (Layout tree, board scope and selection), as recorded in the 62-task graph. No new child-to-child edge is introduced.

**Status:** Published; implementation starts after canonical F3.1 is satisfied. No sibling-ticket dependency is added.

- [ ] The New mirrored pair form provides distinct left/right names, positive per-half rows and columns with the reference 4096-key product limit, a supported key-assembly preset and finite nonnegative edge gap. Its accessible field names and assembly option labels match the reference. Invalid values disable preview/commit and are explained accessibly.
- [ ] Preview shows the proposed linked halves and mirror axis in the Layout canvas. Pointer/keyboard placement and cancel behavior match the existing matrix preview conventions; Escape while placement is active cancels the setup and returns to the Layout without reopening the form. Commit creates both layouts and their distinct matrices through the existing linked-pair edit path, selects/reveals the result, and records a single normal history action.
- [ ] The Mirror existing half action is available only when unpaired matrices exist. It can target all eligible matrices or one chosen source and a finite X axis; it creates linked copies while preserving each original. Cancel, invalid input, and a rejected operation preserve the accepted document and expose a correctable error.
- [ ] In the linked-layout inspector state, accurately show the partner relationship and support unlinking into independently editable layouts. Linked geometry/key/component propagation follows the existing saved relationship. The explicit local component substitute/replace controls belong to F3.2d; do not duplicate those controls or create new mirror semantics.
- [ ] Verify a new pair at boundary dimensions/gaps and an existing-half mirror of all and one source, distinct stable identities, original-preservation, tree/canvas membership, relationship display, linked geometry/key/component propagation, unlink, Cancel/error, one-step Undo/Redo, archive reopen, keyboard/focus, compact and both themes against the same React fixture/actions. Compare the form's paired field layout, heading/control dimensions, compact bounds, and padded-workspace height against React; the form must remain centered and reachable without subtracting overlay padding twice. Local member replacement/substitution is covered by the separate inspector owner and F3.2 combined acceptance, not a blocker for this ticket.
- [ ] Use existing mirrored-layout operations and current document/session/history path. Add no schema, public API/type/visibility, mirror algorithm, new persistence authority, or copied geometry logic. The public Dioxus route must be exercised; a pair form/helper alone is insufficient.
- [ ] Feature ownership is the private linked-layout setup/relationship UI module. Root owns page mount, identity/callback adapters, selection/tree updates, global CSS and build integration; coordinate with root rather than editing shared owners.
- [ ] Record source/build provenance and update an existing RF only for a demonstrated structural finding; otherwise write “No new refactoring takeaway observed.”

**Parent acceptance:** This is one child of F3.2 and adds no canonical graph edge. F3.1 remains the start gate; F3.2's complete criteria and F3.3/F3.7 integration joins stay open.

**Suggested routing:** Luna High author; separate Luna verifier; Astra independent Spec/Standards review.
