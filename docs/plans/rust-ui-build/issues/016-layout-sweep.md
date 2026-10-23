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
