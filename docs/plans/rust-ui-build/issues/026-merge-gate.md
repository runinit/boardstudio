# 026: Merge gate and single merge

Status: claimed
Owner: lane/026-merge-gate
Type: build
Blocked by: [024: Delete the old layer](024-delete-the-old-layer.md), [025: Rewrite DESIGN.md for the new structure](025-rewrite-design-md.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [branch plan](../../rust-ui-migration/branch-plan.md) section 5, [shared contract](../map.md#shared-contract)

## What to build

The migration branch passes the gate and is merged to `dev` once.

## Acceptance criteria

- [ ] Full `scripts/check.py` and the browser tests pass; the accessibility checks pass; the old-CSS check passes in strict mode
- [ ] The user has reviewed every Workspace in both Themes and both Appearances at desktop (above 1150px), mid (981 to 1150px) and narrow (980px and below) widths, with screenshots
- [ ] One two-axis code review (Standards and Spec) of the whole branch is done and its findings resolved or recorded
- [ ] `dev` was merged into the branch before the merge; the single merge to `dev` is done and nothing is pushed without the user's say-so
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches

## Current

Gate run on `lane/026-merge-gate`, based on `36bca9c6a7`; **not resolved or
ready to merge**. The table records the literal result, not a waiver. Logs and
receipts are local to this worktree under `.evidence/026/`; browser captures are
under `web/target/test-results/026/`.

| Step | Result | Evidence | Classification |
| --- | --- | --- | --- |
| Default `python3 scripts/check.py` | **FAIL** at tooling, 21 unregistered scripts; remaining steps not reached by this invocation | `full-final.log`, `full-final.json.md` | **Pre-existing**, as adjudicated in 022 Comments and 024 Outcome; still open. The failing registry guard and `TOOLING_TESTS` are AST-identical to the base and the missing sets match (`known-signatures.log`). The scheduler fix below does touch `check.py` and `test-check.py`, but not those guard definitions or the registered set. |
| Remaining tooling commands after that fail-fast | **PASS** individually, including import's Cargo build | `tooling-remainder.log`, `tooling-import.log`, `tooling-gitnexus-canonical.log` | No new open failure. Two stop-hook assertions initially failed because this run used a symlinked `TMPDIR`; using its canonical path passed 9/9. |
| Checked build | **PASS** | `build.log`, `build.json.md` | Passed on clean `d02226b74`; subsequent fixes are scheduler/test-only. |
| Lint | **PASS after fix** | initial `lint.log`; `lint-fixed.log`, `css-plan-red.log`, `mutations.log` | **Owned-fixed**: a fresh checkout lacked generated Tailwind CSS required by WASM test fixtures (including 024's Keycaps fixture). `a52342fec` schedules the CSS-producing build before lint and CSS-dependent subsets. Fresh-asset lint passed; four plan mutations were killed. |
| Native tests | **PASS**, all 2,110 baselined names executed | `test.log`, `test.json.md` | Passed on clean `d02226b74`; product source is unchanged by the subsequent fixes. |
| Full browser (`browser --keep-going`) | **FAIL** only at drag save wait; all crate launches, CAD provider/browser smoke and all 91 page tests passed | `browser.log`, `browser.json.md` | **Pre-existing**, identical `.m1-save-state` timeout at `scripts/test-drag-interactions.cjs:87` as adjudicated in 024 Comments. Script and header are byte-identical to the base (`known-signatures.log`). Drag beyond that wait is unverified; the full browser gate remains open. The earlier ToastCtx and dock-page failures did not recur. |
| Accessibility floor browser coverage | **PASS, scoped**: UI-shared 79 + 2 mounted tests include Select/Combobox, action-menu and 16 Toolbar/Toggle/Slider/Toast cases; all other crate launches and the page suite passed. Production header keyboard/navigation test passed. | `browser.json.md`, `css-subsets.json.md`, `production-header.log` | Floor-test coverage passed, not a complete accessibility certification: the original 010 scratch-lab axe scan and screen-reader testing were not rerun. |
| Theme production test | **PASS after fix**, all four Theme/Appearance pairs | initial `production-theme.log`; `production-theme-fixed.log` | **Owned-fixed**: `06a545b7` updates the test's obsolete legacy-alias expectation to assert the 024 retirement; the other token, canvas, firmware, size and font assertions remain. The complete test file passed. |
| `python3 scripts/check-old-css.py --strict` | **PASS**, 0 legacy selectors and 0 variables | `old-css.log` | Pass. |

The isolated generated-CSS-dependent browser subset also passed from an absent
generated stylesheet (`css-subsets.log`); fresh lint did likewise. The original
default run's missing Playwright preflight (`full-preflight.log`) was remedied
with the CI-pinned `playwright-core@1.62.1` under this worktree's target. No
Graphite opt-in suite or separate collision/qualification follow-up is claimed;
the CAD browser smoke and mounted collision tests included above did pass.

### Human Workspace review instructions (still required)

Use the checked packaged app with a representative populated project. Review
**Layout, Case, Parts, PCB, Keymap and Keycaps** at each combination of Theme
**Original / shadcn**, Appearance **Light / Dark**, and viewport **1440×900**
(desktop, >1150px), **1024×900** (mid, 981–1150px) and **390×844** (narrow,
≤980px): 6 × 2 × 2 × 3 = **72** labelled screenshots. Set both choices in
Settings, verify the selected Workspace and capture each view; do not treat the
automated captures as the user's approval. Include a Case project with a real
switch courtyard/populated 3D canvas, since the earlier Split 42 demo did not
cover it. Also inspect and capture Library, the project menu and Export in the
same Theme/Appearance/width combinations.

For each view inspect desk/card contrast, Theme colours and typography, header
and Workspace navigation, canvas and geometry visibility, Objects and Inspector
docks (including the narrow Sheet tabs), controls and labels, selected/disabled
states, menus and dialogs, overflow/clipping, focus visibility and touch-target
spacing. Scroll long inspectors and open representative controls; record defects,
the exact combination and screenshot path. The user must explicitly approve or
report the results; none is claimed here. The two-axis branch review, its fixes,
the subsequent `dev` merge, final resolution and single merge remain coordinator/
user-gated and have not been performed.
