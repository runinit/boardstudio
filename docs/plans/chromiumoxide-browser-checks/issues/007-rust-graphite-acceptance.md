# 007: Rust Graphite acceptance checks

Status: claimed
Type: build
Blocked by: [Rust drag smoke end to end](005-rust-drag-smoke.md)
Parent: [Chromiumoxide browser checks](../map.md)
Related: [Real-browser checks drive Chrome from Rust](../../cad-deepening/issues/016-rust-browser-harness.md)

## What to build

A designer can invoke the existing Graphite acceptance workflows through the Rust harness
with unchanged checks and readable PASS/FAIL output, without Playwright. Inventory both the
feedback and usability entry points; do not silently port only the one named in older prose.
Keep these manual against-server checks manual unless the current check already runs them.

## Acceptance

- [ ] Preserve existing assertions, environment/input contract, viewport, dark-scheme
  emulation, optional screenshot behaviour, output labels and exit status for each consumer,
  except the three superseded design assertions explicitly reconciled below.
- [ ] Run old/new against the same source/build/server state and compare the assertion inventory,
  verdicts and screenshot conditions. Existing app failures stay failures; unrun checks do not pass.
- [ ] Reuse the minimal harness's isolated launch, trusted input, bounded waits and fatal page
  error handling; add only helpers exercised by these consumers. No visual redesign in this slice.
- [ ] Negative controls demonstrate failed assertions/page errors produce failure output and
  nonzero exit. Preserve the supported check's explicit manual-exclusion list and entry-point docs.
- [ ] Keep old entry points available until parity passes; verify browser consumer dependencies
  remain installed for other unmigrated workflows.

## Verification and scope

Use TDD at the existing public launcher/output seams: demonstrate the expected regression
before the fix, then run affected tests and typechecking during development. At closure run
the full supported check and combined Spec + Standards review; record terminal verdicts,
source/build identity and any blocked gates honestly. No weakened assertions, masked page
errors, baseline deletions or retry-until-green. Preserve the archived prototypes and unrelated
work. Builds follow repository concurrency/lock rules; evidence uses supported receipts and
logs, not custom hash manifests. All checks must be runnable by humans and CI without agent tools.

## 2026-10-26: entry-point inventory

`test-graphite-usability.cjs` directly requires `test-graphite-feedback.cjs`;
both are manual exclusions in `scripts/test-check.py`. The feedback runner uses
full `playwright`, `CHROME_BIN` (default `/usr/bin/chromium`),
`BOARDSTUDIO_PREVIEW_URL` (default `http://127.0.0.1:8182/`) and optional
`BOARDSTUDIO_SCREENSHOTS`. It starts at 1513x1043 in dark mode, then checks each
of six workspaces at 1513x1043, 1024x1043 and 390x844. With screenshots set,
it creates the directory and writes one `<workspace>-<width>.png` per iteration.

A completed run prints 75 labeled checks: 27 before the workspace loop, six
Case/Parts checks, six workspace reference checks and 36 viewport checks. Waits
and interactions are also required: some labels print `true` only after a
successful wait. Failed labels and page errors accumulate; interaction or wait
errors abort the sequence, and either route exits nonzero. There is no structured
source/build receipt in this runner. A same-site comparison must record identity
outside it and must not treat unprinted labels as passing.

The shared Rust driver supplies isolated launch, role lookup, trusted clicks and
drags, bounded waits and fatal page-error collection. The port still needs the
consumer-specific setup, dark emulation, viewport changes, screenshots and all
checks, followed by negative controls and same-build comparison. No Rust Graphite
acceptance implementation or parity result is claimed. The public launcher RED
at this HEAD is `.evidence/ticket007-launch-red.log`: `cargo run --offline
--manifest-path tools/browser-harness/Cargo.toml --bin graphite-feedback` exited
101 because that binary does not exist. The current worktree also contains
uncommitted 006 drafts, so this is not a clean-source acceptance receipt.


## 2026-10-26: feedback port draft and setup blocker

The draft `graphite-feedback` binary and opt-in `scripts/run-graphite-feedback.py`
compile through the shared locked harness build. The launcher RED above remains
recorded. A connection-refused launch exited nonzero
(`.evidence/ticket007-rust-launch.log`). Full Playwright 1.62.1 was installed
into an isolated external task cache; repository and global dependencies were
not changed.

The old Node and Rust feedback runners were each run once against the same
existing packaged site and Chrome for Testing 145. Both exited nonzero while
waiting for `Add key matrix`, before printing any assertion labels
(`.evidence/ticket007-node-diagnostic.log` and
`.evidence/ticket007-rust-diagnostic.log`). The site is not bound to the current
dirty source; these are diagnostic failures, not clean parity or acceptance.
The source contains the `Add key matrix` control, but the old runner's later
`New matrix rows`, `New matrix columns` and `Edit existing objects` labels were
not found in Rust source. This needs investigation before changing the workflow
or claiming the remaining assertions can run. No product semantics were changed.

The port is unverified beyond setup and must not replace the Node entry points.
Its gesture, actionability, visibility, navigation and viewport settling semantics
still need direct comparison, as do assertion and page-error negative controls
and screenshot conditions. The second usability entry point, clean same-build
parity, full supported check, hosted CI and combined review remain open. No
acceptance box is cleared.

`python3 -B scripts/test-check.py` ran 31 tests and failed one inventory test:
the protected untracked `cad/scripts/test-cold-responses.py` is not registered.
The launcher passed Python compilation and `git diff --check` passed. The final
Rust source compiled after the diagnostic run; its later gesture changes were not
executed in a browser. The draft remains uncommitted.

## 2026-10-26: setup workflow investigation and partial comparison

On branch `fix/drag-save-undo` at `b817585c1946d4d8716439baa1af8d39e6cdb4e8`,
`web/target/site/site-subpath/boardstudio` was served at `/boardstudio/` on port
4187. Its index and service worker were packaged on October 26 at 22:25 local;
no source binding was available, so every browser result below is diagnostic
against an unbound build. The site returned HTTP 200 and accepted Create new
keyboard, Split keyboard and Continue to layout. The guide then already had
its matrix form open: `Add key matrix` and `Set up key matrix` were both absent.
Source shows Rows and Columns sliders and Hide guide; the old input labels and
Edit existing objects are absent. A direct diagnostic operated the sliders,
created matrix geometry and dismissed the guide. Home did not reset the slider,
so the runners instead use arrow keys from the reported value and verify 10.
The Node and draft Rust setup now retain the intended 10 by 10 fixture and
original assertions; they do not treat setup as an acceptance pass.

The original Node and Rust runs stopped at Add key matrix with no labels (RED,
prior logs above). With the updated setup, Chrome for Testing 145 against the
same unbound site produced Node 17 PASS and 3 FAIL before a 30 second wait for
`#m1-snap-dock-options` failed; remaining labels and screenshots were not run
(`.evidence/ticket007-node-workflow-cft145.log`). The failures were workspace
navigation placement, stagger/splay center alignment and Layers opening upward.
The Rust port initially failed selecting Key size because a section and select
share the label; targeting the select corrected that error. The subsequent Rust
run reached 12 PASS and 2 FAIL, including a failed real offset edit where Node
passed, then failed evaluating the missing stagger/splay handle after Undo;
its handler also reported a WebSocket reset
(`.evidence/ticket007-rust-workflow-cft145.log`). These are failures, not
parity. Chromium 156 additionally produced unsupported protocol messages in
the harness (`.evidence/ticket007-rust-workflow.log`).

The draft contains all 33 static Node check labels plus the corresponding dynamic
workspace and viewport labels by source inventory; this does not prove their
behavior. The gesture mismatch and later navigation/actionability checks need
investigation before further claims. No source-bound comparison, screenshot
parity, negative controls, usability launcher, full check or final review was
performed, and no acceptance boxes are cleared. No ticket 006 files were edited
for this investigation.

## 2026-10-26: offset and Snap investigation (still open)

The same packaged site was restarted at `http://127.0.0.1:4187/boardstudio/`
with Chrome for Testing 145. It remains unbound to the dirty source. A focused
Node/CDP probe used the original setup and Rust's trusted mouse event parameters:
a cancel followed by a second 30-pixel drag moved Offset X from 0 to
-14.287500000000001. Its second drag acquired pointer capture; the event log,
coordinates, targets and accepted value are in
`.evidence/ticket007-offset-cdp-probe.log`. This does not prove Rust parity.

Instrumenting the actual Rust runner reproduced its failed commit. Both pointer
presses landed on the offset handle, but neither drag acquired capture in the
first trace (`.evidence/ticket007-offset-rust-probe.log`). The second Undo had
not visibly settled. The runner now waits for its actual 1u × 1u preview before
starting the offset sequence. In the changed instrumented run the first drag
acquired capture, but the second still did not after Escape, and offset remained
unchanged (`.evidence/ticket007-offset-undo-probe.log`). The final uninstrumented
runner again failed the offset commit and then hit the missing stagger/splay
handle evaluation plus WebSocket reset (`.evidence/ticket007-offset-undo-final.log`).
The printed one-undo-step PASS following a failed commit is not evidence of undo.
The cancellation/readiness cause remains unresolved; no delay or capability
emulation was added and no product fix is claimed.

A separate live Snap probe clicked the actual Snap button. It changed
`aria-expanded` from false to true and exposed the inline `Snap increment`
selector; `#m1-snap-dock-options` remained absent
(`.evidence/ticket007-snap-probe.log`). Current source renders `LayoutSnapDock`
as an accordion and retains a separate menu in other contexts. The old check
requires an upward popover, so substituting an accordion assertion would change
the acceptance contract. The owner must decide whether that old visual requirement
still applies before changing or removing it. The original Node failures and
unrun checks remain as recorded above; no acceptance box is cleared. The 006
files remain untouched, and these overlapping 007 drafts are not staged.

## 2026-10-26: cancellation liveness trace (open)

On the same unbound packaged site and Chrome for Testing 145, a temporary
instrumentation of the actual Rust runner reproduced the offset failure under
`boardstudio-tests.slice` (`.evidence/ticket007-targeted-liveness.log`). The
first trusted press acquired capture; Escape targeted the focused offset handle,
then capture was lost. At the second press the same handle was still connected
and focused, and all 10 selected key overlays remained. That trusted press hit
the handle but did not acquire capture; the offset commit failed. The existing
unchanged-value wait does not establish that the cancellation transition has
settled. The later Undo PASS again cannot prove undo of a failed edit.

The earlier Node/CDP diagnostic is not an equivalent successful cancellation:
its first press did not acquire capture, even though its second did and changed
the value. A separate diagnostic sampled the Offset X input's disabled property
and waited for it to be enabled before the second drag; it was already enabled
and the second commit still failed (`.evidence/ticket007-readiness-probe.log`).
This does not rule out other runtime state or ownership guards. Source admits a
transform only for a current owner, saved lifecycle, no display preview or
runtime gesture, and an available interaction arbiter; cancel submits a preview
clear and releases capture and arbiter. The observed DOM alone cannot identify
which condition rejected the second press, and the packaged site is not bound to
this source. No readiness wait or product change was retained. The temporary
instrumentation was removed and the draft restored. A source-bound trace of the
actual admission state and a Node comparison that first verifies capture and
cancellation are still needed before a root-cause fix or parity claim.

## 2026-10-26: source-bound second-press admission (open)

In `/home/chris/.t3/worktrees/ts-boardstudio2/t3-8258b150`, branch
`fix/drag-save-undo` at `b817585c1946d4d8716439baa1af8d39e6cdb4e8`, a
fresh checked app and providers were built under `boardstudio-tests.slice`, the
shared build lock and `TMPDIR=$HOME/.cache/tmp`. App source was committed HEAD
plus a temporary tagged pointerdown diagnostic; existing 006/007 drafts remained
present. The first full `build-web.py --fast` completed providers but its page
failed to compile due to a type error in that diagnostic
(`.evidence/ticket007-admission-build.log`). After correcting the diagnostic,
`build-web.py --fast --skip-providers --output
$HOME/.cache/tmp/ticket007-admission-site` succeeded using those freshly built
providers (`.evidence/ticket007-admission-page-build.log`). The resulting site
was served at `http://127.0.0.1:4188/boardstudio/`; Chrome for Testing
145.0.7622.0 ran the draft Rust feedback sequence. This is an instrumented
source-bound diagnostic, not a clean acceptance build or Node/Rust parity run.

`.evidence/ticket007-admission-source-probe.log` records first pointerdown with
`button=0`, no drag, current owner/context, editable `Ready`, saved revision,
no preview or runtime gesture, matching matrix and free arbiter. It acquired
capture; Escape lost capture. The second pointerdown reached the offset handle
with all those same guards true except `editable=false` and lifecycle
`Applying`. Thus `transform_start_is_editable` rejected it at the requirement
`Lifecycle::Ready`, before pointer capture. The commit assertion failed; the
subsequent one-undo-step PASS does not prove Undo. The run also retained the
workspace-navigation failure, later missing-handle evaluation and browser
handler reset; the remaining checks were not run. The probe was removed from
both app and draft runner after capture.

Session sets `Applying` for active Core work, including preview requests and
other operations. `ClearPreview` queues behind existing preview work, marks a
matching in-flight preview cancelled and preserves transaction ownership. This
trace does not identify the particular active request or prove how long the
state lasts. The existing mounted lost-capture recovery test waits for preview
clear before the next drag; it does not test immediate trusted Escape followed
by another press. The old acceptance sequence intends a genuine cancel and
subsequent edit, but its unchanged Offset X wait can pass before cancellation
settles. The inputs are trusted and the sequence is physically possible, though
its automated interval is not established as representative of a human gesture.

A product/ownership decision remains: should an immediate press during
cancellation be captured or queued safely while distinguishing this transaction
from unrelated Core edits, or should acceptance wait for an observable completed
cancellation and test eventual recovery? Simply allowing `Applying` is unsafe:
it also covers unrelated work and captured-revision ownership. No product fix,
new readiness API, assertion change, Snap change or U1 fixture change is claimed.
A real same-build Node comparison with first capture verified, negative controls,
full acceptance and closure review remain open.

## 2026-10-26: remember the immediate second drag (focused, open)

The user chose to remember a second press during cancellation. The transform
handle now observes its own ClearPreview outcome and can capture the next drag
while that same owner's cancellation is pending. It retains the existing
selection, matrix, saved revision, runtime gesture and arbiter admission checks;
Applying work without a pending cancellation remains blocked. Preview and commit continue through
Session's existing ordered operations and transform delta resolver. No visibility
or public API was widened.

The mounted regression `escape_then_immediate_drag_is_remembered_while_preview_is_applying`
failed before the fix specifically because the immediate second press was not
admitted (`.evidence/ticket007-remember-red.log`), then passed 1/1 after the fix
(`.evidence/ticket007-remember-green.log`), including one commit and Undo.
`unrelated_applying_work_does_not_admit_transform_drag` and the existing
`lost_capture_cancels_and_a_subsequent_drag_recovers` each passed 1/1
(`.evidence/ticket007-remember-unrelated.log` and
`.evidence/ticket007-remember-lost-capture.log`). Mounted pointer events are
synthetic and capture is stubbed.

A fresh checked page from this HEAD plus the fix, using freshly built providers
from the preceding diagnostic, was served at port 4188; Chrome for Testing
145.0.7622.0 ran the draft Rust feedback sequence. The tagged capture trace
shows both trusted presses acquired capture and each lost it; the second offset
edit and its one Undo check passed (`.evidence/ticket007-remember-real-browser.log`).
The capture probe was removed. The same run still failed workspace navigation,
stagger/splay alignment and Layers upward, then stopped at Snap with a page
exception and browser-handler reset. These are not full acceptance or parity.

The user's no-subagents boundary prevents the required combined closure review;
no closure, clean-build parity or full check is claimed. Snap and U1 remain
unchanged, and all pre-existing drafts remain unstaged.

## 2026-10-26: observable cancellation verdict (focused, open)

Both feedback runners now first observe the selected key move on screen during
a captured drag and return to its original position after Escape, with Offset X
unchanged. A separate cancellation immediately followed by the second drag
retains the queue behavior check and the no-commit blocks Undo rule. Separating
them avoids waiting for visual restoration before the rapid second press.

On `fix/drag-save-undo` HEAD `76402f6f6bc3b49d7ec429b015137fcb3a546091`, the
dirty runner source identity at execution was
`ed48bc934b323ba3a625d76036344ee769644b4b9fc382c858993ea50b1cf308`.
The previously built site had build identity
`871fc7d5d8b50d6e2c138e44a12a5ce981df6eb6ac612a0de9bbab51a1a85db7`;
its recorded before/after build source identity was
`b5c7c91ed904de90903eefb5aea4e63b86bd0407c5caeebf976ab41baa8aded1`.
The runner changes postdate that build. The local server served that build's
index; the identity helper does not independently bind all served assets.

Chrome for Testing 145.0.7622.0 ran Node and Rust against the same local site:
`.evidence/retro-graphite-restoration-node.log` and
`.evidence/retro-graphite-restoration-rust.log`. Both observed restoration,
then a second drag commit and one Undo. Both exited 1 after workspace
navigation, stagger/splay alignment and Layers upward failures; Node timed out
at Snap, and Rust hit a Snap page exception and browser-handler reset. Later
checks did not run. These are focused observations, not full acceptance or
closure; Snap and U1 decisions remain pending.

## 2026-10-26: navigation, rail and Layers design drift (diagnostic, open)

On `fix/drag-save-undo` HEAD `76402f6f6bc3b49d7ec429b015137fcb3a546091`,
with the existing 006/007 drafts preserved, a focused Playwright/Chrome for
Testing 145 run used an isolated browser context and the existing packaged
`/home/chris/.cache/tmp/retro-graphite-site` served by `scripts/serve-web.py`
at port 4191. The server's returned index matched that package's index. The
prior build receipt identifies its source and package, but this run did not
verify every served asset or freshly rebuild; it is a diagnostic, not a
source-bound acceptance receipt. The initial server command specified the
wrong build-root level and failed before opening a port; the corrected command
used the package root. The run exited 0 and its raw observations are in
`.evidence/ticket007-navigation-rail-layers-probe.log`.

- Navigation: the real UI had one workspace nav in the header, none inside
  Objects, six named buttons and Layout marked current. The source composes
  `WorkspaceObjectNavigation` inside the header. The original assertion
  expressly requires the opposite. The approved
  `docs/design/graphite/usability-2026-10-13/README.md` expressly supersedes
  the earlier Objects placement; `DESIGN.md` also contains conflicting older
  Objects-placement paragraphs alongside its newer header guidance.
- Stagger/splay: ten selected key overlays had center x=606.495; the Stagger
  and Splay hit areas were centered at x=596.495 and 616.495 respectively.
  The group center matched the selection, while the old assertion demands
  each individual center within 3 px. The resolved handle-rail ticket 03
  specifies adjacent tokens in one centered rail, and the connected code
  implements that layout. This observation did not test gesture behavior.
- Layers: the collapsed trigger was at y=900; opening moved it to y=668 and
  showed its accordion list at y=708–916, below the expanded trigger. The
  old assertion compares the list to the *pre-click* trigger and requires a
  popover above it. The approved usability document describes in-pane
  disclosures; source uses `CanvasLayers` with an accordion. Its initial
  collapsed state and bottom placement passed in the prior broad run.

These failures are consistent with superseded visual assertions, not proof of
an app regression. The navigation and rail checks could be replaced with
checks for a single header nav with six current/routable workspaces and a
centered adjacent rail; Layers could assert an accessible, visible disclosure
and its visibility controls. Those checks would change the original assertion
inventory and need an explicit reconciliation with this ticket's requirement
to preserve every assertion. Keeping the old checks preserves parity but
makes acceptance red against the approved newer design. No assertion or app
behavior was changed here. Snap's accordion/popover decision and U1's format-7
fixture decision remain pending. The remaining workflow, screenshot parity,
negative controls, full check and combined review remain unrun or blocked;
the earlier Rust Snap exception and handler reset are still failures.


## 2026-10-26: user-authorized design reconciliation (focused, open)

The user explicitly authorized assessing and replacing wrong tests, narrowly reconciling
007's preservation wording for the three superseded assertions, while retaining meaningful
coverage and leaving Snap and U1 decisions untouched. The approved Graphite usability
README expressly replaces Objects navigation with six header workspaces and specifies
in-pane Layers disclosures; resolved layout-controls-polish ticket 03 specifies adjacent
handles in a centered rail. The older contradictory Objects paragraphs in DESIGN.md are
not authority to reverse that later decision. No other assertion is waived.

In both feedback runners the old Objects placement assertion is replaced by a unique
labelled header nav with all six named controls, their panel relationships and one current
Layout; the existing workspace tour also checks the current marker after each route. The
individual handle center assertion is replaced by visible, focusable Stagger and Splay
buttons in the same adjacent row, centered as a group above selected geometry. Existing
Ctrl offset capture, cancellation, commit and Undo checks continue to exercise gesture
interaction; the resolved rail ticket records separate mounted Splay/Stagger interaction
coverage. The upward Layers assertion is replaced by the labelled expanded disclosure,
its aria-controls relationship and visible controls, followed by hiding and restoring
keycaps and closing the disclosure. The original initial-collapse and bottom-placement
checks remain. This does not establish every rail gesture in a live browser.

The prior old-assertion RED is `.evidence/retro-graphite-restoration-node.log` and
`.evidence/retro-graphite-restoration-rust.log`; the focused geometry and DOM observations
are `.evidence/ticket007-navigation-rail-layers-probe.log`. On HEAD
`76402f6f6bc3b49d7ec429b015137fcb3a546091`, with existing dirty 006/007 drafts and these
runner changes, Chrome for Testing 145 ran both updated consumers under
`boardstudio-tests.slice` against the same existing package at port 4191. This is the
unbound package described above: no fresh build or full served-asset identity check.
The initial server invocation used an incorrect root and the Node run failed before
opening the page; the corrected server was started and readiness observed before the
reported runs.

`.evidence/ticket007-reconciled-node.log`: 24 PASS including the replacements and
visibility restoration; exit 1 at the existing 30-second Snap options wait, with later
checks unrun. `.evidence/ticket007-reconciled-rust.log`: 76 PASS and 8 FAIL, including
passes for the replacements, visibility restoration and current marker in all six
workspaces. Failures: Snap upward, PCB layout grouping, Case camera controls, Case toolbar
centering at 1024/390, Parts toolbar centering at 1513/1024/390; a WebSocket handler reset
also caused failure. Rust built through its locked launcher. These are observations, not
full acceptance or old/new parity; the later Node workspace checks did not run. No Snap
assertion or U1 fixture was changed. No acceptance box is cleared. Screenshot comparison,
negative controls, clean source-bound full checks and combined closure review remain open.
The user's no-subagents instruction blocks that review in this slice.

## 2026-10-27: PCB expansion and Case readiness (focused, open)

On `fix/drag-save-undo` at `76402f6f6bc3b49d7ec429b015137fcb3a546091`,
with existing dirty drafts, Chrome for Testing 145 ran against the existing
`retro-graphite-site` package at port 4191. This is the previously described
unbound package, not a fresh source-bound acceptance build. The Rust launcher
built under its shared lock and exited 1: 77 PASS and 7 FAIL, plus a WebSocket
handler reset (`.evidence/ticket007-pcb-validation-rust.log`). The corrected
expansion of Left half passed `layout grouping restores matrix tree` and the
category check passed. A separate Node diagnostic also passed both checks.

The original Rust Case camera assertion failed before the viewer mounted; its
1513px centering check could pass without a toolbar. A Node diagnostic that
recorded the Snap failure without waiting for the absent popover, then sampled
Case, found a mounted viewer with its camera group and persistent off-center
Case toolbar (`.evidence/ticket007-case-probe.log`). Both runners now wait for
`.m1-case-view` before their existing Case assertions; this does not assume the
camera exists or change the centering check. The earlier Rust run is the RED
for the premature camera result. The revised Rust run printed 77 PASS and
7 FAIL, including camera and PCB grouping PASS and Case centering FAIL at all
three widths (`.evidence/ticket007-case-wait-rust.log`). It also reported a
WebSocket handler reset and exited 1. A Node diagnostic with only the same Snap
short-circuit passed the camera and PCB checks and failed the same Case and
Parts centering checks at all widths (`.evidence/ticket007-case-wait-node.log`).
These diagnostics are not full Node/Rust parity: the normal Node runner still
stops waiting for the Snap popover.

The current Layout Snap implementation is an inline accordion, matching the
newer DESIGN.md direction; the prior upward-popover assertion remains intact
pending its explicit reconciliation. Approved usability guidance requires
primary toolbar centering. Current CSS places the Case toolbar at flex-start
and the Parts 2D controls at the right; the observed failures therefore merit
product/design investigation rather than a silent test waiver. No product or
CAD files were changed. `node --check`, `git diff --check`, and `rustfmt
--check` on the Graphite binary passed. Manifest-wide `cargo fmt --check`
failed on an independently dirty CAD worker file; that file was not changed.
Clean source-bound parity, screenshots, negative controls, the full supported
check and closure review remain open.

## 2026-10-27: Case and Parts toolbar centering (focused, open)

On `fix/drag-save-undo` at `76402f6f6bc3b49d7ec429b015137fcb3a546091`,
with the existing dirty drafts present, a full `build-web.py --fast` ran under
`/home/chris/.cache/tmp/boardstudio-build.lock` and built the source into
`/home/chris/.cache/tmp/ticket007-centering-site`. It was served at
`http://127.0.0.1:4194/boardstudio/`. A focused real Chrome for Testing 147
probe produced `.evidence/ticket007-centering-source-red.log` (exit 1): at
1513, 1024 and 390 px respectively, the Case toolbar was 279.54, 62.5 and
5.5 px left of its actual viewer center, Parts 2D was 411.04, 194 and 137 px
right of its preview center, and Parts 3D was 367.04, 150 and 93 px left of
its viewer center. The viewer and workspace horizontal bounds matched in these
states; the 3D viewer is vertically nested in Parts. No page overflow was seen.

The product CSS now centers the shared 3D toolbar within its viewer and places
Parts 2D controls in their own centered row below the title, avoiding overlap
with long titles. The 3D settings panel is centered under its toolbar to keep
it within the narrow viewport. A page rebuild using the freshly built providers,
`--fast --skip-providers`, produced `/home/chris/.cache/tmp/ticket007-centering-fixed-site`
under the same shared lock. Against that package, the same focused probe exited
0 (`.evidence/ticket007-centering-green.log`): all nine toolbar centers were
within 0.02 px of their viewer centers, with no page overflow or page errors.
The Parts 3D settings panel was also measured inside the 390 px viewport
(`.evidence/ticket007-centering-settings.log`). These are focused browser
observations, not full Graphite acceptance or visual review of every state.

The existing Rust feedback launcher against the fixed package exited 1
(`.evidence/ticket007-centering-feedback.log`): 83 PASS and 1 FAIL, including
all six Case/Parts centering checks PASS; the remaining assertion failure was
Snap's upward-popover requirement. The Chrome 147 run also reported 26 browser
handler errors, including unsupported protocol messages and a WebSocket reset.
No Snap assertion or other draft was changed. `git diff --check` passed. The
full supported check, same-build Node/Rust parity, negative controls and final
combined review remain open; no acceptance box is cleared.
At 390 px, a further browser pass closed the compact Inspector and visually
inspected the settled Case and Parts 2D layouts (`.evidence/ticket007-centering-mobile-settled.log`);
the Parts 3D screenshot was obscured by a panel transition, so its narrow layout
has geometry evidence but no unobscured visual inspection in this pass.

## 2026-10-27: Snap accordion and Chrome pairing (partial, open)

`DESIGN.md` specifies inline Snap in Objects; `LayoutSnapDock` uses the shared
accordion. The prior upward-popover check is stale. Node and Rust now exercise
keyboard Enter, expanded/collapsed ARIA state, focus, and changing and retaining
`Snap increment` across closure. The previously recorded old check failed on the
source-built centering package. The updated Node feedback run against that package
at `http://127.0.0.1:4194/boardstudio/` with Chrome for Testing 145.0.7622.0
exited 0 with 87 PASS (`.evidence/ticket007-snap-node.log`). The Rust feedback
run exited 0 with 86 PASS (`.evidence/ticket007-snap-enter-rust.log`); its
existing rapid-cancellation capture guard passed but lacked the Node output label,
which was subsequently added. This package predates runner edits; no independent
served-asset identity or clean-source acceptance is claimed.

The shared Rust key helper sends raw keydown/keyup; Enter did not activate the
button with that sequence (`.evidence/ticket007-snap-rust.log`). A complete trusted
Enter keydown including character text works. A Space experiment failed in both
Node and Rust (`.evidence/ticket007-snap-space-node-usability.log` and
`.evidence/ticket007-snap-space-rust.log`); this remains an accessibility issue
for the product owner, not an accepted waiver. The Node test was restored to Enter.

The prior Chrome 147 run reported `Network.requestWillBeSentExtraInfo` protocol
decode errors with full event payloads and a reset. Chrome for Testing
145.0.7622.0 is the documented companion to chromiumoxide 0.9.1. In the completed
Rust 145 run there were no decode errors; one WebSocket reset occurred after the
existing successful shutdown boundary and was reported as such. Earlier aborted
runs reported a reset as fatal because their boundaries were incomplete. No handler
filter or vendored CAD/handler code was changed.

Screenshot parity, failure/page-error negative controls, clean source/build/served
identity, full supported check, and combined closure review remain open. The
Space accessibility defect and any failure in remaining paths must be resolved
before acceptance. No acceptance box is cleared; no push was made.

The Node and Rust usability entry points were then run against the same package
and Chrome 145: both exited 0 with 87 PASS and identical PASS label sets
(`.evidence/ticket007-snap-enter-node-usability.log`,
`.evidence/ticket007-snap-enter-rust-usability.log`). Rust reported the same
post-boundary reset after clean exit. These runs did not enable screenshots and
do not close the outstanding gates above. `node --check`, Graphite binary
`rustfmt --check`, and `git diff --check` passed.

## 2026-10-27: Space verification and comparison (diagnostic, open)

On dirty `fix/drag-save-undo` HEAD `76402f6f6bc3b49d7ec429b015137fcb3a546091`,
Chrome for Testing 145.0.7622.0 exercised a fresh package served locally at
`/boardstudio/`. A trusted Space-primary drag moved a matrix element 80 by 40
pixels on release and it stayed at that position after further pointer movement;
Snap opened with Space and closed with Enter
(`.evidence/ticket007-space-verify-release-probe.log`). The first probe timed out
because it incorrectly expected movement before pointer-up; navigation applies
the camera delta at release (`.evidence/ticket007-space-verify-probe.log`).

Node and Rust usability and feedback entry points each exited 0 with 87 PASS and
no FAIL (`.evidence/ticket007-space-verify-{node,rust}-{usability,feedback}.log`).
The usability runs each produced 18 screenshots with identical filenames; they
were not visually compared. Node labels Snap opening with Space and focus, whereas
Rust labels opening by keyboard and exercises Enter. This is a real assertion and
label parity gap, despite the successful separate Space probe. Rust reported the
post-boundary WebSocket reset after clean child exit.

The shared locked build completed, but the full dirty source identity changed
from `c70b2f4ed1da92b6347ec4858719b31d2ce9d72abe083d22a8812527d5a946bb`
to `b6934535ab067f758561386d45999b7f9591e6d48e707c19497ed5522f521cfa`
during it. A second locked build also saw identity change
(`.evidence/ticket007-space-rebuild-before.log` and
`.evidence/ticket007-space-rebuild-after.log`). Other work was active in the
shared checkout. The first package identity was
`d081d1484a9b1b39edef6023c044413f1fb85d117abe6eee01df695e73fafcd0`;
the identity helper does not independently bind the served assets. These runs
are diagnostic, not source-bound acceptance. Assertion and page-error negative
controls, source/build/served binding, screenshot visual comparison, full
supported check and combined closure review remain open. No acceptance box was
cleared.

## 2026-10-27: Rust Space parity and negative controls (focused, open)

The Rust feedback runner now sends a trusted Space keydown and keyup to open Snap,
using the same `Snap accordion opens with Space and keeps focus` label as Node.
The subsequent Enter close/reopen and increment persistence checks remain. The
previous section documents the pre-change mismatch: the Rust run exercised Enter,
so its 87 PASS could not establish Space parity.

Against the previously built `ticket007-space-verify-site`, served at
`http://127.0.0.1:4197/boardstudio/`, Chrome for Testing 145 ran the Node and
Rust feedback entry points: each exited 0 with 87 PASS and identical ordered
labels (`.evidence/ticket007-space-parity-node.log` and
`.evidence/ticket007-space-parity-rust.log`). The Rust check verifies the
increment element exists; Node additionally checks its visibility. Rust reported
its existing post-boundary WebSocket reset after clean child exit. These runs did
not repeat the usability entry points or visually compare screenshots.

Controlled HTML injections against the same package added a canvas measurement
card or raised an uncaught page error. Node and Rust each exited 1 for each
control. The assertion controls reported `FAIL canvas numeric card removed`
(`.evidence/ticket007-negative-assertion-{node,rust}.log`); the Rust control
also reported a browser-handler reset. The page-error controls reported the
injected `ticket007 negative page error`
(`.evidence/ticket007-negative-error-{node,rust}.log`). The Rust assertion
control used its launcher; the page-error control used the already-built binary
because another task held the shared build lock.

`node --check`, `rustfmt --edition 2021 --check` on the Graphite binary and
`git diff --check` passed. The harness build used the shared lock. The served
package predates the runner change, and concurrent edits and another build in
the shared checkout prevented a stable source/build binding; no source-bound
full comparison is claimed. Screenshot visual comparison, full supported check,
hosted CI and combined closure review remain open. No acceptance box was
cleared and no push was made.

## 2026-10-27: Snap visibility parity (focused, open)

Rust now checks that `Snap increment` is visible when Space opens the accordion,
requires a visible and enabled select before setting it, and checks the expanded
state on persistence. It retains the existing stricter absence check on close.
A Chrome 145 controlled hidden-select probe showed the old Rust expression true
while Node's `isVisible` returned false (`.evidence/ticket007-snap-visibility-red.log`).
This is a focused browser probe, not an end-to-end negative control of the runner.
The updated Rust feedback launcher then exited 0 with 87 PASS on the earlier
package (`.evidence/ticket007-visibility-rust.log`). Rust reported the existing
post-boundary WebSocket reset after clean child exit.

A fresh `build-web.py --fast --routes subpath` ran under the shared build lock;
the dirty source identity before and after was
`9a0711d109b0a720ea8ff9899a08e8a56d65b50d614057f0cf400cb92da907d0`
(`.evidence/ticket007-parity-build-{before,after}.log`,
`.evidence/ticket007-parity-build.log`). The local package digest remained
`339e42050921d04d6a8bb1668fbc4843dd95285e74b8dc0fcbe322245c48b300`
through the four comparisons, served directly from that package by a local HTTP
server. Each Node and Rust feedback and usability run exited 0 with 87 identical
ordered PASS labels, and produced 18 matching screenshot filenames and dimensions
(`.evidence/ticket007-parity-{feedback,usability}-{node,rust}.log`). A thumbnail
contact-sheet comparison of feedback screenshots showed corresponding layouts;
pixel differences remain, and that thumbnail inspection is not detailed visual
acceptance. The existing assertion and page-error negative controls are recorded
in the preceding section and were not repeated.

The dirty source identity changed during the comparison, from the build identity
above to `13726ea93c60350a3f1b9c4e235c5b4609acce364cb6720a26a5e006f42b3034`
(`.evidence/ticket007-parity-run-{before,after}.log`); concurrent U1 work was
active. The identity helper does not independently bind HTTP responses, so this
is diagnostic same-package evidence, not source/build/served-bound acceptance.
`rustfmt --edition 2021 --check` on the Graphite binary and scoped ticket
`git diff --check` passed. The final repository-wide diff check reports a new
blank line at EOF in concurrently edited ticket 006. Full supported check,
detailed screenshot review, source-bound parity,
visibility-specific end-to-end negative control, hosted CI and combined closure
review remain open. No acceptance box was cleared and no push was made.

## 2026-10-27: Snap design authorization

The user explicitly authorized the current inline Snap accordion as the Graphite
acceptance design, replacing the old upward-popover requirement. Retain the
keyboard, focus, visibility, and increment behavior checks.

## 2026-10-27: Case visibility parity (focused, open)

The Rust Case stackup and footer assertions now use the shared driver's visible
box check. A Chrome 145 probe confirmed the previous width-only expressions
accepted both `visibility:hidden` controls while Node `isVisible` rejected them.
On the existing `ticket007-visibility-site` package, Rust feedback exited 0 with
87 PASS (`.evidence/ticket007-case-visibility-rust.log`). Against that same
package with both Case controls hidden by injected CSS, Node and Rust each
reported both Case labels FAIL and exited 1 (`.evidence/ticket007-case-hidden-
{node,rust}.log`). The Rust negative run also reported a browser-handler reset.
These are focused checks against an existing package, not source-bound acceptance;
the outstanding ticket gates remain open.
