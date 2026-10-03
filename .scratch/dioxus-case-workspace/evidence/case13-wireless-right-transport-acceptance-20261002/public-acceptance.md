# Case13 Right wireless transport and battery acceptance

Date: 2026-10-02
Scope: bounded paired public check of accepted wireless transport, battery editing/history, archive save/reopen and per-instance projection. This does not claim Case parent closure or Case11 manual preview-regeneration acceptance.

## Sources and fixture

- Candidate: `http://127.0.0.1:34739/`, source commit `a8fd8988f649c70c11393066314095447535dce5`, build `frontend-chooser-pcb-navigation-integrated-20261002`. Verified artifact provenance is `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/web/target/builds/frontend-chooser-pcb-navigation-integrated-20261002/provenance.json`, SHA-256 `183c7cd7dd133c6cdaf9c423d49877e37b1bfbe0ed2469e85a26fb02dd048c52`.
- React reference: `http://127.0.0.1:5173/`, pinned source `5a472a9426e6e38993361da402cd4ec730feb369`.
- Both began with the same `layered-sofle-export.boardstudio`, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.
- Browser profiles were isolated and named: `case13_wireless_candidate_34739`, `case13_wireless_react_5173`, `case13_wireless_candidate_reopen_34739`, and `case13_wireless_react_reopen_5173`.

## Paired interaction

In each original session, selected Right PCB/right half and Case → Right case assembly → Configure mechanical stack → Assembly setup. Changed Half connection from Wired to “Wireless · local battery on each half.” Candidate displayed “Physical setup saved.” Its battery branch immediately matched the accepted React wireless branch: the wired-only Include checkbox disappeared, wireless guidance appeared, and all eight numeric fields were present. The unflipped Right defaults were width/depth/height 30/20/6, cable width 2, position (251.0183896, −57.5529677), and cable exit (268.0183896, −57.5529677), preserving the +17 mm X offset.

Edited Width from 30 to 31.5 mm through the visible numeric field using keyboard input and Tab blur. In the candidate, Undo restored 30 and Redo restored 31.5 while wireless remained selected. React showed the same 30→31.5→30→31.5 history. Both projects were then saved through Export → Save .boardstudio project.

The candidate saved archive is `candidate-wireless-right-width-31_5.boardstudio`, SHA-256 `f25b923da8cab2f42c6ac1464011addb23b03c36b352bc8cc24393fd08b05235`. React's paired archive is `react-wireless-right-width-31_5.boardstudio`, SHA-256 `2afe0cb4f33af3711c9377c2bf5d7bc8dc197ee697960b380c2e033d652163c8`. Their embedded `project.json` values are captured in `payload-comparison.txt`.

Both archives contain `hardware.transport = wireless`. The candidate is revision 18 and React revision 16. Each stores Right battery width 31.5, depth 20, height 6, cable width 2, position X/Y 251.0183896/−57.5529677 and cable exit X/Y 268.0183896/−57.5529677. Both retain `instances[id=left].mechanical = null`; the changed battery resides on Right.

Imported each saved archive into its own fresh named profile and reopened Case → Right. Both restored wireless transport, guidance and all eight fields, including Width 31.5. Returning to Left in the original profiles showed width 30 and its own position (71.1045191, −53.658), cable exit (88.1045191, −53.658), with cable width 2 and the other dimensions unchanged at 20/6. The Right width edit did not appear on Left. The imported archive payloads and fresh-session UI agree.

## Retained captures

- `candidate-right-wireless-width-31_5.png` and `react-right-wireless-width-31_5.png` show the accepted Right wireless projection after the edit.
- `candidate-left-instance.png` and `react-left-instance.png` show the unchanged Left projection.
- `payload-comparison.txt` records the concise accepted archive state for both implementations.
- `SHA256SUMS` records the fixture, provenance file, screenshots and saved archives.

## Limits

This verifies only the exercised Case13 transport/battery path. It does not test battery geometry export, warnings acceptance, generated-case regeneration after the Case11 live-preview repair, every assembly method, or full Case/F7 parent closure. No source or build changes were made for this QA.
