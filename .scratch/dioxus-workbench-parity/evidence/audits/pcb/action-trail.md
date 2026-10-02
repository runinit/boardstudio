# PCB parity audit action trail

Date: 2026-10-02 (America/Toronto)

## Checkouts and browser sessions

- React source: `/home/chris/01_Projects/ts-boardstudio2` at `5a472a9426e6e38993361da402cd4ec730feb369`.
- Dioxus integration source: `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001` at `89b1de8a28fdf02db91d972c90a69235bfbbbffb`.
- React browser sessions: `pcb-parity-reset-20261002`, `pcb-parity-react-sofle` (agent-browser isolated named sessions).
- Candidate browser session: `pcb-parity-reset-candidate` (agent-browser isolated named session, `34687/boardstudio/`).
- Evidence screenshots are in `screenshots/`; no save/edit/import action was committed during this audit. UI-only state changes were PCB layer visibility, board selection, workspace navigation, and opening menus.

## Actions

1. Read the version-matched `agent-browser` core and dogfood instructions; opened the React app at `http://127.0.0.1:5175/` in a named isolated session and recorded the initial UI.
2. Opened the React `VIK module review · above and below` demo. Inspected Layout then PCB, opened Layers, toggled B.Cu hidden, and selected SW1 from Objects. Captured screenshots for Layout, PCB, layer defaults, hidden B.Cu, and selected-part inspector. Attempted direct module-overlay click; the SVG polygon intercepted the automated button hit, so module selection was not treated as verified.
3. Opened the React `Sofle v2` demo, inspected PCB on Left PCB, selected Right PCB, and captured both board states. Read the wiring controls for controller, Matrix/Direct GPIO mode, assignments, pin options, peripheral pins, lock buttons, Resolve and Apply. Opened Export and recorded the available PCB and firmware outputs. Opened Case and recorded Left/Right assembly tree plus routed-reference import control.
4. Opened the Dioxus app at `http://127.0.0.1:34687/`, loaded the saved Sofle copy, inspected Layout, then selected PCB. Captured the PCB placeholder while confirming that the shared board/instance/object shell remained present. Candidate browser `errors` and `console` commands returned no entries.
5. Ran `codegraph explore` in the indexed React checkout before source searches. Follow-up source exploration covered `Workbench`, `WorkbenchLayers`, `CanvasLayers`, `usePcbWorkspace`, `WiringPanel`, `BoardReferencePanel`, `FirmwareKeymapPanel`, `useElectricalPlanning`, Core electrical and artifact dispatch.
6. In the unindexed integration checkout, read the F5 parent, F5.1 host-scene/layer dispatch notes, F5 graph/tasks, relevant F6K.4b draft, current page presentation, Runtime/Core worker host dispatch, Core electrical/module/artifact types, and current presentation helper ownership. No Cargo command or repository source edit was run.

## Main browser evidence

- React VIK: `react-vik-layout.png`, `react-pcb-initial.png`, `react-layers-open.png`, `react-hidden-back-copper.png`, `react-selected-part.png`.
- React Sofle: `react-sofle-pcb.png`, `react-sofle-right-pcb.png`, `react-export-menu.png`, `react-sofle-case.png`, `react-routed-reference-panel.png`.
- Dioxus: `dioxus-current.png`, `dioxus-pcb.png`.
