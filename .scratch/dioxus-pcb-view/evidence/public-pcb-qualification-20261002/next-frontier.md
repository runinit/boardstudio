# Next public gap from PCB qualification

The paired PCB07/08 controls work on the mounted public route for the fixtures exercised. The generic binding journey required creating a second fixture in React because the candidate cannot currently place a selected library component from its Parts workspace.

In Dioxus, searching `smd_0805` shows a real catalogue result, footprint preview, and selected-definition metadata. The corresponding React screen also provides **Place component** and its generator settings action. I used that React action to add J1 to the derived fixture, which the candidate then imported and edited successfully. The difference is visible in `candidate-parts-library.png` and `react-parts-catalog-smd0805.png`.

This is already mapped by the published F3.2c issue [Add a library component to the board or selected key](../../../dioxus-layout-authoring/issues/03-component-placement.md), SHA-256 `e18acd7294b1fb63854207e3a399aef7a433ca2111f6d6f989587f115c58874d`. It retains F3.1 as its only canonical start gate; the issue says implementation starts after F3.1 is satisfied. Its standalone `Add object → Parts` path and contextual Parts action cover the missing route, including normal edit/history and reopen behavior. Keep that existing ticket as the next planning frontier; do not create a duplicate ticket or widen F4/PCB08 scope. Apply any independent review refinement to that ticket in place.

The initial React press screenshot in this packet showed 71 parts because it was captured after creating the separate J1 fixture. It has been replaced with a fresh isolated React session that re-imported the original 70-part Sofle archive, selected `left-SW25`, changed Press scan mode to Unassigned, reloaded, and verified Unassigned again after returning to PCB. The canonical `react-press-unassigned.png` and `react-press-postreload.png` now show 70 parts, matching the candidate press captures. The derived J1 archive remains separate and is used only for the generic Ergogen binding comparison. The reproducible screen record is [f32c-start-gate-join.md](f32c-start-gate-join.md).

The public catalogue and packaged schema readiness are truthful for the supported fields: `infused-kim/smd_0805` exposes net parameters but the current 36-source package has no anchor-typed field. Keep anchor support conditional and do not manufacture a test definition to make it appear reachable.

No new RF finding was observed. Preserve RF-001/RF-006/RF-009 and the full parent acceptance ledger.
