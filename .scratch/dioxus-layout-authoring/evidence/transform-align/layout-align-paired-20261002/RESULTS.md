# Layout Align paired browser journey

Ticket SHA: `eb2586d81883c19d7576af8d994eabeda309045d6eb5c2f6d5537ec1149b26ea`  
Spec SHA: `b5582c2e4a3b829284f2de6337dd2dfc6bc82daa9a945064c849f2180ee7c1b5`  
Fixture: layered Sofle archive, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.

## React reference

Public app: `http://127.0.0.1:5175/`, isolated `agent-browser` session `layout-align-276a150779db`. Imported the saved archive above. Selected `Key 1.1 left-keys-SW1`; Align chose the first eligible reference, `left-diode-24-keys`. Before each command SW1 was `(9.18, -78.048)` and the reference was `(16.58, -79.548)`. Each command produced one accepted position change, left the reference unchanged, and one Undo restored SW1 to baseline before the next command.

| Command | SW1 after command (mm) | Reference after command (mm) |
|---|---:|---:|
| Left | `(24.98, -78.048)` | `(16.58, -79.548)` |
| Center X | `(16.58, -78.048)` | `(16.58, -79.548)` |
| Right | `(8.18, -78.048)` | `(16.58, -79.548)` |
| Top | `(9.18, -86.448)` | `(16.58, -79.548)` |
| Center Y | `(9.18, -79.548)` | `(16.58, -79.548)` |
| Bottom | `(9.18, -72.648)` | `(16.58, -79.548)` |

Screenshots: [React Align menu](react-align-menu.png), [React baseline](react-baseline.png). The browser JavaScript error buffer was empty after the journey.

## Corrected source verification

The corrected Align lifecycle source is commit `71075e71040728c19eab0d9aa0bedd487e3e133c` (after lifecycle repair `80ad6603` and feature leaf `533f9a50`). It preserves the chosen eligible reference when the workspace or current projection is hidden/unresolved, writes the preference only when it changes, and retires a completed old-scope request before checking the current scope's accepted revision.

- `cargo fmt --manifest-path web/Cargo.toml --all -- --check` passed.
- `cargo test --manifest-path web/Cargo.toml --locked --features page` passed (19 library, 23 binary, 10 integration tests). The mounted VirtualDom regression calls the production `reconcile_reference_choice` seam used by the Editor hook; it verifies empty-reference effect settling. Policy tests verify hidden-workspace preference retention and fallback only after ineligibility.
- Red-before-fix output is preserved in [reconcile-red.log](reconcile-red.log): restoring unconditional writes through the production reconciliation seam made the mounted effect run 30 times instead of once. [settlement-red.log](settlement-red.log) captures the lower-revision scope switch stranded as `WaitForAcceptedAdvance` when accepted-revision gating is evaluated before scope retirement. Both regressions pass with the corrected production seams.
- `cargo check --manifest-path web/Cargo.toml --locked --target wasm32-unknown-unknown --no-default-features --features page --bin boardstudio-web` passed.
- Strict all-target WASM Clippy passed with `-D warnings`.
- Non-test core-worker check passed. Exploratory core-worker-only tests stop on the existing optional-`sha2` import in `persistence_contract.rs`; this is outside the Align change.

The mounted lifecycle regression exercises the production reconciliation seam in a Dioxus `VirtualDom`; it does not mount the full application hook. The stale-scope regression exercises the extracted settlement gate used directly by the production controller. Full candidate browser behavior remains pending below.

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
