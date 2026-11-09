# 007: Accessible action menus

Status: resolved
Type: build
Blocked by: [003: Base component set](003-base-component-set.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [accessibility floor](../../rust-ui-migration/accessibility-floor.md), [mapping](../../rust-ui-migration/mapping.md), [shared contract](../map.md#shared-contract)

## What to build

Dropdown Menu is usable for the Project, Settings, Transform and Layout command menus. This is the largest unknown in the plan: estimate it first and record the finding in this ticket.

## Acceptance criteria

- [x] The menu uses `menu` and `menuitem` roles (and the checkbox and radio variants), focuses the first item when opened, supports arrows, Home, End and typeahead, activates with Enter and closes
- [x] Escape and item activation return focus to the opener
- [x] Repository browser tests cover the floor rows and pass
- [x] If the fix proves larger than one ticket, this ticket records that and the documented fallback (keep the `Popover`-based menus used today) is adopted, with the decision noted in the plan map
- [x] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches

## Estimate (2026-10-22)

The copied source now exists and was inspected at local base `ec6318c6d`. It forwards to pinned
`dioxus-primitives` DropdownMenu. That primitive's menu context and ordered focus collection are
private; no public hook lets the copy request initial focus or typeahead. It also couples open state
to that private collection's focus. Merely overriding ARIA attributes cannot meet the floor.

Estimated scope: keep the copied styling and public props, use the existing public Popover primitive
for overlay lifecycle/dismissal, and add menu-specific focus/navigation in this copy. Checkbox and radio
items can use the existing item's role/checked attributes with caller-owned selection. No dependency,
visibility change, Select/Combobox edit or application menu migration is needed. This is a coherent
007-sized adaptation if the representative mounted floor passes. The permitted fallback remains the
existing application Popover menus if that cannot be demonstrated; it is not a claim those menus
already pass the action-menu floor.

## Outcome

The adaptation fits this ticket: `dropdown_menu.rs` retains its public props and all copied class
strings, uses the pinned public Popover primitive for overlay lifecycle and owned dismissal, and
implements action-menu focus/navigation locally. The existing Item accepts `menuitemcheckbox` /
`menuitemradio` and `aria_checked`; values, selection state and callbacks remain caller-owned. No
dependency, visibility widening, shared registration, Select/Combobox, icon, Theme or application-menu
migration change was needed. Dedicated tests live in `ui-shared/src/action_menu_tests.rs`, included
only from the menu module. The fallback to retaining today's application menus was **not selected**:
the new menu passes the floor; later shell/Layout tickets still own adoption. No plan decision changes.

The menu names its popup from the actual trigger, honours custom trigger/content/root IDs, initially
focuses the first enabled item in index order, supports looping/non-looping arrows, Home/End, case-insensitive
prefix search and repeated-letter cycling, and closes on activation or Escape. Actual open-to-closed
transitions restore the opener unless another outside control already owns focus, preserving the
existing BoardPopover dismissal model. Popover's real exit animation is retained; closed content is
inert and cannot activate an action. No-op dismissal requests do not echo callbacks.

Saved work: claim `9d0514d`, retained RED `75d35d85`, implementation `1507c643`, lifecycle corrections
`3485a9d98e0dd942526cc94546204451c149a4fb`. Local feature was merged twice, through reviewed
`20dd558763bf05681a305c227f58f5eca9375601`; 002/004/006 changes are preserved.

Verification, 2026-10-22 (logs under this lane's gitignored `.evidence/`):

- **RED**, actual copied control in Chromium: `007-menu-red.log`, 0 passed / 1 failed at
  `focus reaches menuitem Align did not hold within 5000 ms` (default-open role/initial-focus floor).
  Initial command/fixture compilation mistakes were corrected before this retained browser RED.
- Additional test-found failures were corrected without weakening assertions: repeated-letter then
  multi-letter typeahead (`007-menu-edges-2.log`), outside dismissal with a caller root ID
  (`007-menu-edges-3.log`), and retained closing content remaining in the focus order
  (`007-retention-red.log`). `007-retention-green.log` then first failed at focusing Mirror because
  Dioxus 0.7 serializes `inert=false` as a present attribute; optional attribute presence fixes that.
- **GREEN**, clean source `3485a9d98e0d`,
  `NODE_PATH=/home/chris/.cache/boardstudio-web-tools/node/node_modules BOARDSTUDIO_CHECK_JOBS=1 python3 scripts/check.py browser --only ui-shared --receipt .evidence/007-browser-post006.json`:
  **47 + 2 isolated panel tests passed**, zero failures/ignored/incomplete. Receipt and summary:
  `007-browser-post006.json` / `007-browser-post006.json.md`; log `007-browser-post006.log`.
  The three dedicated menu tests include pointer/keyboard opening, all menu floor rows, checkbox/radio
  state and command outcomes, disabled/refused-controlled opening, custom trigger/index ordering,
  outside-focus preservation, and five controlled/uncontrolled Escape/Enter/external-close cases
  with a paused real Web Animation. Finishing the animation, rather than sleeping, releases the popup.
- `cargo test --locked -p boardstudio-web-ui-shared --lib`: **14 passed** (`007-native-post006.log`).
- `cargo clippy --locked -p boardstudio-web-ui-shared --target wasm32-unknown-unknown --tests -- -D warnings`:
  **passed** (`007-clippy-wasm-post006.log`). The preceding run found two own lints, corrected here.
- `cargo fmt --all -- --check`, `git diff --check`, `python3 scripts/check-wasm-tests.py`,
  `python3 scripts/check-doc-links.py`: **passed** (`007-fmt-post006.log`,
  `007-wasm-declarations-post006.log`, `007-doc-links-post006.log`). No native test was renamed.

One intervening browser run, `007-retention-green-2.log`, stalled before WASM loading, with
ChromeDriver reporting ready and no Chrome child. It was stopped after exceeding the existing
120-second test bound (SIGTERM); it is **incomplete startup evidence**, not GREEN. The subsequent
fresh supported scheduler run above passed; the startup cause remains unexplained. An optional
`test-theme.py` invocation stopped at argument validation because it requires a running page URL;
no Theme/first-paint validation is claimed from it.

Scope limits: these are mounted Chromium checks, not the full build/browser/product gate or a
screen-reader/visual review. No full quick or CAD fix is claimed: the inherited first failure remains
`worker.onerror` at `cad/scripts/browser-smoke/qualification.mjs:22:63`, unexplained despite later passes.
First-paint, global gates and final human review/026 remain open. No origin push or dev merge is part
of this ticket.

### Closure review and local landing authorization

The coordinator supplied actual **Standards: 0 findings** and **Spec: 0 findings**, both approving
`c1e2f7b574cc65d42fabb20813d41c43b641eedc` against base
`20dd558763bf05681a305c227f58f5eca9375601`. Both reviews covered all three owned files and the
connected public Popover, controlled-state, dismissal, animation and attribute behaviour. Props,
classes and action ownership are preserved; no scope creep was found.

Review receipts:

- Standards: `node:delegated-task:command%3Amcp%3A2a86db4d-5ca5-4383-b139-deebb4fe7ee7%3Adelegate-task%3Arust-ui007-standards-review-20261022-r1`
- Spec: `node:delegated-task:command%3Amcp%3A2a86db4d-5ca5-4383-b139-deebb4fe7ee7%3Adelegate-task%3Arust-ui007-spec-review-20261022-r1`

Reviewers reused the author's 47+2 browser receipt at `3485a9d98e0d`; they did not independently
rerun runtime checks. The subsequent review-outcome/resolution changes are documentation only.
Before resolution, local feature still equalled `20dd5587`, the lane was clean, and both owned Rust
files were identical to the validated source. Sufficient executed checks are reused. The coordinator
authorized the plan's local fast-forward landing after this review record; its exact resulting HEAD
and ref state are reported in the coordinator handoff.
