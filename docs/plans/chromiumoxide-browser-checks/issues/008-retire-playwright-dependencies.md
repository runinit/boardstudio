# 008: Retire Playwright browser dependencies

Status: ready-for-agent
Type: build
Blocked by: [Rust drag smoke end to end](005-rust-drag-smoke.md), [Rust CAD runtime capture](006-rust-cad-runtime-capture.md), [Rust Graphite acceptance checks](007-rust-graphite-acceptance.md)
Parent: [Chromiumoxide browser checks](../map.md)
Related: [Real-browser checks drive Chrome from Rust](../../cad-deepening/issues/016-rust-browser-harness.md)

## What to build

Humans and CI run every migrated real-page workflow without Playwright/npm browser setup.
Remove the obsolete runners and browser dependency installation only after every consumer's
parity gate has passed. Keep this a dependency retirement, not a new test migration.

## Acceptance

- [ ] Inventory all remaining browser consumers and runtime imports before deletion; no live
  workflow loses its runner, dependency or manual entry point. Preserve archived prototypes
  as evidence rather than deleting their historical dependency references.
- [ ] Remove obsolete Playwright runners, personal-path workarounds, browser npm install and
  NODE_PATH prerequisites from migrated workflows. Update CI caches, portable discovery,
  setup instructions and failure messages for the Rust path.
- [ ] Retain Node tooling/self-check tests still in use and their CI prerequisite. Do not
  advertise the whole repository as Node-free or touch wasm-bindgen-test/ChromeDriver.
- [ ] Clean-environment prerequisite tests and actual supported real-page checks work without
  Playwright installed or NODE_PATH set. Missing Chrome/Cargo errors remain actionable.
- [ ] Rust harness formatting/lint and the jointly pinned CI browser/library policy remain
  enforced. Document the version-bump compatibility check and upgrade path.
- [ ] Full supported verification passes on the final committed source; record any inaccessible
  CI gate explicitly rather than treating a local pass as a CI run. Existing app bugs are not
  waived and pending per-consumer parity is not grounds to retire the old dependency.

## Verification and scope

Use TDD at the existing public launcher/output seams: demonstrate the expected regression
before the fix, then run affected tests and typechecking during development. At closure run
the full supported check and combined Spec + Standards review; record terminal verdicts,
source/build identity and any blocked gates honestly. No weakened assertions, masked page
errors, baseline deletions or retry-until-green. Preserve the archived prototypes and unrelated
work. Builds follow repository concurrency/lock rules; evidence uses supported receipts and
logs, not custom hash manifests. All checks must be runnable by humans and CI without agent tools.

## 2026-10-27: cutover inventory preparation

Inventory taken at HEAD `76402f6f6bc3b49d7ec429b015137fcb3a546091` with
concurrent uncommitted 006/007 drafts present. Refresh it against the final source
before deletion; this is preparation, not a parity or acceptance verdict.

- Drag: `scripts/run-drag-browser.py` defaults to Node and launches
  `scripts/test-drag-interactions.cjs` (`playwright-core`); `scripts/check.py`
  preflights its selected driver. CI selects Rust explicitly. Ticket 005 records
  local closure, with hosted CI still unverified.
- CAD: `cad/scripts/measure-cad.py` defaults to Node and launches
  `cad/scripts/measure-cad-runtime.cjs` (`playwright-core`). The diagnostic
  `tools/browser-harness/compare-cad-responses.py` also invokes the Node runner;
  account for its purpose and owner before retirement. Ticket 006 remains open.
- Graphite: manual `scripts/test-graphite-feedback.cjs` imports full `playwright`;
  `scripts/test-graphite-usability.cjs` forwards to it. The Rust launchers are
  experimental and ticket 007 remains open. CI's `playwright-core` install does
  not provide the full `playwright` package used by the manual Node entry point.
- Shared setup: `scripts/browser_tools.py` probes Node and `playwright-core`;
  `scripts/test-browser-tools.py` tests that contract. The web-tools action installs
  `playwright-core@1.62.1` and exports `NODE_PATH` in both web and release-build
  jobs; README describes the prerequisite. Preserve the action's other web tools.
- `cad/scripts/test-measure-cad-runtime.cjs` mocks the Node runner and is in the
  tooling checks; reassess it with that runner rather than dropping Node tooling
  generally. `scripts/test-check.py` inventories the manual and delegated scripts.
  Ergogen fixture generation uses a separate `NODE_PATH`; it is not a Playwright
  prerequisite and must remain intact. Mounted WASM tests and ChromeDriver are
  outside this ticket. Historical research, archived prototypes and dated design
  evidence retain their references.

No runner or dependency was removed. CAD worker response parity and Graphite
same-build parity, failure controls and closure are still prerequisites; final
clean-environment checks, full verification, review and hosted CI remain open.

## 2026-10-27: CAD prerequisite selection

Build preflight and runtime/validation now select Chrome-only discovery for the
Rust driver and Node, `playwright-core`, and Chrome discovery for the Node driver.
The Rust capture executable receives `CHROME_BIN`; the Node runner imports
`playwright-core`. With `NODE_PATH` unset and the module unavailable, a controlled
build-preflight exercise passed Rust's real Chrome discovery and rejected Node for
missing `playwright-core`; compilation was stubbed. The launcher regression covers
build and validation selection, and the CAD and shared browser-tools Python suites
passed (19 and 12 tests respectively). This is prerequisite evidence, not real-page
capture or parity evidence; ticket 006 and the remaining gates above stay open.

## Final removal sequence

1. Complete the outstanding drag, CAD and Graphite parity and failure-control gates;
   qualify CAD startup timing before changing its runtime default. Keep the old
   drivers available until their individual gates pass.
2. Refresh the browser-consumer inventory against the final source. Switch eligible
   defaults to Rust, then remove only obsolete Playwright runners and their setup,
   updating CI, documentation and diagnostic entry points together. Preserve Node
   tooling and the separate Ergogen and mounted WASM prerequisites.
3. In a clean environment without Playwright or its `NODE_PATH`, exercise the
   supported real-page workflows and missing-prerequisite errors. Run the full
   supported check on the final committed source, complete the required review and
   record hosted CI separately before declaring retirement complete.

On `fix/drag-save-undo` at `f4d6587c`, the scheduler inventory suite passed 31
tests under `boardstudio-tests.slice`. The final full check and timing qualification
were not run in this session: observed disk I/O pressure was approximately 45–56%
full stalls. This is not cutover or acceptance evidence.

## 2026-10-27: CI dependency retirement and remaining consumers

The supported drag and CAD launchers now default to Rust; manual Rust Graphite
feedback and usability entry points are available. The complete supported check
passed at `0b1ed972` with `NODE_PATH` unset. The web-tools action no longer
installs Playwright or exports its module path. The web job checks that neither
Playwright package resolves and invokes its browser gate without `NODE_PATH`.
The pinned Chrome for Testing and chromiumoxide pairing, mounted WASM runner and
Node tooling remain unchanged.

Playwright retirement is not complete: optional Node fallbacks, Graphite scripts,
the Parts inspector scroll regression, CAD response comparison and startup
readiness diagnostic still use the old runners. The Node CAD protocol self-check
also exercises the old runner. Preserve those entry points and checks until their
coverage and diagnostics are replaced or their retirement is explicitly approved.

### CI verification plan

1. Run the web job on a hosted pull request or workflow dispatch, verifying the
   no-Playwright preflight, pinned Chrome provisioning, Rust drag receipt, mounted
   browser checks and failure-artifact upload behavior. Record the actual run URL
   and verdict; a local check is not hosted CI evidence.
2. Verify the native job retains Node for its protocol and other tooling tests,
   and the release-build job still provisions wasm-pack and Dioxus without npm.
   Keep the existing Ergogen-specific module path separate.
3. Before deleting the remaining old runners, replace their live diagnostic and
   protocol coverage, run the no-Playwright clean-environment checks and full
    supported verification, and complete the required closure review.

## 2026-10-27: remaining runtime consumers at dd6c8a6c

The Node drag fallback remains selectable by `BOARDSTUDIO_DRAG_DRIVER=node`;
ticket 005 records local same-build parity, but its source is also cited by
historical evidence. CAD's Node capture remains used by the response comparison,
startup readiness diagnostic and protocol self-check. The Graphite Node manual
entry points and Parts inspector scroll check remain separate live consumers.
Neither the shared Playwright probe nor these live checks can be removed as a
complete retirement until those consumers and their diagnostics have equivalent
coverage or explicit retirement approval. Mounted WASM and Ergogen remain out of scope.

The shared Node prerequisite error no longer claims the CI web-tools action installs
Playwright; it directs users to install it separately and set `NODE_PATH`. This is
an interim correction, not removal of a runner or a retirement acceptance verdict.

## 2026-10-27: Parts inspector Rust cutover

`python3 scripts/run-parts-inspector-scroll.py` replaces the opt-in Node Parts
inspector check. It requires `BOARDSTUDIO_PREVIEW_URL` and uses `CHROME_BIN` when
provided. It exercises the same wheel, viewport, hit, horizontal-overflow and
pointer/keyboard board-side checks at 390x844 and 1513x1043; the wheel must also be
observed as trusted. Each viewport has an isolated browser context.

The old and new checks passed on the same freshly built local site under the shared
build lock, `boardstudio-tests.slice`, and Chrome for Testing 145.0.7622.0. The
existing identity helper reported the same dirty source and site digest before and
after, and its served-file verifier matched every local site file at both boundaries.
The site was freshly built in this checkout, but no official source-bound build
receipt was collected; this comparison is not that stronger binding. Logs are in
`.evidence/ticket008-parts-bound-{before,after,node,rust}.log` and the build log is
`.evidence/ticket008-parts-site-build.log`. The Node script has been removed; CAD,
Graphite and drag legacy consumers remain, and full retirement remains open.
As a browser failure control, a separate server injected `overflow-y:hidden` into
the page response; the Rust check failed at 390x844 with zero scroll and a missed
hit (`.evidence/ticket008-parts-control.log`). The unmodified site was not changed.
