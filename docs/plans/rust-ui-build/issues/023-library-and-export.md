# 023: Library and Export

Status: claimed
Type: build
Blocked by: [006: Accessible Select and Combobox](006-accessible-select-combobox.md), [011: Docks and narrow-screen sheet](011-docks-and-narrow-sheet.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [inventory](../../rust-ui-migration/inventory.md), [mapping](../../rust-ui-migration/mapping.md), [shared contract](../map.md#shared-contract)

## What to build

The project library and Export page, the project-delete confirmation as an Alert Dialog, and the setup guide with a vertical Stepper inside its four accordion sections. Applies the shared contract in the plan map: raw controls and `Board*` become the mapped components, inline messages are classified, icon-only buttons get Tooltips, selects follow the 12-option rule, tests find controls by role and name, and the lane deletes only the old CSS its files used.

## Acceptance criteria

- [ ] The library, Export, firmware export and setup-guide rows of the inventory have zero raw controls and zero `Board*` uses
- [ ] Deleting a project uses an Alert Dialog that starts on the safe action and returns focus; the setup guide's current and ready steps use Stepper and stay freely navigable
- [ ] Existing tests pass after the rewrite rule; Export blockers stay inline
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches


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
