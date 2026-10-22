# 020 Parts library / Inspector continuation

Status: claimed, implementation in progress; no landing approval and no local integration landing.

## Binding, source and ownership

T3 child and CWD: `/home/chris/.t3/worktrees/ts-boardstudio2/t3-1e8d1a7b`, branch `feat/rust-ui-020-parts-inspector`.
Initial clean base `b237903705244a266b07a64e5f23b8a3b5219c03` fast-forwarded to integration `f11e6bd84456b94b2d7657f57a0d05b42184c4e7` before claim `1e530ea`.
Read ticket 020, map/shared contract/lane protocol, migration spec, inventory, mapping, theme contract, accessibility floor, AGENTS, CODING_STANDARDS and DESIGN. Used Ponytail and Dioxus skills. No `.codegraph` exists; Gortex is uninstalled.

Own `parts.rs` library/chooser/Inspector, `parts/preview.rs` controls, `parts/modules_catalogue.rs` Inspector, `parts/modules_catalogue/module_attachment.rs`, Parts-only `presentation/parts_workspace.rs`, corresponding tests. No 021 editor implementation or domain owner changes. `AddObjectChoice` and `AddObjectComponentChooser` are 020's lease even though Layout calls them. Public props/callbacks/selection epochs are unchanged; the new option-count composition is private.

Approved shared lease: `case/shared_viewer.rs` imports and NONINLINE docked layer/color controls and NONINLINE view controls only; renderer lifecycle, state/owner, public API, DesignViewToolbar, inline branches and 017 settings are outside the lease. Controlled CheckboxState re-export is isolated commit `150670bad0c4add2e4f6b63bcc49b6fe68d0868b`, one file/one added line. Subsequent rustfmt ordering is recorded in WIP, not rewritten into that commit. Searchbox helper arm is isolated 020 support commit `2df24ee1d8dd1c818a0dd214d63a19088dc75607`; it preserves the explicit-role prefix.

`1e2a8815` is an honest, interwoven Parts + chooser + leased viewer WIP checkpoint, not an isolated chooser adoption. Keep it intact. Coordinator chose: reviewed shared control prerequisite first, then 016 standalone canvas_layers slice, then full 020, then 016 Layout sweep. No extra checkout/history rewrite or unlanded WIP copying.

## Build ownership

New exclusive physical target `/home/chris/.cache/boardstudio-target/020-parts-library-inspector-lineage`, linked by this lane's `target`; `.worktreeinclude` settings and `cad/.cache` / `cad/wasm/pkg` copied before builds. No inherited 015/root target, custom source manifests or source copies under evidence. Every Cargo/wasm-pack invocation is serialized on this target. Build logs are `.evidence/020/`. No agent, new thread, watcher or status poller was created.

## Executed evidence

- RED test pin `d94a562f`: `python3 scripts/check.py browser --only parts` -> expected first failure in `parts_preview_mode_arrows_rove_without_changing_the_local_view`, actual production PartsPreviewPanel controls, ArrowRight focus did not move from 2D to 3D within the shared five-second bound. `.evidence/020/red-parts-browser.log`. 27 passed; 42 failed, consisting of that one failure and 41 later shared-WASM panic-hook contamination symptoms, not 41 independent regressions.
- First migration compile: same command -> compile stopped before browser tests. TooltipTrigger has no onclick; copied Button had no attributes passthrough; also a local Select value needed a Memo. `.evidence/020/green-parts-browser-1.log`. Local Select fixed. No successful GREEN is claimed.
- `cargo test --locked -p boardstudio-web-parts --lib` -> 43 passed, zero failed. `.evidence/020/native-parts.log`. Native tests do not compile the wasm-only UI.
- `cargo fmt --all` and `git diff --check` executed during edits. Final checks await prerequisite merge.

Next runtime execution must filter the original ArrowRight test before broader Parts/browser checks. Shared Button prerequisite is independently reviewed/landed by coordinator; no local Button workaround or API copy. 016 is sole canvas_layers writer; reuse only its landed commit. No local InputGroup changes; 017 owns that prerequisite contribution.

## Additional CSS scope decision requested

The two approved viewer button selector exclusions at old tailwind.css 1439/1446 are applied. Own noninline settings content now uses utility classes instead of the legacy settings-body class; inline class is untouched. Obsolete exclusive Parts catalogue/search rules were removed only after enumerating their production uses.

Source tracing found more inherited viewer rules: m1.css 1086/1087/1105 and tailwind.css 938/939/940/1387/1389/1390/1426/1434/1435/1437. They still force padding, heights, backgrounds and pressed colors onto copied groups/buttons under `.m1-case-view-toolbar`. Editing every selector would expand the shared CSS change.

Proposed smaller additional hunk, before editing: at SharedViewer's toolbar root, make its class conditional. For `inline_case_controls`, retain exactly `m1-case-view-toolbar board-tool-pill`; for the already leased noninline branch use utility-only `relative flex flex-none flex-wrap items-center justify-center gap-1 rounded-md bg-toolbar p-1`. This removes all legacy descendant selector inheritance from copied controls without changing any inline declaration/callback/state/API. Verify positions, computed values, pointer/keyboard, themes/appearances, and native inline controls. If approved, the two earlier selector changes become unnecessary and can be removed from this lane's own diff. No broader CSS sweep is proposed.

## Remaining gates and limits

Focused ArrowRight GREEN, new picker and leased viewer owner tests, Parts mounted tests, affected Case tests, page composition check/build, real-page role/name browser floor, widths × Themes/Appearances screenshots/computed-style evidence, same-node retained-content isolation/drafts, and appropriate native/lint/docs checks remain pending. New tests retain production transient/query owners; no fixture substitutes for their state.

Request exactly one coordinator Standards + Spec closure at a clean implementation pin after merging latest integration. HOLD landing until substantive approval. Never origin/dev push. Global full/quick/CAD worker.onerror/first-paint/screen-reader/human026/final merge remain OPEN. Stop owned previews, report exclusive physical target size/process ownership and evidence at handoff; clean generated output only safely/when authorized. Preserve lane branch/worktree/evidence.

Parent notifications are now limited to blocking decisions/scope approvals, required review readiness and final result (at most five lines with one pointer). Detailed progress stays here and in ticket outcome.

Coordinator approved the conditional toolbar-root hunk above. Applied: inline exact old class retained, noninline utility-only; the two earlier shared viewer selector exclusions were removed from our own diff. Remaining CSS changes delete only proven-exclusive obsolete Parts selectors. Runtime positioning and computed-style verification remain pending.

Further own work while shared contract is pending: added unexecuted actual PartsChoiceField Select/Combobox query-owner test and actual SharedViewer transient/fraction/identity test; repository real-page script `scripts/test-parts-library.py` uses trusted agent-browser role/name actions and prepares 24 library/preview theme-width captures. Native `<th scope="row">` is retained inside copied Table rows to keep row-header semantics without adding public TableHead props; it is not an interactive control. Attachment success becomes a transient Toast (actual settlement owner unchanged), progress a Spinner, persistent errors Alerts and guidance Callouts. Existing attachment lifecycle test now mounts the standard ToastProvider and dismisses the landed success before continuing its unchanged retirement/failure assertions. Those WASM changes are not claimed green yet.

`python3 scripts/check.py repo` passed (including doc links and source guards; old-css still reports migration debt). An initial `check.py docs` invocation was rejected because that step does not exist; corrected to the documented `repo` step. Python syntax/help checks passed for the new browser script. All detailed output remains under `.evidence/020/`.
