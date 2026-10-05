# PCB host geometry and desktop themes

Candidate: `frontend-preset-customization-20261004`, source `4e716156a0692972b1309798cdc883739c7ff73b`, http://127.0.0.1:34822/. Browser DOM script confirmed `assets/boardstudio-web-dxh7a84b355b3d74c.js`. Current HEAD `e2ec1c1e` has no changes to pcb_workspace.rs, pcb_scene.rs, or pcb_layers.rs from that source. Reference: running TypeScript app http://127.0.0.1:5175/.

Both frontends opened the identical `input.boardstudio` through public Open project. See input.json for archive hashes and the only changes from the retained PCB02 fixture: isolated project ID/name. Browser restored the same saved fixture across tabs. Viewport: 1280 × 720. No mobile qualification.

## Paired results

- Project `PCB grouped qualification20261004`; Left PCB and Right PCB each show 70 parts.
- Left PCB: all 70 SVG part identity labels (before coordinates) and trimmed SVG transforms match exactly. Generated host-outline polygon point strings match exactly.
- Public Board / Selected board selectors switch to Right PCB: all 70 identity labels and trimmed transforms again match exactly, as do the host-outline polygon points. Both render 160 pad elements. Return to Left PCB succeeds.
- All 23 named layer visibility buttons have identical initial pressed states. B.Cu, F.Cu, B.SilkS, Dwgs.User, F.CrtYd, F.Fab, F.SilkS, Edge.Cuts, Courtyards, Pads, Holes, References, Footprints, Front silkscreen and Back silkscreen start shown. Board outlines, Clearance & service, Mounting holes, Standoffs, Front fabrication, Back fabrication and Findings & clearances start hidden.
- Public Project → Workspace settings → Color theme changes both applications to Light then Dark. HTML data-theme confirms each choice. Layer button foreground changes from rgb(24,35,49) in Light to rgb(237,241,247) in Dark in both applications.
- In each theme, Hide Pads changes the control to Show Pads and removes all 160 rendered pad elements in both applications. Show Pads restores 160. Holes remain shown independently. Light-theme B.Cu Hide/Show also works in both; restored control is pressed and within viewport (top318.33/bottom350.33).
- After theme and selected-board changes, candidate still exposes Revision9 · Saved and the same project name. Reference retains the same name and Saved in this browser / Saved locally; its menu exposes no numeric revision.
- Returned both to Left PCB, restored layer defaults, closed Layers and restored Color theme to System.

## Scope and reuse

This qualifies F5.1-C01's current paired host geometry/defaults and selected-board route. It supplements F5.1-C03 with actual desktop theme/control behavior. F5.1-C02 retains the earlier PCB02 independent-toggle/history/export-input receipt; no duplicate archive-export run was added here.

This is DOM-backed visible geometry and interaction evidence, not a pixel comparison or an exhaustive footprint primitive equivalence claim. Equivalent reference footprint Y inversion is composed through a wrapper transform; this receipt does not compare raw child point strings across different coordinate frames. No screenshot was needed for the measured geometry checks.

Do not infer all controls are simultaneously visible from their bounding boxes: both layer lists intentionally scroll; after clicking Pads, upper rows are outside the list viewport. Clicking B.Cu returns that row into view. Candidate also showed ancestor scroll displacement during automated scroll-into-view; this observation alone does not establish an inaccessible control. Exhaustive visual/clipping qualification is not claimed. Mobile/compact checks are deferred by the user's instruction and are not a passing gate. No parent acceptance is claimed by this receipt. No new architectural/refactor finding established.
