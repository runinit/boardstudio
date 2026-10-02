# Next public gap from PCB qualification

The paired PCB07/08 controls work on the mounted public route for the fixtures exercised. The generic binding journey required creating a second fixture in React because the candidate cannot currently place a selected library component from its Parts workspace.

In Dioxus, searching `smd_0805` shows a real catalogue result, footprint preview, and selected-definition metadata. The corresponding React screen also provides **Place component** and its generator settings action. I used that React action to add J1 to the derived fixture, which the candidate then imported and edited successfully. The difference is visible in `candidate-parts-library.png` and `react-parts-catalog-smd0805.png`.

This is already mapped by the bounded F3.2c draft [Add a library component to the board or selected key](../../../../dioxus-layout-authoring/drafts/03-component-placement.md). Its standalone `Add object → Parts` path and contextual Parts action cover the missing route, including normal edit/history and reopen behavior. Keep that existing draft as the next planning frontier; do not create a duplicate ticket or widen F4/PCB08 scope. It remains a draft pending the required independent review and retains F3.1 as its canonical start gate.

The public catalogue and packaged schema readiness are truthful for the supported fields: `infused-kim/smd_0805` exposes net parameters but the current 36-source package has no anchor-typed field. Keep anchor support conditional and do not manufacture a test definition to make it appear reachable.

No new RF finding was observed. Preserve RF-001/RF-006/RF-009 and the full parent acceptance ledger.
