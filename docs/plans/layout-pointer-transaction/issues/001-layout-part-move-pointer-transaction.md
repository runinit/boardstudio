# 001: Own the Layout part-move pointer transaction

Status: ready-for-agent
Type: build
Related: [Architecture candidate 01 research](../../../research/2026-10-20-layout-pointer-transaction-deepening.md), [plan map](../map.md)

## Problem Statement

The ordinary Layout 2D part drag is one transaction spread across part hit,
selection-move handle, canvas move/up/cancel, Escape, scope cleanup and Session
handoffs. Editor must coordinate pending versus active movement, pointer capture,
selection generation, current owner, pan and competing tools in separate closures.
That makes stale releases and lost capture difficult to reason about. Concentrate
this bounded transaction without moving the whole canvas or changing accepted
document, selection, navigation or gesture-commit authority.

## Solution

Give Layout presentation a cohesive owner for the ordinary part-move pointer
transaction at its 2D canvas handoff. It manages pending/active/retired state,
eligibility, capture lifetime and Session begin/sample/end/cancel ordering, using
page-provided current selection and shared navigation. Editor composes the canvas
and supplies the App-owned selection and lifetime inputs rather than rebuilding
the transaction across event handlers. Preserve the existing visible interaction
and one accepted Undo step per completed drag.

## User Stories

1. As a Layout editor, I can press and release a part without client-pointer movement so I select it without creating an accepted edit or Undo step.
2. As a Layout editor, I can drag an eligible part with primary-button pointer movement so I see a live preview, release to save the intended placement, and Undo it in one step.
3. As a Layout editor, I can move an eligible replace-selection of parts, including through the selection-move handle, so the same selection, snapping, companion/matrix and transaction rules apply to each entry point.
4. As a Layout editor, I can use replace, range and toggle selection as before so a modifier click or non-eligible selection does not accidentally start a move.
5. As a Layout editor, if my selection changes during a drag, the original drag cancels rather than switching its targets or committing a partial move.
6. As a Layout editor, I can press Escape during a pending or active ordinary part drag so capture and preview end without a saved edit, while higher-priority tool Escape behavior still wins.
7. As a Layout editor, if the browser sends pointercancel or capture is lost during my part drag, I can continue editing without a stale preview, commit or stuck gesture.
8. As a Layout editor, I can hold Space and drag on the canvas or over a part to pan, then release to change only the camera, not the document or Undo history; Space works without first focusing the canvas and does not steal typing in a form control.
9. As a Layout editor, I can switch board, project or workspace during a pending or active drag so the departed interaction retires, releases its capture and cannot affect the destination if a late event arrives.
10. As a Layout editor, an accepted revision, selection generation, scope or Session gesture that changes mid-drag cannot turn my obsolete pointer-up into an edit or cancel a newer owner's interaction.
11. As a Layout editor, I can use placement, outline, matrix-transform and mirrored-pair tools alongside ordinary part movement so existing first-owner arbitration and each tool's distinct commit/cancel behavior remain intact.
12. As a keyboard or assistive-technology user, I retain existing canvas instructions, focus, accessible labels, selection shortcuts, Escape and Undo behavior while pointer handling is reorganized.
13. As a Layout editor, if coordinates are unavailable or a gesture is rejected/fails, I get no falsely accepted movement; the pending preview and owned capture retire, and the accepted design remains authoritative.

## Implementation Decisions

- Locate the owner in Layout presentation at the Editor-to-2D-canvas interaction seam. Keep a small page adapter for App selection identity and scope cleanup; do not install a second document or selection store.
- Encapsulate the transaction sequence, not a bag of old event callbacks: part-hit and selection-move admission, pending client-motion threshold, pointer ID and capture, active gesture identity, final sample, cancellation and retirement belong to one owner. Preserve matrix-cell selection as selection-only.
- Read live selection and owner identity at admission and at sampled/final events. Bind captured work to scope, selection generation, accepted source identity and Session gesture generation; stale or duplicate release/cancel must neither commit nor clear another interaction. Keep callback freshness on rerender and release resources on unmount/departure.
- Freeze the admitted movement targets for the transaction. A changed selection cancels the whole pending or active drag without moving a newly selected part or committing the original set.
- Retain the existing canvas interaction arbiter and its tool priority. Placement, outline, pair and transform keep their own workflows; this owner only handles the ordinary part-move route after they decline it. Escape follows existing higher-priority branches.
- Reuse shared navigation for Space state, pan lifecycle, camera math and release. The part-move owner arbitrates against pan but never turns a pan into Session movement or duplicates the navigation implementation.
- Submit gestures to the real Session, which retains generation, snapping/normalization, preview scheduling, strict captured-revision commit and cancellation. Runtime continues browser capture/frame/persistence effects; Core remains writable document/history authority. Preserve existing operation error and rejection behavior.
- The extraction passes the deletion test only if Editor no longer reconstructs part-move ordering and lifetime checks; leave shell composition, accessible canvas behavior and other tool policies in their existing owners. Fix all affected callers, tests and fixtures within this slice.

## Testing Decisions

- **Primary acceptance seam:** extend or run the packaged real-page browser drag flow against a freshly built app. Use trusted pointerdown, multiple trusted moves and trusted pointerup on a Layout part; assert pointer capture and visible transient preview before release, then wait for the accepted result and durable browser save. Assert the intended part placement changed while unrelated placement stays put. Click the real Undo control and assert the complete saved document and visible scene return exactly to their pre-drag state as a new saved revision. This must exercise the mounted page, worker, Runtime, Session and persistence, not a fabricated Session event or script-dispatched drag.
- Add targeted mounted/browser behavior checks for ordinary part drag (not only a transform handle): Escape after movement, pointercancel, lost capture, selection change mid-drag, and a release after each cancellation leave no accepted/saved revision or Undo entry, restore the scene and release owned capture. Induced cancellation/lost capture is valid mounted browser evidence but must be labeled as such; it does not replace the trusted-pointer happy-path drag.
- Exercise real UI board/project/workspace departure during pending and active drags, then deliver or provoke late move/up/cancel; assert no edit in either scope and no disturbance to a newly started owner. Also cover accepted-revision or selection-generation change and Session invalidation while captured, including duplicate terminal events.
- Exercise Space+primary-button pan over empty canvas and over a part with trusted page input; verify camera movement on release, unchanged accepted document/revision and no part gesture. Check global Space before canvas focus and typing-control exclusion. Check competing placement/outline/transform/pair admission and existing selection modifiers, including matrix-cell down remaining selection-only.
- Verify accessibility and errors through the mounted surface: canvas instructions/focus/labels and keyboard Escape/Undo still work; absent/non-finite final coordinates or rejected/stale gesture cannot be reported as a landed edit. Assert cleanup, not merely absence of a thrown exception.
- Prior art: the existing drag smoke already proves trusted part and column drags, capture, preview, durable save and exact Undo, but its Escape case covers a **stagger transform handle**, not ordinary part movement. Keep its useful checks and add the missing part-specific cancellation/departure cases. Existing first-wins arbitration, client-motion threshold, global Space, scene callback-refresh and native Session cancellation/release/stale-preview tests remain supplementary regression gates. Run documented changed-scope checks, relevant mounted WASM/page tests and the packaged browser drag smoke; report exact commands and failures.

## Out of Scope

- Canvas-wide extraction, a universal Layout gesture owner, PCB hit rules, creation/placement, outline/perimeter, mirrored-pair or matrix-transform transaction redesign.
- Replacing App selection or shared pan/navigation, changing Core geometry/snapping, Session's strict gesture policy, persistence format or UI styling.
- A synthetic-only browser test suite or a new generic interaction framework.

## Further Notes

- [Candidate 01 research](../../../research/2026-10-20-layout-pointer-transaction-deepening.md) maps the existing split transaction, deletion test and gaps. [Glossary](../../../../GLOSSARY.md) distinguishes canvas preview from accepted document and Undo history.
- [ADR-0005](../../../adr/0005-resolve-queued-edits-at-execution.md) preserves captured-revision gesture commits; [ADR-0014](../../../adr/0014-document-authority-and-captured-lifetimes.md) sets document and captured-lifetime authority; [ADR-0021](../../../adr/0021-test-real-workflows-natively-and-browser-behavior-mounted.md) requires native real workflows plus mounted browser-specific evidence. This ticket's explicitly chosen primary seam is the stronger real-page pointer-to-durable-result-and-Undo path.
- Coordinate with [Editor workspace extraction 012](../../editor-workspace-extraction/issues/012-layout-canvas-and-gesture-ownership.md) if it is being implemented concurrently; its broader ownership scope is not a dependency or a substitute for this ticket's part-move browser gate.
