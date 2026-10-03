# PCB02 host layer controls — implementation receipt

Date: 2026-10-02

## Frozen source and accepted start boundary

- Source worktree: `/home/chris/.local/share/boardstudio/worktrees/pcb-host-layer-controls-20261002`.
- Implementation commit: `192dfe943c1147cdebc4efb3a5e7b62ad5668ec6` (`pcb: add accepted host layer controls`).
- Source base: `d585492012a60937879e89a8618c1d533fddba4e` on the isolated `codex/pcb-host-layer-controls-20261002` branch; root integration sources were not edited.
- Capability-start ticket: `.scratch/dioxus-pcb-view/issues/02-host-layer-controls.md`, file SHA-256 `638d93403ab07375af5c762ca7b92431fdfefb200daab448a84ee81d6e098f62` in docs commit `46acf0a7c4d797cf4dd64cf397ce789924b5e9ea`.
- Public audit: `.scratch/dioxus-pcb-view/evidence/public-pcb-audit-20261002/audit.md`, file SHA-256 `8a57141ecbf9c51a4d52abd825eef3c268ea35584db0a5dd03be66e9c965039a`.
- Baseline public comparison: React `5a472a9426e6e38993361da402cd4ec730feb369`; candidate build `frontend-command-icons-reuse-20261002`, source `8cfd6bb79e9e10b788e007fd428145b1e37095d1` at `http://127.0.0.1:34733/`. The baseline archive is `layered-sofle-export.boardstudio`, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.
- Start review: [SolPCB capability-start review](../../../../../reviews/sol-review-wave-20261002/pcb02-host-layer-start-review-46acf0a7-20261002.md), SHA-256 `55fd582634ab6dad9816175d1fde8ee46bea4678928975dca38e54e7ceef501e`.

The immediate implementation consumes accepted contours, board-member definitions, plated pads/drills, references, and emitted generator graphics. It discovers generator layer controls from the same cached `Graphic.layer` output used by the scene and resolves front/back pad and generated graphic layers against the part side. Through-hole pads remain while either copper side is shown; Pads and Holes remain independent. The canvas menu reuses the editor-owned transient `LayerVisibility.hidden` signal. Layout’s special Footprints toggle remains separately wired to its existing signal. No core model, Rust public API, generated contract, or provider category was widened, and no absent board-level copper/artwork row is fabricated.

The controls use the reference labels and flat row order (`B.Cu`, `F.Cu`, generated source layers, `Edge.Cuts`, then supported host objects); the separate React Footprints/module row is not emitted because this host scene does not render that module geometry. The implementation does not claim the adjacent PCB toolbar, Add object, wiring Apply, selection/transform/snap parity, or the broader open F5 gates.

## Verification

- `cargo fmt --manifest-path web/Cargo.toml --check` — passed.
- `git diff --check` — passed.
- `cargo check --manifest-path web/Cargo.toml --locked --target wasm32-unknown-unknown --no-default-features --features page --bin boardstudio-web` — passed.
- `cargo clippy --manifest-path web/Cargo.toml --locked --target wasm32-unknown-unknown --no-default-features --features page --bin boardstudio-web -- -D warnings` — passed.
- Mounted headless Chrome test on the actual `PcbScene` and shared menu: 2 passed, 0 failed. This verifies exact row availability, layer-specific pad visibility, through-hole behavior, independent Pads/Holes, outline/courtyard/reference toggles, Escape close and focus restoration.
- Focused WASM `layer` filter: 11 passed, 0 failed. This includes the above mounted tests, front/back mapping tests, and adjacent existing layer tests.

Exact command output is retained in this directory:

| Log | SHA-256 |
| --- | --- |
| `mounted-layer-suite.log` | `75e1e30e28ed1a540fa0fbae3fe534e605058a7c63ed6ed0b12c17db88a81c1c` |
| `layer-controls-focused.log` | `2a8f7efad4b65539565163ae08ced0c865d6f68e05b5d60d34842b6aa75d5418` |
| `mounted-pcb-layers.log` | `d69f689f407d3275007388daecfd29350d75913f6f936d6194bc29a8e59c4014` |
| `catalogue-layer-inventory.log` | `6be40e58c2ddc75f713dbffaa0ded05d0546197f4883854c5e7fc074c428a3b6` |

The focused filter was `layer`; it ran 11 tests, including 5 new PCB layer tests and 6 existing layer-related tests. The `mounted-layer-suite.log` is the exact final code run after label parity and Escape/focus assertions. The shorter earlier logs are retained as historical runs, not substitutes for the final suite.

## Remaining gates

SolPCB source review is requested for the exact frozen commit above. Root integration, full packaged build, actual packaged compact/desktop geometry, public paired fixture journey, visibility retention across workspace/board switches, parent acceptance, RF handoff, and provider-backed board-level copper/technical categories remain open. The source tests mount the production PCB scene/control components in a test composition; they do not claim a packaged editor journey.
