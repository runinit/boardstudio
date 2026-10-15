# Graphite continuation — PCB through onboarding

The approved Graphite direction now extends across the remaining workspace
presentations and entry flows. The single header, semantic `--wb-*` tokens,
pinned Dioxus Components controls, and existing application edit owners remain
the foundation. [Approved reference images and provenance](approved-suite/) are
preserved outside the ignored mock directory.

## Implemented surfaces

| Surface | Implemented behavior | Ownership retained |
| --- | --- | --- |
| PCB | Separate Placement/Wiring Inspector views; resolved assignments use a counted disclosure with separate Row pins, Column pins and optional Peripheral pins groups in matrix mode; direct assignments remain flat | Both panes stay mounted; controller placement, pin edits and Resolve/Apply use existing routes |
| Case | Assembly dock, opaque preview/view controls and named specialist disclosures | Existing mechanical controllers, field drafts, CAD currency and export blockers |
| Keymap | Layers and searchable keys in the left dock; Assign/Macros/Encoders focus shortcuts | Existing layer operations and key/macro/encoder editors; no layer-wide transparency toggle |
| Keycaps | Global board appearance in the left dock; matrix and selected-key disclosures; Profile/Legend/Color shortcuts | One settings owner, existing profile/legend/override editors and fit findings |
| Parts | Selected catalogue groups open; search reveals matching groups; specialist generator sections remain collapsible | Existing import, generation, preview/apply, assembly and placement owners |
| Export | Handoff list with actual availability, current accepted revision/local-save summary and explicit portable download | Existing stale/invalid output guards and export providers |
| Project browser | Saved-project selection, actual document geometry preview, explicit Open, deletion confirmation and current-project copy/rename routes | Existing local storage, demo-copy/import/new-project runtime operations; editor remains mounted while browsing |
| Setup | Dedicated five-stage process surface beside the real viewer; schematic hardware preview before layout geometry | Existing project creation, matrix creation, optional/resumable guide and stage readiness |

Contextual shortcuts reveal and focus the existing Inspector. They do not submit
edits or duplicate controllers. Hidden panes and native disclosures preserve
mounted drafts. The project browser removes irrelevant header history/save/export
controls; setup removes header history/export actions.

## Regression fixes

The Align popover assertion was unchanged. A preceding matrix-focus fixture left
its DOM mounted with an outside button focused; the popover correctly respected
outside focus. The reduced two-test sequence reproduced the failure. Removing
the completed matrix fixture fixes the pair and all 174 Layout browser tests.
See `focus-red.log`, `focus-green.log` and `layout-suite.log` under
`.scratch/graphite-continuation/`.

The Case thickness test now opens the native Manufacturing disclosure before
focusing its field. The original Enter/blur assertion remains intact; all 81 Case
browser tests pass.

Mobile browser selection originally left Open keyboard below the demo list
(measured Y=4021px in an 844px viewport). The selected-project handoff now moves
above the list at narrow widths and receives guarded focus. A mounted regression
proves selection previews the chosen document without opening it, then focuses
Open keyboard. The same regression verifies the selected preview follows an
accepted rename instead of keeping an old document snapshot. All 17 Library
browser tests pass.

## Verification

- `python3 scripts/check.py`: repository/docs, tooling, formatting, strict native
  Clippy, fast build, workspace native tests and CAD tests passed. The initial
  browser phase exposed the Case disclosure test above; it was corrected and
  rerun rather than treating cascade failures as separate defects.
- Browser phase resumed from the affected boundary: Case 81, Parts 68, PCB 49,
  Layout 174 plus isolated setup 1, and page presentation 72 tests passed.
  Earlier browser groups passed: storage 10, Runtime 33, UI model 5, shared
  controls 38 plus isolated panels 1, Keycaps 30, Keymap 40 and catalogue 12.
- Library was rerun after the mobile handoff change: 17 passed. Page presentation
  was rerun against the final Library code: 72 passed.
- Real entry flow passes: named project and matrix creation, portable download
  without embedded models, demo copy, import of the downloaded archive, and
  explicit reopening of the original saved project.
- Browser scripts use a dedicated verification profile, actual app workers and
  actual generated geometry. Source/logs are retained in
  `.scratch/graphite-continuation/`; final screenshots are preserved below.

## Visual comparison

Comparison follows the approved dock proportions, graphite surfaces, periwinkle
selection and opaque labeled action groups. Actual geometry, counts, pins and
readiness come from the project, not generated labels. The Case screenshot shows
a real PCB-only assembly because that fixture has no authored case bodies.

Desktop captures: [PCB](continuation-PCB.png), [Case](continuation-Case.png),
[Keymap](continuation-Keymap.png), [Keycaps](continuation-Keycaps.png),
[Parts](continuation-Parts.png), [Export](continuation-Export.png),
[project browser](continuation-Browser.png), [hardware setup](continuation-Setup-project.png)
and [Layout setup](continuation-Setup-layout.png), and
[real matrix creation](continuation-Setup-matrix.png).

Measured dark-theme contrast is 13.89:1 for body/panel, 8.16:1 for muted/panel,
8.36:1 for action foreground/accent, 7.77:1 for selection and 9.34:1 for
body/toolbar. [Computed token measurements](continuation-contrast.json) are from
the actual application. The 1359px/1024px/390px sweep checks page overflow and
reachable Objects/Canvas/Inspector views across all editor families. Context and
view controls are checked for overlap at intermediate width. Parts' view switch
stays in flow; compact Case panels sit above the scrim and occupy the available
width; setup suppresses the old drawer scrim.

Narrow captures: [Export](continuation-Export-mobile.png),
[setup](continuation-Setup-mobile.png), and
[selected-project handoff](continuation-Browser-mobile-selected.png).

## Fidelity follow-up — setup and assignment review

The setup Project and Layout stages were compared with the approved suite at
1586×992 and checked at 1024×768 and 390×844. The setup dock now uses a
step-navigation column beside the stage form at desktop and stacks both at
narrower widths; the real workspace preview stays docked on desktop and follows
the form on mobile. Page heading scale comes from the shared `--wb-type-page`
token, also exposed through the Tailwind theme. All six screenshots are linked from the
[visual-fidelity gap review](visual-fidelity-gaps.md). The run used the real,
single-board Graphite verification project, so its stage preview differs from
the mock's split wired keyboard by design. Horizontal overflow was absent at all
three widths and the setup footer was hidden at each width.

Resolved PCB assignments now use the shared disclosure while remaining open by
default. Matrix row and column pins, plus peripheral pins when present, are
separate collapsed groups; direct-mode assignments stay flat. Existing IDs,
controls and edit ownership are preserved. Case, Layout, Keycaps and Parts
already use the same shared control for right-pane specialist fields. Fixture
mismatches remain listed in the gap review instead of being corrected with
invented selection or model data. The focused `scripts/check.py quick` run after
this change passed repository and documentation checks, formatting, strict
Clippy, web typecheck, native tests and all 49 PCB browser tests. The test fixture
proves grouping from real assignment IDs; the visual project lacks a matching
controller plan, so no selected-wiring screenshot is claimed.

## Explicit boundaries

- Case height/thickness drag handles are excluded from scope per user direction.
  Existing numeric mechanical fields remain available.
- Renaming or downloading a non-current saved keyboard goes through Open keyboard,
  then the existing Current project field or download action. No second project
  editing/storage API was introduced.
- Hardware silhouettes are schematic. Matrix setup retains the real existing
  creation form and preview; this migration does not replace its document owner.
- The bundled-model preview failure was a proxy containment bug, not missing
  catalogue content. The proxy now permits direct staged model symlinks only
  into the vendored catalogue, while retaining traversal and unrelated-symlink
  rejection. All 88 model URLs return HTTP 200 with bytes matching their targets.
  The HTTP regression first reproduced the model 404, then passed after the fix.
  `python3 scripts/check.py quick --files scripts/dev-web.py scripts/test-dev-web.py`
  passed repository and tooling checks. A fresh browser context downloaded an
  archive with `embedUsedModels: true`, then imported it and reached the editor.
  ZIP integrity and SHA-256 checks of both embedded assets passed. The older
  verification profile cancelled its download; the fresh-context check avoids
  relying on that profile. Logs are `export-embedded.log` (download success) and
  `import-embedded.log` (import and editor handoff) under the scratch evidence root.
- Existing WASM warnings and the malformed Layers chevron are not new failures.
  Native Clippy passes. No visibility was widened to silence warnings.
- Impeccable workflow closure remains unclaimed. `build-phase advance` was
  executed: it refused at `plates` because "The plan and asset review is not
  accepted for this build." `build-phase completion` reports incomplete.
  The underlying plate-content check passes ("no plates owed"); `comps` and
  `spec` are closed. `hero`, `sections`, `motion`, `responsive` and `review`
  remain pending behind that receipt, rather than proven unsupported.
  Chat approval and saved notes remain valid project evidence. The skill's
  component-review rule says "Never submit the page or write a receipt on the
  user's behalf"; no CLI receipt was manufactured and no repeat approval was
  requested. The separate automatic pixel comparison reports drift. Manual
  review previously understated the material visual gaps. Functional verification
  remains valid, but visual acceptance is reopened. The [visual fidelity gap
  review](visual-fidelity-gaps.md) supersedes the earlier "no material defect"
  conclusion. Different project geometry and the user's removal of header
  navigation and Case handles remain intentional departures; the typography,
  control hierarchy and page composition gaps are unfinished work.

## Preview and process ownership

The frontend preview remains at [127.0.0.1:8182](http://127.0.0.1:8182/), backed by
its worktree-local build. The old long-running preview exited and was replaced
on the same ports. Only the Python proxy was replaced for the model fix; the existing Dioxus
process and its single Tailwind watcher were retained without recompilation.
Current process ownership and health are recorded in
`.scratch/ui-dev/runtime.json`. Temporary test/browser processes are closed after
verification; the saved-note recovery proxy at port 42679 is retained.

One pinned Tailwind v4.1.5 watcher is managed by the restored Dioxus preview.
The fast full build temporarily shared Dioxus generated output with the watcher;
its subpath build made the preview's JavaScript unavailable until the watcher
completed. Routine iteration continues with frontend-only compilation and the
pinned Tailwind compiler. No extra watcher or duplicate preview was added.
