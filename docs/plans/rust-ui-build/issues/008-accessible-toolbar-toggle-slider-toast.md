# 008: Accessible Toolbar, Toggle Group, Slider and Toast

Status: claimed
Type: build
Blocked by: [003: Base component set](003-base-component-set.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [accessibility floor](../../rust-ui-migration/accessibility-floor.md), [shared contract](../map.md#shared-contract)

## What to build

The remaining controls with measured gaps pass the floor.

## Acceptance criteria

- [x] Toolbar has one tab stop with arrow roving, and End reaches the last item
- [x] Slider supports Home (minimum) and End (maximum) and PageUp and PageDown
- [x] Toggle Group requires a group label and exposes a group role
- [x] Toast items are announced with `alert` or `status`, not `alertdialog`, with a reachable dismiss button
- [x] Repository browser tests cover the floor rows and pass
- [x] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches


## Outcome

Implemented the four copied controls, dedicated tests, a fixture entry point/driver, and the approved three-line test registration in `web/crates/ui-shared/src/lib.rs`. The existing `web-sys` dependency enables only two additional platform types (`MutationObserver`, `MutationObserverInit`), acknowledged by the coordinator as necessary for the scoped focus repair. No dependency was added, and no member visibility, tokens, icons, workspace migrations or global configuration changed.

### Behavior and ownership

- Toolbar retains the primitive's buttons, disabled state, callbacks and separator. Component-owned DOM events maintain one enabled tab stop, skip disabled items, move with the orientation's arrows, and handle Home/End. Tab leaves the toolbar and Shift+Tab returns to the recent item. A component-owned observer watches only this root's child changes and `tabindex`/`disabled` attributes, including nested Toggle Groups. It writes only changed tab indices and disconnects on component drop before releasing its Rust callback. This keeps a valid entry after removal or disabling. The pinned primitive's `CollectionState` and focus methods are private, so the repair stays on this owned DOM; no upstream patch, visibility widening, script or document listener was added. Required mount/focus/key listeners are composed with caller listeners at Dioxus's raw `PlatformEventData` boundary, preserving both behavior and caller attributes/callbacks.
- Toggle Group now requires `aria_label: String` and renders `role=group`. The public primitive still owns controlled/uncontrolled selection, single/multiple mode, `aria-pressed` and keyboard roving. Tests cover disabled-item skipping, both orientations, wrapping, Home/End, Enter/Space and unchanged callback counts.
- Slider uses the primitive's public `use_controlled` hook to add Home=min, End=max and PageUp/PageDown=ten steps, clamped to the range. Pointer and arrow handling remain with the primitive. The thumb now has native `disabled` and `aria-disabled`, so it is skipped by Tab. Controlled and uncontrolled tests assert the exact value notification sequence, including the unchanged arrow behavior. Caller key listeners are composed with the value-key handler rather than replacing it.
- Toast is a single atomic `role=status` announcement (implicit polite live region), named by its title, containing title and description. The inner content has `role=none`, avoiding a second alert; no `alertdialog` or modal semantics remain and the item is not a tab stop. It does not move focus when created. Its existing `close` button is reachable by Tab and dismisses by Enter or Space. Escape is deliberately not captured or used to dismiss: the floor's Toast row explicitly leaves this decision open, and these passive notifications leave workspace Escape handling available. The provider's notification dispatch, timing and close behavior remain primitive-owned. No screen-reader speech measurement is claimed.

### RED evidence

All logs are under `.evidence/008/` in this bound lane. Before production edits, nine independent mounted runs failed for the expected gaps: Toolbar tab stops/End, Toggle Group role, Slider Home/End/PageUp/PageDown/disabled, and Toast `alertdialog`. `independent-red-summary.log` records all nine, including the explicit `red-toggle.log` reference. The initial broad `mounted-red.log` first failed with four Toolbar tab stops instead of one; later panic-hook cascades were excluded as independent evidence.

`trusted-red-example.log` independently reproduced failures for all four controls with agent-browser's real keyboard input against the actual mounted copies and the Rust role/name helper. An earlier export-only test-WASM initialization attempt failed before interaction; `trusted-red.log` is explicitly excluded from behavioral RED. The tiny example exists only to initialize that same fixture without the test harness's main; it has no success branch or replacement control behavior.

An additional post-fix lifecycle regression, `red-toolbar-removal.log`, showed that removing the current Toolbar item left the remaining item at `tabindex=-1`. The final owned observer also covers this case; the same assertion remains in the dedicated suite. A further composition test (`toolbar-toggle-composition.log`) confirmed that an embedded Toggle Group restored a second tab stop. The observer fixes that without replacing Toggle selection or widening private APIs. An attempted private-context/attribute handoff did not override the primitive's own extended `tabindex`; those diagnostic runs are not GREEN evidence.

Caller-hook regressions then failed independently at runtime: `red-toolbar-caller-runtime.log` found two tab stops when the caller mount hook replaced setup; `red-slider-caller-runtime.log` confirmed that Home did not change value with a caller key hook. Initial fixture event-prop compilation errors are not behavioral RED. An intermediate attempt to call raw attribute callbacks with converted event data caused a downcast panic and is excluded from GREEN. The corrected raw-event composition passed all 16 dedicated mounted tests in `mounted-new-source-green-r3.log`, including exact callback counts, dynamic disabled/child changes, nested Toggle keyboard navigation and observer teardown. The teardown test mutates retained detached toolbar children after unmount and crosses the observer microtask checkpoint; it does not sleep or retry an assertion.

### Verification and integration

Initial base: local `feat/rust-ui-migration` at `ba8b881c392cb2dde8fc7931f80348b234ee6e2c`. Claim saved as `f5b13c0`; mounted RED checkpoint `56d12e3f`; implementation `c2f918482`; merged 006's local feature tip `20dd558763bf05681a305c227f58f5eca9375601` at `e62b08bc5`, preserving Select and `primitives_mount_tests.rs`; lifecycle/lint correction `4853ba5b0b07313d42ff6dd086bb7b909bca3686`. Subsequently merged 007 at `f88217ee0`, saved composition/caller regressions at `257ecce0`, corrected raw listener composition at `69f2b7ba`, and merged 009's feature tip `44de5fad27b255b34167aa1481c8f6ff422e048c` at `7a63cd8c3489ea8c99b76d0e0020c73a6377f3c8`. The Select/menu/dock source and registrations were preserved.

The existing bound checkout was retained. `target` points to `/home/chris/.cache/boardstudio-target/008-accessible-toolbar-toggle-slider-toast`; the new-lane protocol's ignored settings and CAD caches were copied without overwriting existing files. Builds ran serially per target using project concurrency. Browser commands used `NODE_PATH=/home/chris/.cache/boardstudio-web-tools/node/node_modules` and `BOARDSTUDIO_CHECK_JOBS=1`.

Prior-source receipts `build-lint-repo-receipt.json` and `browser-receipt.json` refer to `4853ba5b` before the observer/caller fixes and latest integrations; they are not final-source acceptance. Final checks executed on clean `b36fb14d7c8de63e0db646b1b930e377d51781b7`, including latest local feature `44de5fad27b255b34167aa1481c8f6ff422e048c`:

- `cargo test --locked -p boardstudio-web-ui-shared --lib`: **15/15 passed**, `final-source-native.log`.
- `cargo clippy --locked -p boardstudio-web-ui-shared --target wasm32-unknown-unknown --tests --example ticket_008_fixture -- -D warnings`: **passed**, `final-source-wasm-clippy.log`.
- `python3 scripts/check.py build lint repo --receipt .evidence/008/final-source-build-lint-repo.json`: **all passed**, 188.0s. `final-source-build-lint-repo.log` and the receipt record the checked packaged build, native/WASM Clippy and repository checks.
- `python3 scripts/check.py browser --only ui-shared --receipt .evidence/008/final-source-browser.json`: **63 + 2 mounted tests passed**, including all 16 dedicated 008 tests, 006's Select/Combobox and retained-animation callbacks, 007 action-menu tests and panel checks. `final-source-browser.log`; this is explicitly a partial browser subset.
- `cargo build --locked -p boardstudio-web-ui-shared --target wasm32-unknown-unknown --example ticket_008_fixture`: **passed**, `final-source-fixture-build.log`.
- `python3 web/crates/ui-shared/ticket-008-keyboard.py target/wasm32-unknown-unknown/debug/examples/ticket_008_fixture.wasm /home/chris/.cache/.wasm-pack/wasm-bindgen-9411bdb1a3e2bbb9/wasm-bindgen`: **4/4 trusted-key scenarios passed**, `final-source-trusted-browser.log`. Toolbar includes real Tab/Shift+Tab out of and back into nested Toggle items; Toggle Group selection, both Slider ownership modes/exact callbacks and Toast announcement/dismissal retain the original assertions.
- `git diff --check`: **passed**. No native test was renamed or removed; no native test-name baseline change was needed.

The check receipts identify a clean source revision and no local changes. Subsequent ticket-only documentation reuses those executed code checks; repository documentation checks are rerun for the final text. Closure review is pending. Before local landing, merge any newer local feature tip, rerun checks affected by its inputs, and wait for the coordinator's real Standards+Spec review resolution. No origin push or merge to dev is authorized here.

Full browser/quick/CAD acceptance, the unexplained CAD `worker.onerror`, first-paint timing and human workspace review remain open; scoped evidence here does not close those gates.
