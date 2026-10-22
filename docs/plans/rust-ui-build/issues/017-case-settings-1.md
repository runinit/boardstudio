# 017: Case settings, part 1: dimensions, construction and profiles

Status: claimed
Type: build
Blocked by: [006: Accessible Select and Combobox](006-accessible-select-combobox.md), [008: Accessible Toolbar, Toggle Group, Slider and Toast](008-accessible-toolbar-toggle-slider-toast.md), [011: Docks and narrow-screen sheet](011-docks-and-narrow-sheet.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [inventory](../../rust-ui-migration/inventory.md), [shared contract](../map.md#shared-contract)

## What to build

The first half of the Case mechanical settings: dimension fields, construction controls, profile guidance and the profile geometry editors. The settings file is the largest single sweep (86 `Board*` uses), so it is split here. Applies the shared contract in the plan map: raw controls and `Board*` become the mapped components, inline messages are classified, icon-only buttons get Tooltips, selects follow the 12-option rule, tests find controls by role and name, and the lane deletes only the old CSS its files used.

## Acceptance criteria

- [ ] The dimension, construction, profile and resolved-stack groups of the settings have zero raw controls and `Board*` uses, with precise numeric entry unchanged
- [ ] Existing tests for these groups pass after the rewrite rule
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches

## Claim and ownership

- Bound lane: `feat/rust-ui-017-case-settings`, `/home/chris/.t3/worktrees/ts-boardstudio2/t3-111fcd9a`; clean initial `b237903705244a266b07a64e5f23b8a3b5219c03`, fast-forwarded before claim to local migration `f11e6bd84456b94b2d7657f57a0d05b42184c4e7`.
- Exclusive target: `/home/chris/.cache/boardstudio-target/017-case-settings-1-lineage`, linked as `target`; copied ignored sandbox settings, `cad/.cache` and `cad/wasm/pkg` per lane protocol. No inherited 015/root target.
- Owned production hunks in `web/crates/case/src/mechanical_settings.rs`: `ConstructionControls`, `MechanicalLegacyConstructionControl`, `ProfileGuidance`, `ProfileGeometryEditor`, profile polygon/volume editors, `DimensionControls`, `ResolvedStack`, and their enclosing profile/resolved disclosures.
- Shared internal hunks: imports plus `DimensionField` markup only, keeping their domain targets, draft/settlement controllers, parsing and callbacks intact for 018 callers. `TextDraftField` is entirely 018 and stays unchanged. The only approved public API change is the separately reviewed shared InputGroupInput prerequisite; no visibility or dependency changes.
- Owned checks: Case mounted tests and a role/name repository browser test of the adopted production controls. Any shared test registration or CSS coexistence hunk will be inventoried before editing. Shared legacy CSS stays for 024; 018 groups and 019 viewers/rest stay outside this ticket.
- Concurrent lanes: 016 Layout and 020 Parts library. Parked extraction is complete; only an actual matching active writer blocks work. Landing waits for one substantive coordinator Standards+Spec closure at a clean source pin.

## Verification in progress

- Isolated shared prerequisite `df96cddad9a326a25bdfd0acff6692ef6eacfd3d` changes only `web/crates/ui-shared/src/ui/input_group.rs`: approved optional `max` and GlobalAttributes, filtered by exact native name and namespace so explicit props (including omitted options) and `oninput` retain ownership. Raw Dioxus event attributes forward blur/key/mount without new typed props. The coordinator owns prerequisite review/landing; other lanes reuse only the landed feature pin.
- Shared GREEN: `CARGO_TARGET_DIR=/home/chris/.cache/boardstudio-target/017-case-settings-1-lineage BOARDSTUDIO_CHECK_JOBS=1 CARGO_PROFILE_DEV_DEBUG=false wasm-pack test --headless --chrome web/crates/ui-shared --locked --lib -- ui::input_group::`: 2/2 passed (`.evidence/017/shared-input-forwarding-v3.log`). Collision, default omission, styles/ARIA, native precision/constraints and once-only listeners exercised.
- Shared native GREEN: `CARGO_TARGET_DIR=/home/chris/.cache/boardstudio-target/017-case-settings-1-lineage cargo check --locked -p boardstudio-web-ui-shared --all-targets --features test-support` (`.evidence/017/shared-input-native-all-targets.log`). The initial default native check encountered the existing runtime import cfg without test-support (`shared-input-native-check.log`); the shared compile diagnostics and controlled-value fixture correction are retained and are not behavior RED evidence.
- Case RED: the production `DimensionField`, using real native focus/blur, rejects `-1` locally and retains its draft, then fails `validation must describe the precise input` because error ARIA is absent (`.evidence/017/red-dimension-description-native-blur.log`). Earlier test compile and synthetic-focusout setup failures remain separate (`red-dimension-description.log`, `red-dimension-description-runtime.log`). GREEN is pending.
- Source comparison confirms the entire DimensionField owner/controller/parser/commit closure and complete 018 TextDraftField remain byte-for-byte unchanged. Case adoption is not yet build/browser acceptance.
- Sole CSS coexistence hunk is the mechanical-settings descendant control rules in `web/assets/m1.css`: approved exclusions will skip adopted data-slot roots while preserving all 018 native element/focus/disabled/mobile declarations. Four-combination computed-style RED/GREEN is pending. Exclusive unused number/stack/layer CSS can then be deleted; shared legacy rules remain for 024.

- Fresh native Case check: `CARGO_TARGET_DIR=/home/chris/.cache/boardstudio-target/017-case-settings-1-lineage cargo test --locked -p boardstudio-web-case --features test-support`: 34/34 passed (`.evidence/017/case-native.log`). This does not compile WASM-only controls.
- Static inventory of the six owned component spans reports zero raw controls/Board uses; profile coordinates now use Field + FieldLabel + InputGroup/InputGroupInput + unit text while retaining immediate f64 parsing/callbacks. Document links passed across 567 Markdown files (`.evidence/017/docs.log`).
- Browser fixture uses actual `MechanicalSettings` inside the production `m1-inspector-body` wrapper; accepted values/callback observations are supplied, local draft/extraction owners remain production components. It covers the exact 12/13 option threshold and two real resolved-layer Toggle items. Browser runs remain pending; this fixture alone cannot prove application Undo.
