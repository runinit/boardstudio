# F3.2d: Edit matrix structure and key-cell configuration

**Parent:** F3.2 — Matrix and component authoring.

**What to build:** Selecting a matrix or one of its keys in Layout reveals contextual controls for matrix structure and per-key assembly/components; committed changes update the accepted project through existing edits and history without taking ownership of transform/constraint behavior.

**Blocked by:** Canonical F3.2 start gate F3.1 (Layout tree, board scope and selection), as recorded in the 62-task graph. No new child-to-child dependency is introduced.

**Status:** Implementation in progress after the F3.1 start gate. Matrix configuration fields and structural actions are being delivered as one private Inspector workflow; integrated browser acceptance remains open. No sibling-ticket dependency is added.

- [ ] Selecting a matrix exposes its name, row/column dimensions, pitch, supported key assembly/switch choice, diode direction, and keycap edge-gap preview settings. Dimension inputs follow the existing validation and matrix-resize rules; preset updates retain compatible saved key/member information and report source validation errors without partially applying a change.
- [ ] Preserve related structural actions at their reference entry points: Add object offers Add row/Add column for the selected matrix; its inspector supports Delete matrix and a working Duplicate design as variant handoff. The variant action clones the whole current ProjectDoc to a new UUID/name, opens the copy, applies the selected preset to the target matrix, and accepts/saves that project. If the preset edit fails, restore/open the original project and report the error. This root-owned callback is required for completion, not an optional permanently-disabled control; do not treat it as Undo in the original project.
- [ ] Selecting a key exposes its enabled state, supported switch/key assembly selection, and attached component list. The designer can replace or remove a pre-existing attached member and make the reference's explicit local override on a mirrored target. Only applicable definitions appear for each control; errors and unavailable inputs remain truthful and correctable. Do not duplicate insertion from the separate F3.2c contextual Parts picker.
- [ ] Structural and cell changes commit through the existing SetMatrix/document edit path, synchronize linked layouts as the domain currently specifies, preserve unrelated definitions/members, update the visible tree/canvas, and participate in one normal Undo/Redo and archive save/reopen cycle.
- [ ] Verify matrix and cell scope switching, defaults, valid/invalid resize boundaries, preset/orientation options, diode and gap edits, Add row/column, Delete matrix, Duplicate design variant success/failure recovery, key enabled/assembly changes, replacement/removal of fixture-provided attached members, mirrored linked versus local component behavior, validation rejection, Undo/Redo and reopen against a multi-layout reference fixture. F3.2 combined integration may use F3.2c to create a member; this ticket itself has no child dependency. Test mouse and keyboard, visible focus, compact and both themes on the integrated public route.
- [ ] This inspector child owns matrix/cell structural configuration and attached-component membership only. Position, rotation, local offsets, stagger/splay/origin, align, constraints, gesture/nudge and direct manipulation remain F3.3; keycap dimensions/reflow remain F6C.3; outline edits remain F3.4. Do not duplicate these controls or their authorities.
- [ ] Reuse current read projection and typed edit/history/session path. Add no geometry algorithm, schema, public API/type/visibility, alternate document store, or copied selection state. Root owns mount, IDs, callback translation, global CSS and build wiring; feature module completion must be integrated and publicly exercised.
- [ ] Record evidence and a scoped RF handoff, or state “No new refactoring takeaway observed.”

**Parent acceptance:** F3.2's full matrix/component workflow remains the acceptance scope. Canonical F3.1 start and F3.2/F3.7 relations are unchanged; this ticket neither closes F3.2 nor F3.3/F6C.3.

**Structural browser receipt:** [Candidate 34760 Right PCB duplicate context](../evidence/f32d-matrix-settings-20261003/structural-context-candidate34760.md).

**Suggested routing:** Luna High author and separate Luna verifier; Astra independent Spec/Standards review.
