# 026: Merge gate and single merge

Status: resolved
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
- [x] `dev` was merged into the branch before the merge; the single merge to `dev` is done and nothing is pushed without the user's say-so
- [x] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches

## Current

See [Outcome](#outcome) for the scoped resolution, evidence and open gates.

## Outcome

**Scoped resolution:** the requested agent implementation and evidence scope is
complete; this is not full acceptance or permission to merge to `dev`. Logs and
receipts are local to `lane/026-merge-gate` under `.evidence/026/`.

| Gate or step | Result | Evidence |
| --- | --- | --- |
| Registry | **PASS**: the exact 20 previously unregistered scripts were declared opt-in with their ticket owners, not registered. `python3 scripts/test-check.py`: 64/64 | `finalizer-registry-red.log`, `finalizer-registry-green.log`; commit `31f17dd56` |
| Old CSS strict | **PASS**, 0 legacy selectors / 0 variables | `finalizer-old-css.log` |
| `python3 scripts/check.py quick` | **PASS** after the fixes below: 2,411 native tests passed, 126/126 page tests passed, browser crates and CAD checks passed; the quick plan excludes drag-smoke and is not the full default gate | `finalizer-quick-final.log`, `finalizer-quick-final.json.md` |
| `python3 scripts/check.py build` | **PASS**, checked packaged site | `finalizer-build.log`, `finalizer-build.json.md` |
| `python3 scripts/check.py browser --only host runtime ui-model ui-shared keycaps library keymap catalogue case parts pcb layout --keep-going` | **PASS**, all 12 presentation crates (14 launches); deliberately partial, no drag-smoke, packaged page or separate provider gate | `finalizer-crates.log`, `finalizer-crates.json.md` |
| Full default / full browser | **OPEN**, not rerun or claimed green; prior pre-merge results are below. The focused browser evidence is not a waiver of missing coverage. | `full-final.log`, `browser.log` (historical) |

The first quick run exposed an owned merge-reconciliation compile error in the
toolbar example; `ec108844` supplies the existing optional field, with expected
RED in `finalizer-quick.log` and focused GREEN in `finalizer-toolbar-green.log`.
The next quick run exposed concurrent page fixtures selecting another
Inspector's tab; `31c71643` isolates the short-lived fixture, adds a selector
precondition and retains the focus assertion. Diagnostic RED and combined-page
126/126 GREEN are in `finalizer-page-diagnostic.log` and
`finalizer-page-fixed.log`; the full quick rerun then passed.

Review-finding fixes are recorded in `0c2fb2bac` through `a33df1d77` (findings
1–8, including the selection-role disclosure); the local record does not
contain the two-axis review receipts or an approval of the subsequent merge and
reconciliation, so the whole-branch review checkbox remains open. `dev` was
merged into this lane at `c79de4708`; follow-on reconciliation commits are
`8a6a2b1f0`, `c67c30dfd`, `382ee33c8`, `1478c26af`, `ec108844` and
`31c71643`. The single merge *to* `dev` remains open.

The research verdict in the main checkout's untracked
`docs/research/2026-11-06-drag-smoke-and-tooling-registry.md` defers drag-smoke
from this scoped evidence set, not its unique gesture → save → Undo coverage.
The recorded merged-tree signature (`run4-verified-drag.log`) is:

> timed out waiting for slider Rows to receive focus: document.activeElement === document.querySelector('[role="slider"][aria-label="Rows"]'); drag browser smoke saw 1 browser handler error(s): WebSocket protocol reset without closing handshake

The `.m1-save-state` save selector is also dead on the merged tree; the
WebSocket reset is shutdown noise, not the cause. No drag harness was changed
for this deferral. The registry is not stale; these are agent-browser migration
debt and are explicitly opt-in pending retirement.

### OPEN — user-gated work and follow-ups

- **LIVE human review:** the user elected live review, not approval by automated
  screenshots. Serve the checked site with
  `python3 scripts/serve-web.py web/target/site 4173` or use the dev server at
  `127.0.0.1:8080`. Review every Workspace × both Themes × both Appearances ×
  desktop/mid/narrow, capturing screenshots as the review record. Detailed
  instructions and viewport sizes follow below. The expected 72 agent captures
  under `web/target/test-results/026/` are backup only; this worktree currently
  contains 24 PNGs, so the 72-image backup could not be verified here.
- **One merge to `dev`:** only after user approval; not performed here, and no
  origin push is authorized by this scoped resolution.
- **Follow-ups to file (not created):** a drag-smoke rewrite for the merged tree
  and an agent-browser retirement ticket to port unique coverage or delete the
  20 opt-in scripts per owner.

### Earlier pre-merge evidence (historical)

The following gate run was based on `36bca9c6a7`, before the later review-fix
commits and dev merge. Its results are retained literally, not as current green
claims or waivers. Browser captures are under `web/target/test-results/026/`.

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
report the results; none is claimed here. Scoped resolution does not close the
whole-branch review, human review or single merge to `dev`.

2026-11-06: single merge to `dev` completed per user approval: `dev` fast-forwarded 510861b11 -> 64bc0e8fa (feat tip; dev was already merged into the branch and verified). No push to origin performed. Criterion 2 (human review) remains open as the user's live review.

2026-11-06 (addendum, ui-integration-recovery 008): the drag-smoke deferral above is partly superseded; the results recorded
above are unchanged. On HEAD `ca065e679` the `.m1-save-state` wait still timed out, and the Rows focus failure did not
reproduce. The harness now observes the header `.header-save[data-state]` and `[data-transform-handle]`. All Part, Column,
Stagger cancel, Stagger handle, Ctrl Offset and Matrix move scenarios and their exact Undo checks pass on that build, but the
complete runner still fails later at the Case collision scenario (`Case mounting style` is no longer a native select), so
there is no success receipt and the full browser gate stays open. Details and identities:
[ui-integration-recovery 008](../../ui-integration-recovery/issues/008-restore-drag-save-undo-proof.md#answer).

2026-11-06 (addendum, ui-integration-recovery 022): the bounded packaged smoke now
passes all 17 drag/save/Undo/reload and collision/provider checks, including the
overflowing Case viewer, qualified fixture and unavailable-PCB uncertainty.
The original native-select failure was reproduced; the harness now drives the
shared accessible Select, dismisses the setup guide and observes current shared
panel/provenance/alert structure. No product source or semantic assertion was
relaxed. Final run `20261106T233935Z-abbc5bec9637` has a success receipt validated
against fresh source/build bytes and browser identity. Earlier failures above
remain history. This is not the full default/browser gate: 022's independent
spec+standards review, 010's combined verification and human acceptance remain
open. Commands, identities, RED/GREEN and rejection controls:
[022 implementation checkpoint](../../ui-integration-recovery/issues/022-restore-collision-provider-browser-proof.md#implementation-checkpoint-2026-11-06).
