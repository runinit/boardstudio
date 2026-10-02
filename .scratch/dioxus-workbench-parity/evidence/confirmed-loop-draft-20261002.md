# Proposed six-stream frontend migration loop

Draft for the current grilling session. It records confirmed design choices and pending factual audits; it is not an implementation dispatch or replacement of the existing parent roadmap.

## Settled choices

1. Match current TypeScript workbench UI and behavior: placement, labels, hierarchy, contextual Objects and Inspector, menus, visible controls, theme, and interactions. Record any confirmed bug/platform exception explicitly.
2. Six persistent streams: Layout, PCB, Keymap, Keycaps, Case/shared 3D, Parts. Project-menu/new-keyboard/two-demo work is scheduled with Parts and coordinator-owned project operations.
3. Ticket acceptance begins with a paired browser journey on the same real fixture/state. It covers visible surfaces, real edits, and applicable cancel/Undo/save/reopen. Compilation or unmounted source is an intermediate state.
4. Allow one bounded extraction of private workbench composition to establish disjoint ownership. Preserve the authoritative Session/Core and public contracts. Defer the wider architectural refactor.

5. Use proven capability-level start gates for reviewed children. Preserve full parent acceptance and record prior dependency rationale; never waive a missing capability or completion requirement.

## Proposed continuous cycle

- Explore TypeScript interactively: open every applicable menu/tab/disclosure and select each distinct context. Record positive/negative and loading/empty/error states. Trace the observed behavior to source.
- Maintain a workbench × selected-context × visible-action inventory. A source-file owner alone is insufficient coverage.
- Classify each gap: missing Dioxus UI; implemented but unregistered/unmounted; missing existing-service adapter; genuinely absent capability; known defect; acceptance-only gap.
- Reconcile each gap with existing 62 parent packages and child tickets. Preserve IDs, history, original criteria, blockers and RF takeaways. Add/split reviewed vertical tickets automatically under standing user authority; never replace the parent graph with a short checklist.
- Use to-spec to synthesize the concrete observed user journey and highest available public testing seam. Use to-tickets for one complete demoable interaction per slice, sized for one context window. A group of unrelated files is not a user outcome.
- Use implement-spec on the existing integration branch. Each implementer starts from a current integration-based isolated worktree, uses the relevant tdd workflow, and owns a named private module area. Never reset/discard existing work to align a worker; create an appropriate fresh branch/worktree when needed.
- Maintain six live queues. Within each stream, authors work independent slices concurrently when ownership and inputs allow. One real blocker stops only affected tickets; other contexts continue.
- Merge through a designated integration/merger role. Freeze candidate source for shared builds; isolated authoring continues. Integrate continuously when a vertical slice passes rather than waiting for all streams to complete a wave.
- Require paired TypeScript/Dioxus interaction evidence and independent Spec/Standards review before completed status. Record real provider/API or AT gates at the scope they actually block. Preserve original evidence failures and exact candidate lineage.
- After every integration, rescan the same workbench for missed controls and adjacent contexts. Feed new observations back into to-spec/to-tickets and dispatch the next real frontier automatically. Keep architecture/design/refactor takeaways linked to observed evidence.

## Capacity

Luna low/medium/high remains the implementation/exploration default; Astra high/xhigh remains review/behavioral diagnosis. Six workstreams are six accountable queues, not permission for conflicting shared-file edits. Up to six active authors plus coordinator, merger, independent verifier and two review axes fit the eleven-slot limit; roles rotate when a queue waits for integration or review. Root retains the one heavy-build lock.

## Audit evidence still needed

Which existing start blockers represent actual unavailable capabilities, and which incorrectly require a whole parent to reach final acceptance before an unrelated UI slice can start? The user approved precise capability-level child dispatch. Record each correction with source proof and preserved historical rationale; do not silently drop a gate.
