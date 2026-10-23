# 023: Library and Export

Status: resolved
Type: build
Blocked by: [006: Accessible Select and Combobox](006-accessible-select-combobox.md), [011: Docks and narrow-screen sheet](011-docks-and-narrow-sheet.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [inventory](../../rust-ui-migration/inventory.md), [mapping](../../rust-ui-migration/mapping.md), [shared contract](../map.md#shared-contract)

## What to build

The project library and Export page, the project-delete confirmation as an Alert Dialog, and the setup guide with a vertical Stepper inside its four accordion sections. Applies the shared contract in the plan map: raw controls and `Board*` become the mapped components, inline messages are classified, icon-only buttons get Tooltips, selects follow the 12-option rule, tests find controls by role and name, and the lane deletes only the old CSS its files used.

## Acceptance criteria

- [x] The library, Export, firmware export and setup-guide rows of the inventory have zero raw controls and zero `Board*` uses
- [x] Deleting a project uses an Alert Dialog that starts on the safe action and returns focus; the setup guide's current and ready steps use Stepper and stay freely navigable
- [ ] Existing tests pass after the rewrite rule; Export blockers stay inline
- [x] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches


## Implementation journal

Binding verified via T3 `t3_worktree_status`: attached child worktree
`/home/chris/.t3/worktrees/ts-boardstudio2/t3-b3957019`, branch
`feat/library-export-ticket023`; initially clean at `b237903705244a266b07a64e5f23b8a3b5219c03`.
Fast-forwarded committed local `feat/rust-ui-migration` to
`c032d902ba3ae24c20ea588e5349fbe1e9559af5` before claim/setup. Parent dirty docs
were neither copied nor touched. Claim commit precedes implementation.
Exclusive new target: `/home/chris/.cache/boardstudio-target/023-library-export-lineage`;
ignored settings and CAD caches copied using the established lane setup, no new lane.

Pre-agreed seams (before tests):
- Mounted actual Library controls and Runtime owner callbacks: catches deletion
  identity/Cancel/safe initial focus and rename/demo lifecycle regressions; misses
  packaged CSS and trusted keyboard behavior.
- Mounted actual Export/firmware rows with real local owner callbacks: catches
  eligibility, inline blockers and operation outcomes; misses production providers.
- Mounted ProjectSetupGuide with owner stage/name callbacks: catches current/ready
  composition, free navigation and draft continuity; misses packaged responsive CSS.
- Packaged page role/name workflows: catches trusted keyboard, overlay focus return,
  Ctrl+K integration, popup clipping, touch targets and Theme styling; misses screen
  readers, native desktop and global CAD/human/final merge acceptance.

Scope reaches Library/KeyboardCard/DemoActionCard and owned tests, ExportWorkspace/
ExportRowView/ZmkFirmwareExportRow and test-only ExportPanel, ProjectSetupGuide and
its Matrix focus integration, plus only their owned legacy CSS. Preserve 012 tool
request/name ownership, 013 outcome classification, 016 controls and stage/draft
owners, 017 mechanical settings, 020 Parts and canonical renderer unchanged.


Deletion slice: `07ffde6c` is actual product RED (`delete-red.log`, 0/1): the named
Alert Dialog does not exist. Initial GREEN compile failures (`delete-green.log`)
were prop mismatches, corrected using existing attributes and a typed callback.
`delete-green-corrected.log` and `delete-focus-asset-diagnostic.log` distinguish a
mounted-fixture limitation: `window.createFocusTrap` is undefined; wasm-pack does
not serve the primitive's manganis focus asset. The identical initial Cancel,
Tab trap and opener-restoration assertions are assigned to the pre-agreed actual
packaged browser workflow; the mounted test retains named modal semantics,
Cancel and persisted identity assertions. No product focus workaround or shared
primitive change is introduced to satisfy this fixture. Diagnostics retained.

### Intermediate slice evidence

- Normal deletion named-modal/Cancel identity GREEN: `delete-green-mounted.log`,
  1/1. Focus-asset diagnostic records the unresolved manganis placeholder URL and
  `window.createFocusTrap === undefined` in the wasm-pack fixture, not a product
  focus workaround.
- Unreadable deletion: the first compile attempt retained a removed fixture
  variable (`unreadable-red.log`); the next attempt hit the incumbent native
  prompt's WebDriver transport error (`unreadable-red-corrected.log`), neither is
  product RED. Restoring only the exact committed incumbent native-confirm block
  and its existing native-dialog boundary fixture establishes actual RED in
  `unreadable-red-native-boundary.log`: the named Alert Dialog never appears.
  The copied-Alert implementation passes 1/1 in `unreadable-green.log`; healthy
  project retention, canceled raw-record retention and confirmed identity remain
  asserted. Git checkpoint `653d8279` preserves this slice; no source-copy or
  custom-manifest evidence was generated.
- Guide expected RED: `guide-red.log`, 0/1, named current stage unavailable.
  The first implementation compile needed the quoted ARIA description attribute
  (`guide-green.log`). Corrected GREEN: `guide-green-corrected.log`, 1/1,
  including all four vertical status compositions, independent current/readiness,
  free navigation and the same mounted name field/draft across stage changes.
  Only keyed status presentation remounts; Accordion and form owners do not.
- Native Library: `cargo test -p boardstudio-web-library --lib --locked`, 5/5
  (`library-native.log`). The canonical renderer, all manifests/locks, Runtime
  operation classification, Parts and mechanical implementations are unchanged.
- Library batch failures are retained (`library-mounted*.log`). The first was an
  old immediate-parent CSS assertion after adding Input Group; equivalent label
  ownership and draft/commit assertions pass in `library-name-green.log` (1/1).
  A subsequent isolated palette failure exposed `inert="false"` on the caller's
  open content. The caller now omits inert when open, and forwards its explicit
  expanded/controls owner values through Button's existing canonical props.
  `library-palette-green-aria-corrected.log` passes the unchanged consume-once,
  stale-project rejection, focus and no-replay workflow (1/1). No shared control
  API or renderer was changed. Initial compile/diagnostic failures remain in the
  adjacent logs. A later retry test's wrong role/name fixture label is corrected
  from Current keyboard to its actual Current board; durable retry assertions
  are retained and run separately before another batch.

Live app lineage was additionally verified from the server's thread metadata:
current task thread `thread:delegated-task:command%3Amcp%3Add75e685-6104-454e-94dd-e2e71f7bb137%3Adelegate-task%3Arust-ui023-implement-library-export-r1`
is a `subagent` of coordinator `9b186519-9e71-4e26-a24b-6587e94c69e4`, with the
same child branch and worktree path recorded above. No additional thread, agent
or watcher has been started. `delete-retry-green.log` now passes the isolated
real replacement-failure/retry workflow, 1/1.

### Coherent scoped checks

- `python3 scripts/run-wasm-tests.py --files web/src/presentation/export_workspace.rs web/src/presentation/zmk_firmware_export.rs --result-json .evidence/023/export-mounted-result.json`
  passes all **6** actual ExportRowView/firmware controls and callback workflows;
  `export-mounted.log` and the supported runner JSON retain names/results.
- `wasm-pack test --headless --chrome --mode no-install web/crates/layout --lib -- setup_guide::`
  passes **2/2** (`guide-mounted-complete.log`), including 016's existing desktop
  guide/panel composition. The first Matrix command selected no tests
  (`matrix-focus.log`), not a pass. Corrected exact module filter
  `objects::matrix_setup::focus_tests::` passes **2/2** in
  `matrix-focus-corrected.log`, retaining both required Matrix focus behaviors.
- `wasm-pack test --headless --chrome --mode no-install web/crates/library --lib`
  passes **19/19**, no filtered tests (`library-mounted-complete.log`), after the
  diagnosed fixture-label correction. This includes real replacement failure and
  retry, Cancel/Confirm identity, raw unreadable records, current/noncurrent
  deletion, sorting, preview selection, rename drafts, ordered Undo and palette
  consume-once/stale-owner behavior. Earlier failed batches are not passes.
- All direct wasm-pack launches above use the unchanged repository runner's
  `runner_environment` and `desktop_webdriver_config` through an absolute
  temporary wrapper, with its locked runner and `CARGO_PROFILE_DEV_DEBUG=false`.
  They ran sequentially against the one exclusive target. No test was skipped.
- `BOARDSTUDIO_CHECK_JOBS=1 python3 scripts/check.py build --receipt .evidence/023/build-receipt.json`
  passes the documented checked build (**320.5s**); receipt and summary are in
  `.evidence/023/`. A supported `build-web.py --fast --skip-providers` refresh
  packages the final owned presentation CSS using the already-built providers.

Scope inspection additionally found the existing full-case-settings Back button
in Editor's guide composition: it now uses the copied Secondary text Button with a decorative back arrow,
leaving MechanicalSettings and its callbacks untouched. The explicit text avoids
an icon-only popup at the drawer edge; its actual navigation/44px target are in
the packaged workflow. Guide name
failures use FieldError. `setup_workspace.rs` is state-only in production and is
unchanged. Compact callers use the actual copied `Sm` 36px size, per the pixel
mapping rule; ordinary Buttons are 40px and narrow/coarse targets 44px.

The only 012 shared hunk is the production browser test's deletion block: native
delete `cancel`/`close` event assertions become copied-Alert state, Cancel/Escape,
identity and exact opener assertions. All native **palette host** assertions,
trusted Tab boundaries, rapid Ctrl/Cmd+K reopening, AX role checks and one trusted
Confirm click remain. No palette implementation or project-tool owner is edited.


### Packaged regressions and owned corrections

The actual packaged role/name workflow catches a pre-existing delayed Preview
focus task reading its dropped Library signal after Open keyboard unmounts it
(`browser-export-layout-red-styled.log`, panic overlay capture). This is product
RED, not a notification animation failure: the panic overlay covers Export.
The same callback now uses the existing signal's fallible read and returns when
its owner is gone. It retains the selection identity guard and focus behavior.
No task/domain owner, listener or shared seam is added.

`browser-r2.log` catches native Space changing the real Embed used models owner
without refreshing the controlled Checkbox. The trusted click/key diagnostic
(`checkbox-diagnostic.log`) confirms reopening Export shows the changed value.
ExportWorkspace now subscribes to its existing Runtime version context, like
its connected callers. No keyboard handling or duplicate draft state is added.
The card delete-target geometry regression (`browser-card-red.log`) is fixed by
positioning the Tooltip wrapper inside its Card; subsequent runs pass that
assertion. Early stylesheet readiness failures are fixture setup, not touch RED;
the browser workflow waits for linked styles/fonts before measuring actual 44px
coarse targets. All earlier failed runs/captures remain in `.evidence/023/`.

`browser-export-layout-red-owner.log` is actual RED after fixing the dropped
Preview read: the public Export AX tree still exposes Objects/Inspector docks.
Its legacy Export-only selectors targeted the retired panel hosts; they now hide
the current dock hosts only while Export is present, retaining mounted owners
and panel preferences. Owned ItemContent alignment and rail border/fill rules
also stop legacy CSS from overriding copied control presentation. The full-case
Back text Button's navigation and coarse 44px target pass in this same run.

`browser-secondary-red.log` catches the remaining portable-copy legacy primary
fill overriding its copied Secondary Button. Only that owned paint/hover rule
is removed; its full-width sizing remains. The same run passes the styled guide
panel-fit assertion after waiting for its actual expansion transition, both
normal deletion paths and exact opener restoration, native Checkbox Space with
its real Runtime owner, hidden workspace docks and the .boardstudio download
outcome. Responsive header menu tests wait for the actual resized control before
activation; earlier immediate-lookup failures are retained as harness failures.

`browser-complete.log` catches actual narrow Library overflow from the focused
card delete Tooltip (PNG retained). The card-local content now aligns to its
trigger's right edge and wraps long names within the viewport using owned CSS;
shared Tooltip behavior/API remains unchanged. The regression checks the open
Tooltip bounds and Library width at both desktop and 390px. `browser-final.log`
passes all Export Theme/Appearance captures; its last Escape check used malformed
selector quoting, while its failure capture proves the app returned to Layout.
The corrected selector retains the assertion.

The full existing palette script was executed in `palette-corrected.log` after
fixing two introduced JavaScript statement delimiters (`palette-final.log`). Its
entire deletion/native palette integration block passes, including exact safe
focus, trusted boundary events, retained Alert owner, rapid reopen, Cancel/Escape
identity and exactly one trusted Confirm. The later unchanged Add-object check
fails at script line 394: it expects role dialog named Add, while the captured AX
tree exposes the actual region named Add with its Add button expanded. No
unrelated Add workflow or its test assertions were repaired/relaxed in this lane.


### Final verification attempt

The documented default full pre-PR command was invoked once on clean source
`098dea0dfeb1d52f77887be9727407ce50e3beb2`:
`NODE_PATH=/home/chris/.cache/boardstudio-web-tools/node/node_modules BOARDSTUDIO_CHECK_JOBS=1 python3 scripts/check.py --receipt .evidence/023/full-receipt.json`.
It stops in tooling's test-script registration guard, listing 20 unchanged
migration browser scripts plus the new 023 script. The 023 script is now explicitly
registered as opt-in using the existing guard's convention; the other 20 remain
untouched. The full receipt truthfully marks lint/build/native/browser not run
by that invocation. Required remaining test/check steps are executed separately
below rather than claiming that fail-fast invocation executed them.

`browser-final-controls.log` passes the complete actual packaged workflow with
coarse pointer, including normal deletion/Ctrl+K keyboard floor, all free guide
stages and Back navigation, guide draft/input targets and narrow panel, native
Checkbox Space, inline Export blockers, real portable download, all eight Export
Theme/Appearance/width captures, and the viewport-fitting focused Library delete
Tooltip at desktop and 390px. `browser-verified.log` records a harness stale Toast
close ref; the unnecessary Toast-dismissal setup is removed. Pointer Export still
requires an actual hit target and uses a trusted click; no product queue change,
force activation, delay, retry or weakened assertion was added.


## Outcome

Author implementation is complete and ready for the coordinator's **one** combined
Standards + Spec review. Status remains **claimed**: no ticket resolution,
feature-branch fast-forward/landing, target cleanup, or final merge is authorized
by this handoff. No review agent/thread/watcher was spawned in this child.

### Candidate and ownership

- Bound child branch: `feat/library-export-ticket023`, worktree
  `/home/chris/.t3/worktrees/ts-boardstudio2/t3-b3957019`; fresh T3 status confirms
  that binding. Local feature base remains
  `c032d902ba3ae24c20ea588e5349fbe1e9559af5` and is an ancestor of the candidate.
- Claim `2adfe82` precedes implementation; product/source completion is committed
  in `098dea0dfeb1d52f77887be9727407ce50e3beb2`. The clean full-check source pin is
  `744d70f77a5c316972373287332207d9724a4a37`. The final handoff commit adds this
  Outcome and waits for the real theme-menu close transition before captures;
  it changes no Rust, product CSS, dependencies or domain state from `744d70f7`.
- Review command: `git diff c032d902ba3ae24c20ea588e5349fbe1e9559af5...HEAD`.
  Parent dirty docs, origin/dev, other worktrees, canonical renderer, Runtime
  queue/classification, shared primitive APIs, Parts, PCB and mechanical owners
  are untouched. The only registration hunk adds this lane's browser script to
  the existing explicit opt-in list in `scripts/test-check.py`.
- Native hidden file inputs remain the shared mapping's platform import boundary.
  The setup guide's injected PCB project/hardware controls retain their existing
  022 owner; the 023 guide composes their existing callbacks without migrating
  that foreign leaf. Owned production Library/Export/firmware/guide markup has
  no `Board*` uses or visible raw controls. Test fixtures may use native buttons.
- The exclusive generated target remains
  `/home/chris/.cache/boardstudio-target/023-library-export-lineage`. Worktree,
  branch, ignored settings, CAD caches and all `.evidence/023/` artifacts remain.

### Executed final checks

The user-required full default invocation was attempted once, as recorded above:
`full-receipt.json`/`.md` and `full.log` preserve its fail-fast tooling result.
It is **not** full acceptance. The own-script opt-in correction leaves exactly
20 unchanged migration scripts unregistered (`tooling-registration-owned.log`);
that global guard remains blocked rather than being weakened or repaired here.

The remaining documented build/test gates then ran on clean `744d70f7`:

`NODE_PATH=/home/chris/.cache/boardstudio-web-tools/node/node_modules BOARDSTUDIO_CHECK_JOBS=1 python3 scripts/check.py lint build test browser --keep-going --receipt .evidence/023/full-tests-receipt.json`

All **27/27** selected commands executed. The invocation exits **1**, with the
following literal results in `full-tests.log` and its supported JSON/Markdown
receipt:

- Full native and WASM Clippy, rustfmt, and the checked provider/page build pass.
- Complete native step passes: **2,110 passed, 0 failed, 12 existing ignored**,
  40 binary runs; retention guard **2,104 matched, 0 missing, 6 new**. No baseline
  or ignored-test configuration was changed. The scripted native CAD smoke passes.
- Final mounted Library **19/19**, Layout **156/156** plus isolated guide **2/2**
  pass, including both retained Matrix focus tests. Host 45, Runtime 50, UI Model
  9, UI Shared 81 + 2, Keycaps 30, Keymap 35, Catalogue 12, Case 85 and Parts 71
  all pass in their actual launches. Exact names/groups are in the receipt.
- The page suite is complete: **82 passed, 6 failed, 88 expected**, no incomplete
  results. All six owned Export/firmware tests pass in this full run, including
  inline blockers, live readiness/provider guards, owner callbacks and failed
  export/successful retry from the same row. `page-full-result.json` preserves
  the runner's result artifact; no source snapshots/manifests were generated.
- Provider smoke and CAD browser smoke pass (the latter reports 364 checks).
  The trusted drag smoke, PCB launch and page suite fail as diagnosed below.
  The browser gate is **not accepted**.

Remaining tooling after the registration failure was executed independently in
`tooling-remaining.log`. An auxiliary import-tooling test queued Cargo behind
lint's target lock; its waiting Cargo process was terminated before compiling
and that setup interruption is retained, not counted as product RED. Once the
full runner released the target, the exact command
`python3 -B scripts/test-import-kicad-parts.py` passes **1/1** serially in
`tooling-import-serial.log`. All other remaining tooling commands pass; the
unchanged registration guard remains the tooling blocker.

Actual packaged role/name checks:

`AGENT_BROWSER_ARGS='--blink-settings=primaryPointerType=2' python3 scripts/test-library-export.py http://127.0.0.1:4183/boardstudio/ --evidence .evidence/023/browser-settled`

passes in `browser-settled.log` against the fresh checked package from the final
full build. It exercises safe Cancel, exact card/sidebar focus return, trusted
Tab/Escape/Confirm and identity, native Ctrl+K palette over Alert, free guide
Home/End/Space stages, real full-case Back navigation, coarse 44px controls,
narrow guide fit, native Checkbox Space, persistent Export blockers, a nonempty
real `.boardstudio` download and outcome, hidden editor docks, Secondary fills,
and both Themes/Appearances at 1440/390px. The menu's actual closing animation
settles before capture; no assertions, native activation or owner callbacks are
bypassed. Earlier product RED, setup/fixture failures and all PNGs remain.

Representative captures under `browser-settled/`: `delete-confirmation.png`,
`setup-guide.png`, `setup-guide-390.png`, `export-outcome.png`, all eight
`export-{original,shadcn}-{light,dark}-{1440,390}.png`, and
`library-shadcn-dark-{1440,390}.png`. Desktop/narrow captures were visually
inspected; current 013 operation notifications remain visible where applicable.
`browser-final-controls.log` is the earlier passing identical control workflow;
its product inputs are unchanged. Earlier scoped native/mounted receipts above
are retained as slice evidence, not substituted for the requested final suite.

### Diagnosed unresolved gates

- **Full tooling guard:** the 20 other migration scripts lack registrations at
  the committed feature base too. Only 023's entry is added. No global opt-in
  list repair or check weakening is included.
- **Trusted drag smoke:** times out at `scripts/test-drag-interactions.cjs:87`
  while waiting for `.m1-save-state[data-state=saved]`, before the first Part
  drag begins. Current production Rust contains no `.m1-save-state`; the script
  and Header implementation are unchanged from base. This is not a proven drag
  regression. `full-drag-failure.log` preserves the original runner log from
  `web/target/test-results/drag-browser/20261023T052655Z-573ddd164599/run.log`.
  An earlier progress message inferred success before the receipt; this Outcome
  and receipt correct it: drag **failed**, not passed.
- **PCB mounted launch:** **33 passed, 16 failed**. First failure is
  `pcb_scene::mounted_layer_tests::mounted_controls_toggle_only_their_accepted_pcb_scene_layers`,
  `web/crates/pcb/src/pcb_scene.rs:587`: selector requires `.m1-layer-label`
  inside the Hide F.Cu button, while existing LayerToggleButton renders that
  label as an Item sibling. Both source and test are unchanged from base.
  Fifteen later tests report the poisoned panic hook; independent root causes
  for those follow-on failures are not established. No PCB or shared layer edit.
- **Page suite:** first failure is
  `presentation::layout_key_selection_actions::tests::editor_owned_selection_kinds_apply_only_eligible_companions_and_undo`:
  missing `dioxus_primitives::toast::ToastCtx`. The fixture mounts selected-key
  actions without a ToastProvider; its connected LayoutAlign uses `use_toast`.
  These sources are unchanged. Five later failure records include panic-hook
  poisoning and a keycaps finding wait timeout; they are not claimed as five
  independently diagnosed defects. All six names remain in the full receipt.
- **Existing packaged palette script:** `palette-corrected.log` passes the entire
  deletion/native palette integration block, then fails its unchanged line-394
  Add assertion: expected dialog named Add, actual region named Add. That same
  region markup exists at base. No unrelated locator/product repair was made.

No base-revision full execution was performed; unchanged-source diagnosis is
source inspection, not a claimed baseline rerun. Required coordinator-owned
Standards + Spec review is pending at the clean final handoff pin. Full quick,
global CAD worker qualification, first paint, screen-reader, human 026 acceptance
and final dev merge remain separate gates. Keep landing/resolve/cleanup on HOLD
until substantive review and explicit coordinator approval.


Final documentation check:
`python3 scripts/check.py repo --receipt .evidence/023/final-repo-receipt.json`
passes (5.6s); its non-strict old-CSS report is informational, not the separate
024 strict gate. `git diff --check` passes. The lane's temporary 4183 static
server is stopped after captures; generated packages, caches and evidence stay
available for coordinator review. No build/test command remains running.

## Standards round 1 response: accessible style locators

Coordinator-reported initial review at `54ac7050f8cedd3256d5d5f9cbd362465bb5e2db`:
scoped Spec **APPROVE**, zero findings; Standards **one P3**, the shared-contract
role/name breach in the two Export style assertions. This correction changes
only the owned browser test and this ticket. Prior Spec approval carries for
unchanged product code; one targeted coordinator-owned Standards re-review is
requested for `git diff 54ac7050f8cedd3256d5d5f9cbd362465bb5e2db...HEAD`.

**Finding response:** the existing exact role/name snapshot-ref extraction is
reused through `role_ref`. Both style checks now resolve button **Download
.boardstudio project** and button **Review wiring** from current accessible
snapshots, then pass those exact refs to the supported `agent-browser get styles`
API. Background-color equality and the exact `0px` top-border assertion remain.
The root viewport/overflow, touch/geometry, token, focus, keyboard and operation
assertions are retained; no activation fallback, retry or assertion weakening.

The owned style-locator inspection also found Export's pointer hit-test selected
its control with an ARIA-attribute CSS selector. It now focuses the exact
role/name snapshot ref before measuring that resolved active element, matching
the existing touch-target measurements; its center-point containment assertion
and subsequent trusted role/name click remain. Other control geometry uses the
same role/name focus refs (including the retained card opener). Structural
panel/Card/page bounds and the noninteractive Tooltip use their existing context
selectors. Remaining `textContent` uses assert focus text, inline blocker text or
absence of a panic; none selects a control for a style measurement. Inventory:
`.evidence/023/standards-r1-locator-inventory.log`.

Fresh affected checks:

- `AGENT_BROWSER_ARGS='--blink-settings=primaryPointerType=2' python3 scripts/test-library-export.py http://127.0.0.1:4183/boardstudio/ --evidence .evidence/023/standards-r1-browser`
  **passes** (`standards-r1-browser.log`), including the actual ref-based computed
  style reads, all eight Theme/Appearance/width captures and the unchanged
  deletion/guide/Export workflows. Captures and the real project download are
  under `.evidence/023/standards-r1-browser/`.
- `PYTHONPYCACHEPREFIX=/tmp/boardstudio-023-standards-pycache python3 -m py_compile scripts/test-library-export.py`
  and `python3 scripts/test-library-export.py --help` pass; output is retained in
  `.evidence/023/standards-r1-python.log`.
- `python3 scripts/check.py repo --receipt .evidence/023/standards-r1-repo-receipt.json`
  and `git diff --check` both pass (repo 5.4s; no diff errors).

The existing checked package and build/native/mounted receipts from clean
`744d70f77a5c316972373287332207d9724a4a37` are reused: no product or build inputs
changed since that pin. No build/native/mounted/full-suite rerun is claimed or
needed for this locator-only correction. All earlier full-suite failures and the
lack of a baseline-green run remain unresolved exactly as recorded above.
No agents, reviews, watchers, landing, resolution or generated-target cleanup
were performed; those holds remain in effect.

The static 4183 test server was restarted for this workflow and remains available;
worktree, generated target, caches and evidence are preserved for coordinator
review. This is a test/docs-only response to P3, not a new product acceptance or
baseline-green claim.


## Scoped resolution and authorized local landing

The coordinator explicitly authorizes **scoped 023 resolution and local
fast-forward landing**, despite the separately documented failed full
acceptance. This section supersedes the earlier claimed/review/landing/cleanup
holds; their historical evidence remains intact. This is neither a baseline-green
claim nor a waiver of any global gate. The full-suite acceptance checkbox stays
unchecked.

Actual completed approval nodes were retrieved read-only through T3 and their
final messages/run identities are preserved in
`.evidence/023/closure-review-receipts.json`:

- **Standards r2: APPROVE, 0 findings**, reviewed
  `94f31e1652365898da2af378b8d765922da898f1`, completed
  `2026-10-23T05:49:52.201Z`.
  Node: `thread:delegated-task:command%3Amcp%3Add75e685-6104-454e-94dd-e2e71f7bb137%3Adelegate-task%3Arust-ui023-standards-review-r2`.
- **Spec r1: APPROVE for the scoped implementation, 0 findings**, reviewed
  `54ac7050f8cedd3256d5d5f9cbd362465bb5e2db`, completed
  `2026-10-23T05:45:03.696Z`.
  Node: `thread:delegated-task:command%3Amcp%3Add75e685-6104-454e-94dd-e2e71f7bb137%3Adelegate-task%3Arust-ui023-spec-review-r1`.
  This approval carries for unchanged product; the intervening locator/test
  correction is covered by the actual Standards r2 approval. These were source
  and author-evidence reviews, not reviewer-executed runtime checks.

Closure changes only this ticket and the build-plan map. Approved product and
tests must remain byte-for-byte equal under Git to `94f31e16` (and product equal
to Spec-reviewed `54ac7050`). Fresh `check.py repo` includes the real doc-link
check; diff/equality checks and their exact results are retained in
`.evidence/023/closure-repo-receipt.json` / `.md`, `closure-repo.log`,
`closure-diff.log` and `closure-source-equality.log`. Existing build, native,
mounted and packaged receipts are reused at their recorded pins; no rebuild or
full-suite rerun is claimed for this docs-only closure.

The feature was reverified at
`c032d902ba3ae24c20ea588e5349fbe1e9559af5` before closure. It must be rechecked
immediately before `git push . HEAD:refs/heads/feat/rust-ui-migration`, a local
fast-forward only, with no force/remote/dev operation. Any advanced feature tip
requires ancestry/overlap inspection rather than overwrite. The exact landed
pin, refs and clean inventory are recorded outside the generated target in
`.evidence/023/closure-landing-receipt.json`.

The only authorized deletion is the exclusive physical generated directory
`/home/chris/.cache/boardstudio-target/023-library-export-lineage`, after local
landing and complete audit plus immediate live recheck. Audit/cleanup receipts
are `.evidence/023/closure-target-classified-audit.json` and
`.evidence/023/closure-target-cleanup-receipt.json`, including allocated/apparent
bytes before and after, symlink/other-checkout references, ownership, mounts,
processes/environments/open fds and lsof. Only the verified owned 4183 preview
was stopped (`closure-preview-stop.json`, pidfd SIGTERM). Preserve the worktree,
child branch, source, target symlink, evidence, valuable ignored settings, CAD
caches and all other targets. Audit setup errors remain in
`closure-target-initial-audit.log`; an exited `/proc` process is not a product RED.

The first conservative audit (`closure-target-audit.json`) surfaced two orphan
system Chromium crash reporters with this lane's inherited `LD_LIBRARY_PATH`.
Inspection finds no target executable/cwd/open fd/mapping or build argument;
these are inactive lookup references, not live build consumers, and are left
running. Four protected, pre-existing systemd/PAM/GPG services have unreadable
environment/fds and started before this target existed; they are explicitly
listed as visibility limits, not killed or claimed fully inspected. The classified
audit requires no unclassified same-user visibility gaps, no active consumers,
empty lsof results, no other checkout reference and exact ownership/path guards.

**Unresolved gates remain OPEN/FAILED:**

- Full tooling: 20 unchanged browser-script registrations remain absent;
  `full-receipt.json.md` / `tooling-registration-owned.log` record the first guard.
- Drag: `full-drag-failure.log`, first wait at
  `scripts/test-drag-interactions.cjs:87` for retired `.m1-save-state`, before the
  first actual drag. No passing baseline or working drag is inferred.
- PCB: `full-tests-receipt.json.md`, first legacy descendant-label assertion at
  `web/crates/pcb/src/pcb_scene.rs:587`; 15 follow-on panic-hook failures remain
  incompletely diagnosed.
- Page: the same full receipt and `page-full-result.json`, first missing
  `ToastCtx` in the selected-key fixture/connected LayoutAlign; the other five
  failure records are not five independently established root causes.
- Palette: `palette-corrected.log`, later unchanged Add dialog-vs-region
  assertion after the successful deletion/native-palette block.

The first-failure attributions above are unchanged-source inspection supported
by the reviews, **not** base-revision execution. Global quick/full acceptance,
CAD-worker qualification, first paint, screen-reader, human 026 and final merge
remain open/failed as previously recorded. Scoped ticket resolution/local landing
and generated-target cleanup do not certify or waive them. No new review,
agent, watcher, remote operation or dev merge is authorized or performed here.

### Local landing and generated-target cleanup result

The authorized scoped closure commit
`42c328cdbd26c3c320ffb41c545b63bdc6579cab` was locally fast-forwarded from the
verified feature tip `c032d902ba3ae24c20ea588e5349fbe1e9559af5` using exactly
`git push . HEAD:refs/heads/feat/rust-ui-migration`. The feature was not checked
out elsewhere, ancestry and clean child inventory were checked, and `dev`
remained `b237903705244a266b07a64e5f23b8a3b5219c03`. Final documentation-only
receipt updates use the same guarded local fast-forward; all operations and the
final pin are in `.evidence/023/closure-landing-receipt.json`.

Cleanup completed only after that landing. The fresh audit covered **47 registered
worktrees / 52 checkout candidates**, found exactly this child's target symlink,
checked **44,575** target ownership entries, no nested mounts and no active
consumer. Immediate process/environment/open-fd/lsof and checkout-binding/path/
inode checks ran before fd-anchored `shutil.rmtree` of the exact physical target.
The inherited crash-reporter lookup paths and protected host-service visibility
limits remain explicitly recorded; no such process was stopped.

`closure-target-cleanup-receipt.json` records:

- Allocated bytes: **29,096,194,048 → 0**.
- Apparent bytes: **29,011,279,287 → 0**.
- After measurement: physical target absent. The child's original target symlink
  is preserved and intentionally dangling.
- Preserved identities: evidence root, ignored settings, CAD caches/pkg and the
  packaged `web/target`; worktree, source, child branch and other targets remain.
- Only the verified owned 4183 preview was stopped; its pidfd receipt is retained.

The final docs-only `check.py repo` (including doc-link checks), `git diff
--check`, approved source/tests equality and final inventory/ref checks are
recorded in `closure-final-repo-receipt.json` / `.md`, `closure-final-repo.log`,
`closure-final-diff.log`, `closure-source-equality.log` and the landing receipt.
Parent/main checkout HEAD/status inventories are compared outside the target in
`closure-parent-inventory-before.json` and `closure-parent-inventory-after.json`.
No source build, native/mounted/full-suite rerun, remote push or dev merge was
performed. All listed full tooling/drag/PCB/page/palette failures and global
quick/full/CAD-worker/first-paint/screen-reader/human026/final-merge gates remain
**OPEN/FAILED**; scoped resolution is not global acceptance.
