# PCB07/08 public qualification — 2026-10-02

## Provenance

- Documentation/evidence branch: `codex/pcb-public-qualification-20261002`, based on root `dev` commit `5a472a9426e6e38993361da402cd4ec730feb369`.
- Dioxus candidate: `http://127.0.0.1:34731/`, integrated source/build stamp `792e88af` (coordinator-provided); isolated browser session `pcb-public-qualification-7dd31abc1fc4`.
- React reference: `http://127.0.0.1:5173/`, pinned commit `5a472a9426e6e38993361da402cd4ec730feb369`; isolated browser sessions `pcb-public-react-bbdc227a1db0` and `pcb-public-generic-bbdc227a1db0`.
- Original paired fixture: `layered-sofle-export.boardstudio`, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.
- React press recheck: isolated browser session `pcb-react-original-20261002-2318ea1a1376` re-imported that exact archive, showed `Left PCB · 70 parts`, selected `left-SW25`, chose Unassigned, then reloaded and verified Unassigned after returning to PCB. This replaces an earlier React press capture taken from the later 71-part derived J1 project; the corrected same-fixture captures are `react-press-unassigned.png` and `react-press-postreload.png`.
- The source app and candidate sessions used isolated browser storage. The original fixture was not overwritten.

## Sofle rotary encoder journey

The same fixture opened in both public apps. In **PCB**, selecting `left-SW25 · rotary encoder ec11 ec12` exposed `Press scan mode` with Direct GPIO selected and Unassigned available. Matrix key was disabled for this part. Both applications also displayed the same board-scoped terminal connections. The candidate note says rotation uses separate GPIOs and asks the user to apply the board wiring plan after changing the press connection.

Changing Direct GPIO to Unassigned updated the Inspector. Undo returned to Direct GPIO and Redo restored Unassigned. After a browser reload and reselection, both applications showed Unassigned. The candidate's post-reload screenshot and both Press screenshots are retained here.

Selecting built-in `left-keys-SW1` in both apps routed to the inherited Named terminals view and retained the **Edit board wiring** action; neither showed the generic Press editor. Selecting controller `left-U1` routed to the board Electrical wiring summary in both apps. It showed the current Matrix wiring mode; neither route treated the controller as a generic part.

## Packaged Ergogen binding journey

The original Sofle fixture has no generic `infused-kim/smd_0805` board member, so I created a separate derived fixture through the React UI: Parts → search `smd_0805` → **Place component**, then **Save project copy**. The added board part is `J1`, definition `ergogen:infused-kim/smd_0805`. The derived archive SHA-256 is `17abc9fc33f62e83557b6da5e1f2da1fde6e946c3194fe96e5351a88e802cd79`. Both applications opened this same archive in separate browser sessions.

Selecting `J1 · smd 0805` in each PCB Inspector showed the four `net_1_*` / `net_2_*` terminals and the packaged Ergogen net controls from `net_3_from` through `net_6_to`. The choices were scoped to the active Left PCB; both apps offered the same visible left-board nets. The checked-in packaged 36-source catalogue has no anchor-typed parameter, so no anchor control or anchor pairing is claimed. This agrees with the retained package evidence in `evidence/pcb08-implementation-20261002/implementation-status.md`.

I changed `net_3_from` from Default to `COLUMN_0` in both apps. Undo returned it to Default; Redo restored `COLUMN_0`. After reload, React showed `COLUMN_0`. The candidate's packaged schema/accepted projection took longer to settle: an early read briefly showed Default, then the control showed `COLUMN_0` after 2.5 seconds. I verified the candidate through its **Save .boardstudio project** export before and after reload:

| Candidate export | SHA-256 | Project revision | J1 generator parameters |
| --- | --- | ---: | --- |
| `candidate-before-reload.boardstudio` | `4a16702baf320272444cd444f7897d5319452c06945194c47fb3c3e9b227805c` | 17 | `net_3_from: COLUMN_0` |
| `candidate-after-reload.boardstudio` | `4a16702baf320272444cd444f7897d5319452c06945194c47fb3c3e9b227805c` | 17 | `net_3_from: COLUMN_0` |

The exported `project.json` confirms J1 ID `ui-b08a5e94-2888-4948-b78c-c195f961b1b0`, definition ID `ergogen:infused-kim/smd_0805`, and `generatorParameters: {"net_3_from":"COLUMN_0"}` in both archives. The files are byte-identical. The transient Default value was a load/selection settling window, not a project serialization loss; no persistence repair is proposed from this observation.

## Evidence files

- `candidate-press-unassigned.png`, `candidate-press-postreload.png`, `react-press-unassigned.png`, `react-press-postreload.png`
- `candidate-switch-inspector.png`, `react-switch-inspector.png`
- `candidate-controller-wiring.png`, `react-controller-wiring.png`
- `candidate-generic-default.png`, `react-generic-default.png`
- `candidate-postreload-binding-default.png` (early transient), `candidate-postreload-binding-column0.png`, `react-postreload-binding-column0.png`
- `candidate-parts-library.png`, `react-parts-catalog-smd0805.png`, `react-placement-mode.png`
- `paired-generic-smd0805.boardstudio`, plus candidate before/after reload archives

The React console contained only Vite connection and React DevTools informational messages. The Dioxus sessions reported no browser-console errors.

## Remaining acceptance gates

These journeys qualify only the controls exercised above. Delayed-schema stale selection/workspace rejection, theme parity, compact Inspector scrolling, keyboard/focus and accessibility checks remain open. PCB07/08, F5.1/F5.2/F5.3 and all 62 canonical parent acceptance joins remain open. Source checks from the reviewed `94480901` integration are reused as prior evidence; this turn adds browser evidence and does not claim new source checks.

No new refactoring takeaway observed. Carry forward RF-001/RF-006/RF-009 without modifying their ledger.
