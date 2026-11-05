# 024 sub-lane C — PARTIAL / NOT READY FOR INTEGRATION

Initial base `64763a3`; continuation started clean at `d93fb9e1d` on `lane/024c-rest-migrate`. No push, CSS, or ticket edits. This continuation removed 16 distinct previously inventoried CSS-linked names from Keymap by moving their declarations to Tailwind utilities and theme tokens, including preserving compact-vs-inspector layer spacing and tabs orientation. It did not change shared component APIs. It did **not** complete migration: **143** of the prior 159 live names remain untriaged, plus earlier 13 unclassified candidates and 24 additional raw quoted/dotted candidates. Raw scan has 169 distinct names including IDs and prefixes; see `continuation-enumeration.log`. Do not classify these as genuinely needs-home or delete their CSS. `sub-c-needs-home.md` is an incomplete triage inventory, not a finalized needs-home list.

## Per-file before/after vs base `64763a3`

Numbers count individual regex matches for quoted-start or dotted `m1-`/`board-` names, not distinct class uses; this includes IDs and selector-like strings and misses later classes within a quoted string. They are not the 143 live-name count.

| File | Before | After |
|---|---:|---:|
| `web/crates/keycaps/src/keycaps_scene.rs` | 6 | 5 |
| `web/crates/keymap/src/keymap/binding_editor.rs` | 3 | 0 |
| `web/crates/keymap/src/keymap/canvas.rs` | 9 | 8 |
| `web/crates/keymap/src/keymap/encoder_editor.rs` | 4 | 2 |
| `web/crates/keymap/src/keymap/macro_editor.rs` | 3 | 1 |
| `web/crates/keymap/src/keymap/panel.rs` | 25 | 3 |
| `web/crates/library/src/library.rs` | 120 | 76 |
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

## Verification and limitations

- Run-1 focused mounted and structural RED/GREEN, 103 native, 303 browser, 6 page tests, WASM typecheck and prior parent repeats: see the surviving `.evidence/024/*-green.log`, `post-merge-{native,browser,page,typecheck}.log`. Run-1 PCB mutation was killed.
- Continuation: mounted RED/GREEN cycles for binding fields/editor, key navigation, guidance (panel and empty encoder), layer name, controls, navigation shell, macro card/step, encoder row, editor sections, layers content, layers/tabs, heading, panel. Logs `continuation-*-{red,green}.log`; assertion-based layout checks verify utility presence, not rendered pixel equivalence.
- `cargo test --locked --lib -p boardstudio-web-keymap` — 8 passed (`continuation-native-final.log`).
- `python3 scripts/check.py browser --only keymap` — 47 passed (`continuation-browser-final.log`).
- `python3 scripts/check.py typecheck` — passed (`continuation-typecheck-final.log`).
- `python3 scripts/run-wasm-tests.py --files web/src/presentation/keymap_workspace.rs` — 1 passed (`continuation-page-final.log`).
- Tailwind CLI compiled to stdout and emitted required utilities (`continuation-tailwind-final.log`). `scripts/mutation-check.py` navigation-grid mutation was killed and restored (`continuation-mutation.log`). Rustfmt and `git diff --check` passed.
- Packaged build NOT run: `scripts/build-web.py` stages into unowned `web/assets/ergogen-models`, `renderer/pkg`, `cad/wasm/pkg` and generated asset paths; this conflicts with this sub-lane's hard file ownership/no CSS edits. Coordinator must run the build and resolve this gate. No full gate was run as instructed; no visual/theme comparison performed.
- Parent merge/reverification status is recorded below.

## Run-1 incident

Run 1 accidentally edited eight tracked files in the main checkout, inspected and reversed only its own hunks, and verified its tracked diff empty. This continuation used the explicitly specified worktree for every file operation and Git command.
