# 016: Layout sweep

Status: claimed
Type: build
Blocked by: [006: Accessible Select and Combobox](006-accessible-select-combobox.md), [015: Layout toolbar strips](015-layout-toolbar-strips.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [inventory](../../rust-ui-migration/inventory.md), [mapping](../../rust-ui-migration/mapping.md), [shared contract](../map.md#shared-contract)

## What to build

The rest of the Layout workspace is migrated. The Add view and Mirror existing half become in-dock views instead of non-modal dialogs. Applies the shared contract in the plan map: raw controls and `Board*` become the mapped components, inline messages are classified, icon-only buttons get Tooltips, selects follow the 12-option rule, tests find controls by role and name, and the lane deletes only the old CSS its files used.

## Acceptance criteria

- [ ] Every Layout row of the inventory (the layout crate and the Layout files in the app shell) has zero raw controls and zero `Board*` uses
- [ ] Add and Mirror existing half work as in-dock views with the same operations, undo and measurements as today
- [ ] Layout works in both Themes and both Appearances at the three widths, with screenshots in the outcome
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches

## Ownership (016 lineage)

Bound lane: `/home/chris/.t3/worktrees/ts-boardstudio2/t3-721450bb`, branch
`feat/rust-ui-016-layout-sweep`; clean initial `b237903` fast-forwarded to migration
`f11e6bd84456b94b2d7657f57a0d05b42184c4e7` before claim `2ff0be1`.

- Layout production files: `board_inspector.rs`, `layout_findings.rs`, `layout_viewer.rs`,
  `objects.rs`, `objects/{existing_half,keycap_size,matrix_inspector,matrix_setup,matrix_transform_inspector,mirrored_pair}.rs`,
  `outline/inspector.rs`, numeric `OutlineCoordinate`/`OutlineDimension` controls in `outline/overlays.rs`,
  `setup_guide.rs`. The 015 toolbar/align files and canvas/3D drawing surfaces retain their behavior.
- Shell Layout files: `presentation/{layout_workspace,layout_key_selection_actions,layout_matrix_relations}.rs`
  and `presentation/inspector/{layout_component_inspector,companion_placement}.rs`.
  `presentation.rs` only Layout placement/splay guidance or Layout-specific action markup if needed;
  no command registry, callback ownership or other Workspace hunks.
- Shared lease: `ui-shared/src/canvas_layers.rs` only `LayerPanel` and `LayerToggleButton`
  plus their tests. Existing props/targets/visibility owners stay intact; 020 reuses the landed adoption.
  Case `shared_viewer.rs` / `DesignViewToolbar` is outside this lease.
- Tests: affected Layout crate mounted tests and Layout presentation mounted tests, their role/name
  locators, new adopted-control coverage; native test-name baseline only if a native name changes.
- CSS: only selectors exclusive to these files. Shared legacy selectors wait for 024.
- Proposed shared event-forwarding props and the mapping's missing `NumberField` composition await
  coordinator authorization before edits. No dependencies or member visibility changes proposed.

Exclusive generated target: `/home/chris/.cache/boardstudio-target/016-layout-sweep-lineage`;
ignored `.claude/settings.json` and CAD caches copied before builds. No Gortex or CodeGraph directory.
Historical workflow documents are references only, per the current user instruction.

## Continuation: shared contracts and standalone layer slice

- Approved Button passthrough scope: optional `onkeydown`, `onfocus`, `onblur`,
  `onpointerenter`, `aria_describedby` on both existing native/link branches. No
  generic attributes API or owner/state/class changes. Clean additive contract
  branch `feat/rust-ui-016-button-contract`, commit
  `01c631e8ba1033515d8716db8ffec32c3673add6`, parent `f11e6bd84456b94b2d7657f57a0d05b42184c4e7`,
  sole tree path `web/crates/ui-shared/src/ui/button.rs`. Created using a temporary
  index; lane index, WIP and history preserved. Mounted forwarding test GREEN 1/1
  with 71 filtered: `CARGO_PROFILE_DEV_DEBUG=false wasm-pack test --headless --chrome
  web/crates/ui-shared --locked --lib -- forwarding_tests::`;
  `.evidence/016/button-contract-browser-compile-corrected.log`. This ran against
  the dirty lane dependency tree, not the isolated contract pin. The coordinator's
  prerequisite slice owns verification of the combined clean contract pin.
- 017 is sole InputGroupInput writer (approved optional max and native GlobalAttributes
  forwarding). 020 is sole Label writer (`None` omits `for`, explicit targets unchanged).
  No unlanded changes are copied. The NumberField proposal was withdrawn: the only
  owned BoardNumberField caller is NumericTransformField, which now directly composes
  Field/InputGroup while retaining its original draft, baseline, parsing and commit owner.
- 016's separate Combobox lease routes only the existing non-namespaced `id` from
  root to actual input. Committed callers at `f11e6bd` had no root IDs; only the
  006 lab consumed the copied component. The primitive input's generated fallback ID
  is independent of list/active-descendant IDs. Actual FieldLabel click RED is in
  `.evidence/016/red-combobox-label-compile-corrected.log` (BODY retained focus).
  GREEN: `.evidence/016/combobox-identity-navigation-green.log` (1/1), and existing 006 Select/Combobox keyboard/filter/controlled selection/focus floor `.evidence/016/shared-006-floor-green.log` (2/2). An earlier GREEN attempt reached the next relationship assertion before active-descendant rendering; its failure is retained in `.evidence/016/combobox-identity-green.log`. The test now closes the native label-opened popup before independently checking keyboard navigation and waits for the actual active option relationship. Final implicit long-Combobox/020 Label integration remains pending. This identity change is
  outside the shared control prerequisite and outside the standalone layer slice.

Standalone layer slice exact paths (all other Layout WIP excluded):

1. `web/crates/ui-shared/src/canvas_layers.rs`: only LayerPanel/LayerToggleButton,
   imports; CanvasLayers target mutation, props and visibility owner unchanged.
2. `web/crates/ui-shared/examples/canvas_layers.rs`: actual CanvasLayers context,
   actual LayerPanel and a child-owned retained draft; disabled/unmount host controls.
3. `scripts/test-canvas-layers.py`: repository trusted browser interaction checks.
4. `web/assets/m1.css`: seven existing common-layer button selectors exclude
   copied `[data-slot]` controls; shared legacy rules retained for 024.

LayerPanel uses copied Collapsible with initial-open behavior, retained content,
Escape close/trigger focus, hidden/inert exclusion. Layer rows use Item plus named
one-item ToggleGroup/ToggleItem eye with original labels/pressed callback/targets;
Tooltip composes through attributes. Trigger and eye targets are at least 44px.

Executed layer evidence so far is on dirty lane source, not a clean isolated slice:

- Actual inert regression RED: `.evidence/016/shared-layers-open-inert-red.log`.
  Expanded retained content had `inert="false"`, which HTML treats as inert.
  Fixed presence to `(!open()).then_some("")`; expanded content now has no inert
  attribute. No source snapshots or SHA manifests were generated.
- GREEN: `python3 scripts/test-canvas-layers.py`,
  `.evidence/016/shared-layers-touch-targets-green.log`. Trusted Enter/Space activate
  once; actual hidden/module/footprint state changes; Home/End, Escape trigger focus,
  hidden/inert Tab exclusion, local child draft and node retention, native disabled
  pointer refusal/Tab exclusion, unmount/remount and 44px targets verified. Role/name
  locators identify controls; DOM attributes inspect relationships/state only.
- 12 expanded-state captures: `.evidence/016/layer-captures-20261022-194600/`
  (1440/1100/390, Original/Shadcn, light/dark). These are shared-layer fixture
  screenshots, not full Layout acceptance screenshots. Prior `.evidence/016/`
  harness failures are retained (box JSON shape, integer mouse coordinates, Tooltip
  overriding the final trigger data-slot); no retry masking or weakened behavior assertions.
- GREEN: `CARGO_PROFILE_DEV_DEBUG=false cargo clippy --locked -p
  boardstudio-web-ui-shared --target wasm32-unknown-unknown --all-targets -- -D warnings`,
  `.evidence/016/shared-layers-final-clippy.log`. Earlier clippy before target-size
  refinement also passed. `rustfmt` and Python syntax checks passed.

Required order: coordinator lands CONTROL prerequisite; then isolate/recheck this
completed shared feature slice from integration base for ONE Standards+Spec closure
and approved local feature landing; then 020 Parts/chooser lands; then full 016
merges 020. No self landing or intermediate micro-reviews. Full Layout WIP is
checkpointed at `2ac74591` and continues independently. Ticket remains claimed.
Global full/quick/CAD worker.onerror/first-paint/screen-reader/human026/final merge
remain OPEN; no claim of those gates is made by focused checks.

Standalone slice prepared additively (review/landing still held for CONTROL order):
`feat/rust-ui-016-layer-controls` at `69643366e0c35ad08fd78076a008288cf0dc303f`,
base `f11e6bd84456b94b2d7657f57a0d05b42184c4e7`, tree
`83edf368d6992450732e6be8d5bc658b9ada0d0c`; exactly the four layer paths above.
The temporary index was removed; lane HEAD/index/WIP/history were unchanged.
Tests above executed with these exact four file contents but dirty lane dependencies;
this does not claim execution against the isolated clean commit. Merge the landed
CONTROL feature pin into this additive slice before the one required coordinator review.

Confirmed guide split: 016 owns only existing Button/Input/Accordion adoption in
`web/crates/layout/src/setup_guide.rs`. 023 owns Stepper/current-ready composition
and its presentation/state. Stage availability/readiness, links, callbacks, local
name draft/commit/focus and test names stay unchanged. 023 handoff: review the
ProjectSetupGuide control hunks plus `sync_stage_trigger` and the existing guide /
`objects/matrix_setup_focus_tests.rs` mounted focus checks; retain its existing
`.m1-setup-guide*` / `.m1-setup-stage*` rules until that migration. 016 does not claim
whole-guide mapping completion or introduce Stepper/API/readiness changes.

The four-file `69643366` slice is superseded and was not reviewed. Actual dock
container regression exposed two missed legacy interactions before review:
`.evidence/016/shared-layers-actual-dock-red.log` reports expanded list
`position: absolute` and eye height `40` (width `44`, trigger height `44`).
The user approved exactly two `web/tailwind.css` selector hunks: retain the old
`.board-accordion` inline-list selector and add the layer-owned
`.m1-object-layer-dock .m1-layers[data-slot="collapsible"] .m1-layer-list`
alternative with unchanged declarations; exclude copied `[data-slot]` layer buttons
only from the later dock legacy sizing rule. No 023 or other tailwind hunks changed.

Replacement completed slice: `feat/rust-ui-016-layer-controls-dock`,
`d0633057b7dc5698aa95cd5cf2cc68ee7e876eea`, base
`f11e6bd84456b94b2d7657f57a0d05b42184c4e7`, tree
`b1713bc8a4c2bb02995e20ebfbd3c54a1a5da7c4`. Exactly the original four paths plus
`web/tailwind.css`; additive temporary index, lane WIP/index/history preserved.

- GREEN `python3 scripts/test-canvas-layers.py`,
  `.evidence/016/shared-layers-actual-dock-matrix-green.log`. The fixture now mounts
  the actual shared controls inside production `.m1-object-layer-dock` containers,
  with no global layer-position override. Each of the 12 width/theme/appearance
  combinations checks pointer/Space toggling, Escape trigger focus, hidden/inert
  collapse and Tab exclusion, retained actual local draft and exact input node,
  computed static/in-flow list geometry and at least 44px trigger/eye rectangles.
  Initial suite also checks trusted Enter/Space once, module/footprint independence,
  group Home/End, Tooltip description, disabled pointer refusal/Tab skipping,
  unmount/remount with original visibility-owner state and reset child-local draft.
- Latest 12 captures: `.evidence/016/layer-captures-20261022-200045/`;
  inspected 390px Shadcn dark image. This is shared dock integration, not full Layout.
- GREEN shared all-targets wasm clippy with `-D warnings`:
  `.evidence/016/shared-layers-actual-dock-clippy.log` (same command above).
- These runs used exact five slice file contents with dirty lane dependencies;
  they do not claim isolated clean-pin execution. CONTROL merge and the one
  coordinator Standards+Spec review/local landing remain required; no self landing.

Full-lane continuation (not closure evidence): copied Layout inputs now compose
Field/FieldLabel/InputGroup directly, with explicit per-instance label IDs and native
constraints/blur/key/mount/ARIA forwarded through the approved pending InputGroup
attributes boundary. No new NumberField abstraction or state/parsing/commit owner.
Board rename, key-size and transform tests are being converted to role/name locators;
actual Combo option selection replaces native-select value/change injection, and
copied Checkbox clicks replace native checked/change injection. Test names retained.
Relation references/actions use copied Button; diagnostic data stays on structural
row wrappers inside Item, and description IDs stay per instance. A new actual Editor
Relations-tab unmount test and description-row checks are written, not yet executed.

`layout-adoption-check-{1,2,3}.log` preserve the compile trajectory. Check 3 still
requires the 017 max/attributes API and exposed formatter damage in NumericTransformField;
that stray statement and missing terminator were corrected locally afterwards.
The Dioxus formatter also damaged EventHandler callbacks; their exact domain bodies
were restored from owned checkpoint `2ac74591`, and native input event bodies were
reconstructed from the original owned controls with explicit attribute vectors.
Do not run the Dioxus whole-file formatter on these complex callbacks again.
`rustfmt --config skip_children=true` keeps formatting within owned files.
No GREEN claim is made for full Layout or the pending actual Mirror in-dock RED.
The required Mirror behavioral RED must run with the correct `--lib -- <filter>`
after the landed control prerequisite makes this crate buildable, before its dialog
presentation is changed. No other lane's unlanded InputGroup/Parts/Label work is copied.


Landed CONTROL `c38dc6bf4c52808663a7f5e78761bbfe99abb984` merged into this lane at
`353e291d` without discarding WIP. The sole merge conflict was Button: retained the
canonical landed file and all its composition/effective-ID/listener-update tests,
byte-identical to feature; InputGroup and Checkbox likewise match feature. The
vendored interpreter and lock path are canonical. Incoming vendor `.gitignore`
contains a trailing blank line reported by diff-check; left unchanged as landed.
No post-merge Cargo or wasm-pack has started. Explicit build-slot yield confirmed
to coordinator after a `/proc` argv/environment/open-fd scan found no process owning
`/home/chris/.cache/boardstudio-target/016-layout-sweep-lineage`; wait for explicit
slot return while clean layer child verifies `7ac31403` on CONTROL.

Full Layout source continuation uses canonical Button `attributes` composition for
the Objects disclosure Tooltip, keeping the actual Button as the sole focus owner.
The Layout view-mode root retains its original `m1-layout-view-group board-tool-pill`
placement. The user-approved Layout-only legacy button CSS exclusions remain
unmodified pending an actual computed-style RED; this is outside the shared layer
slice. No full Layout runtime or compile GREEN is claimed yet.


Coordinator returned the 016 exclusive build slot after clean layer child verification
at supplied source `16947138`; two-axis layer review is running separately, not accepted
or landed by this lane. Expanded process-yield receipt is
`.evidence/016/control-slot-yield-process-scan.log` (222 readable same-UID processes,
zero target matches; four old user-service environments unreadable). No build started
until the explicit slot return.

Post-CONTROL compile progression: checks 4–6 preserve actual type/ownership errors
(native event-vector type annotations, canonical CheckboxState reexport, moved
projection/control-label values), then GREEN check 7:
`CARGO_PROFILE_DEV_DEBUG=false cargo check --locked -p boardstudio-web --target
wasm32-unknown-unknown --all-targets`, `.evidence/016/layout-adoption-check-7-control.log`.
This compiles production Layout and shell targets plus shell tests; it does not execute
browser tests or compile every dependency crate's mounted tests. The actual Mirror
behavioral RED is still required before changing its dialog presentation.

Source continuation: actual Inspector tests use role/name controls, real Select option
clicks, copied Checkbox clicks and named precise InputGroup fields; existing Undo,
rejection, stale-owner and local draft assertions/test names remain. Reference navigation
uses the actual named Button, including a retained queued stale Button for deletion.
Layout findings now use Item/Badge and Empty; persistent viewer errors use Alert,
notices use Callout, and pending CAD text includes Spinner. Existing operations and
state owners remain. Copied private Collapsible headers gain 44px targets/chevrons.
Only the shared layer CSS is changed so far; the unused-rule inventory is in
`.scratch/016/owned-legacy-css-inventory.json`, not a source/SHA manifest. The
023 guide chevron rules and shared legacy rules are retained.


Additional affected integration-test inventory before editing: in
`scripts/test-dock-layout.py`, only `production_tests.add_view`'s three Add-view
state/containment selectors change from dialog to region, matching the decided
016 presentation change. Workspace panels keep dialog semantics; all 011 sheet,
draft, focus and operations assertions stay. No matching active writer exists.
The new owned `scripts/test-layout-view-controls.py` will inspect the actual
Editor-owned view controls in a packaged page across the 12 combinations; disabled
CSS is explicitly a DOM style probe, not a domain-owner disabled outcome. Its native
style reference is not used for adopted-control interaction or draft acceptance.
No Layout legacy style exclusion has yet been applied without its runtime RED.


Meaningful actual Mirror RED now obtained before its presentation edit: clean lane
source `13f5fc79`, `CARGO_PROFILE_DEV_DEBUG=false wasm-pack test --headless --chrome
web/crates/layout --locked --lib --
mirror_existing_half_is_an_in_dock_view_with_its_own_precise_axis_draft` failed 0/1
at the assertion “Mirror must stay in the Objects dock, without dialog semantics”.
Receipt `.evidence/016/mirror-dock-actual-red-control-compiled.log` is the earliest
actual behavioral failure. The preceding `mirror-dock-actual-red-control.log` is
separately retained as a compile failure (a missed guide test locator), not RED proof.
The new implementation changes only the presentation to a named region/Card;
Escape retains its original cancellable guard and callback/owner, and all existing
source choice/precise axis draft/parsing/create/cancel code is unchanged.


Reviewed LAYER feature `b9371dbdabf23f1758282b898313ae7d75bae0ae` merged at
`7761aca3719c19fc137c004bc83f5912b6024ae4` after the target command completed.
Only conflict was add/add in the browser harness; retained the canonical landed
CARGO_TARGET_DIR/session routing. Layer production/CSS/example source and canonical
CONTROL files match feature. Shared-layer closure is inherited from its reviewed
clean pin and [handoff](../shared-layer-prerequisites.md), not duplicated acceptance.
Full 020 chooser/Label inheritance is still pending.

First Mirror GREEN attempt passed the dialog/region assertions but failed the precise
submission wait (`mirror-dock-actual-green-control.log`), retained honestly. Its input
wait only inspected a DOM value that the test had just assigned, so it could click the
previous render's submit closure. The test now checks the actual required-value
validation transition (empty disables Create, precise valid input enables it) before
clicking, then asserts the submitted value exactly. It also checks cancellable Escape
once and the pending cancellation guard, retaining the real child's local draft.
No production parsing, commit, draft or callback was changed to accommodate that wait.


GREEN actual Mirror validation/cancellation/precision check 1/1 (156 filtered):
`.evidence/016/mirror-dock-actual-validation-green.log`, same focused command above.
This is mounted actual-form proof, not full Core/Undo or full workspace acceptance.

Owned CSS deletion inventory: repository-wide source searches found no callers for
`m1-outline-number`, `m1-matrix-slider-{track,range,thumb}` or the old findings empty
class after their adoption. Removed their eight old rules/alternatives, including
Layout-only findings back-button and old count/empty styling (all remaining callers
are the two owned copied Back buttons and Badge/Empty). Structural panel/heading/row
rules remain. The 023 guide chevron/stepper rules, shared legacy styles, 015 and Case
selectors remain. These deletions are outside the canonical bounded layer hunks.


At `42b5134e`, supported `BOARDSTUDIO_CHECK_JOBS=1 python3 scripts/check.py browser
--only layout` rebuilt the page/CSS successfully, then failed in the first mounted
Matrix name test because its old label/tag locator no longer found the adopted field.
Receipt `.evidence/016/layout-browser-owned-1.log` identifies that earliest real failure;
41 later panic-hook failures and the batch timeout are cascading symptoms, not separate
product findings or a pass. The role/name field helper correction now passes the actual
same-owner local-name-draft test (1/1, 156 filtered),
`.evidence/016/matrix-owner-draft-role-green.log`. The unchanged assertions still check
actual mounted drafts through unrelated accepted revisions. Remaining Outline/native
choice/checkbox locators are being migrated before another full Layout launch.


Outline tests now use named copied controls, actual popup option enumeration and
native label/checkbox semantics. The first focused run used the wrong role for the
short-list Select trigger; `.evidence/016/outline-active-choice-role-1.log` preserves
that test failure. The existing 006 mounted floor confirms this primitive's trigger
is a named button (Combobox's input has role combobox). Corrected Select roles in
Outline, Objects grouping and shell Mode/Offset axes tests; no product role changed.
The corrected focused command is still active at this checkpoint; it has not reached
“Loading Wasm module”. Its live matching ChromeDriver155 returns status, while its
session inventory request timed out after 3 seconds. No retry or GREEN claim is made.


The corrected Outline run at source `68f03fbf` compiled, but never entered WASM:
its browser remained at “Loading scripts...” with two Dioxus module resource
requests ending after about 300 seconds with zero transferred bytes. Targeted CDP
inspection confirmed the real page and hidden output; the runner HTTP server later
served the same resource with HTTP 200. This is a pre-test harness failure, not
product RED or GREEN. The owned hung process is being terminated after this receipt;
a fresh supported runner launch is justified by those failed module requests, with
no source assertion changes or retry masking. Raw initial log is
`.evidence/016/outline-active-choice-role-corrected.log`.

The approved later theme-contract §8 decision now changes only the 78 owned016
secondary Button variants from Outline to Secondary, preserving sizes and callbacks.
ToggleGroup Outline, canonical Button and landed tickets are unchanged. The sole
mapping edit corrects the Button row with a §8 link; NumberField is untouched.
Actual Theme checks and final closure remain required.


Fresh browser after the documented pre-test module failure passed the corrected
actual Outline saved-choice test 1/1 (156 filtered),
`.evidence/016/outline-active-choice-fresh-browser.log`. No assertions were changed
between the stalled browser and this run; two unused test locals were removed.

Additional connected production caller inventory before edit: Layout-owned
`layout_findings_state.rs::focus_layout_finding_return_destination` still searches
the removed legacy tab class. Both actual TabsList owners now use
`layout-inspector-tabs` to avoid legacy look overrides. A mounted actual Inspector
regression in `layout_component_inspector_tests.rs` will prove selected-tab return
focus before correcting this single selector; no new state/API/listener is needed.


Second supported full Layout browser launch (`layout-browser-owned-2.log`) passed
page/CSS build and the earlier Matrix tests. Its first real failure was the Objects
grouping test: synthetic `HTMLElement.click()` on a copied Select option does not
invoke its pointerdown/up selection contract. Focused unchanged test reproduces
that exact failure (`grouping-actual-focused-red.log`). Source inspection confirms
the primitive owns pointer selection, with no click listener. Corrected the four
owned test option-selection sites (Objects grouping, key size, attached component,
shell Inspector helper) to dispatch the complete primary pointer gesture on the
role/name located actual option. No production behavior or existing domain assertions
changed. Later panic-hook failures/timeouts in the full launch are cascades.


Focused actual Objects grouping GREEN 1/1 (156 filtered),
`.evidence/016/grouping-actual-pointer-compiled.log`; the original domain/cache/
accepted-commit assertions remain. The first pointer test attempt did not compile
because this crate does not enable PointerEventInit (`grouping-actual-pointer-green.log`);
used the already-enabled PointerEvent/Event initialization API instead, without a
manifest, feature or dependency change. The event's default primary button and
complete bubbling down/up gesture exercise the existing primitive listener.


## Additional Layout view CSS lease proposal (actual Editor RED)

Packaged production build at `96e00553` passed (`layout-production-fast-build-1.log`);
`--fast` skips wasm-opt and is not the shipping/global build gate. The owned site is
under the exclusive target and served only on 127.0.0.1:4186. After that command
completed, reviewed 017 feature `43fae25436776955ec1b3f363e61be30f3425661` merged
at `4d3b9d5256798c3c65ce51c71aabab9fc28af29c`, clean automatic CSS merge. Mechanical
source/tests, CONTROL and lock match canonical feature; only the existing owned
016 CSS deletions differ. The existing site remains attributed to its earlier build pin.

Actual Editor view-control RED in all 12 combinations is captured in
`.evidence/016/layout-view-controls-count-aware-red.log` and
`.evidence/016/view-controls-20261022-204613/`: 12px font, 10px padding, 40×40 target,
legacy selection fill and solid outline instead of the copied control look.
Its secondary-action probe incorrectly picked the already-landed 014 Ghost footer;
that part is not valid Secondary evidence and the footer is unchanged. The corrected
probe targets actual owned Add object; eight desktop combinations show its Secondary
class but legacy accent fill/white text (`layout-view-controls-owned-action-red.log`).
The first run's hard-coded zero findings count was a locator setup failure. These
raw receipts are retained, and the script now waits for real CSS animations to finish
before measuring, rather than accepting transient colour/focus values.

The previously approved m1.css button/pressed/focus and tailwind.css control-height
exclusions alone cannot remove later overrides. Request exactly three additional
Layout-only selector portions in `web/tailwind.css` (current lines 809, 1436–1437):

- `.m1-layout-view-group button` in the narrow media rule becomes
  `.m1-layout-view-group button:where(:not([data-slot]))`.
- `.m1-layout-view-group.board-tool-pill button` becomes
  `.m1-layout-view-group.board-tool-pill button:where(:not([data-slot]))`.
- `.m1-layout-view-group.board-tool-pill button[aria-pressed='true']` becomes
  `.m1-layout-view-group.board-tool-pill button:where(:not([data-slot]))[aria-pressed='true']`.

All declarations and Case/design alternatives stay byte-for-byte; root/015 placement,
44px sizing-only branches and SVG geometry are unchanged. These three extra portions
remain unedited pending the coordinator decision. Original approved portions will
use the same zero-specificity exclusion. This is outside the canonical landed layer slice.


The compiled actual selected-tab focus RED is
`finding-return-tab-actual-red-compiled.log` (0/1: destination was not found). The
first attempt was a test-only Element/HtmlElement compile mismatch, retained in
`finding-return-tab-actual-red.log`. Corrected only the stale class in the existing
focus selector; actual mounted selected-tab return GREEN 1/1, 87 filtered,
`finding-return-tab-actual-green.log`. All fallback order and focus/state owners stay.

Completed valid 12-combination RED for both actual view controls and the owned
Secondary Add object: `layout-view-controls-owned-action-complete-red.log`. Earlier
owned-action capture stopped at the narrow opener because the responsive transition
had already opened the Sheet; clicking Objects again closed it. The script now
inspects actual Sheet state and explicitly selects its Objects tab before locating
the same role/name Add button. No product change was made for that harness issue.

Coordinator approved the three extra CSS portions. Applied exactly the seven
approved selector edits (three m1.css and four tailwind.css); all declarations,
Case/design alternatives, 015 placement/SVG and sizing-only branches remain.
No Add/action style change is made: a fresh page will independently report its
Secondary result, and any remaining legacy accent cause will be identified before
further scope. Source scope proof for the 78 variant replacements is
`.evidence/016/secondary-owned-source-scope.log`; mapping has only the approved row.


The full Layout run at `54f64bae` now executes 156 tests: 148 pass and eight fail.
First failure is `mirrored_pair/canvas_overlay_tests.rs` still requesting the removed
native Key assembly select; later failures follow a corrupted panic hook and must
be rerun after that correction (`layout-browser-owned-3.log`). Before the test-only
edit, this additional owned integration-test path is inventoried: replace control
locators/native option enumeration only; retain canvas geometry/placement, keyed
form draft, exact preview values, owner cancellation and Escape assertions. Canvas
production source and all domain callbacks remain unchanged.


## Add object legacy accent cause and narrow correction proposal

Fresh page build `54f64bae` after the approved seven selector edits and 017 merge
passed (`layout-production-fast-build-2.log`); providers are honestly reused from
`96e00553` because their source/contracts are unchanged. Actual copied Layout view
controls now pass all 12 combinations, with native legacy branch unchanged, 44×44
rectangles, correct pressed fill, focus ring and disabled CSS:
`layout-view-controls-verified-copy-padding.log` reports `View style failures: []`.
Captures: `.evidence/016/view-controls-20261022-205239/`. These are style/empty-Layout
captures, not complete populated Layout acceptance or owner-disabled behavior.

The first post-fix probe expected zero block padding without a source basis. Targeted
CDP matched-rule inspection of the actual role/name focused 2D control proves its
only block padding comes from Chromium155's user-agent 1px rule; the copy sets px-2
(8px inline), with no vertical padding class. No legacy rule matches it now. Receipt
`view-padding-cascade-node-diagnostic.log`; the preceding diagnostic lacked a CDP
document node and is retained separately. Corrected the expectation to that proven
canonical contract; no production style was changed for the test.

Owned Secondary Add object independently still fails fill/ink in all 12 combinations
(the same log fails on that assertion). Exact cause: `web/assets/m1.css` lines 280–281:
`.m1-layout-add-object > .m1-layout-add-trigger` supplies accent/on-accent, and
`.m1-layout-add-trigger:hover:not(:disabled)` supplies accent-dark. Their only Rust
callers are the two owned copied Secondary Buttons in `objects.rs::AddObjectEntry`
and `presentation/layout_workspace.rs::add_objects`; no other native/landed caller
uses that class. Proposed smallest correction: delete only those two exclusive old
control-look rules. Preserve the adjacent width/min-height owner rule, container, SVG,
all callbacks, native sizes and all other actions. No action style edit is made yet.

Actual mirrored-pair overlay integration is now GREEN 1/1 (156 filtered), preserving
its exact preview values, keyed draft identity and both cancel-owner outcomes:
`mirrored-overlay-roles-green.log`. Only role/name form locators and real popup option
enumeration changed; the canvas production source remains unchanged.


Approved deletion applied to only the two Add accent rules. The adjacent
width/min-height/container/SVG rules are retained byte-for-byte. Corrected caller
name: the shell entry is `layout_workspace::add_object_header_action` (called by
Editor), alongside `Objects`' private `AddObjectEntry`; “add_objects” above was an
inexact continuation label. To verify both actual owners without widening visibility
or copying their markup/state, additional owned test inventory is
`web/crates/layout/examples/layout_add.rs` and `scripts/test-layout-add-actions.py`.
The example mounts the actual public Objects component over a real Session/Core
Runtime, using its internal local menu state; the packaged Editor covers the shell
entry. Existing test-support APIs/features suffice, no manifest/API changes. The
script will check both entries at all 12 combinations, default/hover/focus styling,
actual open/Escape/focus callbacks and native disabled CSS/gesture probe explicitly
distinguished from any nonexistent owner-disabled branch. No field draft is hoisted.


The fourth full Layout attempt compiled but stalled before “Loading Wasm module”;
after five minutes the actual page was still “Loading scripts...” with two Dioxus
module requests pending, while targeted HTTP HEAD requests for both returned 200.
`layout-browser-4-startup-diagnostic.log` captures its real CDP page state. No test
started, so this is neither RED nor GREEN. Terminated only that owned hung runner/
Cargo/driver/browser, preserving `layout-browser-owned-4.log`. The prior third full
run's 148 passes and first real removed-select failure remain distinct. Subsequent
WASM runs will run without our separate agent-browser sessions as a scoped
infrastructure experiment; no product assertions/timeouts are weakened.

Manual actual Editor exploration at packaged `54f64bae` already confirmed Add
creates a 1×1 matrix, Mirror accepts the exact local axis draft 12.123456789, the
accepted mirrored key is drawn at translate(24.246913578 0) with unchanged 18×18
measurements, and Undo restores only the original key at translate(0 0):
`layout-owner-manual-first.log` and `layout-owner-manual-mirror.log`. This is fresh
actual-owner evidence, with automation assertions/captures still to be consolidated.
The first exploratory wait incorrectly demanded an explicit DOM region attribute
from the unchanged native named section; the browser AX snapshot correctly reports
its implicit New matrix region. No production role change was made.


The actual-owner exploration is consolidated in the additional owned
`scripts/test-layout-adoption.py`: production Add/Matrix/Mirror, actual private axis
draft/node identity through all 12 combinations and hidden/inert Sheet close,
explicit label identity, precise accepted SVG geometry/measurements and separate
Undo for the numeric edit, linked-half creation and original Add. Controls use
observed role/name locators; canvas/output structure is inspected only as evidence.
No source fixture, state hoist, domain callback or public API is introduced.


The first standalone Add run reached its actual button and failed opening the menu.
A no-source-change diagnostic reproduces the render panic: missing required
`PartsSelectionGeneration` context (`add-standalone-open-diagnostic.log`). Supplied
that existing public context in the example, matching the real owner; no production
Parts/selection code changed. Original failure remains in `add-two-actual-owners-1.log`.

Remaining owned Outline messages now use FieldError for name validation, copied
Alert for the active persistent Core problem, and Callout for inactive alternatives.
Diagnostic finding/ownership attributes stay on their original structural owner.
Text, target identity, selectable state, export availability and callbacks are
unchanged. The existing mounted finding test now asserts the actual copied Alert
and exactly one announcement while retaining every Core-message/export/editability
assertion; this is the decided message adoption, not a domain behavior revision.


The actual-owner adoption script passes at packaged `769570df`: all 12 populated
Mirror captures, exact retained local axis node/draft through Theme/width/Sheet
hide and inert focus refusal, accepted mirror at x24.246913578, precise Origin edit
with preserved 18×18 measurements, and three exact separate Undo outcomes. Receipt
`layout-actual-owner-adoption-1.log`, captures
`.evidence/016/layout-adoption-20261022-210401/`. The later Outline message-only source
change is not in that packaged page and has no callback/geometry overlap.

The two-entry test with corrected real context reaches all 12 standalone combinations:
Secondary default/focus/native disabled and actual Enter/Space open/Escape/focus work,
but the narrow target is 40px (`add-two-actual-owners-context.log`). Its Editor run
stops on Space. Targeted actual Editor diagnostic confirms Enter: one trusted native
click, expanded true; Space: trusted keydown defaultPrevented=true, no click, expanded
false (`add-native-space-hover-diagnostic.log`). The shared window pan listener only
excludes text fields; 015's existing guard is correctly scoped to its toolbar root.
No 015/012/Canvas/shared helper is changed. Added only a native/raw plus virtual
Space propagation guard to each of the two owned Add Button callers, preserving
default native activation and callbacks. Their copied caller class also supplies a
44px narrow minimum (important only to coexist with the retained old owner minimum);
desktop sizing and every adjacent CSS declaration stay unchanged.

The apparent hover failure is separate: the same diagnostic proves `:hover` true
but the headless browser reports `(hover:hover)` and `(pointer:fine)` false. The
canonical copy correctly conditions hover on that media feature. No production
hover CSS is altered; the harness will explicitly establish real fine-pointer
emulation before checking that state and retain this original result.


At `16cedbff`, fresh packaged build 4 and both actual Add owners pass all 12
combinations: Secondary default/hover/focus, native disabled refusal/CSS, desktop
40px and narrow 44px targets, trusted Enter/Space exactly once, Escape and original
opener identity/focus. `add-two-actual-owners-hover-space-green.log`, captures
`.evidence/016/add-entries-20261022-211614/`. Hover capability uses the verified
Chromium `--blink-settings=primaryHoverType=2` flag and asserts real `(hover:hover)`;
this is hover emulation, not fine-pointer or human evidence. Build packaging's
wasm-bindgen overlapped the subsequent standalone example's compilation; Cargo
compilations did not overlap and both used exact `16cedbff` source. Future build
pipelines remain sequential through packaging completion.

Strengthened actual-owner operations to use trusted native Space for the Matrix
chooser, Create matrix and Create linked half. Actual first RED at `16cedbff`:
Matrix chooser gets zero native clicks (`layout-native-space-red.log`), the same
window pan cause outside 015's intentional toolbar boundary. Move the internal Add
Space guard to the existing owned Objects aside, protecting its native tree/Add/
Matrix/Mirror descendants without preventing their default action, handling Escape,
changing callbacks, or covering canvas/015/shared listener source. External header
Add retains its own guard. No new prop, visibility, dependency or state owner.


## Self-inspection continuation: actual controlled choices and dock layout

- Full Layout run 5 at `8c431873` reached 149/156. First real failure was Key size
  after Redo (display 1u instead of 2u). Same focused unchanged assertion fails in
  `key-size-redo-focused-red.log`; stable derived value from the existing draft
  passes `key-size-redo-derived-green.log` at `515ca647`. No parsing, callback,
  commit, pending-edit or draft owner changed, and no assertion/wait was weakened.
- Full Layout run 6 at `515ca647` reaches 152/156; all key-size edit/Undo/Redo and
  owner/timer tests pass. Next first failure is the old warning-class/role conjunction
  after Callout adoption. Keep diagnostic class on the original status owner and
  assert its actual copied child. Later guide failures still need isolation.
- A second actual-owner RED, `grouping-controlled-trigger-red.log` at `99d8ea89`,
  proves the local grouping and tree change while the actual Select trigger still
  says By layout instead of By part. `grouping-controlled-trigger-derived-green.log`
  passes the unchanged new assertion with the existing by_part signal-derived Memo.
- Connected inventory: Group; Matrix preset/orientation and MatrixField's two
  short/long branches; Mirror source and mirrored-pair preset; transform Splay affects,
  Mirror, Origin, Key Assembly and keyed attachment choices; active outline/corners/
  selected connection-point attachment; component Layout, Constraint type/source,
  Offset axes, Mirror axis; companion Mode. All replace freshly allocated constant
  ReadSignals with stable derived existing owner values. The attachment row maps
  the original keyed assembly through a derived accepted-value map; removed keys
  return None. No query/filter, selection, callback, state or public API is moved.
  Existing 006/canonical controls remain untouched. All-target shell WASM check is
  in `layout-adoption-check-8-derived-values.log` (source before dock-class fix).
- Visual self-inspection falsified the earlier Mirror screenshot's layout: old
  `.m1-matrix-setup` makes its Card absolute, 26px high, internally scrolled 290px
  with a zero-height named region. `mirror-actual-flow-asserted-red.log` and PNG
  reproduce it against served `16cedbff`. The preceding diagnostic serialization
  failure is retained separately. The in-dock Card now uses only its owned new
  class and copied spacing, with explicit form/footer layout; the canvas/guide
  legacy classes/rules are untouched. Strengthened the actual-owner script to
  assert static/in-flow visible overflow and full form/region heights in all12
  combinations alongside the exact original local draft, geometry and Undo checks.


At `c6f4075b`, full Layout browser tests pass: 156/156 with only the separately
scheduled setup guide filtered; its separate invocation passes 1/1. Receipts
`layout-browser-owned-7.log` and `layout-setup-guide-browser-green.log`. This includes
all current owner/draft/Undo/precision, Outline messages, Matrix/Key size/rotated
Checkbox and form-focus regressions. The earlier startup-stall hypothesis remains
unproven: this successful full run ran with no separate agent-browser session.

Actual field computed-style RED at served `16cedbff`, all12 combinations:
`layout-actual-field-styles-red.log`. Real Matrix name has opaque panel fill, 1px
border, 4px radius and legacy solid focus outline; both actual Matrix name and
Origin X are 40px at 390px. Native references and actual node/label identity are
recorded. The initial probe incorrectly expected Input's 14px desktop text from
InputGroupInput: its actual canonical source is `text-base` at every width. Corrected
the assertion to 16px (retained it); this invalidates only that initial font attribution.
The border/fill/focus and narrow target REDs are independent and real.

Connected owned CSS inventory before editing: `.m1-matrix-field-input` production
exists only in MatrixFieldEditor; NumericTransformField uses the parent field class
but not that legacy input wrapper. Add only copied-data-slot exclusions to the exact
old input appearance/focus selectors, preserving every declaration and native
reference. Both existing private MatrixFieldEditor and NumericTransformField callers
will supply narrow 44px Group/Input height; desktop/default classes, properties,
original callbacks, accepted baselines and local drafts remain unchanged. No shared
primitive, 015 source, Case/Parts branch, 023 guide rule or broader selector changes.
The first scripted CSS edit correctly aborted on its non-unique substring (the
error selector contains the same suffix); no CSS or height edits entered f06acbb7.
That commit's title overstates its content: it contains only the field probe and
fixture context correction. The subsequent edit uses an asserted unique whole-line
prefix and preserves history.

First root shell browser invocation executes 41 tests, 34 pass; first failures of
both actual-Editor fixture modules are the missing existing ToastCtx required by
landed shell notifications (`layout-shell-owned-browser-1.log`). Its later hook
panics are cascades. The fixtures now use the application's existing
NotificationProvider around the actual Editor, matching `presentation.rs:264`;
no control/owner substitutes, notifications behavior or assertions change.


Root fixture correction at `f06acbb7` passes all7 formerly blocked actual-Editor
tests: `layout-shell-context-green.log`, structured results and full per-filter
logs at `layout-shell-context-green.json.logs/`. This includes actual reference
hover/leave, focus/blur with a unique description resolving to its Item row,
selection/removal cleanup, actual Relations unmount cleanup, independent layer
visibility and paired-preview findings. Combined with the unchanged 34 passing
root tests in run1, all41 selected tests have passed; no known failures are waived.
Departure/constraint actions and companion placement include the original action,
Undo and disabled-reason checks in the passing ComponentInspector module.

The current owned Rust formatting check reports real formatting differences
(`owned-rustfmt-check.log`). These will be corrected with rustfmt on owned changed
files only (skip_children), preserving the checkpointed code and checking the diff;
no dx formatter, shared-file overwrite or history surgery.


Native Space floor continuation: actual 2D receives zero trusted clicks at served
`16cedbff` (`layout-view-native-space-red.log`), independently confirming the same
window-pan interference as the actual Matrix choice. The persistent view test now
requires one trusted Space click and retained pressed state in every combination.
Connected inventory shows MatrixSetup is mounted on the canvas/guide, not inside
Objects, so the Objects fence alone is insufficient. Added the same Space-only
raw+virtual propagation stop to existing owned control containers: MatrixSetup,
MirroredPairForm, MatrixInspector, MatrixTransformInspector, KeySizeControls,
Outline's normal/point editors, guide, findings, component/companion/relations/
selected-keys panels, Layout toolbar and Inspector tabs/return action. The canvas
Delete container retains its original broad virtual stop and gains only raw Space
isolation; viewer Retry actions and findings footer are guarded individually.
Native default is never prevented, existing Escape/typing/arrow handlers remain,
and canvas drawing roots, the global listener, 015 exact scoped guard, state and
callbacks are unchanged. The two primitive roots use their existing attributes
vectors: primitive Tabs/Collapsible roots have no internal key handler to overwrite;
unsupported convenience-prop compile errors are retained in clippy3.

Owned rustfmt completed at `293e2acb`; cargo fmt --all --check passed. Read the
whitespace-ignored connected diff (`owned-rustfmt-connected-diff.log`); differences
are wrapping, block formatting and optional trailing commas, no callback rewrites.
Clippy1 found generated useless string formatting plus two redundant test borrows;
corrected only owned expressions, preserving Display text and precise arithmetic.
Clippy2 found a collapsible conditional in the new raw-only Delete fence; fixed it.
`layout-owned-clippy-4.log` passes all-target WASM clippy with warnings denied for
boardstudio-web, boardstudio-web-layout and boardstudio-web-ui-shared, including
all connected test compilation. Prior failures remain; no lint/behavior is waived.

### Shared Select Space isolation decision needed (2026-10-22)

At production source `f99ea8802ecded90adc82246789c97950aa96b3a`, the actual
`Group objects by` button opens its actual listbox with trusted Space, but that
keydown also reaches window (`trusted: true`, `prevented: true`, target BUTTON).
`select-space-window-diagnostic.log` records the event and open listbox. The
persistent assertion in `scripts/test-layout-adoption.py` now reproduces the first
behavioral RED in `select-space-actual-owner-red.log`; screenshot
`layout-adoption-20261022-215207/failure.png`. Browser closes in finally.

Source cause: copied shared `ui/select.rs` stops only Dioxus propagation in its
existing closed-to-open Enter/Space handler, so the owned Objects ancestor's raw
Space fence is bypassed. The unchanged window pan listener admits BUTTON Space
without checking defaultPrevented. This proves raw-event leakage; held-key drag
and visible pan movement have not been tested by this diagnostic.

Minimal proposed additional lease: only shared Select's existing closed-to-open
Space branch also stops raw KeyboardEvent propagation, using the existing
WebEventExt pattern. Preserve Enter, prevent-default, open callback, focus return,
controlled values, attributes/classes, state and all props. No API/dependency or
listener ownership change. Do not supply caller onkeydown attributes: their
spread would overwrite the internal handler. Shared Select has no existing016
writer lease, so its production source remains untouched pending coordinator
decision. Verify this actual-owner RED goes GREEN plus the existing006 Select
navigation/selection/focus-return and retained-popup tests before acceptance.

Full020 Parts chooser and Label landing remains the separate dependency for final
Add integration; no unlanded source is copied. Global gates remain open.

Coordinator approved the exact shared Select lease: only the existing
closed-to-open Space handler additionally stops the raw event through existing
WebEventExt. Implemented without changing Enter/default prevention, spread order,
callbacks, classes, props, state, focus or selection. Connected caller inventory
is `select-connected-callers-inventory.log`: Layout short choices, component and
companion inspectors, 015 align/key-size choices, 017 SettingsSelect and the dock
example use the same copy. No connected production caller supplies an onkeydown
attribute; no internal handler is overwritten. Public API/visibility/dependencies
are unchanged; this is a shared behavior contract for the one full016 final
Standards+Spec review, not a separate micro-review.

`CARGO_PROFILE_DEV_DEBUG=false wasm-pack test --headless --chrome
web/crates/ui-shared --locked --lib -- ui::primitives_mount_tests::select_`
passes all3 (`select-space-shared-contract-green.log`): existing006 navigation,
controlled selection/focus return and retained-popup close floor plus a focused
raw-event test. The latter confirms Space does not reach window, Enter still
does once with default prevented, an unhandled key still bubbles once, caller
click still fires once, and open/close callbacks are not duplicated. This is
synthetic mounted shared-contract evidence; actual trusted/held-key evidence is
still pending the production rebuild, and no visible pan RED is claimed.

Production rebuild6 at `e0227abb` passes; same actual-owner regression and the full
Add/Mirror operation script pass (`select-space-actual-held-green.log`, captures
`layout-adoption-20261022-215700/`). Native Space's window trace is now empty.
The documented agent-browser CDP bridge supplies one held trusted keydown/up
(press otherwise releases immediately); controls/canvas still come from role/name.
Holding opening Space while dragging blank canvas opens once, makes no native
click, preserves the Group choice and camera viewBox, and delivers only the
trusted release to window. Independent canvas-focused held Space then changes the
actual viewBox, and a drag after release preserves that viewBox. This is measured
GREEN pan isolation/positive canvas behavior; the retained earlier RED measured
only raw event leakage. The actual precise Mirror draft, 12 combinations,
measurements and three separate Undo outcomes pass in the same execution.

### Landed020 integration and remaining owned inspector styles

Safely merged landed020 `de0dce979e8b06391ae9d80cfe41efae952e463c` at
`5cf7d016b198d641fab8974b75024c48ca840438`. Inspected both automatic CSS merges;
the canonical Parts crate, Case shared viewer/tests, Parts workspace, Label and
searchbox role helper are byte-identical to020. Own Layout view/Matrix exclusions
remain; no older shared contracts or other lane content overwrite the landing.
Overlap reads are `020-overlap-before-merge.diff` and `owned-css-after-020-merge.diff`.
Build7 packages this merge successfully, retaining the same --fast/provider limits.

Actual remaining-owner RED on that production pin:
`layout-remaining-field-label-styles-red.log` checks Matrix name/Origin X, Board
name, Outline margin and real SW1 Component X across all12 combinations. Board
name retains a legacy opaque fill/border/radius and 32px height. Outline retains
legacy fill/border/radius/focus; Component additionally retains 12px input and
label text. Narrow Outline/Component inputs are40px. Existing Matrix fields pass.
An earlier invocation hit a test sequencing error after valid Board12 and
Outline8 measurements: clicking the already-open compact Inspector toggle closed
it. The test now reads its actual expanded state before toggling and waits for
the actual Sheet, rather than retrying clicks; state-aware and final label REDs
complete all owners. Earlier failure is retained, no product behavior assertion
was removed.

Inventory before routine owned correction: Board-name class occurs only in
board_inspector; Outline-field/inspector classes only in owned Outline controls;
Component-inspector class only in owned component/relations/selected-keys panels.
Six exact m1.css selector corrections will exclude copied data-slots from Board
input; Outline input/focus; Component input/focus; and only the label alternative
of the Component label/definition compound selector. All declarations, native
reference branches and the definition alternative remain. Existing InputGroup
and InputGroupInput callers in BoardInspector, the two private Outline numeric
fields and ComponentInspector will supply narrow44 height. No new wrapper, API,
visibility, dependency, parsing, draft, callback, state or domain change.

Correction `67558cef` packages as build8 and all60 actual field combinations pass
`layout-remaining-field-styles-green.log`: five fields across12 variants, including
16px inputs/14px associated labels, transparent square borderless inputs, Field
token group fill, copied focus ring, desktop40/narrow44 input rectangles, native
reference1px border and retained actual nodes. Captures are
`layout-field-styles-20261022-220403/`.
The source diff has exactly six selector guards with unchanged declarations and
13 existing InputGroup/Input pairs with a narrow height class, reviewed at
`remaining-field-caller-classes.diff`. No domain expression or event changed.

Further actual owner REDs at the same production bytes: Mirror width/target
checks now record all12 overflows (`mirror-width-target-actual-red.log`). The
source choice extends past the form right edge (262.8/287px vs252px desktop),
while the actual axis input remains40px narrow. Attribution at
`mirror-choice-width-css-attribution.log` shows the grid form228px wide but its
Field/Select children retain their intrinsic238.8px minimum; this is caller
layout, not shared Select identity/selection. Bound the source Field's minimum
and the existing Select's width/value span, retain full accessible text, and give
both existing short/long choice and axis callers a narrow44 target. No shared
Select look or panel width changes.

New mirrored-pair actual textbox/number/gap variants fail all36 combinations in
`layout-new-pair-field-styles-red.log`: opaque bordered/radius inputs and legacy
36px height. The real child-owned Left-name draft/node survives the12 variants;
Cancel preserves the existing single-key matrix before aggregate assertion.
Two legacy appearance selectors (`.m1-matrix-setup` and
`.m1-mirrored-pair-setup` input/select branches) are exclusive to owned forms;
MatrixSetup itself now has no visible native inputs, and its guide/canvas
positioning/layout rules must stay for023. Exclude copied data-slots only from
those two appearance selectors, keeping declarations/native branches, and add
narrow44 classes to the five existing paired InputGroup/Input callers. No
source/draft/submit/Cancel/geometry/state contract changes.
