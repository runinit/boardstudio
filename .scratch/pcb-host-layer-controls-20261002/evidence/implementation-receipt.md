# PCB02 host layer controls — implementation receipt

Date: 2026-10-02

## Frozen source and accepted start boundary

- Source worktree: `/home/chris/.local/share/boardstudio/worktrees/pcb-host-layer-controls-20261002`.
- Source commit chain: `192dfe943c1147cdebc4efb3a5e7b62ad5668ec6` (initial controls), `9db88e8f8711c24e1db728208209d55879d018af` (component hook ownership, async inventory freshness/per-source errors, React groups and labels), and final `32a3b34b6cfbf7d112846b6418ea9d73c7f71b88` (generated Edge.Cuts visibility without board contours).
- Source base: `d585492012a60937879e89a8618c1d533fddba4e` on the isolated `codex/pcb-host-layer-controls-20261002` branch; root integration sources were not edited.
- Capability-start ticket: `.scratch/dioxus-pcb-view/issues/02-host-layer-controls.md`, file SHA-256 `638d93403ab07375af5c762ca7b92431fdfefb200daab448a84ee81d6e098f62` in docs commit `46acf0a7c4d797cf4dd64cf397ce789924b5e9ea`.
- Public audit: `.scratch/dioxus-pcb-view/evidence/public-pcb-audit-20261002/audit.md` at approved docs commit `46acf0a7c4d797cf4dd64cf397ce789924b5e9ea`, file SHA-256 `c9392452de5de5e47e1928048a729be686ace4d1c32a38a64a0562ba7f633600`.
- Baseline public comparison: React `5a472a9426e6e38993361da402cd4ec730feb369`; candidate build `frontend-command-icons-reuse-20261002`, source `8cfd6bb79e9e10b788e007fd428145b1e37095d1` at `http://127.0.0.1:34733/`. The baseline archive is `layered-sofle-export.boardstudio`, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.
- Start review: [SolPCB capability-start review](../../../../../reviews/sol-review-wave-20261002/pcb02-host-layer-start-review-46acf0a7-20261002.md), SHA-256 `55fd582634ab6dad9816175d1fde8ee46bea4678928975dca38e54e7ceef501e`.

The immediate implementation consumes accepted contours, board-member definitions, plated pads/drills, references, and emitted generator graphics. It discovers generator layer controls from the same cached `Graphic.layer` output used by the scene and resolves front/back pad and generated graphic layers against the part side. Through-hole pads remain while either copper side is shown; Pads and Holes remain independent. The canvas menu reuses the editor-owned transient `LayerVisibility.hidden` signal. Layout’s special Footprints toggle remains separately wired to its existing signal. No core model, Rust public API, generated contract, or provider category was widened, and no absent board-level copper/artwork row is fabricated.

The controls use the reference groups (`Copper`, `Technical`, `Objects`), visible labels (`Front copper`, `Back copper`, `Board outline`), accessibility layer IDs, and stable order. The `Edge.Cuts` row is backed by either accepted board contours or actual emitted generator graphics, and is deduplicated if both exist. The separate React Footprints/module row is not emitted because this host scene does not render that module geometry. The implementation does not claim the adjacent PCB toolbar, Add object, wiring Apply, selection/transform/snap parity, or the broader open F5 gates.

## Verification

- `cargo fmt --manifest-path web/Cargo.toml --check` — passed.
- `git diff --check` — passed.
- `cargo check --manifest-path web/Cargo.toml --locked --target wasm32-unknown-unknown --no-default-features --features page --bin boardstudio-web` — passed.
- `cargo clippy --manifest-path web/Cargo.toml --locked --target wasm32-unknown-unknown --no-default-features --features page --bin boardstudio-web -- -D warnings` — passed.
- Mounted headless Chrome tests on the actual `PcbScene` and production `PcbLayerControls` component: 3 passed, 0 failed. This verifies board-scope switching, exact row availability/visible labels, layer-specific pad visibility, through-hole behavior, independent Pads/Holes, outline/courtyard/reference toggles, Escape close and focus restoration.
- Focused WASM `layer` filter: 15 passed, 0 failed. This includes the mounted tests, front/back mapping, stale board/definition inventory rejection, per-generator failure isolation, generated Edge.Cuts without board contours, grouped row order, and adjacent existing layer tests.
- Two bounded expected-red mutants demonstrate the regressions: using the visible copper label as the accessible ID fails the `Hide F.Cu` row assertion; removing generated Edge.Cuts from the object inventory fails the no-outline Edge.Cuts assertion. Both logs are retained below.

Exact command output is retained in this directory:

| Log | SHA-256 |
| --- | --- |
| `mounted-layer-suite.log` | `75e1e30e28ed1a540fa0fbae3fe534e605058a7c63ed6ed0b12c17db88a81c1c` |
| `layer-controls-focused.log` | `2a8f7efad4b65539565163ae08ced0c865d6f68e05b5d60d34842b6aa75d5418` |
| `mounted-pcb-layers.log` | `d69f689f407d3275007388daecfd29350d75913f6f936d6194bc29a8e59c4014` |
| `catalogue-layer-inventory.log` | `6be40e58c2ddc75f713dbffaa0ded05d0546197f4883854c5e7fc074c428a3b6` |
| `layer-controls-final.log` | `eb174f34b9ac9f7c2d23743e4a0bd8238442ccfa14246ba3a489969f55283e09` |
| `board-scope-switch-red-green.log` | `53433dc9699bdd1504f4c1e1bbedf3e93cd328994f9fc66bd73da4f0c95b59f2` |
| `inventory-key-tests.log` | `a2b6b0819e2cf5ec2358fa88fd127fbfe37e08ddcf430e8a4db7117b87b414d9` |
| `visible-label-mutant-red.log` | `b04ca601e5a393121edc77099ef3d15e776b84fc7776b67d8363dc3532625c2f` |
| `generated-edgecuts-mutant-red.log` | `8710f24c7ee68161ae4b9812a6d71a91c77d10f8900f313e52cde487e07f9ffa` |
| `layer-controls-repair.log` (initial nested-hook test-fixture failure; not a product red) | `34602874204438fa8bbc07d181e494ec35750177bb91a9190a2a7cd6f67514d2` |

The final focused filter was `layer`; it ran 15 tests, including the production component board-switch journey and inventory safety tests. The initial board-switch test attempt had a test-fixture-only nested-hook panic from constructing signals inside a context-provider closure; the fixture was corrected to create signals before registering the provider. `layer-controls-repair.log` preserves that failed fixture run and is explicitly not product failure evidence. The two mutant runs are the expected-red regressions; `layer-controls-final.log` is the authoritative passing run.

## Remaining gates

SolPCB re-review is requested for exact source commit `32a3b34b6cfbf7d112846b6418ea9d73c7f71b88`. Root integration, full packaged build, actual packaged compact/desktop geometry, public paired fixture journey, visibility retention across workspace/board switches, parent acceptance, RF handoff, and provider-backed board-level copper/technical categories remain open. The source tests mount the production PCB scene/control components in a test composition; they do not claim a packaged editor journey.
