# PCB contextual inspector: source and test preparation

Date: 2026-10-02  
Worktree: `f56-physical-intents-ticket06-20261002`, HEAD `da1abc4076d0b15c5de5ac24fc345c622bb8e127`  
Status: inventory and implementation preparation only; no production source or shared ledger changed.

## Evidence boundary

The TypeScript app at `http://127.0.0.1:5173/` was inspected through the PCB board panel and selected-part contexts. The Dioxus app at `http://127.0.0.1:34722/` was an older, stale build; the current candidate port `34723` was not listening when checked. The projects were separately opened Sofle v2-like documents, not proven identical serialized fixtures. These captures establish contextual feature differences, but do not satisfy the paired same-project acceptance journey.

Browser snapshots and screenshots are alongside this file. TypeScript captures: `typescript-pcb-initial`, `typescript-selected-switch`, `typescript-board-wiring-from-switch`, `typescript-selected-trrs`, `typescript-selected-encoder`, and `typescript-selected-controller`. Dioxus stale-candidate captures: `dioxus-pcb-initial`, `dioxus-selected-switch-stale-build`, `dioxus-selected-trrs-stale-build`, `dioxus-selected-encoder-stale-build`, and `dioxus-selected-controller-stale-build`.

## TypeScript behavior observed

- Board context exposes Wiring mode, automatic resolution, Apply wiring, and per-signal pin assignments with Lock controls. The captured Matrix view contains row/column, I2C, encoder, and split-controller assignments. Its option lists show candidate controller pins.
- Selecting a built-in switch does not expose an editable wiring form. It shows the inherited-wiring explanation, named terminal assignments, and `Edit board wiring`, which clears the part selection and returns to the board panel.
- Selecting the generic TRRS connector shows Board details, four terminal-to-net selectors, Ergogen bindings, and new-net creation. The observed R1/R2/SL/TP values are retained in the snapshot. A new net is a separate edit from assigning terminal pins.
- Selecting the supported rotary encoder shows Press scan mode and five named terminal mappings (A/B/C/S1/S2), then Ergogen bindings. The captured scan mode is Direct GPIO.
- Selecting the controller returns to the board wiring panel.

Detailed control text is captured in `typescript-pcb-initial.txt`, `typescript-selected-trrs.txt`, and `typescript-selected-encoder.txt`. The screenshots supplement these snapshots. The exploratory net-add/Undo interaction was not retained as acceptance evidence; use a controlled paired journey for that behavior.

## Dioxus candidate behavior observed

- Board context renders read-only electrical wiring status and pin-use summaries in this older candidate; the TS controls listed above are absent in the capture.
- Built-in switch has a selected-part view with inherited wiring and named terminals, although its copy/context framing differs from React.
- Selecting the generic TRRS or rotary encoder yields an empty Inspect pane.
- PCB left-pane Physical instance and Group objects selectors are visible in the candidate; the captured TypeScript PCB left pane has a Board selector without those controls. This is an observed parity discrepancy, not a request to remove controls; resolve against current source and the full TypeScript workflow.

Because the Dioxus page was served from stale `34722`, these points are historical evidence only until refreshed against a verified packaged current candidate.

## Source-backed route and behavioral rules

`web/src/presentation/pcb_wiring.rs` currently builds a read-only `PcbWiringSource` from the accepted document, board scope, selected part, and executor epoch. `project_display` maps no selected part to Board, controllers to Board, and switches to Switch. Other part kinds map to `SelectionView::Unsupported`, rendered as empty RSX. This is the direct source cause for the missing generic TRRS/encoder inspector; the evidence does not establish that a Rust editing engine/API is absent.

The React reference is `app/src/ui/usePcbWorkspace.tsx` and its existing pin transformation helper is `app/src/ui/createWorkbenchEditActions.ts::assignNetPins`:

- Branch order matters: controller/no selection -> board Wiring; built-in switch -> inherited read-only panel; generic selected part -> contextual Inspector.
- The generic inspector's press controls are eligible only for `inputProfile.press` or exact rotary generator source `ceoloide/rotary_encoder_ec11_ec12`. Built-in switches/controllers must remain excluded from this generic branch.
- Matrix scan mode is disabled when no accepted matrix contains the part, or when `inputProfile.press.independent === false`; enablement requires both matrix membership and independence not forbidden.
- Terminal/pad mapping removes only `(partId, targetPadId)` from all nets, then appends those target pins to the selected destination net. Empty destination unmaps only those target pins. All other pins and nets are preserved.
- A visible board net is included if its id is in the selected board's `netIds` or it has a pin from a part on that board (legacy mapping compatibility).
- Adding an empty net separately adds its id to the selected board's `netIds`. Reproduce this with the selected board identity; do not use Core `SetNet` for this UI operation because its current implementation targets `doc.boards.first_mut()`.
- Terminal controls are based on named `PartDefinition.terminals`; standalone pad controls exclude unplated pads, empty pad numbers, Ergogen-generated definitions, and pads already covered by a named terminal.
- Keep Ergogen parameter editing (bindings and anchors), controller/wiring assignments and locks, and Apply/Resolve behavior in separate ownership slices. The generic-part inspector child must not claim those board-wiring or generator-authoring joins.

## Candidate test seam for the generic-part Inspector child

Keep the leaf read-only and add a narrow private typed request/callback seam owned by the Editor-lifetime accepted-context owner. Avoid giving the inspector Session authority. A submitted action should carry the rendered target identity (document/session, board, selected part/pad or terminal, accepted token/revision, and owner generation as appropriate). The owner must reject a stale rendered request before mutation, construct an immutable `ReplaceDocument` proposal from the accepted snapshot, register/observe the exact `OperationId`/outcome slot before submit, and retain its terminal result across view/selection changes. This is consistent with the established physical-setup/wiring owner pattern; no public API or schema change is indicated by this inventory.

Recommended deterministic `ProjectDoc` fixture: two boards; selected board owns generic `left-J2`, controller and switch; a second board owns an unrelated pin/net. Define a generic connector with named pads for R1/R2/SL/TP plus a separate standalone plated numbered pad, a non-plated pad, and an empty-number pad. Seed target-pad pins across two selected-board nets, a different pad on the same part, another part's pins, and a second-board pin/net. Verify projection visibility/eligible controls and these exact cases through the production proposal/controller seam:

1. Mapping one terminal to another net moves only that terminal's pads; mapping to Unmapped removes only those pads.
2. Mapping a standalone eligible pad uses the same target-scoped replacement rule; the excluded pads never become controls.
3. Adding an empty named net appends the new net and id only to the selected board, preserving all existing nets/boards.
4. Legacy pin-derived board-net visibility works even if that selected board omitted a net id.
5. Controller/switch route selection remains unchanged while generic connector and exact rotary definitions produce the generic contextual Inspector.
6. The real mounted hook rejects retained controls after project/board/selection/token/revision/session/generation changes; a valid request emits the intended `ReplaceDocument`; exact accepted/save failure feedback remains observable. Follow existing mounted-hook production test patterns in `web/src/presentation/pcb_physical_setup/tests.rs` instead of reimplementing admission logic in a pure helper test.

The paired browser journey should use a saved project opened identically in both apps: select generic TRRS, assign one named terminal, add another net, Undo/Redo, save/reopen, and verify all unrelated pins/nets. Then repeat selection contextuality for a switch and controller. Board mode assignment/lock/Apply parity is a separate ticket/journey.

## Refactor ledger handoff

No new RF id is proposed. Preserve existing RF-001 (shared composition/private ownership), RF-006 (board electrical scope versus physical Case selection), and RF-009 (served-build and fixture provenance) handoffs. The current generic inspector is a UI projection gap rather than evidence for a new backend-engine gap. Keep all parent/F5.6 joins and browser gates open.
