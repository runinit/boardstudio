# Layout Align paired browser journey

Ticket SHA: `eb2586d81883c19d7576af8d994eabeda309045d6eb5c2f6d5537ec1149b26ea`  
Spec SHA: `b5582c2e4a3b829284f2de6337dd2dfc6bc82daa9a945064c849f2180ee7c1b5`  
Fixture: layered Sofle archive, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.

## Historical React capture (5175; cached page, freshness unverified)

The earlier capture used `http://127.0.0.1:5175/`, isolated `agent-browser` session `layout-align-276a150779db`. It imported the saved archive above. The listener was absent and browser page was cached, so retain its screenshots/notes as historical only; the freshness-verified comparison is below. Selected `Key 1.1 left-keys-SW1`; Align chose the first eligible reference, `left-diode-24-keys`. Before each command SW1 was `(9.18, -78.048)` and the reference was `(16.58, -79.548)`. Each command produced one accepted position change, left the reference unchanged, and one Undo restored SW1 to baseline before the next command.

| Command | SW1 after command (mm) | Reference after command (mm) |
|---|---:|---:|
| Left | `(24.98, -78.048)` | `(16.58, -79.548)` |
| Center X | `(16.58, -78.048)` | `(16.58, -79.548)` |
| Right | `(8.18, -78.048)` | `(16.58, -79.548)` |
| Top | `(9.18, -86.448)` | `(16.58, -79.548)` |
| Center Y | `(9.18, -79.548)` | `(16.58, -79.548)` |
| Bottom | `(9.18, -72.648)` | `(16.58, -79.548)` |

Screenshots: [React Align menu](react-align-menu.png), [React baseline](react-baseline.png). The browser JavaScript error buffer was empty after the journey.


## Fresh React oracle (5173; paired comparison)

Public app: `http://127.0.0.1:5173/`, fresh named browser session `layout-align-react-live-276a150779db`. The page was served by Vite PID `32534` (`node .../vite.js --host 127.0.0.1`) with working directory `/home/chris/01_Projects/ts-boardstudio2/app`; repository HEAD was exactly `5a472a9426e6e38993361da402cd4ec730feb369`, matching the ticket's pinned React commit. The browser loaded live `/@vite/client`, `/src/main.tsx`, React refresh, and project modules; `navigator.serviceWorker.controller` was null. Root separately verified the relevant TypeScript source blobs against the pin. Imported the exact fixture above (SHA-256 `5b17071a...0776df`) and selected `Key 1.1 left-keys-SW1` on Left PCB. Default reference was `left-diode-24-keys` at `(16.58, -79.548)`. Before each command the target was `(9.18, -78.048)`; each action produced one accepted position change, left the reference unchanged, and Undo restored baseline.

| Command | SW1 after command (mm) | Reference after command (mm) |
|---|---:|---:|
| Left | `(24.98, -78.048)` | `(16.58, -79.548)` |
| Center X | `(16.58, -78.048)` | `(16.58, -79.548)` |
| Right | `(8.18, -78.048)` | `(16.58, -79.548)` |
| Top | `(9.18, -86.448)` | `(16.58, -79.548)` |
| Center Y | `(9.18, -79.548)` | `(16.58, -79.548)` |
| Bottom | `(9.18, -72.648)` | `(16.58, -79.548)` |

Bottom Undo returned `(9.18, -78.048)`; Redo restored `(9.18, -72.648)`, and reload retained that position and the reference. `agent-browser errors` returned no entries. Console output contained Vite-connected and React DevTools informational messages only. Fresh screenshots: [Align menu](react-live-5173-align-menu.png), [Bottom](react-live-5173-bottom.png), and [Bottom after Redo](react-live-5173-bottom-redo.png). This fresh oracle matches the Dioxus results above exactly; the 5175 material remains separately marked historical.

## Corrected source verification

The corrected Align lifecycle source is commit `71075e71040728c19eab0d9aa0bedd487e3e133c` (after lifecycle repair `80ad6603` and feature leaf `533f9a50`). It preserves the chosen eligible reference when the workspace or current projection is hidden/unresolved, writes the preference only when it changes, and retires a completed old-scope request before checking the current scope's accepted revision.

- `cargo fmt --manifest-path web/Cargo.toml --all -- --check` passed.
- `cargo test --manifest-path web/Cargo.toml --locked --features page` passed (19 library, 23 binary, 10 integration tests). The mounted VirtualDom regression calls the production `reconcile_reference_choice` seam used by the Editor hook; it verifies empty-reference effect settling. Policy tests verify hidden-workspace preference retention and fallback only after ineligibility.
- Red-before-fix output is preserved in [reconcile-red.log](reconcile-red.log): restoring unconditional writes through the production reconciliation seam made the mounted effect run 30 times instead of once. [settlement-red.log](settlement-red.log) captures the lower-revision scope switch stranded as `WaitForAcceptedAdvance` when accepted-revision gating is evaluated before scope retirement. Both regressions pass with the corrected production seams.
- `cargo check --manifest-path web/Cargo.toml --locked --target wasm32-unknown-unknown --no-default-features --features page --bin boardstudio-web` passed.
- Strict all-target WASM Clippy passed with `-D warnings`.
- Non-test core-worker check passed. Exploratory core-worker-only tests stop on the existing optional-`sha2` import in `persistence_contract.rs`; this is outside the Align change.

The mounted lifecycle regression exercises the production reconciliation seam in a Dioxus `VirtualDom`; it does not mount the full application hook. The stale-scope regression exercises the extracted settlement gate used directly by the production controller. The paired public candidate journey is recorded below; the acceptance audit distinguishes this proven key/cell scope from wider F3.3c cases that remain open.

## Dioxus integrated candidate

Candidate: `frontend-layout-align-wave-20261002`, source commit `14d1bfeb5228c7a82f3be7ac7fa322ea61b30b02`, public subpath `http://127.0.0.1:34721/boardstudio/`. Root’s packaging gate reported 22/22 commands, 1,255 source hashes with zero mismatches, and 145 assets verified at both root and subpath. Browser session: `layout-align-fixed-276a150779db`. Imported the exact archive and selected `Key 1.1 left-keys-SW1` on Left PCB. Initial target was `(9.18, -78.048)`; retained reference was `matrix/left-keys/r0c0/diode` (`left-diode-24-keys`) at `(16.58, -79.548)`. Each command below started from that baseline, produced one accepted move, kept the reference ID unchanged, and Undo returned the target to baseline before the next command.

| Command | SW1 after command (mm) | Undo returned to baseline |
|---|---:|---|
| Left | `(24.98, -78.048)` | yes |
| Center X | `(16.58, -78.048)` | yes |
| Right | `(8.18, -78.048)` | yes |
| Top | `(9.18, -86.448)` | yes |
| Center Y | `(9.18, -79.548)` | yes |
| Bottom | `(9.18, -72.648)` | yes |

Bottom was then undone to `(9.18, -78.048)` and redone to `(9.18, -72.648)`. After waiting for Saved, page reload restored revision 30 and the same target coordinate; selecting SW1 again showed the same retained reference ID. Switching Board to Right PCB produced no selection and a disabled Align control with “No independent reference parts”; switching back to Left PCB and selecting SW1 restored `(9.18, -72.648)` and the same reference ID. `agent-browser errors` and `agent-browser console` returned no entries; the page-visible error search was empty.

Screenshots: [Dioxus Align menu](dioxus-align-menu.png), [baseline](dioxus-baseline.png), [Bottom](dioxus-bottom.png), [Bottom after Redo](dioxus-bottom-redo.png), and [after reload](dioxus-after-reload.png). The browser journey is complete for the paired public behavior; the mounted VirtualDom test remains limited to the extracted production reconciliation seam rather than the full Editor hook.

## Remaining reviewed-ticket acceptance audit

This is a bounded acceptance receipt for the paired key/cell journey, not closure of all F3.3c criteria or parent F3.3/F3.7.

| Reviewed criterion | Source evidence | Public evidence in this run | Disposition |
|---|---|---|---|
| Six one-shot anchors from a selected matrix key/cell; reference stays fixed; Undo each, plus final Redo/reopen | `layout_align_controller.rs::make_edit` uses `MoveParts` for `TreeContext::Key`; `layout_align_geometry.rs` computes transformed AABB anchor deltas | Same archived fixture in React and Dioxus; all six key results, reference identity, Undo each, Bottom Undo/Redo/reload recorded above | **Proved for key/cell context** |
| Standalone real-part, whole matrix, and column targets | Source has `Component`→`MoveParts`, `Matrix`→`SetMatrix` origin, `Column`→`SetMatrix` local offset; unit test covers rotated/mirrored/splayed local delta | Not exercised in either public journey | **Not publicly proved** |
| Row target disablement and feedback | `project` explicitly disables `TreeContext::Row` and `make_edit` rejects Row | Not exercised | **Source-backed only** |
| Rotated courtyard / switch keycap envelope behavior | `transformed_envelope` uses four accepted rotated corners; exact keycap override and unsupported/nonfinite inputs have native unit tests | The chosen moving key is a matrix key; the reference was a diode. No rotated free-part or keycap-envelope public comparison | **Source/unit-tested; public parity open** |
| Linked partner exclusion across whole mirrored matrix, including primary component vs assembly-child exception | `project` obtains partner matrix `part_ids`; `linked_selection` excludes for non-component scopes and primary matrix components, but returns false for assembly components | No linked-layout fixture journey | **Source-backed only** |
| No reference / cross-board isolation | Same-board `references` are drawn only from selected board; empty list disables picker/actions with “No independent reference parts” | Right PCB switch showed disabled Align/no reference; return to Left PCB restored target and chosen diode reference | **Proved for this board switch and empty-reference state** |
| Changed/deleted reference fallback; no selection; locked mover; driven target; locked fixed reference | Projection has eligibility fallback and checks moving locks/constraint targets; references are not filtered by lock, preserving fixed locked-reference eligibility | No public mutation/selection/lock/constraint scenario exercised | **Source-backed only; public cases open** |
| Stale admission after revision/scope/workspace/context changes; exact outcome settlement | `on_align` revalidates workspace, full scope/generation, selected IDs, reference, token/revision, accepted/saved lifecycle and constraints; pending outcome is keyed to captured target/reference/command; production settlement gate covered by red/green regression | Normal accepted actions, hidden popover/reload, and board owner switch exercised; no forced stale callback race, target deletion, or workspace-change-during-submit | **Source/regression-backed; adversarial public race open** |
| Actual edit/history identity and browser errors | Controller submits one `Event::Edit` with matrix `SetMatrix` or key `MoveParts`, sets exact `target_ids`, registers outcome before submit, and checks accepted expected values | Visible accepted revisions advanced once per command; Undo restored coordinates; browser errors/console empty. Browser instrumentation did not capture serialized Core operation or changed-ID list | **Behavior proved; exact operation/changed-ID trace not captured in browser** |
| RF-005 ownership and wider parent acceptance | No new RF ID; source keeps matrix-local SetMatrix distinct from world MoveParts | This child run does not accept F3.3/F3.7 or close parent joins | **Keep parent and remaining acceptance open** |

The eligible-reference list is built from active-board `part_ids`, minus current moving IDs and (when React's linked-selection rule applies) the entire linked partner matrix. The source review approved that policy, but a browser fixture with a linked mirrored pair is still needed to turn that static/source result into public acceptance evidence.
