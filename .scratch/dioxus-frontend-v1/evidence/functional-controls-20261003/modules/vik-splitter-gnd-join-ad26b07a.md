# VIK splitter circuit join and constituent Place qualification

Candidate: `frontend-functional-controls-20261003`, source `ad26b07a`, served at `http://127.0.0.1:34778/`. The imported fixture was `.scratch/dioxus-frontend-v1/evidence/case-keymap-current/public/layered-fixture/vik-review-ui-export.boardstudio`, SHA-256 `952b26565a4df3ae1122fd0716017560279c938ee19fb6c15af966c88c1d35b0`.

On mounted `review/splitter-above` at Main board, `Circuit gnd host net` offered host `GND`. Selecting it and clicking `Copy circuit to PCB` produced the accepted `Remove copy` control; removing it returned `Copy circuit to PCB`. The final saved archive was exported through the candidate's Portable project UI after re-copying with GND selected.

For constituent placement, inspector HTML identified `J1004 · ~`, footprint `vik:vik-keyboard-connector-horizontal`, nested definition `vik:vik-splitter:pcb-vik-splitter-vik-splitter/component/4`. `Place` entered Layout placement mode and created J7; Enter on the focused layout canvas placed it at x=52.3875 mm, y=14.2875 mm. Board count changed 33→34. Undo changed it to 33; Redo restored 34 and J7. The mounted inspector was reopened afterward.

The exported archive's project JSON records splitter source `pcb/vik-splitter/vik-splitter.kicad_pcb`, SHA-256 `5f0ae2b03b296d7edae170a11d44920095c89d02eaff6819ad0470d2627b9a26`; six constituents (PART0, J1002, PART3, J1004, J1003, J1001); and 12 ports/nets. Copied circuit `circuit/embedded-61` is on Main board and maps its `gnd` net to existing `embedded/review/embedded-haptic/net:474e44` (GND). That saved host net includes copied J1002 pad-10, J1004 pad-1, J1003 pad-10, and J1001 pad-1. Saved J7 references the nested J1004 definition above.

Archive: `vik-splitter-gnd-join-ad26b07a.boardstudio`, SHA-256 `a12e79aa97b6170f4afff8140ab67eeedb6cf957e12c4447058dd011ba23634f`. It records project revision 13, Main board 40 parts (original 33 + J7 + six copied parts), and the temporary empty Board 2 used for the separate connector attempt.

Limit: the exported archive was structurally inspected but not re-imported for a full reload qualification. The separate auto-host-connector attempt on `review/splitter-above` moved to an empty Board 2 and failed its first Save with `This module has no source-backed horizontal VIK connector definition to place.`; no connector was created, so its second Save remains unqualified on this candidate. This receipt covers the splitter's explicit circuit join/removal and one nested constituent, not all 14 haptic components.

## Paired qualification limits

On pinned TS `http://127.0.0.1:5175/`, the same haptic module’s Parts inspector controls were inspected: VIK connection assignment, 12 circuit host-net selectors with `GND`, Copy/Remove copy, and individual `Place` controls including nested U1. No TS connection, circuit, or constituent edit was made, so those are control-existence observations only, not paired action results. The Dioxus GND join and J1004 constituent outcomes above are candidate-only archive evidence.

The candidate’s first auto-connector Save on `review/splitter-above` after selecting the empty temporary Board 2 and `Add source-backed horizontal VIK connector beside module` failed with `This module has no source-backed horizontal VIK connector definition to place.` No connector was created, so a second Save could not be attempted. The next repaired candidate must qualify first and second Save there. No claim is made that F5.5-C02 is paired-qualified or closed from this candidate-only evidence; current support is source-based implementation plus partial Dioxus qualification, with TS actions and connector Save still open.
