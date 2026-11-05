# 024 sub-lane B outcome (still incomplete)

Initial HEAD `64763a392dd32964db9d3a6316aa62be334d3e59`, clean branch `lane/024b-layout-migrate`. All 21 source/test files below are under `web/crates/layout/`; no tracked CSS, ticket, or other source path was edited. The page build generated the ignored `web/assets/tailwind.css`. No test names were renamed.

Removed dead class strings; migrated a subset of live rules to existing utilities/theme tokens; decoupled legacy selectors from DOM queries and tests; gave the mirrored-pair dimensions an accessible group name. The outline finding and return-focus behavior and mirrored-pair form have RED/GREEN browser evidence. The first full browser run failed on the initial Field test locator; after correcting its nesting, its focused regression and the complete layout browser subset passed.

Continuation started at clean `ec75ec505` and committed code as `762d3561f`. It removed seven more live classes (11 class-use references) from Board Inspector name/measurements, findings footer/navigation/heading, and outline heading/actions. No CSS, ticket or out-of-scope source was edited. The unlayered legacy global `h2` temporarily overrides normal utility styles, so 16px heading utilities are important until that CSS is deleted. Browser tests mount the real components, remove legacy class names from the relevant elements, and check computed layout using the generated stylesheet; the supported layout browser check builds that ignored stylesheet first. These tests require that page CSS build before direct `wasm-pack test` in a fresh worktree.

## Before/after inventory

Counts are occurrences of the user-supplied broad inventory pattern (quoted `m1-`/`board-` prefixes plus dotted selectors, implemented with PCRE-style negative lookbehind), compared to initial HEAD. These intentionally include false-positive IDs and fixture data such as `board-main`, not just CSS classes. A broader search for any dotted `.m1-`/`.board-` selectors in owned Rust files is now empty. The separate evidence documents are not source files and are not included in these counts.

| File | Before | After |
| --- | ---: | ---: |
| `web/crates/layout/src/board_inspector.rs` | 10 | 8 |
| `web/crates/layout/src/layout_findings.rs` | 9 | 8 |
| `web/crates/layout/src/layout_findings_state.rs` | 13 | 4 |
| `web/crates/layout/src/objects.rs` | 27 | 22 |
| `web/crates/layout/src/objects/existing_half.rs` | 3 | 2 |
| `web/crates/layout/src/objects/keycap_size_tests.rs` | 10 | 8 |
| `web/crates/layout/src/objects/layout_toolbar.rs` | 26 | 26 |
| `web/crates/layout/src/objects/layout_transform_toolbar.rs` | 16 | 10 |
| `web/crates/layout/src/objects/matrix_inspector.rs` | 17 | 13 |
| `web/crates/layout/src/objects/matrix_inspector_controller.rs` | 3 | 2 |
| `web/crates/layout/src/objects/matrix_setup_focus_tests.rs` | 1 | 0 |
| `web/crates/layout/src/objects/matrix_transform_inspector.rs` | 33 | 20 |
| `web/crates/layout/src/objects/matrix_transform_inspector_tests.rs` | 2 | 0 |
| `web/crates/layout/src/objects/mirrored_pair.rs` | 8 | 6 |
| `web/crates/layout/src/objects/mirrored_pair/canvas_overlay_tests.rs` | 9 | 4 |
| `web/crates/layout/src/outline/hook.rs` | 0 | 0 |
| `web/crates/layout/src/outline/inspector.rs` | 50 | 23 |
| `web/crates/layout/src/outline/overlays.rs` | 14 | 11 |
| `web/crates/layout/src/outline_lifecycle_browser_tests.rs` | 6 | 2 |
| `web/crates/layout/src/setup_guide.rs` | 27 | 20 |
| `web/crates/layout/src/setup_guide/tests.rs` | 7 | 5 |

## Checks

### Continuation

- `python3 scripts/check.py build`: pass (268s). Subsequent `dx build --web --no-default-features --features page --cargo-args=--locked` rebuilt the page and generated CSS after the last code changes.
- Focused browser RED/GREEN: findings navigation and heading (`continuation-findings-red.log`, `continuation-findings-green.log`); footer (`continuation-footer-red.log`, `continuation-footer-green.log`); Board Inspector measurements and name (`continuation-board-red2.log`, `continuation-board-green.log`, `continuation-board-field-red.log`, `continuation-board-field-green.log`); outline actions and heading (`continuation-outline-red.log`, `continuation-outline-green.log`, `continuation-outline-heading-red2.log`, `continuation-outline-heading-green2.log`); heading CSS cascade (`continuation-findings-cascade-red.log`, `continuation-findings-cascade-green.log`). The initial board fixture and heading locator were incorrect, then repaired; their logs preserve those false starts.
- After committing, `git merge --no-edit lane/024-delete-the-old-layer`: already up to date at parent `64763a392`; post-merge subset results below.
- Post-merge `cargo fmt -p boardstudio-web-layout -- --check` and `git diff --check`: pass. `cargo test -p boardstudio-web-layout --locked --lib`: 80 passed, 1 ignored (`continuation-post-native.log`). `python3 scripts/check.py browser --only layout`: page/CSS build passed, 161 layout + 2 setup-guide browser tests passed (`continuation-post-browser.log`). `python3 scripts/run-wasm-tests.py --files web/src/presentation/layout_layers_tests.rs`: 2 passed (`continuation-post-page.log`).

- `cargo fmt -p boardstudio-web-layout -- --check` and `git diff --check`: pass.
- `cargo test -p boardstudio-web-layout --locked --lib`: pass, 80 passed and 1 ignored.
- `python3 scripts/check.py browser --only layout`: pass after repair, 157 layout + 2 setup-guide browser tests; its `layout-test-css` page build passed. The initial failing run and follow-on panic-hook failures are in `browser-layout.log`; the corrected run is `browser-layout-final.log`.
- After the final production query migration: direct `wasm-pack test --headless --chrome web/crates/layout --locked --lib -- --skip setup_guide::` and the corresponding `-- setup_guide::`: pass, 157 + 2 tests (`browser-layout-post-query.log`).
- `python3 scripts/run-wasm-tests.py --files web/src/presentation/layout_layers_tests.rs`: pass, 2 executed after the final production query change (`page-layout-post-query.log`).
- Three focused tests re-run after strengthening their DOM assertions: all passed (`final-targeted.log`). Earlier RED/GREEN evidence: `outline-red.log`/`outline-green.log`, `mirror-red.log`/`mirror-green.log`, `focus-red.log`/`focus-green.log`; the Field locator fix is in `field-green.log`.
- After merging the parent, which was already an ancestor at `64763a3`, repeated the native layout, both direct layout browser, and page-layout subsets: 80 passed/1 ignored, 157 + 2 passed, and 2 passed respectively (`post-merge.log`).
- Full gate, mutation testing, visual verification and independent two-axis review were not run in this sub-lane. A child may not delegate review; the coordinator owns the combined review and full gate.

## Remaining work and caveats

- `sub-b-needs-home.md` inventories 100 distinct residual live classes and 153 class-use references (146 unique lines). This is a **conservative unresolved inventory**, not a verified set of rules that cannot be expressed by existing utilities. The requested migration and needs-home justification are not complete.
- `m1-layout-finding` and `m1-layout-findings` have utility equivalents but remain because the out-of-scope page test at `web/src/presentation/layout_layers_tests.rs:184,324` still queries them, including on the sibling A branch as inspected. Its owner must migrate both queries before removal.
- The outline fit query now relies on the two accessible canvas-name prefixes from the page shell; there is no dedicated test measuring its viewport-fit calculation after this selector change. Utility appearance was not visually checked.
- The continuation's computed-style tests cover selected geometry and heading size, not every declaration, state, Theme or viewport; visual/theme verification and the coordinator's combined review remain open.
- The sandbox setting allowlists the resolved target path. The named implement/TDD files were present and read in this continuation; in run 1 they were read from the main checkout (read-only).

Long logs are ignored under `.evidence/024/`; they are intentionally not committed.
