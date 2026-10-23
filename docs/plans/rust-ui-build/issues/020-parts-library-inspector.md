# 020: Parts library and inspector

Status: resolved
Type: build
Blocked by: [006: Accessible Select and Combobox](006-accessible-select-combobox.md), [008: Accessible Toolbar, Toggle Group, Slider and Toast](008-accessible-toolbar-toggle-slider-toast.md), [011: Docks and narrow-screen sheet](011-docks-and-narrow-sheet.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [inventory](../../rust-ui-migration/inventory.md), [shared contract](../map.md#shared-contract)

## What to build

The Parts library, previews and inspector. Applies the shared contract in the plan map: raw controls and `Board*` become the mapped components, inline messages are classified, icon-only buttons get Tooltips, selects follow the 12-option rule, tests find controls by role and name, and the lane deletes only the old CSS its files used.

## Acceptance criteria

- [x] The Parts library, chooser, preview and inspector rows of the inventory have zero raw controls and zero `Board*` uses
- [x] Existing Parts tests pass after the rewrite rule; the workspace works in both Themes and both Appearances at the three widths
- [x] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches

## Claim and ownership (2026-10-22)

- Bound T3 lane: `feat/rust-ui-020-parts-inspector`, `/home/chris/.t3/worktrees/ts-boardstudio2/t3-1e8d1a7b`.
- Initial clean base `b237903705244a266b07a64e5f23b8a3b5219c03` fast-forwarded to local integration `f11e6bd84456b94b2d7657f57a0d05b42184c4e7` before claim.
- Own `web/crates/parts/src/parts.rs` library/chooser/inspector controls, `parts/preview.rs` preview controls, `parts/modules_catalogue.rs` inspector controls, `parts/modules_catalogue/module_attachment.rs` attachment controls, and `web/src/presentation/parts_workspace.rs` Parts-only composition. Their existing tests and a scoped repository browser test belong to this slice.
- Keep editor implementations and their draft/validation/commit owners (021), canvas/rendering internals, Layout (016), and Case settings (017) outside this slice. No public API, visibility or dependency changes are planned.
- Shared-hunk inventory before implementation: preserve `parts.rs` module declarations, imported editor bindings and child editor calls; only change its owned controls and relevant tests. Shared legacy CSS and shared controls stay for 024/other owners unless a concrete overlap is established.
- Exclusive build target: `/home/chris/.cache/boardstudio-target/020-parts-library-inspector-lineage`; no inherited/root/015 target. Hold local integration landing for coordinator Standards + Spec approval of a clean implementation pin.

## Development checkpoints (superseded by Outcome)

Continuation and exact lease/decision log: [020 continuation](../020-lane-continuation.md).

- Baseline expected RED at `d94a562f`: actual PartsPreviewPanel ArrowRight roving test; `.evidence/020/red-parts-browser.log`. The 41 later panic-hook failures are shared-WASM contamination after that first failure.
- Native Parts tests: `cargo test --locked -p boardstudio-web-parts --lib`, 43 passed (`.evidence/020/native-parts.log`); native execution does not compile wasm UI.
- Reviewed CONTROL `c38dc6bf` and shared LAYER `b9371dbd` were safely merged, retaining their canonical source/contracts/renderer and bounded CSS. Fresh post-LAYER scoped browser runs at `f36890e6`: Parts **71/71** and Case **83/83** passed (`.evidence/020/green-parts-case-landed-layers.log`). This is not the full browser gate.
- Original ArrowRight regression passed before broader checks. Actual implicit long-picker Label association failed before the approved None-omission fix; the same real short Select / long Combobox composition then passed, including label activation, keyboard values, same node and primitive-owned query. Shared Label None / Some ID / Some empty contract passed **1/1**. See continuation for pins and logs.
- Repository checks, documentation links, WASM source guard, Python script syntax/help, and diff checks passed in the scoped development loop. Fresh exclusive-target providers built; page packaging, real-page captures/computed styles, and final lint/review remain pending.
- `1e2a8815` is an interwoven WIP checkpoint, not a completed isolated chooser slice. Landing remains held for one coordinator Standards + Spec closure at a clean verified pin.

## Outcome

Implemented the Parts library, chooser, controller picker, preview modes, module/placement Inspector controls and their messages with copied controls. Select is used through12 options and Combobox above12. Existing selection epochs, callbacks, pending/commit/Undo paths, local search drafts, transient preview values and renderer lifecycle stay with their original owners. No021 editor/import/model/profile/generator implementation changed. Owned interactive source has no raw controls or Board wrappers; noninteractive row headers and the explicitly retained native viewer color wells keep their semantics.

Review base is latest local feature `43fae25436776955ec1b3f363e61be30f3425661`, safely merged at `fa965b7d7f63e8558074b3d35a32f64fc2e61a3e`. Final product source is `6083dd4d7f37955af6b4eb042a4437fbc6004848`; later changes only finish this receipt and the browser probe's incorrect New assembly availability assumption. The branch's clean closure commit is reported to the coordinator. No local integration landing or remote/dev push occurred; one coordinator Standards+Spec review and substantive approval still gate landing.

The exact shared review scope is:

- Case SharedViewer imports, pure derived Select value, NONINLINE docked color/layer controls and view controls; conditional toolbar classes preserve the exact old inline classes. NONINLINE-only container-size style and popup height limit prevent focus from scrolling/clipping the viewer; local Slider classes position visible thumbs and reserve their focus rings. State, callbacks, clamps, renderer, DesignViewToolbar and inline control branches are unchanged.019 must reuse this landed noninline work.
- Exactly seven m1.css shared-viewer control selectors exclude data-slot controls with zero-specificity :where, retaining every declaration/native/Board/inline match. Only exclusive unused Parts selectors were deleted elsewhere.017 mechanical CSS and canonical CONTROL/LAYER source/manifests/lock remain unchanged. No Layout CSS or canvas_layers writer overlap.
- Approved Label None omits for instead of for=""; explicit Some ID and explicit Some empty remain unchanged. Focused actual long-picker association RED preceded the fix, then actual short Select/long Combobox label activation, keyboard values and retained query passed. The searchbox helper adds only native input[type=search], preserving explicit roles.
- Approved public upstream CheckboxState re-export was isolated at150670bad and landed separately in CONTROLc38dc6bf; private CheckboxProps stays private. Current controlled Checked/Unchecked callers compile and run. No new public prop, private visibility widening or dependency.
- AddObjectChoice/ComponentChooser remain020-owned, with unchanged props, layout target, query, selection and placement callbacks. Their actual owner/retained category/search/choice/layout tests pass. This was never an isolated chooser slice;016 reuses the completed landed020 result.

Meaningful RED/GREEN evidence includes original actual Parts ArrowRight, actual implicit long-picker Label association, real selected-control legacy border/token cascade, narrow popup ancestor clipping, clipped/static Slider thumb and its endpoint focus ring, and mode SVG stroke loss. Their original logs/PNGs are preserved under `.evidence/020/`; detailed source pins, approved hunks and first-failure distinctions are in [the continuation](../020-lane-continuation.md). Later panic-hook symptoms were not counted as independent REDs. Self-inspection corrected the owned line glyphs and ModuleInspector secondary Edit action; no shared Button/mapping sweep.

| Verification | Result and evidence |
| --- | --- |
| Final fast page build, using earlier fresh exclusive-target providers | PASS; `page-build-owned-glyphs.log` |
| `BOARDSTUDIO_CHECK_JOBS=1 python3 scripts/check.py browser --only parts case` at product6083dd4d | Parts71/71 + Case85/85; `parts-case-final-6083dd4d.log` |
| Actual packaged page via `scripts/test-parts-library.py`, with before-sizing measurements | PASS; `real-parts-verified-6083dd4d.log`:12 Theme/Appearance/width records,72 slider endpoint/fraction records,12 complete popup Tab walks,28 captures |
| `cargo fmt --all --check`; locked WASM all-target Clippy for web/Parts/Case/ui-shared, no-deps, warnings denied | PASS; `fmt-final-6083dd4d.log`, `clippy-final-6083dd4d.log` |
| Shared disabled Slider native/ARIA contract |1/1; `shared-slider-disabled-contract.log`. Actual owned sliders remain enabled; no synthetic production-disabled state was added |
| Reused unchanged evidence | Native Parts43; page Parts composition1; shared Label contract1; explicit-target provider build. Exact commands/pins in continuation |

Final actual-page command:

```sh
python3 scripts/test-parts-library.py http://127.0.0.1:42020/boardstudio/ --evidence .evidence/020/captures-final-verified-6083dd4d --sizing-baseline .evidence/020/viewer-sizing-before.json
```

Widths1440/1100/390 × Original/shadcn × light/dark use real Settings owners. All12 root/canvas/toolbar rectangles and backing canvas dimensions exactly match before the containment fix. Parent scroll stays0, toolbar heights56/56/64 plus8px gap fit the unchanged80px reserve; popup and focused16px slider thumbs plus4px rings stay inside clipping ancestors. Both sliders preserve precise values/clamps and retained node/value through close/reopen;40px desktop/44px narrow rows and independent mobile targets pass. Selected copied controls have zero borders/current tokens; line glyphs are visible/bounded/currentColor, native color/reset and disabled result remain intact. Native inline four-combination preservation is included in fresh Case85.

Screenshots: `.evidence/020/captures-final-verified-6083dd4d/` contains12 preview,12 library and4 narrow Inspector captures. Representative wide/middle/narrow captures across all four combinations were visually inspected. Actual native label-pointer probes require three consecutive activations each, with exactly one trusted forwarded click per activation; all passed. A captured earlier miss hit the popup DIV after geometry moved, rather than activating the label. The probe now waits for fonts/drag settlement, verifies the actual text hit target and waits on exact controlled state; no retry/skip or value assertion was relaxed.

Limits: browser evidence is scoped Chromium automation, not human026, screen-reader, native OS color dialog or full CAD/GPU acceptance. The empty active project's unrelated Case missing-outline status remains visible while the isolated Parts sample renders. Full/quick, CAD worker.onerror, first paint, screen-reader, human026 and final merge gates remain OPEN. Parent owns final review/landing. Named browser sessions are closed; only the lane-owned static preview remains for review. Exclusive-target cleanup follows verified landing/final checks, preserving this branch, worktree and evidence.

Closure audit: `.evidence/020/process-target-closure.log` records exclusive physical target **18,344,977,794 bytes**, no owned build/WebDriver processes or named browser sessions, and only static preview PID215706 on42020. Target/preview remain available for coordinator review; cleanup follows verified landing. Final repo/doc links, WASM source guard, Python syntax and diff checks passed (`repo-closure.log`, `wasm-guard-closure.log`).
