# F3.2a: Create and place a key matrix

**Parent:** F3.2 — Matrix and component authoring.

**What to build:** In Layout, a designer can configure and immediately create a custom matrix at the reference origin, or start a 1×1 preset assembly placement from the Parts library and position/cancel that pending matrix before committing it. Both paths use the existing matrix edit/history authority.

**Blocked by:** Canonical F3.2 start gate F3.1 (Layout tree, board scope and selection), as recorded in the 62-task graph. No new child-to-child edge is introduced.

**Status:** Published; implementation starts after canonical F3.1 is satisfied. No sibling-ticket dependency is added.

- [ ] From Objects → Add object → Matrix, provide the reference rows/columns and key assembly controls, live matrix preview/count, origin guidance, and Create/Cancel actions. Accept only positive integer dimensions whose product is at most 4096; invalid or incomplete input cannot create a document edit. A valid submit commits a matrix at origin `(0,0)`; it does not create a pending placement ghost.
- [ ] From the existing Parts-library assembly action, begin the reference 1×1 preset matrix preview on the active board. Pointer placement and arrow-key movement follow the existing snap policy; click or Enter commits; Escape cancels. Cancellation leaves accepted document/history unchanged and reproduces the reference selection/scope/anchor cleanup; do not require selection to remain unchanged.
- [ ] Both routes use the existing matrix edit and synchronization path, including any required preset definition. The committed matrix is attached to the active board, appears in the Layout tree/canvas, becomes selected, and participates in normal Undo/Redo and save/reopen behavior.
- [ ] Against a saved project with a selected board, verify 1×1 and multi-key guided previews, boundary/invalid dimensions, immediate origin create/cancel, preset-assembly ghost movement and cancel/commit, board membership, one-step Undo/Redo, archive save/reopen, visible focus, compact and both themes. Keep matrix transformation/constraint controls in F3.3 and keycap sizing/reflow in F6C.3.
- [ ] Use existing document operations, selection/session and history. Add no matrix schema, public API/type/visibility, geometry algorithm, copied CAD behavior, or second document store. The public Dioxus route must exercise the integrated workflow; a standalone setup component is not acceptance.
- [ ] Feature ownership is a private matrix-setup UI module; root owns page mounting, operation/ID adapter, selection, canvas callback and global CSS. Do not edit those shared owners concurrently.
- [ ] Record source/build provenance and public behavior evidence; update an existing RF entry only if implementation yields a distinct finding, otherwise record “No new refactoring takeaway observed.”

**Parent acceptance:** F3.2 remains open for its full matrix/component authoring criteria. The canonical F3.1 start edge and all F3.2/F3.3/F3.7 joins are unchanged; this ticket does not close F3.2 or F3.

**Suggested routing:** Luna Medium author/verifier; Astra independent Spec/Standards review.

## Current Layout implementation slice

The Dioxus immediate-origin Matrix Setup already owns its positive-integer/4096-cell validation and existing `SetMatrix` path. The remaining source-confirmed daily-workflow gap is the Parts-library assembly action: pinned React `5a472a9426e6e38993361da402cd4ec730feb369`, `app/src/ui/Workbench.tsx:1041` delegates to `beginMatrixPlacement(1, 1, preset)`; `createWorkbenchPlacementActions.ts:29-52` prepares the selected preset, switches to Design, and starts the existing matrix ghost. This slice ports that route through the existing `MatrixSetupPreset`, Parts template and Runtime matrix projection contracts.

The changed journey is one selected preset and orientation from Parts → Place assembly → Layout ghost; move with the pointer and the active-snap arrow step, verify Escape leaves the accepted document/history unchanged, then place with pointer or Enter and verify one saved matrix, Undo/Redo and reopen. Reuse existing guided-form and multi-matrix evidence; do not repeat an invalid-dimension matrix. F3.2 remains open for the entire published criterion set, including guided-create public acceptance, multiple preset/orientation and compact/theme coverage. No new refactoring takeaway is observed; the RF ledger stays unchanged.
