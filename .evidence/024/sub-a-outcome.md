# 024 sub-lane A outcome — partial

Base `64763a392dd32964db9d3a6316aa62be334d3e59`; branch `lane/024a-shell-migrate`. The lane is **not complete**: unresolved live and dead references remain. No authored CSS, ticket, other branch, or out-of-scope source was edited.

Removed dead Case and Export classes, preserved touch utilities, migrated five visually hidden labels to the existing `sr-only` utility, and replaced affected queries with named controls/regions, text or styling assertions. No native test names were changed. `web/assets/tailwind.css` was absent initially; the mandatory build generated that ignored output, which was inspected and removed after verification.

Counts below compare base to this lane's source: occurrences of legacy tokens in quoted class expressions plus dotted legacy selectors; board IDs and other quoted domain data are excluded.

| File | Before | After |
| --- | ---: | ---: |
| `web/crates/case/src/cad_presentation.rs` | 8 | 7 |
| `web/crates/case/src/mechanical_settings.rs` | 135 | 69 |
| `web/crates/case/src/shared_viewer.rs` | 23 | 22 |
| `web/crates/case/src/shared_viewer_020_tests.rs` | 4 | 2 |
| `web/src/presentation/case_workspace.rs` | 21 | 14 |
| `web/src/presentation/export_workspace.rs` | 20 | 14 |
| `web/src/presentation/layout_matrix_relations.rs` | 26 | 25 |
| `web/src/presentation/layout_workspace.rs` | 19 | 16 |
| `web/src/presentation/zmk_firmware_export.rs` | 8 | 3 |

## Verification

- RED/GREEN browser slices: export row, Case legacy construction, Case quiet buttons, screen-reader-only viewer label, Case Inspector; logs `red-*.log` / `green-*.log` in this directory. Dead-name CSS absence checked across authored and generated CSS.
- `python3 scripts/check.py build`: PASS after final production changes (`build-final.log`, 84.4s). Generated Tailwind `.sr-only` declarations match the legacy visually-hidden rule.
- `cargo test -p boardstudio-web -p boardstudio-web-case --locked --lib`: PASS; 34 Case tests, web lib 0 (`native.log`).
- `python3 scripts/check.py browser --only case`: PASS 100/100 (`browser-case-final.log`).
- `python3 scripts/run-wasm-tests.py --files` for Case workspace, Export, ZMK, Layout workspace and relation highlight: PASS 18/18 (`browser-page.log`); after final test-only refactor, Case/Export/ZMK PASS 10/10 (`browser-page-final.log`).
- `rustfmt --edition 2024 --check` on the nine changed files and `git diff --check`: PASS. No full gate was run.
- Relation test RED attempts twice timed out at pre-existing canvas setup before reaching the attempted assertion; the unmodified suite passed 5/5 (`baseline-relations.log`), and the attempted slice was reverted. The final affected page subset also passed.

## Remaining work / needs home

`sub-a-needs-home.md` records exact source and CSS locations. Two confirmed shared-toolbar relocation candidates: `m1-case-view-toolbar` and `board-tool-pill`. There are 165 live legacy class names total (275 class uses, 59 dotted query occurrences), including those two; the other 163 names have **not** been fully evaluated for existing utility equivalents. Another 24 dead names remain (32 class uses, 12 dotted occurrences). This is an inventory, not a claim that all live rules require a new home; the integrator must complete migration and assessment before deleting old CSS.

The requested `.agents/skills/implement/SKILL.md` and `.agents/skills/tdd/SKILL.md` were absent in the worktree; the available root `.pi/skills/tdd/SKILL.md` and Rust best-practices guidance were read instead.
