# 024 sub-lane C, run 3 — PARTIAL / NOT READY FOR INTEGRATION

Initial `e40b346a1` was clean; run-1 base is `64763a3`. Seven per-file commits end at `51b9039ce`. Merging `lane/024-delete-the-old-layer` at `64763a392` was already up to date; affected checks were repeated after the merge. No push, tracked CSS edit, ticket edit, public API change or shape change was made. One accidental read of the main checkout's Rust skill file occurred; no write or Git command was made there.

Of the 143 distinct CSS-linked names left untriaged by run 2, 13 were migrated here and **130 remain untriaged**. They are not justified needs-home findings; none may be deleted on that basis. Run 2 also listed 13 unclassified and 24 additional raw candidates, many identifiers or prefixes, that remain unadjudicated. The fresh quoted-start/dotted scan has **156 distinct raw candidates**, including non-class IDs and prefixes (`run3-raw-candidates.log`). See the historical `run3-prior-untriaged.log` and `continuation-enumeration.log`, subtracting the 13 names below. That historical inventory's file locations may have moved. In particular, the project-menu close query/focus dependency and contextual controller-picker styles need coordinated investigation.

Run 3 removed: `m1-project-setup-choice`, `m1-footprint-graphic`, `m1-footprint-graphics-error`, `m1-footprint-keepout`, `m1-footprint-zone`, `m1-footprint-text`, `m1-parts-details`, `m1-parts-detail-list`, `m1-layer-dock`, `m1-keymap-section-body`, `m1-keymap-macros`, `m1-keymap-encoder-editor`, and `m1-keymap-encoder-heading`. The section-body replacement preserves the nested macro border/padding and macro/encoder heading rules with descendant utilities. The old layer-dock `board-disclosure` descendant rules have no such descendant in the current Case or Parts callers; empty outlet behavior is retained with `empty:hidden`.

## Per-file before/after vs `64763a3`

The counts are quoted-start/dotted regex matches, not distinct class uses. They include IDs/selector-like strings and miss classes later in a quoted string. Generated from the complete committed owned-path diff (`run3-per-file.log`).

| File | Before | After |
|---|---:|---:|
| `web/crates/keycaps/src/keycaps_scene.rs` | 6 | 5 |
| `web/crates/keymap/src/keymap/binding_editor.rs` | 3 | 0 |
| `web/crates/keymap/src/keymap/canvas.rs` | 9 | 8 |
| `web/crates/keymap/src/keymap/encoder_editor.rs` | 4 | 0 |
| `web/crates/keymap/src/keymap/macro_editor.rs` | 3 | 0 |
| `web/crates/keymap/src/keymap/panel.rs` | 25 | 0 |
| `web/crates/library/src/library.rs` | 120 | 76 |
| `web/crates/parts/src/parts.rs` | 15 | 8 |
| `web/crates/parts/src/parts/details.rs` | 4 | 0 |
| `web/crates/parts/src/parts/modules_catalogue.rs` | 7 | 2 |
| `web/crates/parts/src/parts/modules_catalogue/module_attachment.rs` | 1 | 0 |
| `web/crates/parts/src/parts/preview.rs` | 24 | 21 |
| `web/crates/pcb/src/pcb_board_reference.rs` | 7 | 6 |
| `web/crates/pcb/src/pcb_module_inspector.rs` | 18 | 8 |
| `web/crates/pcb/src/pcb_physical_setup/controller.rs` | 7 | 2 |
| `web/crates/pcb/src/pcb_scene.rs` | 28 | 10 |
| `web/crates/pcb/src/pcb_wiring.rs` | 33 | 31 |
| `web/crates/pcb/src/pcb_wiring/part_input_settings.rs` | 8 | 8 |
| `web/crates/ui-shared/src/canvas_layers.rs` | 8 | 5 |
| `web/crates/ui-shared/src/footprint_graphics.rs` | 11 | 0 |
| `web/crates/ui-shared/src/layer_dock.rs` | 1 | 0 |
| `web/crates/ui-shared/src/panels_resize_tests.rs` | 7 | 6 |
| `web/crates/ui-shared/src/panels_scroll_tests.rs` | 7 | 6 |

## Verification

- Focused browser RED/GREEN for each migrated file/behavior: `run3-{setup,svg,details,layer-dock,keymap-panel,macro,encoder}-{red,green}.log`, plus `run3-svg-error-red.log` and `run3-svg-green-final.log` for the error branch. REDs failed on the expected missing utility. The per-file native/browser/typecheck/page logs use the same slice names except that setup uses `run3-pcb-*` and `run3-setup-page.log`; all passed before the next file.
- `python3 scripts/check.py build` passed both before changes and at the final committed state (`run3-build.log`, `run3-final-build.log`). Staging outputs are ignored worktree build outputs; the Cargo target resolves to the worktree-specific allowlisted cache directory.
- After the no-op parent merge: `cargo test --locked --lib -p boardstudio-web-ui-shared -p boardstudio-web-parts -p boardstudio-web-pcb -p boardstudio-web-case -p boardstudio-web-keymap` — 125 passed; `python3 scripts/check.py browser --only ui-shared parts pcb case keymap` — 358 passed; `python3 scripts/check.py typecheck` passed; `python3 scripts/run-wasm-tests.py --files` on the setup, Parts, PCB, Case and Keymap workspace modules — 8 passed. Logs: `run3-post-merge-{native,browser,typecheck,page}.log`.
- `scripts/mutation-check.py` killed the empty-outlet and macro-grid mutants and verified restoration (`run3-mutation-{layer,macro}.log`). Tailwind CLI emitted the relevant SVG, nested, grid and empty-state utilities (`run3-*-tailwind.log`, generated output in ignored `web/target/`). Rustfmt and `git diff --check` passed.
- The new browser assertions inspect rendered utility classes and mounted state; compilation of the utilities was checked separately. They do **not** prove pixel equivalence or theme/viewport visual equivalence. No full gate or tooling probe was run, per instruction. Strict old-CSS deletion is not ready while remaining callers are untriaged.
- Earlier run evidence and the main-checkout write/reversal incident are retained in `run3-prior-outcome.log` and in the prior tracked outcome at `e40b346a1`. These two outcome records are tracked; the detailed run logs are gitignored.
