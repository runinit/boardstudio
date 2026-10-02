# Contextual PCB part-net Inspector: paired source and journey evidence

Date: 2026-10-02. Purpose: prove a capability-level start gate and give ticket 07 a concrete TypeScript mapping/Undo fixture. This is planning evidence only. It is not the final paired acceptance journey and it did not edit an accepted production project.

## Reference and candidate provenance

- TypeScript reference: `http://127.0.0.1:5175/`, checkout `/home/chris/01_Projects/ts-boardstudio2` at `5a472a9426e6e38993361da402cd4ec730feb369`. A fresh named `agent-browser` session started the Sofle v2 demo, opened PCB and selected `left-diode-24-keys` in the CAD tree. Relevant source SHA-256: `app/src/ui/usePcbWorkspace.tsx` `f66a97c030c2e747de454ab5ba1b520765b6d9259f09e556a78d026e4dd1bf23`; `app/src/ui/createWorkbenchEditActions.ts` `c9d6764c6742b0086c880303fbba1e8c4d17662faf53d86faa81a8690154cf05`. Screenshots are [initial part controls](pcb-audit-ts-diode.png), [existing-net mapping](pcb-audit-ts-diode-mapped.png) and [new-net mapping](pcb-audit-ts-created-net-mapped.png).
- Dioxus packaged candidate: `http://127.0.0.1:34720/`, root-provided package server and project copy. Read-only source receipts were taken at integration HEAD `427b6c747ca17af6e443d02b346053ad3fd18ed6`; this is not verified as the exact commit served by the package. Opened the saved `Sofle v2 copy`, switched to PCB, and selected the first diode part in the mounted scene. The selected element had `aria-pressed=true`, while `#m1-inspector-panel-content` contained only `Inspect` and `Options`. Screenshot: [selected generic part with empty Inspector](dioxus-selected-part-current.png). Attach root's package manifest/served-asset hashes before using this screenshot for final paired acceptance.
- These are not a final same-document comparison. The TypeScript demo used `left-diode-24-keys`; Dioxus' saved copy has generated identity `matrix/left-keys/r0c0/diode`. The visual/workflow analogy is clear, but exact parity acceptance must import/open the same serialized project into both apps and record project/document hashes. The candidate selection was triggered through the live scene's DOM event after browser pointer hit testing reported that an overlapping SVG polygon covered the center point. This screenshot proves mounted selected-state/Inspector output, not pointer hit-testing acceptance.

## Current TypeScript flow and expected edit

On the reference Sofle v2 `left-diode-24-keys` part:

- Inspector heading is `left-diode-24-keys · diode tht sod123`; breadcrumb is `LEFT PCB / PCB`.
- `Connections` has two named terminal controls, `Net for terminal from` and `Net for terminal to`. Initial values observed were `LINK_MATRIX_LEFT-KEYS_R0C0` (document net ID `generated/electrical/left/link/matrix/left-keys/r0c0`) and `ROW_0` (ID `generated/electrical/left/row/0`). The Ergogen definition supplies the `from` and `to` pad groups.
- Changing `from` to `COLUMN_0` (ID `generated/electrical/left/column/0`) left `to` at `ROW_0`; Undo restored the original `from` net exactly.
- A second isolated run added `PCB_AUDIT_NET`, selected it for `from`, and observed its generated `ui-<uuid>` ID in the select value. `to` remained `ROW_0`. One Undo restored the original `from` assignment while leaving the just-created net present; the next Undo removed that added net. The named demo browser session was closed after exercising the reversals.
- The UI also renders `Ergogen bindings`, `Add net`, and an expandable `Electrical nets` summary. Generic plated-pad controls are restricted to plated pads with a non-empty pad number, and omitted for Ergogen-generated definitions; named terminals remain independently assignable. Do not surface hidden/unplated/padless entries as assignable controls.

The source path is `app/src/ui/usePcbWorkspace.tsx`: `assignTerminal` calls `assignNetPins(document.nets, activePart.id, padIds, netId)` and emits a normal `replace-document` edit. `assignNetPins` removes the selected part/pad IDs from every prior net, then adds those pins to the selected target net, preserving unrelated pins. `addNet` creates a named empty net and adds its ID to the currently selected board's `netIds`, also through `replace-document`. Thus the expected two-step history is: add-net commits one replacement; assigning the terminal commits another replacement; first Undo restores the old terminal pins but keeps the net; second Undo removes that net from both `document.nets` and selected board `netIds`.

## Dioxus production source receipts

Read-only source inspected at integration HEAD `427b6c747ca17af6e443d02b346053ad3fd18ed6`:

- `web/src/presentation.rs` builds `PcbWiringSource` from the current accepted snapshot, board/UI scope, selected part ID and electrical executor epoch. Construction in `web/src/presentation/pcb_wiring.rs::PcbWiringSource::new` rejects instance-scoped board sources, mismatched normalized scopes, session/document/token revisions and parts outside the selected board. Its private accepted `Arc<ProjectDoc>` plus identity is sufficient to render assignment values without inventing another document authority.
- `selection::submit_canvas_selection` validates current runtime scope and generation, current context, and eligible live IDs before submitting selection. The PCB scene callback additionally validates workspace, captured scope/token/generation, current board and part membership.
- `project_display` currently recognizes board/controller wiring and the built-in switch's inherited terminal view. Other non-switch/non-controller definitions produce `SelectionView::Unsupported`; `PcbWiringInspector` renders that branch as an empty element. This matches the observed candidate gap.
- `Runtime::submit(Event::Edit { operation: EditOperation::ReplaceDocument { .. }})` is the existing application history/save route. Existing private controllers allocate an `OperationId`, call `Runtime::observe_operation(id)` **before** submitting, and settle against that exact outcome slot. The PCB composition presently has no generic-part edit callback/controller. Ticket 07 must create one narrow private proposal/admission seam, validate the captured board/part/token/selection at submit, retain the exact outcome through durability settlement, and keep switch edits read-only/inherited.
- Existing Core `SetNet` is not an automatic replacement for the reference semantics: it inserts/replaces a net and may register it on the first board. The reference updates pin membership and the explicitly selected board. Prefer a private immutable `ReplaceDocument` proposal matching the reference; do not widen Core API or silently route to the first board.

## Bounded production test design

Use a small accepted `ProjectDoc` fixture with two boards, one selected component with terminals `from`/`to`, one existing `from` net, one existing `to` net, an unrelated pin sharing the `from` net, and a third unrelated net. Verify the actual private proposal path, not only a test copy of `assignNetPins`:

1. Mapping `from` to an existing net removes only this component's `from` pad IDs from the prior net, inserts them once in the target net, preserves `to`, the shared unrelated pin and all unrelated nets, and does not mutate the captured accepted source.
2. Creating a new net puts it only in the active board's `netIds` and `document.nets`, with empty pins until the terminal assignment proposal; assigning `from` then links the terminal pads.
3. Submit through the existing Session edit operation and verify accepted document/history values; Undo/Redo restores and reapplies the same links; save/reopen retains them. Exercise the actual exact-operation result/durability slot. Stale callbacks for changed token/revision, board, selected part/context or generation must be rejected before submit without disturbing a newer request.
4. A paired UI journey selects the same part in the same serialized Sofle project in both apps, maps `from` to a named new net while `to` stays at `ROW_0`, compares visible dropdown labels and accepted document net/pin values, then checks Undo, Redo, save/reopen and panel scroll/keyboard behavior. Separately retain switch selection's read-only Named terminals/Edit board wiring branch.

Do not pull board Wiring mode/assignment/Resolve/Apply/remap controls (tickets 09–10), scan/generator parameter controls (ticket 08), or Case selector/flip controls (ticket 11) into this child.

## RF handoff

- **RF-001:** the existing root Editor composes `PcbWiringSource`, but no reusable generic-part edit callback exists. Add one narrow, PCB-owned private intent/outcome owner at the Inspector mount and serialize shared composition changes with the coordinator; do not add another Session/document authority.
- **RF-006:** electrical controls are bound to the active board scope and accepted UI selection. Keep `instance_id = None` for board electrical identity while validating the UI's current instance scope; do not reuse Case mechanical selection ownership.
- **RF-009:** the reference and candidate had analogous Sofle layouts but different generated part IDs and different project histories. Final ticket evidence requires one exact imported/serialized project and pinned served source/build hashes, plus actual pointer/browser action; this audit must not be promoted as same-document parity evidence.

No new RF ID is introduced. Parent F5.2/F5.3 criteria, the public paired journey, all parent history/blockers, and manifest review/publication gates remain open.
