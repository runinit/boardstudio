# PCB Wiring and selected-switch Inspector audit

Date: 2026-10-02. This is source and UI exploration for the next F5 child; it does not change canonical task status or accept F5.1/F5.2.

## Sources pinned

- React reference checkout: `/home/chris/01_Projects/ts-boardstudio2` at `5a472a9426e6e38993361da402cd4ec730feb369`.
- PCB author worker: `codex/frontend-pcb-scene-20261002` at source correction `590140b25517dc2bbf8f0d449b45e568afef8237`; selected-board scene leaf exists but awaits root registration/mount and public pairing.
- Integration checkout read-only during this audit: `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001` at `9602017d4a5fe3a1789358adec1f39536daa2c7f`; `web/src/presentation.rs` blob SHA-1 `041d2bfc75246d7a92347f0eae4163acae3fc27b` still contains the explicit PCB placeholder at line 2266. The checkout has unrelated untracked Case evidence, which was left untouched.
- Existing PCB child tickets `.scratch/dioxus-pcb-view/issues/01-selected-board-host-scene.md` and `02-host-layer-controls.md` are unchanged. The proposed Wiring/context child is a draft, not yet published.

## Paired browser observation

An isolated `agent-browser` session named `pcb-wiring-27b06bfce9c3` opened the pinned React app at `http://127.0.0.1:5175/`, then the Sofle v2 demo and its PCB tab. With no individual part selected, the Inspector exposes the `Electrical wiring` region: Matrix mode, Resolve automatically, Apply wiring, assignments, current row pins (P5–P9), column pins and peripherals. The snapshot and screenshot are `react-sofle-wiring-panel.txt` and `.png`.

Selecting `left-keys-SW1` changes the same Inspector to a switch-specific view. It reports the switch wiring is inherited from its key assembly, shows a `Named terminals` section, and lists `from → LINK_MATRIX_LEFT-KEYS_ROW0` and `to → COLUMN_0`; it offers `Edit board wiring`, which clears the component selection. The snapshot and screenshot are `react-sofle-selected-switch.txt` and `.png`. This confirms the component branch is a read-only projection of the accepted board nets; it is not a second wiring planner.

The candidate at `http://127.0.0.1:34689/` was explored in the same isolated session after the React captures. It showed the common PCB route with an explicit `PCB` heading and `Back to Layout`, but no scene or PCB Inspector controls. Its serving build identity was not established, so this is an exploratory observation only; the pinned integration source separately confirms the placeholder. Candidate snapshot and screenshot: `candidate-pcb-current.txt` and `.png`.

The browser actions changed only local demo/workspace/selection state. No net, mode, lock, plan, edit, undo/redo, or save action was invoked.

## Source-backed behavior and capability

- React `app/src/ui/Workbench.tsx:1000` constructs `usePcbWorkspace` from the accepted project, current board, active selected part and definitions; `:1027` chooses the PCB panel. `usePcbWorkspace.tsx:51-57` routes empty selection and controller selection to `WiringPanel`, and built-in switches to the inherited wiring/named-terminal Inspector. Its named-terminal value comes from the first selected-board net containing a pin whose `partId` is the active switch and `padId` belongs to that terminal.
- `useElectricalPlanning.ts:26-45,47-52` sends `ResolveElectrical` from the current document and board configuration, automatically refreshes on accepted project/board changes, and suppresses publication when the captured document or current board changes. The PCB resolver call uses `instanceId = null`; physical-instance electrical projection remains distinct and is not inferred from the current PCB selection.
- `useElectricalPlanning.ts:79-96` derives controller options from board-member parts with a controller definition or controller MCU profile. The current accepted plan carries `boardId`, `revision`, `controllerPartId`, `mode`, row/column/direct assignments, peripheral pins/terminals, free pins and diagnostics. `electricalPlanContext.ts` requires current document identity, revision and board for display, and derives applied state from generated nets plus saved mode/controller config.
- React `WiringPanel.tsx:36-49` renders the Wiring heading/controller/mode, optional no-controller notice, used/free pins, Resolve/Apply actions, diagnostics, and assignment rows. Assignment controls, lock/review, Apply, protected-remap review, connection replacement and embedded firmware controls belong to later editing/handoff slices unless explicitly implemented through the existing guarded actions; this first read/preview slice must not fake enabled actions.
- Existing Rust Core capability is direct: `core/src/model.rs:1539-1552` defines `ResolveElectrical`/`ApplyElectrical`; `core/src/electrical.rs:796-913` builds a plan from document, board, mode, locks and controller; `core/src/lib.rs:267-300` handles resolve and requires revision/fingerprint consistency for apply. The web host already has `CoreWorker::request` (`web/src/host/core_client.rs`) and `Runtime` owns a CoreWorker. There is no PCB resolver UI/controller in the page Runtime today. A private Runtime or root-owned controller adapter is still necessary; this is not a missing Core API and does not require waiting for INT.2 acceptance.
- The scene leaf’s `PcbPartHit` carries the full `Scope`, accepted token, root generation and real part ID. Root must validate current epoch/document/board membership before updating the existing Session selection/context. The Wiring leaf must consume that same accepted selection and not introduce a second writable selection or plan authority.

## Proposed next vertical increment

Draft ticket `../drafts/F5.2a-wiring-preview-and-switch-context.md` isolates one read-only PCB Inspector workflow: resolve and present the selected board’s current electrical plan, and switch the Inspector to the accepted selected switch’s named-terminal/net projection. It deliberately leaves Apply, mode/controller/lock/pin edits, net mutation, protected handoff/remap and firmware controls in the retained F5.2/F5.3 joins. It does not duplicate host scene or layer-control issues 01/02.

The start edge is the real mounted F5.1a board/scope/part-selection callback plus the existing Runtime/CoreWorker adapter contract. It does not require the host layer ticket F5.1b, the complete F5.1 parent, INT.2 completion, F4/F6 completion, or F7. The canonical F5.2 `INT.2` acceptance join and every F5.2/F5.3 parent criterion stay unchanged.

## RF handoff

No new RF ID is justified. Preserve RF-009’s exact source/build/fixture accounting. The scene Standards source review also found that each selection repaint currently rebuilds membership/part projection data and clones the owned `FootprintGraphics` definition/parameter inputs. Root triaged broad projection/definition memoization as an unmeasured RF-001 risk, not a current assembly blocker. The existing `FootprintGraphics` owned-prop boundary is the mitigation/rationale for leaving those copies alone beyond an obvious duplicate clone; do not call this a performance pass. Measure selection-driven rerender allocation/frequency against a real multi-part board before any broader memoization or interface change. RF-001’s machine and readable entries are updated with this handoff.

No Cargo, candidate build, or public acceptance run was performed in this task. Existing paired screenshots are source/UI references, not full F5 evidence.
