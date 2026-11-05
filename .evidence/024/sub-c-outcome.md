# 024 sub-lane C outcome — PARTIAL / NOT READY FOR INTEGRATION

Initial clean branch `lane/024c-rest-migrate` at `64763a392dd32964db9d3a6316aa62be334d3e59`. No push. No CSS or ticket edits in this worktree. The parent branch was still at this base when this record was prepared.

## Change

- Removed 29 class uses of 26 names with no CSS rule, plus dead Library and PCB scene marker classes exercised in focused mounted tests. Migrated 21 uses of 18 direct rules to existing utilities/theme tokens; also migrated PCB visually-hidden label and Keymap editor-group spacing. No shared component APIs or shape changed.
- Replaced Library's search-focus and Parts carousel JS selectors with accessible-name-based selectors. Migrated Library tests, PCB geometry-layer visibility tests (using semantic `data-layer`), two shared dock styling queries, and two PCB/Parts test queries away from legacy selectors. No test names were renamed.
- **Incomplete:** 159 distinct live legacy names remain in owned paths and have not all been adjudicated. `.evidence/024/sub-c-needs-home.md` lists their exact reference/rule locations and properties, plus 13 non-class/unclassified candidates. Do not assume these 159 are all genuinely without utility equivalents. The UI-shared project-menu close selector and Parts contextual controller-picker variant require coordinated follow-up. No request for the integrator to delete CSS until the remaining uses are handled.

## Inventory of changed source files

Counts are matches of the requested quoted-start (`"m1-*`/`"board-*`) or dotted-selector regex per file, not distinct classes; they include IDs/domain values and exclude a second class in one quoted string. Thus an unchanged count can still include a removed second class. Evidence docs are not source files.

| File | Before | After |
|---|---:|---:|
| `web/crates/keycaps/src/keycaps_scene.rs` | 6 | 5 |
| `web/crates/keymap/src/keymap/canvas.rs` | 9 | 8 |
| `web/crates/keymap/src/keymap/panel.rs` | 25 | 20 |
| `web/crates/library/src/library.rs` | 118 | 72 |
| `web/crates/parts/src/parts.rs` | 15 | 8 |
| `web/crates/parts/src/parts/details.rs` | 4 | 3 |
| `web/crates/parts/src/parts/modules_catalogue.rs` | 7 | 2 |
| `web/crates/parts/src/parts/modules_catalogue/module_attachment.rs` | 1 | 0 |
| `web/crates/parts/src/parts/preview.rs` | 24 | 21 |
| `web/crates/pcb/src/pcb_board_reference.rs` | 7 | 6 |
| `web/crates/pcb/src/pcb_module_inspector.rs` | 18 | 8 |
| `web/crates/pcb/src/pcb_physical_setup/controller.rs` | 7 | 4 |
| `web/crates/pcb/src/pcb_scene.rs` | 28 | 10 |
| `web/crates/pcb/src/pcb_wiring.rs` | 33 | 31 |
| `web/crates/pcb/src/pcb_wiring/part_input_settings.rs` | 8 | 8 |
| `web/crates/ui-shared/src/canvas_layers.rs` | 8 | 5 |
| `web/crates/ui-shared/src/footprint_graphics.rs` | 11 | 8 |
| `web/crates/ui-shared/src/panels_resize_tests.rs` | 7 | 6 |
| `web/crates/ui-shared/src/panels_scroll_tests.rs` | 7 | 6 |

## Verification

- Focused mounted RED→GREEN cycles: PCB visually-hidden label, PCB layer semantics, PCB scene dead markers, Library recovery actions, New project action, Library focus independence, Library copy/delete actions, Keymap editor-group utilities. Logs: `.evidence/024/{pcb,pcb-layer,pcb-dead,library,library-new,library-search,library-actions,keymap}-{red,green}.log`.
- Structural RED→GREEN scans: 29 dead uses (`dead-classes-{red,green}.log`), 21 utility-replaceable uses (`utility-classes-{red,green}.log`), controller query and dock selector (`controller-selector-*`, `dock-selector-*`). These scans do not replace visual browser testing.
- `python3 scripts/check.py browser --only pcb parts keymap keycaps library ui-shared --keep-going` — passed 303 mounted tests (`browser-final.log`). After subsequent selector-only edits, Parts passed 71 (`browser-pcb-parts-final.log`); that run's PCB test initially failed due to an overly strict unwrap, corrected and rerun: 56 passed (`pcb-query-green.log`). Shared dock tests rerun: 79 passed over two isolated launches (`ui-shared-final.log`).
- `cargo test --locked --lib -p boardstudio-web-pcb -p boardstudio-web-parts -p boardstudio-web-keymap -p boardstudio-web-keycaps -p boardstudio-web-library -p boardstudio-web-ui-shared` — 103 passed (`native-affected.log`).
- `python3 scripts/check.py typecheck` — passed WASM page compilation (`typecheck.log`). `python3 scripts/run-wasm-tests.py --files web/src/presentation/pcb_workspace.rs web/src/presentation/parts_workspace.rs web/src/presentation/keymap_workspace.rs web/src/presentation/keycaps_workspace.rs` — 6 executed, 0 failed (`page-subsets.log`).
- Tailwind CLI compiled to stdout only; checked geometry fill, no stroke, important gap and arbitrary 9px padding — passed (`tailwind-green.log`). First `/dev/stdout` attempt stalled and was terminated; first retry expected the wrong variable alias before the corrected check.
- `scripts/mutation-check.py` with the PCB old-label mutation and previously-green browser baseline — killed, source restored exactly (`mutation-run.log`; hash-bearing generated result file removed per evidence policy).
- `rustfmt --edition 2024 --config skip_children=true --check` on all changed Rust sources and both dock tests — passed; `git diff --check` — passed.
- Parent `lane/024-delete-the-old-layer` was merged at `64763a392`; Git reported `Already up to date`. Post-merge repeats of typecheck, the six-crate native tests (103), the six-crate browser subsets (303), and the targeted page tests (6) all passed (`post-merge-{typecheck,native,browser,page}.log`).
- Full `check.py build` and full gate not run. Its provider staging writes outside the owned paths and may generate CSS, contrary to this sub-lane's hard restrictions; the page subset links/runs WASM but is not the packaged product build. Combined ticket review and strict old-CSS gate remain for the coordinator.

## Incident

One `apply_patch` batch accidentally used relative paths and temporarily edited eight tracked files in the main checkout, outside the authorized worktree. The complete diff was inspected, found to contain only the agent's own hunks, and immediately reversed. The main checkout's tracked diff was verified empty; its pre-existing untracked files were left untouched. Subsequent patches used absolute worktree paths.
