# F3.3d paired command-pill browser journey

**Result:** bounded menu/scope journey reproduced on both public applications. This is not F3.3d acceptance: the tested Dioxus candidate predates the separate close-control and Inspector pin/reveal source followups, uses a parallel Sofle fixture route rather than the same archive hash, and the full Align/edit/Undo/save/reopen journey is open.

## Build and fixture identities

- React reference: `app/src` pinned at `5a472a9426e6e38993361da402cd4ec730feb369`, served at `http://localhost:5173/`, opened from the `Start Sofle v2` demo entry.
- Dioxus candidate: source/artifact candidate `792e88afecefa032d17b7540fa316e24b709c5ae`, served at `http://127.0.0.1:34731/`, opened from the local `Sofle v2 copy` entry.
- Both journeys used the Left PCB, a two-board Sofle project with 70 parts on the selected board. No archive hash was available to prove byte-identical fixtures; treat the evidence as a matched scenario, not exact saved-fixture equivalence.
- Candidate packaging proof supplied by the coordinator: 8 executed commands, 22 inherited commands, 1,313 source hashes, 145 assets per route, 126 provider assets per route, zero route mismatches. The observation was a 535.29s full build versus 88.55s guarded reuse; the paired UI result does not independently establish packaging performance.
- Browser automation used isolated named sessions `layout-toolbar-parity-20261002` (React) and `layout-toolbar-parity-20261002-dioxus` (Dioxus).

## Observed behavior

- At 1280×577, Dioxus showed the single Select / Transform / Align / Snap pill. Its separate Layout view group was still missing 2D and 3D assembly; only the existing Footprints toggle remained. The React reference showed the separate 2D / 3D assembly / Footprints group.
- Opening a second Dioxus command closed the first. Escape closed the active menu and restored focus to its trigger. An outside click on Layers closed the menu and left focus on Layers. React showed the same exclusivity, Escape restoration and outside-pointer focus behavior.
- React menu open focuses its explicit Close button. The tested Dioxus candidate had no Close button; when the no-selection Transform actions were all disabled, focus stayed on the trigger. The separate close-control source followup (`31240d3a77d7225fefb0c9b5fbf1e2924ee28d9f`) was not in this candidate.
- With the matrix selected, React and Dioxus both routed **Splay & origin** to Column 1 and **Row offsets** to Row 1, then showed that scope's Properties fields. Dioxus preserved the retained cell between those conversions. This proves the navigation route only; it does not qualify editable field commits or history.
- At 375×667, Dioxus Snap and Align popovers were contained within the viewport. Snap measured x=13..273 and its gap input x=26..260; Align measured x=13..273. `document.documentElement.scrollWidth` remained 375. At 1280×577, Snap measured x=482.9..742.9.
- The Inspector happened to be visible in the saved-fixture default. Desktop Autohide/Collapsed Inspector reveal was not tested here; a separate source reproduction found that the tested candidate did not pin/reveal the desktop Inspector for Transform navigation. The root-owned repair is outside this candidate.

No document edit, save, Align operation, Undo or Redo was performed in this bounded journey. Therefore no revision/history/durable-save claim is made. RF-005 remains unchanged; no new RF finding is established by these menu-navigation traces.

## Durable screenshots

Images are retained at `/home/chris/.local/share/boardstudio/retained-tmp/20261002/layout-toolbar-paired/`.

| Image | State | SHA-256 |
| --- | --- | --- |
| `layout-toolbar-react-1280x577.png` | React, toolbar closed, 1280×577 | `61ad788aacaaed16f76c4f3ba6aee8afe5cbcfe1d22fb030d2aaa4185fa3ada3` |
| `layout-toolbar-react-snap-1280x577.png` | React, Snap open with Row selected, 1280×577 | `43914d4eab58a0b24f0be78b845c110c3472732349fb994c77b32b8a8712a6a6` |
| `layout-toolbar-dioxus-1280x577.png` | Dioxus, Snap open with Row selected, 1280×577 | `a1d21a41cf633515b3c41482fe976f34ae7332fddc3a0f9c0ae8395031b7a497` |
| `layout-toolbar-dioxus-375x667.png` | Dioxus, Snap open, compact 375×667 | `e282339b61004d8fe346607b3fb0b1745386893b48fec939d3ff8135096100ec` |

![React Layout command pill with Snap open at 1280×577](/home/chris/.local/share/boardstudio/retained-tmp/20261002/layout-toolbar-paired/layout-toolbar-react-snap-1280x577.png)

![Dioxus Layout command pill with Snap open at 1280×577](/home/chris/.local/share/boardstudio/retained-tmp/20261002/layout-toolbar-paired/layout-toolbar-dioxus-1280x577.png)

## Remaining acceptance

Use one identical saved archive hash, capture every available menu at desktop and compact widths in Light and Dark, integrate the close and Inspector pin/reveal repairs, execute one real Align result and one real transform-field edit, verify one-step Undo/Redo for each, and save/reopen accepted values. Keep view-group and common Layout 3D viewer parity under F3.6 and the F7.3 acceptance join; this F3.3d evidence does not close F3.3 or F3.7.

## Additional pinned-reference statusbar observation

The coordinator's pinned React profile confirmed the **Grid ¼u** statusbar control is a button that opens the same Snap dialog. The Dioxus Layout status area still exposes this as inert status text. This is an existing F3.3 Snap/status-control parity gap to carry in that slice's current queue; no duplicate ticket or canonical parent is created here. The paired journey above did not exercise this added route, so no browser parity result is claimed for it.

The coordinator also reports a later actual paired browser inventory on the integrated `34732` candidate: React's Transform menu has the additional **Stagger**, **Splay**, and **Origin** shortcut buttons; the Dioxus menu has only **Position & rotation**, **Splay & origin**, and **Row offsets**. Keep the missing React actions visible as an F3.3 transform-tool parity gap in its existing queue. Their pointer/preview/cancel semantics remain separately capability-gated; this note does not authorize placeholder controls or close the gap. The exact `34732` build/source receipt belongs to the coordinator's browser evidence and was not independently re-run in this paired journey.
