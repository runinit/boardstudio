# Independent verification — test boundaries

Fresh NORMAL verifier; 12snapshot files through Git at ad9db4730, workflow read separately. No tests/builds/writes. Refs14-17 verified as scoped SOURCE descriptions, not successful execution.

14 verified: host190-258 and mount_document269-291 in layout_field_preview_tests build own composition, use lifecycle!=Closed, no WorkbenchApp. WorkbenchApp constructs Runtime at presentation.rs283-288, contexts462-468. Runtime support open_document8752-8759 replaces document execution with in-process CoreEngine and memory persistence. Precision: new_runtime8708-8709 FIRST calls new_with_restoration(false), so do not claim fixture construction never creates browser infrastructure.

15 verified: PCB tests pcb_wiring/controller.rs1230-1267 host controlled WiringInput/channel resolver;1315-1392 failure/retry/old board;1396-1450 same-board token/revision/controller. Physical setup tests56-134,148-170 fixture controls preparation/stage/generation/owner;248-500 assertions. Placement part_placement.rs2194-2341 hook host;3336-3363 held-save/component unmount/VirtualDom drop/release/completion with no selected parts. Not WorkbenchApp teardown proof.

16 verified: layout_findings_state.rs487-626 handwritten DOM/own Enter listener; keycaps_workspace_tests.rs12-114 fixtures/118-160 header rendering with handcrafted view/no-op callbacks; canvas_navigation.rs312-446 real SVG but direct space_down and methods; durable_session.rs1267-1383 direct save acknowledgment;2014-2089 ReleasePointer/cancel effects;2122-2158 stale-preview suppression. Not trusted event/listener removal/browser capture/durable store proof.

17 verified: verify-update-adoption.py1655-1736 leg_layout_constraint ordinary import/form/toast/header/focus/held save/export/adoption/reopen. takeover2085-2135 explicitly deferred-and-cancelled attempt; comment2112-2113 excludes post-permit probe race. diagnostic_persistence.rs288-292 wraps store/controls;646-694 held save no loaded doc before release then saved name/revision;698-730 fail then retry;820-859 window controls. runtime.rs486-491 feature-gated composition. Script bodies do not establish package built/test passed.

Scoped full-composition statement verified: inspected scenarios do not JOINTLY assert WorkbenchApp drop451-459, Editor observer5218-5222/registration6207-6217 and Runtime destructor8013-8026. This is not repository-wide absence, teardown defect or browser-exit guarantee.

All locations at ad9db473012a4a4201217bd5094a4f24e7801932. Correct enclosing symbols and full citation forms are retained in analyzer-q4.md and main report. Corrections are range precision and initial Runtime-construction wording, not overturned conclusions.
