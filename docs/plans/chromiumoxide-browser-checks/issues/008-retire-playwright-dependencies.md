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

## 2026-10-27: drag Node fallback retired

The supported drag launcher and scheduler preflight now use only the Rust harness.
An unset `BOARDSTUDIO_DRAG_DRIVER` or `rust` is accepted; any other value fails
before building or launching rather than falling back or skipping the smoke.
`scripts/test-drag-interactions.cjs` remains at its historical path solely so the
existing source-linked migration and collision evidence stays readable; it is not
a supported runner. CAD and Graphite Node consumers and their prerequisites are
unchanged. This slice does not close the overall Playwright retirement.
The selector regression was red for `node`, an unknown value and an empty value
before the change and green afterward. The six existing real-consumer assertion,
exception, rejection, close-exception, cleanup and transport-loss controls each
failed without a success receipt on Chrome 145 under the shared build lock and
`boardstudio-tests.slice` (`.evidence/ticket008-drag-consumer-controls.log`).

## 2026-10-27: Rust-only CAD readiness diagnostic

`cad/scripts/test-startup-readiness.py` now builds and runs `cad-capture` rather
than comparing it with the Node capture. Both controlled landing scenarios remain:
the delayed case checks that the exact visible button was not accepted before its
stable transition, and the transient case records the capture's observed readiness
and transitions. Both still require a retained worker reply, landing metric, success
and owned-process cleanup. The transient observation is diagnostic, not a stability
guarantee or benchmark. The historical Node-versus-Rust comparison is no longer
performed by this command; old evidence remains in Git.

With `NODE_PATH` unset the old command failed because it could not load
`playwright-core` (`.evidence/ticket008-cad-readiness-red/`); the Rust-only command
passed both scenarios on Chrome for Testing 145 under `boardstudio-tests.slice` and
the shared build lock (`.evidence/ticket008-cad-readiness-green/`). This controlled
page uses the existing local package, not a source-bound build receipt. The Node
CAD protocol self-check, response comparison, and optional capture fallback still
need their individual coverage replaced before removing their runner.

## 2026-10-27: shared CAD worker protocol coverage

The tooling gate now also runs `cad/scripts/test-measure-cad-worker.cjs` directly
against `measure-cad-worker.cjs`, the source embedded by Rust `cad-capture`. It
checks successful preview, STEP and vendor sequences, one validation observation,
correlation IDs and worker replacement, retained/drop behavior, malformed reply
frames, failed or mismatched replies, identity epoch/token/revision, result
revision, mesh/body/STEP/vendor census validation and the STL decoder/oracle
controls. It needs Node but neither Playwright nor the old browser driver. The
shared worker now rejects malformed JSON or frames without a string request ID
promptly instead of leaving requests to the watchdog. Unknown well-formed request
IDs remain ignored and eventually time out as before.

The existing Node-driver self-check is intentionally still registered: its actual
driver sequencing, CSV status/timing metric and browser-close assertions have not
yet been replaced with equivalent Rust-consumer controls. The Node response
comparison and fallback also remain. This slice is not complete CAD retirement.

The malformed JSON and null-frame direct tests failed with watchdog errors before
the fix and passed after; direct worker, old self-check, scheduler, quick and CAD
launcher tests passed under the shared lock and `boardstudio-tests.slice`
(`.evidence/ticket008-cad-worker-tests.log`). Chrome 145 passed the real Rust
readiness diagnostic on the existing local package
(`.evidence/ticket008-cad-worker-browser.log`), without a source-bound receipt.
The separate cold-response diagnostic failed its missing-core-worker control: the
capture did not report the expected failure
(`.evidence/ticket008-cad-worker-cold.log`). Its temporary output was cleaned by
that diagnostic, so the underlying response cause remains undetermined. No full
suite or timing qualification was run.

## 2026-10-27: cold-response negative-control diagnosis

The prior failure was reproduced once on Chrome 145 under the shared lock and
`boardstudio-tests.slice`, retaining the capture at
`.evidence/ticket008-cold-repro-a49/`. The injected core-worker WASM request
returned 404 and the same-run response log recorded 404; landing and a CAD reply
were retained and the capture exited 0. The diagnostic expected an error from
the old `cad_target_responses` landing-asset boundary, which is now compiled only
for tests. The active observer records response statuses rather than enforcing that
boundary, consistent with ticket 006's same-run response decision. This evidence
does not prove the product can use a missing core worker beyond the measured landing.

The diagnostic now checks any observed missing-core response is a cold 404, rather
than demanding a failure or a fixed landing asset inventory. If no response is
observed it explicitly reports that observation as inconclusive. A separate
missing-CAD-WASM control requires an observed 404, the actual fetch error, a failed
result and no retained reply. A retained three-case run passed
(`.evidence/ticket008-cold-green-a49/`). A subsequent stricter run did not observe
the core WASM response (`.evidence/ticket008-cold-control-green.log`); this is not
a green 404 observation and shows why requiring that response in every capture
would contradict the no-boundary decision. The final adjusted control is recorded
separately. These use the existing package and are not source-bound acceptance.
The adjusted diagnostic passed the three cases and observed the injected core 404
on this run (`.evidence/ticket008-cold-control-adjusted.log`); 35 scheduler tests
passed (`.evidence/ticket008-cold-scheduler.log`).

Remaining before the Node CAD fallback can be removed: exercise Rust's actual
one/six-operation and fresh-STEP-worker sequencing, metric status and browser-close
failure controls in a supported gate; replace the live Node/Rust response comparison
with equivalent retained same-run observation and failure checks; then update the
launcher and prerequisite tests. The Node self-check and comparison remain intact.

## 2026-10-27: Historical reproduction retired

The user explicitly retired historical Cadrum benchmark reproduction, superseding
the earlier prerequisite to preserve that execution path. `build`, `runtime`, and
`validate` now reject historical Cadrum checkouts before creating output; existing
captures remain readable for binding and reporting. CAD capture accepts only the
Rust driver. The old Node CAD runner, its self-check, and the historical drag runner
are retained as unsupported sources in `docs/archive/playwright-runners/`.

The shared Node worker protocol and STL controls remain in the tooling gate. The
manual Rust response fixture retains its literal response matrix and page-error and
missing-worker controls, without requiring Playwright. CI's no-Playwright preflight
remains in place. Historical documents and evidence are retained.

Focused verification: the retirement guard and no-Playwright response fixture had
RED/GREEN runs; launcher (27), browser-tools (9), scheduler (38), quick-selection
(18), shared worker/STL, real browser response and CAD capture controls passed.
The first browser-tools invocation failed because the outer launch exported a lock
descriptor its subprocess did not inherit; the corrected invocation passed using
the shared lock without exporting that descriptor. No full check was run for this
change.

## 2026-10-27: Rust capture memory attribution

The sampler now recognizes the owned Rust Chrome child by its selected executable,
debugging-port and profile flags, and attributes its descendants as browser processes.
Legacy pipe detection and recorded captures are unchanged. If no verified browser
memory sample is obtained, a successful runtime capture is invalidated; use the actual
Chrome executable rather than a wrapper. This also protects against short-lived or
inaccessible processes silently producing a controller-only memory capture.

## 2026-10-27: Rust CAD consumer sequencing controls

The opt-in `cad/scripts/test-cad-capture-controls.py OUTPUT` builds and runs the
actual Rust capture executable against a controlled HTTP host and browser worker.
It checks one validation operation, six latency preview operations on one worker,
six STEP operations on six distinct fresh workers, unique request IDs, retained
reply and replacement event ordinals, cold/warm timing rows and validation versus
observed status. An invalid-mesh reply must produce a failed result and no retained
reply. The output directory retains worker observations, capture events, metrics and
process logs; it is a simulated provider, not performance or source-bound evidence.

The public launcher test first failed because the command did not exist and then
passed. Controlled Chrome 145 capture passed all four scenarios under the shared
lock and `boardstudio-tests.slice`. `scripts/mutation-check.py` killed all three
deliberate changes to six-operation count, STEP worker replacement and validation
status, with zero survivors or compile failures
(`.evidence/ticket008-cad-capture-mutations/`). The old Node self-check remains:
Rust browser-close failure controls and the Node response-comparison replacement
are not established by these sequencing checks. Full CAD retirement remains open.

## 2026-10-27: Rust CAD browser shutdown control

The controlled Rust capture now also kills only its verified Chrome child after a
worker reply is retained, using the owned profile path and a pidfd. The capture must
exit nonzero, report `browser shutdown failed` and write a failed result despite the
retained reply. This is a transport-loss control that causes `browser.close` to fail;
it is not an isolated injected error in a healthy close handshake. If temporary
browser-owned files remain, their directory is reported and preserved.

With Chrome 145 under the shared lock and `boardstudio-tests.slice`, the controlled
run passed (`.evidence/ticket008-cad-close-draft/`). A mutation that discarded the
shutdown error from the Rust consumer failed the new assertion; the unmodified
baseline passed (`.evidence/ticket008-cad-close-mutation/`). The old Node self-check,
Node response comparison and fallback are still present: replacement of the response
comparison and supported-gate integration of the new controls remain before removal.
The final controls and 36 scheduler tests passed
(`.evidence/ticket008-cad-close-verified.log`); temporary Chromium-owned files were
left at the path reported by that run rather than removed speculatively.

## 2026-10-27: CAD controls integrated; historical backend blocker

The supported browser gate now runs the controlled Rust consumer checks and the
real-provider cold-response/failure control, with logs retained under
`web/target/test-results/`. Their harness builds inherit the shared lock. Quick
checks omit the packaged-site cold control and explicitly name the browser gate
when either control script changes. The controlled worker now also tests six
vendor-import operations and the incomplete-vendor failure, replacing the remaining
worker-sequence/census cases of the Node self-check on the Rust consumer path.

The manual `tools/browser-harness/compare-cad-responses.py` now runs Rust capture
and Rust response probe on the same packaged site. Each must observe the five CAD
resources; triangle, buffer and STEP sizes must match. Other response-count
differences are reported, not treated as failures, consistent with the same-run
decision. The old command failed without Playwright
(`.evidence/ticket008-cad-compare-red.log`); the Rust-only command passed without
`NODE_PATH` on Chrome 145 (`.evidence/ticket008-cad-compare-green.log`). This is
two diagnostic captures on an existing site, not source-bound or timing evidence.

**Historical note, superseded by the explicit retirement decision and commit
`731cef14`:** Cadrum reproduction and its Node driver are no longer supported.

The current Monstertruck checkout rejects `--driver node` before creating output;
the new launcher test went red before this restriction and green after. The
historical Cadrum reproduction remains a live documented path in `measure-cad.py`:
it selects backend `cadrum`, whereas the actual Rust `cad-capture` executable
rejects that backend (`.evidence/ticket008-cadrum-rust-rejection.log`). The Node
driver, selector for historical checkouts, and its self-check therefore remain;
deleting them would remove that manual workflow without a verified replacement.
Port and verify the historical backend or explicitly retire that workflow before
removing them. This is not complete Playwright retirement.

The integrated scheduler commands passed on Chrome 145 with `NODE_PATH` unset;
the CAD launcher, scheduler and quick suites passed 28, 37 and 18 tests, and both
Node tooling protocol suites passed (`.evidence/ticket008-cad-integrated-final.log`).
The real current-checkout Node selection exited 2 before build or output creation
(`.evidence/ticket008-retired-current-node.log`). The browser controls use a
simulated worker and an existing local site respectively; the full browser gate,
full verification and hosted CI were not run in this slice.

## 2026-10-27: Graphite Node runners archived

The manual feedback and usability workflows now use only their existing Rust
launchers. Source-bound, same-served-package comparisons and negative controls are
recorded in [ticket 007](007-rust-graphite-acceptance.md#2026-10-27-source-bound-manual-cutover).
The former Node source is preserved under `docs/archive/graphite/`, not as live
entry points. Historical Cadrum remains a separate blocker to full Playwright
retirement; full verification and hosted CI are still outstanding.

**Superseded:** Cadrum is no longer a blocker because its reproduction was
explicitly retired in `731cef14`; the preceding paragraph records the earlier state.

## 2026-10-27: review follow-up on observed core failures

**Historical Cadrum follow-up superseded:** the user explicitly retired that
reproduction path, and `731cef14` archived the driver. The final paragraph below
records the earlier assessment, not a current requirement or blocker.

The ticket 006 decision requires recording responses actually observed, not a
fixed landing-asset inventory or a wait for missing responses. Earlier evidence
also shows unrelated 404s are legitimate observations. The review correctly exposed
a narrower problem: an actually observed failing core-worker HTTP response was
recorded but could still produce a successful capture. The Rust capture and probe
now fail when an observed `/assets/core-worker/` response has status 400 or higher.
The capture retains the observed resources in its failure event log. An absent
response remains inconclusive, not an invented failure; unrelated 404s and redirects
remain observations. This narrows and supersedes the prior conclusion that the old
fatal control was wholly obsolete.

The Rust regression was red before the observed-failure check and green afterward;
the focused unit test covers empty inventory, unrelated 404, redirect and core 404.
A real Chrome 145 controlled page fetched a missing core resource before revealing
the landing button; capture recorded the 404 and failed. Disabling the check was
killed by the public consumer control (`.evidence/ticket008-core-observed-mutation/`).
The existing real-site cold diagnostic also observed the injected 404 and recorded
a failed result (`.evidence/ticket008-core-observed-control.log`). The diagnostic
does not require that response when the browser did not observe it.

The old source commit `6c9c4867755bd2316f751783729a3ff9778fc04a` exists in Git;
the documented `t3-2088b1aa` checkout no longer exists. Its preserved directory
contains the no-opt Cadrum provider and a correctly pinned OCCT archive, and the
pinned container image is locally available. It does not contain a packaged old
page/Host worker. The retained runbook explicitly says no old page/worker pass was
established. Current `cad-capture` rejects `cadrum`, and the historical Host worker
differs substantially from the current one. Accepting the backend string or testing
only a simulated worker would not establish reproducibility. A separate historical
measurement checkout and source-bound old page/worker package, followed by real
Rust/Node protocol and failure comparisons, are required before removing that last
driver. It has not been deleted or relabeled as complete.

## 2026-10-27: final local verification and review disposition

On clean `fix/drag-save-undo` source `cca95c5d0709c4fe9ad0ba82ebaacef5e88e3743`,
the full supported check passed: all 61 commands ran and were accepted, including
repo, tooling, lint, build, native tests and browser. The receipt records 2,396
native tests passed, 10 ignored, and the page suite 113/113 passed; drag and CAD
provider/product smokes also passed. Evidence: `.evidence/full-check-cca95c5d-EZBJYe/receipt.json`
and its adjacent summary and `check.log`. The run used Chrome for Testing 145,
`NODE_PATH` unset, no Playwright, and two browser jobs. Its scope had a 400% CPU
quota, 12 GiB memory limit and no swap; recorded memory events before and after
show zero OOM and OOM kills (`scope.txt`). An earlier full run at `604adc8a` hit
the 12 GiB limit and OOM; this successful run followed reduced browser concurrency,
not a higher cap. The pass applies to the named source, not this later docs commit.

Independent Standards review passed. Independent Spec review identified the missing
final verification record, addressed here, and the unbound diagnostic status of
ticket 006's cache/service-worker controls; that parity gate is not established by
the full check. Ticket 006 remains claimed and this ticket remains open; these
documentation updates have not themselves received a closure review. The user
explicitly retired historical Cadrum reproduction; archived sources and historical
evidence remain, not a supported reproduction path.

Hosted CI has not run. Run the web job on a hosted PR or workflow dispatch and
record its URL and verdict, including the no-Playwright preflight, pinned Chrome,
Rust drag, mounted browser checks and failure artifacts. Also verify the native
job retains Node tooling and the release-build job provisions its web tools, as
in the CI verification plan above. Do not treat this local pass as hosted evidence.

## 2026-10-27: source-bound CAD fixture follow-up

The cache/service-worker fixture at clean source
`8537778b63bcae6e3c68f6b6ee302fe850af0546` now has an official build binding and
per-driver cold, warm and missing-worker observations; see
[ticket 006](006-rust-cad-runtime-capture.md#2026-10-27-source-bound-cache-and-service-worker-fixture).
It used the archived Node driver only as an isolated reference; the supported current
CAD launcher remains Rust-only and does not require Playwright. Both drivers bypassed
the page HTTP cache in cold mode and blocked registration, but neither bypassed the
dedicated-worker cache; warm workers controlled the page. Failure records are from
each failing run. A separate CDP semantics fixture, including an initially failed
controller expectation and corrected check, is not a capture or proof of generic
service-worker bypass in capture.

This does not resolve either ticket or change the approved timing result at `f0b6904`.
The full check passed at `cca95c5d0709c4fe9ad0ba82ebaacef5e88e3743`, not at the
fixture source or this documentation commit. Hosted CI and closure review remain
unrun; remaining retirement and ticket 006 acceptance requirements stay open.
