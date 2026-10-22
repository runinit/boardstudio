# 014: Shell glue sweep

Status: ready-for-human
Type: build
Blocked by: [011: Docks and narrow-screen sheet](011-docks-and-narrow-sheet.md), [012: Command palette](012-command-palette.md), [013: Toasts](013-toasts.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [inventory](../../rust-ui-migration/inventory.md), [shared contract](../map.md#shared-contract)

## What to build

The remaining shell files have no raw controls and no `Board*` left.

## Acceptance criteria

- [x] The inventory's app-shell rows that belong to the shell (workspace navigation, shortcuts, composition, inspector host, status footer, empty canvas) have zero raw controls and zero `Board*` uses
- [x] Messages are classified by the mapping's rule; icon-only buttons have Tooltips; selects follow the 12-option rule
- [x] The shell works in both Themes and both Appearances at desktop, mid and narrow widths, with screenshots recorded in the outcome
- [x] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches

## Claim

- Owner: implementation child in T3 worktree `t3-cff86af3`, branch `feat/rust-ui-shell-glue-sweep`.
- Initial checkout was clean at `b237903705244a266b07a64e5f23b8a3b5219c03`; fast-forwarded the latest local `feat/rust-ui-migration` to `3c0f6e26c17e53312eb29e67ca86b18c6fb2732a` before claiming.
- Lease: shell navigation, shortcuts, composition, inspector host, status footer and empty canvas; `presentation.rs` shell hunks only. Layout workspace hunks and ticket 015's Layout files/CSS/InputGroup API remain reserved. No child agents, threads or watchers.
- Landing held for coordinator approval and one combined Spec + Standards closure review at a clean final pin.

## Owned surface and overlap inventory

Refreshed against base `3c0f6e26c17e53312eb29e67ca86b18c6fb2732a`, before production edits:

| File / hunk | Remaining migration at claim | 014 ownership |
| --- | --- | --- |
| `workspace_navigation.rs` | `InspectorActions`: legacy Toolbar/Button | Wrappers only; delayed owner/generation guard and focus IDs retained. Header workspace Buttons already migrated. |
| `workbench_shortcuts.rs` | No controls | Read and preserve existing shortcut routing. |
| `workspace_composition.rs` | No controls | Read and preserve routing and domain projections. |
| `inspector.rs` | Two native number inputs, legacy Apply button, inline messages | Field/Label/InputGroup composition, Button, FieldError, guidance Callout; parsing, drafts, precision, transactions, IDs and callbacks unchanged. `BoardPartChoice` is a domain type, not a legacy control. |
| `canvas_status_footer.rs` | Four raw camera buttons, Layout-owned raw findings presentation | Toolbar and zoom Tooltips; private findings Button/Badge with existing trigger ID and callback. |
| `empty_board_canvas.rs` | Buttons already migrated; legacy empty markup/CSS | Copied Empty composition; existing two callbacks and disabled state retained. |
| `presentation.rs` | Raw footer Undo/Redo; findings fallback delegates raw presentation | Only shell stylesheet link, footer Toolbar, and findings fallback call. Test-only ExportPanel and Case/setup controls belong to later tickets and remain. Layout/workspace hunks reserved. |
| `test_role.rs` | Number inputs and associated native labels unsupported | Existing test-only helper gains native `spinbutton` and `label[for]` lookup. No API or visibility change. |
| Styles / tests | Shell-only rules and old inspector test locators | Remove only exclusive footer/empty rules; retain shared legacy CSS for 024 and Layout findings CSS for 016. New `shell.css`, shell browser test, mounted owner/callback checks; existing inspector tests keep their names. |

No edits to ticket 015's Layout files, CSS, or InputGroup API. No new dependency, shared product API, visibility widening, domain state or event is introduced. The legacy input's keyboard handler now bubbles from the same native input through a field-local wrapper, preserving native type/step and ownership without changing the shared input API.

### Initial evidence

- `.evidence/014-red-footer.log`: actual mounted RED, one failure at `camera controls adopt the accessible Toolbar` while footer still used a raw group.
- `.evidence/014-green-mounted.log`: first GREEN attempt blocked at compilation by Toolbar integer-index inference; corrected with explicit `usize` indices.
- `.evidence/014-green-mounted-r2.log`: intermediate 21 mounted tests passed. Later shell findings/tooltip identity and navigation fixture changes require fresh final checks.
- `.evidence/014-repo.log`: `python3 scripts/check.py repo` passed.
- Physical target: `/home/chris/.cache/boardstudio-target/014-shell-lineage`, linked by this checkout's `target`; `.claude/settings.json`, `cad/.cache`, and `cad/wasm/pkg` copied from the primary checkout before builds. Single Cargo/wasm-pack launch per target; project concurrency unchanged.

## Outcome

Implementation complete; **coordinator combined Spec + Standards closure review and landing approval remain pending**. This child has not delegated any agent, started any thread/watcher, or landed to the migration branch, origin or dev. Status is `ready-for-human`, not `resolved`, until that gate is approved.

### Result and boundaries

- All six S5 files now have zero production raw controls and zero legacy `Board*` control uses. `workspace_composition.rs`'s four inventory hits were Rust `match input {` expressions, not controls. `BoardPartChoice` remains the existing domain type. Header workspace Buttons and shortcut dispatch were already migrated and remain unchanged.
- Inspector uses native number `InputGroupInput` inside Field/Label/InputGroup, with the same IDs, `step="any"`, event values, parsing, drafts, pending ownership, Enter/Escape, transactions, selection and Undo paths. Field-owned failure remains inline as FieldError; preview/selection guidance uses Callout. Runtime report classification, notification queue and native palette host are unchanged.
- Footer camera/history actions use Toolbar. Zoom controls have keyboard-dismissable Tooltips; their composed attributes retain Toolbar item identity and tab order. Findings use a private Button/Badge with the existing trigger ID, count/name and toggle callback. Footer history is text-labeled; no icon-only history action needs an additional Tooltip. Empty uses the copied Empty composition and existing Buttons/callbacks.
- Source review caught and retained the shortcut toolbar's existing wrapping/centering. Generated CSS inspection confirmed that coordinate measurements must read the defined `--theme-font-mono`, rather than Tailwind's non-emitted inline alias `--font-mono`. Both corrections are in the final UI source pin below.
- Only exclusive footer/empty legacy rules were removed. Shared legacy rules, Layout findings CSS, Layout workspace/toolbar files and InputGroup API remain untouched. Test-only ExportPanel and Case/setup controls in `presentation.rs` remain for their workspace tickets. No new product API, visibility, dependency, domain state, callback or precision policy was introduced. No selects were adopted, so the 12-option rule introduces no new classification here.

### Pins and verification

Latest local feature was `3c0f6e26c17e53312eb29e67ca86b18c6fb2732a`. It was merged before claiming and rechecked/merged before final verification; each later merge was already up to date. Final UI Rust/CSS source is `1ec908c7986db7811a685c236049ab2d25bb49f8`. Subsequent commits `6babf1d8` and `894b7990` change only the shell browser test. `git diff --quiet 1ec908c7..894b7990 -- web` passed, so the page and mounted evidence carry forward without a redundant build. The review-ready head is the clean commit containing this Outcome, reported in the child handoff.

| Executed command / evidence | Result and scope |
| --- | --- |
| `BOARDSTUDIO_CHECK_JOBS=1 python3 scripts/check.py build --receipt .evidence/014-build.json` | Passed, 267.0s. Fresh providers and initial checked page from UI checkpoint `508b8ade`; receipt and `.evidence/014-build.log` retained. This initial page predates the font/wrapping correction. |
| `python3 scripts/build-web.py --fast --skip-providers` | Passed at clean UI pin `1ec908c7`; `.evidence/014-final-page-build.log`. Reuses the freshly generated providers above, whose sources this ticket never changes. Page was rebuilt for the font/wrapping correction. Packaged `assets/shell.css` matched source with `cmp`. |
| `python3 scripts/run-wasm-tests.py --files web/src/presentation/canvas_status_footer.rs web/src/presentation/empty_board_canvas.rs web/src/presentation/inspector.rs web/src/presentation/layout_remainder_tests.rs web/src/presentation/workspace_navigation.rs --result-json .evidence/014-final-mounted.json` | **22 passed, 0 failed, complete**, on final UI source `1ec908c7`; `.evidence/014-final-mounted.log` and exact selected/terminal names in JSON. Covers enabled/disabled camera callbacks, findings dispatch, empty actions, native precision/Enter/Escape/focus identity, failed-save/newer-draft preservation, one Undo entry and mounted Inspector shortcut focus. Also executes the runner-selected existing inspector offset-axis tests. |
| `python3 scripts/test-shell-glue.py http://127.0.0.1:4184/boardstudio/ --evidence .evidence/014-browser-r3` | **Passed** at clean test pin `894b7990` against the final UI page. Actual camera/history Home/End/arrows and disabled skip, one camera tab stop, zoom callback, tooltip description/Escape/focus, matrix creation and actual footer Undo/Redo, palette focus entry/return, and Keymap Inspector shortcut owner focus. Fifteen captures; result JSON, final snapshot and log `.evidence/014-browser-r3.log`. Chromium and ChromeDriver both `155.0.8059.84`. |
| `rustfmt --edition 2024 --config skip_children=true --check` on the seven changed Rust files; `python3 -m py_compile scripts/test-shell-glue.py`; `python3 scripts/check-wasm-tests.py`; `git diff --check` | Passed. No existing test name was renamed or removed; native name baseline unchanged. Final formatting/structure logs retained under `.evidence/014-final-*`. |
| `python3 scripts/check.py repo` | Initial pass in `.evidence/014-repo.log`; final documentation pass/receipt in `.evidence/014-final-repo.log` and `.evidence/014-final-repo.json`. |
| Impeccable detector on changed UI targets | Ran once, exit 2 with 157 existing findings in `m1.css`/`tailwind.css`; `.evidence/014-detector.json`. No findings in new shell CSS or changed Rust UI files. Existing palette/legacy patterns remain outside 014; detector output is not accessibility, screen-reader or human approval. |

The earlier RED and compile error are recorded above. Browser attempts are retained without overwriting: r1 failed on a test wait for explicit `[role=region]` while the AX snapshot and screenshot already showed the implicit New matrix region; the wait now uses the actual surface text. R2 completed matrix creation/Undo/Redo, then attempted workspace navigation while the retained palette host was opening: the test had only awaited the retained combobox node. R3 awaits actual combobox focus before Escape and actual Redo opener focus afterward. Both earlier failure snapshots/logs remain in `.evidence/014-browser-r1*` and `.evidence/014-browser-r2*`. No product change, delay, extra dismissal, retry loop or weakened behavioral assertion was used to turn those into a pass. Notification dismissal is confined to removing capture occlusion before the required empty-state screenshots.

### Screenshots and visual limits

All fifteen final captures were visually inspected from `.evidence/014-browser-r3/`:

- `shell-{original,shadcn}-{light,dark}-{1440,1100,390}.png`: the twelve required Theme/Appearance/desktop/mid/narrow combinations (heights 960/900/844). Empty actions and footer are readable and inside the viewport; narrow footer wraps without horizontal scrolling.
- `keymap-shortcuts-shadcn-dark-{1440,1100,390}.png`: additional actual Inspector shortcut and retained compact Sheet composition. The existing Keymap editor and queued save notifications are visible; those workspace controls/classification are not migrated by 014.

The passing browser script checks shell bounds and DOM/keyboard behavior; screenshot inspection is author visual evidence only. No measured screen-reader speech, touch hardware, other-browser, first-paint or human acceptance is claimed. Existing 011/012/013 Outcome evidence is reused only for unchanged source contracts, never presented as a fresh global run.

### Cleanup and handoff lease

Owned preview server PID `3935144` (port `4184`) was stopped after verification; every script-owned agent-browser session closes in `finally`. Server termination receipt remains in this session and `.evidence/014-preview-server.log` is retained. No build/test process remains owned by this child.

Physical target is `/home/chris/.cache/boardstudio-target/014-shell-lineage`: **10,845,125,476 bytes** (`du -sb`, after builds/tests). It remains protected pending approved landing and cleanup authorization. Disposable generated outputs in this checkout, also preserved for review:

| Output | Bytes at handoff inventory |
| --- | ---: |
| `web/target` (providers and packaged page) | 457,726,495 |
| `renderer/pkg` | 1,409,409 |
| `cad/.cache` (protocol copy) | 740,834,072 |
| `cad/wasm/pkg` (protocol copy) | 10,721,445 |
| `web/assets/cad` | 10,721,445 |
| `web/assets/renderer` | 1,409,409 |
| `web/assets/core-worker` | 17,165,742 |
| `web/assets/cad-worker` | 14,886,875 |
| `web/assets/ergogen-models` | 11,894 |
| `web/assets/tailwind.css` | 205,002 |
| `web/assets/imported-modules.json` | 43,356,120 |

Retain this committed ticket plus `.evidence/014-build.json*`, `.evidence/014-final-mounted.json`, `.evidence/014-final-*.log`, `.evidence/014-final-repo.json*`, `.evidence/014-browser-r3/`, all RED/failure receipts and detector output before any authorized fully-merged worktree removal. Evidence is valuable untracked handoff material, not disposable generated build output. No target/source copies/custom hash manifests were created inside `.evidence/`, and no worktree, target or unrelated work was deleted.

Global full/quick/full-browser acceptance, CAD `worker.onerror`, first-paint timing, screen-reader speech, human review/ticket 026 and the final merge remain open. Coordinator's one combined Spec + Standards review at the clean handoff pin and explicit landing approval are still required. Only after approval may local fast-forward landing occur; origin/dev remain untouched.
