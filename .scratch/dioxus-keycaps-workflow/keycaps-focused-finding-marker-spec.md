# F6C.4 focused Keycaps finding marker specification

## Problem Statement

Following a Keycaps fit finding now has a reviewed path to its destination object, but the Dioxus Layout canvas does not visibly mark the focused finding. React keeps the focused finding ID and displays the corresponding accepted marker contours on the active board. The Rust scene already supplies Core-generated `finding_markers`; the Dioxus Layout canvas does not project them.

## Solution

Retain the identity and accepted owner of the focused Keycaps finding during navigation. On the destination Layout canvas, display only the matching marker from the current accepted scene and active board, using its existing contour geometry and a focused visual treatment equivalent to React. Keep marker creation and geometry under Core ownership. Clear or suppress the marker whenever its accepted owner, board, or finding no longer matches.

## User Stories

1. As a keyboard designer, I want the destination Layout canvas to outline the geometry associated with the fit finding I followed, so that I can see which item the Inspector is discussing.
2. As a keyboard designer, I want the marker to use the accepted scene contour, so the highlight matches the same geometry used by Core's fit analysis.
3. As a keyboard designer, I want a focused marker to appear only on its owning board, so switching boards does not suggest that a different board contains the finding.
4. As a keyboard designer, I want the marker to disappear when a newer accepted source no longer contains that finding marker, so stale geometry is never presented as current.
5. As a keyboard designer, I want the marker to survive the Layout workbench transition and delayed camera-fit render, so navigation does not lose the visual focus before I can inspect it.
6. As a keyboard designer, I want following a finding to remain a view/selection action, so showing its marker does not edit the project or add an Undo entry.
7. As a keyboard designer, I want marker focus to use the existing accepted finding and navigation owner, so a second source of selection or geometry cannot drift from the Inspector.
8. As a keyboard designer, I want the marker to retain the same finding identity as its Inspector action, so duplicate labels or nearby findings cannot highlight the wrong contour.
9. As a keyboard designer, I want focused contours to remain visible with the existing canvas scale/flip transforms, so the outline is drawn over the intended physical location.
10. As a keyboard designer, I want the focused marker to clear when the owning document or session changes, so navigation state from an earlier project cannot leak into the next one.

## Implementation Decisions

- Preserve one focused finding identity together with the accepted scope, snapshot token, revision, and board that admitted navigation. Reuse the finding carried by the existing Keycaps navigation request rather than constructing a second finding resolver.
- Render the focused marker from the active accepted `SceneDelta.finding_markers` entry whose `finding_id` and `board_id` match the focused owner. Do not derive contours in the presentation layer or call Core geometry independently.
- Compose the marker into the current Layout SVG under the existing coordinate transform and use the established focused finding class/data attribute pattern from the TypeScript workbench.
- The root Editor retains authority over accepted snapshot identity and active board. The canvas receives a private marker projection only after those owner checks; it does not create a selection, camera, or geometry owner.
- If the current accepted scene has no matching marker, render no focused contour. Keep the Inspector finding readable and preserve its stale/current/error state.
- Clear focused state when the project/session owner changes or a new focus action replaces it. Scope the marker to the accepted board so board navigation cannot reuse it.
- This ticket covers the Keycaps fit marker in the 2D Layout destination. Existing Case assembly finding visualization and the full Keycaps navigation/browser parent journey remain open unless covered by their own reviewed criteria.

## Testing Decisions

- Test the production focused-marker projection through the mounted Layout canvas composition, using an accepted scene fixture with two finding IDs and two board IDs. Assert the focused ID/board contour is rendered with the focused class and the other marker is not.
- Rerender with a replaced token/revision, changed board, removed finding marker, and changed session/document owner. Assert the old contour disappears without changing document revision, selection history, or camera beyond the already accepted navigation fit.
- Exercise the route-to-layout transition and marker retention through the same production owner seam used by the Editor. A helper-only test does not close the mounted projection criterion.
- Use paired pinned React/Dioxus browser evidence on the same byte-identical retained fixture and source-stamped build. Record the focused finding ID, target IDs, active board, marker DOM/class/contours, camera, history/revision before and after, and screenshot. Do not use generated or synthetic contours when an existing finding fixture provides real Core markers.

## Out of Scope

- Creating or changing Core finding-marker geometry, serialized document data, TypeScript contracts, public APIs, or history entries.
- Implementing cross-board pending finding navigation or new finding target resolution.
- Completing Case 3D finding overlays, all F6C.4 parent journeys, F6C.2 settings, F6C.3 size/reflow, F6C.5 model/export, or F6.6 shared Keymap/Keycaps acceptance.
- Broadly extracting the Editor, selection, camera, or Inspector architecture. Required parity is implemented now; larger reuse proposals remain refactor carry-forward only.

## Further Notes

- React source evidence: `Workbench.showFinding` sets focused finding identity; the 2D Design/PCB canvas filters `findingMarkers` by active board and focused ID, applies `wb-outline-finding` and `is-focused`, and sets `data-finding-id`. Core `keycaps::finding_markers` constructs marker contours from the same resolved keycap envelopes used for fit findings.
- The reviewed production route-owner slice and this marker continuation are separate gates. The issue05/F6C.4/INT.2 joins, paired public journey, original missing fixture identity, and Case-focused marker work stay open.
- Refactor evidence is carried in the source-owner report. The post-port RF ledger remains coordinator-owned; this spec does not modify it.
