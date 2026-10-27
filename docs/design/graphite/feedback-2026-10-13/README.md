# Graphite browser feedback — 2026-10-13

This pass applies the 21 browser comments while keeping the Graphite palette, single header, docked panels, shared BoardStudio controls and pinned Dioxus Components. It supersedes the previous expanded Layers and on-canvas numeric-card decisions.

## Feedback implementation

| Comments | Change |
| --- | --- |
| 1, 14 | Layers is collapsed by default in a persistent bottom dock, outside the object-tree scroll area. Its bounded popup opens upward. Layout and PCB use the shared popover primitive, including Escape and focus restoration. |
| 2 | Snap opens upward from the bottom dock with increment, geometry snapping and envelope-gap settings. |
| 3 | Removed the on-canvas numeric card. Numeric editing remains in the Inspector. |
| 4, 5 | Stagger and splay targets align with the selected column's key geometry. Targets remain separated. |
| 6 | Ctrl changes the stagger handle to column offset; pointer-down latches the operation for the gesture. The separate column-offset target is removed. |
| 7 | Removed the board picker and add-board control. Existing split boards remain navigable through tree roots. |
| 8, 9 | Key size is the first, initially expanded shared disclosure. An SVG preview accompanies one size selector and a Rotate checkbox, replacing width/height sliders. Existing custom sizes remain intact until explicitly changed. |
| 10 | Splay scope sits beside the Splay field. |
| 11 | Delete is always available below Inspector geometry and below the selected geometry on canvas, using the existing typed operation. |
| 12 | Removed redundant footer grid and geometry-snap status. |
| 13 | Objects offers By layout and By part grouping. Layout retains the hierarchy; part grouping supplies categories with existing selection callbacks. |
| 15, 20 | Shared command-pill styling is used across workbenches. Parts 3D mode and camera controls share the viewer toolbar; the 2D toolbar is centered. |
| 16 | PCB Inspector defaults to board-wide Routing & nets, with actual net names and pin counts. Selected-part editing remains accessible through Selection. |
| 17 | Case view and camera controls sit in one pill inside the viewer. Live preview, Update and Cancel move to a compact bottom bar. |
| 18 | Removed the Routed PCB reference editor from the UI. Existing saved reference data is retained. |
| 19 | Case construction exposes a default SVG stack preview and basic settings immediately. Linked-half controls are collapsed; the resolved stack is expanded. |
| 21 | The new-assembly action is a plus beside Key assemblies in the left pane and opens the existing assembly editor. |

The default Case construction values are a read-only preview until explicitly configured; viewing the page does not silently save a mechanical configuration. Routing & nets presents existing board information and operations, without introducing a routing engine. No schema changes or Case height/thickness handles were added.

## Verification

- Live regression: **74 assertions passed**, including all six workbenches at 1513, 1024 and 390 pixels, toolbar centers within two pixels of viewer centers, and no page overflow.
- Interaction checks include Ctrl-offset commit/cancel/undo, handle alignment, key-size selection/rotation/undo, split-board tree navigation, layer toggles and upward popup placement, canvas Delete/undo, and the relocated assembly action.
- Layout mounted WASM suite: **90 passed**. PCB: **49 passed**. Case: **81 passed**. Parts: **68 passed**.
- Native Layout/shared checks: **85 passed, one existing ignored test**.
- WASM frontend typechecks passed. Scoped Impeccable layout detector: **zero findings**. This is a scoped result, not a claim that the entire legacy stylesheet is clean.
- PCB's old hidden-DOM assertion was updated to the shared popover's unmount lifecycle while retaining scene visibility and focus-restoration assertions. The Case override test now locates its disclosure by accessible name rather than positional CSS.

The historical regression entry point was `scripts/test-graphite-feedback.cjs`; its source is now archived under `docs/archive/graphite/`. The current manual command is `python3 scripts/run-graphite-feedback.py`. Scratch logs live in `.scratch/graphite-feedback/`.

## Captures

A fresh demo profile was used; user browser storage was not modified. These are implemented states, including selection and assembly-editor interaction states.

| Workbench | Desktop | Intermediate | Narrow |
| --- | --- | --- | --- |
| Layout | [1513](Layout-1513.png) | [1024](Layout-1024.png) | [390](Layout-390.png) |
| PCB | [1513](PCB-1513.png) | [1024](PCB-1024.png) | [390](PCB-390.png) |
| Keymap | [1513](Keymap-1513.png) | [1024](Keymap-1024.png) | [390](Keymap-390.png) |
| Keycaps | [1513](Keycaps-1513.png) | [1024](Keycaps-1024.png) | [390](Keycaps-390.png) |
| Case | [1513](Case-1513.png) | [1024](Case-1024.png) | [390](Case-390.png) |
| Parts | [1513](Parts-1513.png) | [1024](Parts-1024.png) | [390](Parts-390.png) |

The requested changes intentionally alter the earlier Graphite mockup's control placement. Broader typography, Inspector density and canvas-label polish remain in the [drift backlog](../drift-2026-10-13/README.md).

Preview: http://127.0.0.1:8182/. Existing preview processes and the worktree's Tailwind polling compiler remain running.
