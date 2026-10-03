# VIK splitter circuit join and constituent Place qualification

Candidate: `frontend-functional-controls-20261003`, source `ad26b07a`, served at `http://127.0.0.1:34778/`. The imported fixture was `.scratch/dioxus-frontend-v1/evidence/case-keymap-current/public/layered-fixture/vik-review-ui-export.boardstudio`, SHA-256 `952b26565a4df3ae1122fd0716017560279c938ee19fb6c15af966c88c1d35b0`.

On mounted `review/splitter-above` at Main board, `Circuit gnd host net` offered host `GND`. Selecting it and clicking `Copy circuit to PCB` produced the accepted `Remove copy` control; removing it returned `Copy circuit to PCB`. The final saved archive was exported through the candidate's Portable project UI after re-copying with GND selected.

For constituent placement, inspector HTML identified `J1004 · ~`, footprint `vik:vik-keyboard-connector-horizontal`, nested definition `vik:vik-splitter:pcb-vik-splitter-vik-splitter/component/4`. `Place` entered Layout placement mode and created J7; Enter on the focused layout canvas placed it at x=52.3875 mm, y=14.2875 mm. Board count changed 33→34. Undo changed it to 33; Redo restored 34 and J7. The mounted inspector was reopened afterward.

The exported archive's project JSON records splitter source `pcb/vik-splitter/vik-splitter.kicad_pcb`, SHA-256 `5f0ae2b03b296d7edae170a11d44920095c89d02eaff6819ad0470d2627b9a26`; six constituents (PART0, J1002, PART3, J1004, J1003, J1001); and 12 ports/nets. Copied circuit `circuit/embedded-61` is on Main board and maps its `gnd` net to existing `embedded/review/embedded-haptic/net:474e44` (GND). That saved host net includes copied J1002 pad-10, J1004 pad-1, J1003 pad-10, and J1001 pad-1. Saved J7 references the nested J1004 definition above.

Archive: `vik-splitter-gnd-join-ad26b07a.boardstudio`, SHA-256 `a12e79aa97b6170f4afff8140ab67eeedb6cf957e12c4447058dd011ba23634f`. It records project revision 13, Main board 40 parts (original 33 + J7 + six copied parts), and the temporary empty Board 2 used for the separate connector attempt.

Limit: the exported archive was structurally inspected but not re-imported for a full reload qualification. This receipt covers the splitter's explicit circuit join/removal and one nested constituent, not all 14 haptic components.

## Paired qualification limits

On pinned TS `http://127.0.0.1:5175/`, the paired public-UI actions completed in named isolated session `f55c02-ts-20261003-7dd31abc1fc4`:

- Parts source Inspector: selected Haptic DRV2605L `3V3 pullups, JP1 bridged`, with `Assign host connection` checked and Board 2 empty; `Attach module` added module `ui-0d56141a-ee87-4d9e-b4a3-69c33aafc0df` plus its J_VIK1 host connector. Changing module X to 1 mm and saving placement retained the connector and module.
- Mounted splitter-below/Main: selected GND on `Circuit gnd host net`; Copy changed to Remove copy, and Remove restored Copy.
- Parts source Inspector: splitter J1004 (`vik-keyboard-connector-horizontal`) Place created J7 on Main at x=52.3875 mm, y=14.2875 mm; one Undo returned the board part count from 34 to 33.

Candidate-only Dioxus evidence remains the GND join/removal and nested J1004 Place/Undo/Redo/archive state described above. The TS route for fresh automatic connector creation differs: it is available in the Parts source Inspector, while the mounted splitter placement Inspector exposes no connection-edit action on TS.

The repaired candidate `frontend-functional-controls-repair-20261003`, source `177f990b`, used the same imported fixture. Its `review/splitter-above` already had `review/splitter-above/vik-host-connector` (J_VIK1) globally on Main. Moving that mounted module to empty Board 2 and saving twice did not create a Board 2 connector: the saved relation still references that existing Main connector and Board 2 remained empty. This is a cross-board ownership case, not fresh materialization. The exported archive is `vik-splitter-connector-repair177f990b.boardstudio`, SHA-256 `6a5cf6ce26c433642f4513028ab3a17952964bf982c71af72a8215f05004eb30`.

Fresh Dioxus creation was not qualified: selecting the Haptic 3V3 module on Parts displayed the source preview and project module profile only; the full Parts page exposed no `Attach module` or host-assignment controls. Layout’s Add object → Parts offered `Browse all parts`, with no attach action. Board 2 remained empty, and no existing fixture part was deleted. Thus TS confirms the fresh-attach journey, but the public Dioxus route is unavailable in the packaged candidate.

The initial `ad26b07a` splitter connector failure (`This module has no source-backed horizontal VIK connector definition to place.`) was a real save bug: optional source-definition materialization was treated as missing even though the connector already existed. Source `7ddd10e3` repairs it. The fixture already contained that connector on Main, so the original attempt was not a fresh-creation test.

Final ownership check used candidate `frontend-module-ownership-20261003`, source `7ddd10e3`, and the saved cross-board archive above. On PCB with Board 2 selected, the mounted splitter canvas selector opened its placement Inspector. Before Save it reported: `The automatic connector ID is already used by a part outside this board. Choose a valid host connector or clear the connection.` Save rejected with `Choose a VIK host connector on this board, or clear the connection. This connector ID is missing, belongs to another board, or is not a host connector.` Board 2 remained at 0 parts; switching the board selector to Main showed 33 parts and J_VIK1 still present. Unchecking `Assign host connection` and saving the same placement completed with `Placement saved.`; Board 2 remained at 0 and Main retained all 33 parts including J_VIK1. Thus the foreign-connector guard rejects the invalid scope while a cleared connection permits the placement save.

No claim is made that F5.5-C02 is fully paired-qualified or closed. Circuit join/removal and one nested constituent have paired TS outcomes and candidate archive evidence; Dioxus now has the explicit foreign-connector rejection/clear-save qualification. Fresh Haptic automatic attachment is still unqualified on Dioxus: selecting the Haptic 3V3 source in Parts shows only source preview/profile, and Layout Add object → Parts offers Browse all parts without an attach action. No existing fixture parts were deleted. The current partial qualification covers neither all module constituents nor a full archive re-import after the final clear-connection save.
