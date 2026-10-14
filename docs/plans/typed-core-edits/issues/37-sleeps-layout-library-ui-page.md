# 37: Replace fixed sleeps in the remaining browser suites (layout, library, ui-shared, keycaps, keymap helpers, page; plus what 36 left in case, pcb and parts)

Status: implemented (awaiting verification)
Type: build
Blocked by: [36](36-sleeps-case-pcb-parts.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Audit: [test-audit.md](../audit/test-audit.md)

## Request

Continue ticket [34](34-test-speed-browser.md): replace fixed settle loops with `wait_until` (`web/crates/ui-shared/src/test_wait.rs`) in the suites named below. The keymap
`queued_edit_tests` pilot took that suite from 37.0 s to about 9.7 s with the assertion count and test names unchanged. Pattern to follow: each wait checks the precondition the next
assertion needs (including a `rendered_accepted`-style wait for the accepted revision before an action); every wait is bounded; no test depends on elapsed time except that bound; unbounded
`.await` gates in these tests get a bounded variant.

## What to do

- `web/crates/layout` (35-80 ms settles, outline settle up to 100x10 ms; 13.7 s), `web/crates/library` (80 ms and 40x20 ms polls; 11.1 s), `web/crates/ui-shared` (`pending_edit_helpers.rs` `settle` 12x30 ms; `geometry_scripts` up to 100x10 ms), `web/crates/keycaps` (40 ms), the 40 ms helpers in keymap `macro_controller`, `macro_editor` and `panel`, the page binary tests under `web/src/presentation` (40 ms x27; `case_workspace` settle 100 ms). Stop where the gain per suite is under about 1 s.

## Acceptance

- [ ] Per-suite before/after table in the Outcome; `browser` three consecutive passing runs; executed test names equal the baseline except deliberate merges (diff the crate `--list` outputs before/after).
- [ ] Mutation checks: three representative waits per ticket made never-true fail with the helper's message within the bound; the production step each depends on removed makes the test fail fast.

## Verification

`repo lint typecheck browser` (sandbox lifted for wasm-pack and Chrome), run three times. Process: [verification-process.md](../../../agents/verification-process.md). Time box about two hours.

## Carried from 36

Ticket [36](36-sleeps-case-pcb-parts.md) was resolved for its delivered scope only (verified 2026-10-10; Outcome and Verification there). The work below was not
done and belongs to this ticket, with the same pattern (each wait is the precondition of the next assertion, bounded, plus the render of the accepted revision where
the next assertion reads a field or hook) and the same stopping rule (stop where a suite gains under about 1 s). Counts are from 36's Outcome; re-count with `rg` first.

- **case** (`web/crates/case/src/mechanical_settings.rs`): 76 `rendered()` (60 ms) sites in `contextual_layer_tests` (pure DOM test pages, before `mod
  mounted_settlement_tests`; about 4.5 s; no shared render signal, so give each page a render counter as the keymap pilot's `Probe.renders` does);
  17 intermediate `rendered()` (40 ms) sites in the mounted module (about 0.7 s); 3 negative-only `settle_fields` sites (about 1.8 s):
  `preparation_from_an_unmounted_field_cannot_bind_a_replacement_field`, `catalogue_latest_preparation_keeps_identical_newer_text_while_pending`, and the second
  settle in `catalogue_latest_preparation_error_survives_older_failure_arriving_last`. They assert only absence; they need an idle signal or a sentinel edit
  queued behind the held ones, and `settle_fields` stays only until then.
- **pcb**: about 24 intermediate `rendered()` (30 ms) waits (12 in `pcb_wiring/queued_edit_tests.rs`, 12 in `pcb_physical_setup/tests.rs`; about 0.7 s) and 6
  `pcb_physical_setup/tests.rs` settle sites still on the fixed `settle` (lines near 343, 395, 419, 511, 518, 560; about 2 s). 343, 395 and 419 assert only
  absence or follow an `Event::Open`; 511, 518 and 560 (layout tests) were not read.
- **parts** (`web/crates/parts`): not started. About 84 `tick()` (30 ms) sites (`component_model_editor`, `module_attachment`, `module_profile_editor`,
  `mechanical_profile_ui` define `tick`; `parts.rs`, `parts_new_component.rs`, `parts_import_footprint.rs`, `generator_settings.rs` and `assembly_editor.rs`
  also hold 30 ms waits); suite 16.1 s, about 2.6 s of waits. Each site needs its own predicate.
- **PCB recovery case**: `failed_field_ticket_restores_accepted_value_and_keeps_feedback_across_another_edit`
  (`pcb_wiring/queued_edit_tests.rs`) injects an executor failure, which correctly moves the session
  into save recovery. Its following edit is therefore expected to end as
  `BlockedByRecovery("save recovery is required")`; it is not a landed-edit regression. The test
  now waits for and asserts that terminal outcome, verifies the accepted document remains unchanged,
  and confirms the inline failure remains visible. A landed unrelated-edit check needs a separate
  refusal fixture that does not put the runtime into recovery; this suite currently has no such
  Core response hook.
- **Residual negatives**: negative assertions that follow a positive wait (case held landing, presentation return, profile siblings; pcb `feedback.is_none()`
  after the net landing) still rely on effect-driven re-renders having run. A test-support idle signal (no dirty scopes, no held effects, no in-flight
  Core work) would close this; consider adding it to `test_wait.rs` before converting more suites.

Acceptance for this section: the list above is converted or each remaining site is recorded with its reason, per-suite before/after table for case, pcb
and parts, name diff empty, mutation checks as in the section above.

## Outcome

Implemented by Claude Haiku 5.5 (implementer), 2026-10-10, in this worktree, about 1 h 40 min
(claim 14:05, last run 14:50). **Partial scope.** Converted: library (all of it, test module only),
the layout outline file (16 of about 38 sites that have a predicate), and two page files
(`layout_relation_highlight_tests.rs`, `layout_layers_tests.rs`). Not converted: ui-shared, keymap,
keycaps, the case, pcb and parts carry-over from 36, and most of the page binary (list below). The
acceptance boxes above stay unchecked.

**Order.** The ticket asked for the page binary first. I took library first: its 16 tests held about
95 `settle()` calls at 80 ms and two 20 ms and 10 ms poll loops, about 8 s of fixed waiting in one
file. The page's remaining fixed waits are spread over eleven files (the largest is 47 sites at 40 ms),
and the page binary's wall is launch-dominated (see below), so it was not where the time went.

**Commits (mine).** claim `af9373112`; library `f339c7b00`; layout outline `8e26b1a8e`; page layers
and relation highlight `a1672ca91`. The branch also holds six commits by another session between 14:37
and 14:50 (`3e2d079e4`, `4dca82054`, `8ed2e3592`, `9b2be0d1e`, `d30774dad`, `ad30ef592`; docs under
`docs/agents` and tickets 39 to 41). I did not touch them; they changed no Rust.

**Before and after (`python3 scripts/check.py browser`).** Baseline is one run before any edit (14:05,
load 8.0 at start). Load at the start of each after-run is in the header. Values are the wasm-pack
`finished in` lines, except the page binary, which the runner reports only as a whole.

| Suite (tests) | Baseline | Run 1 (load 3.9) | Run 2 (load 5.1) | Run 3 (load 15.3) | State |
| --- | ---: | ---: | ---: | ---: | --- |
| library (16) | 11.00 s | 1.34 s | 1.87 s | 1.49 s | converted |
| layout (166) | 15.41 s | 13.09 s | 13.48 s | 15.35 s | outline file partly converted |
| layout setup_guide (1) | 0.37 s | 0.39 s | 0.38 s | 0.42 s | unchanged |
| page binary (70, runner total) | 47.2 s | 45.0 s | 48.4 s | 58.1 s | two files converted; launch-dominated |
| ui-shared (33) | 8.84 s | 8.87 s | 9.01 s | 8.86 s | not started |
| keymap (40) | 9.67 s | 9.67 s | 9.82 s | 9.76 s | not started |
| keycaps (30) | 2.54 s | 2.59 s | 2.67 s | 2.58 s | not started |
| case (81) | 15.68 s | 14.85 s | 15.79 s | 16.40 s | carried from 36, not started |
| pcb (49) | 19.48 s | 18.45 s | 18.31 s | 19.06 s | carried from 36, not started |
| parts (68) | 20.41 s | 16.16 s | 16.78 s | 17.31 s | carried from 36, not started (no source change) |
| `browser` step | 143.8 s | 137.9 s | 146.3 s | 167.0 s | |

The parts suite, which I did not touch, moved 4 s between the baseline and run 1, so differences
under about 3 s here are noise. The library drop (9.7 s) is far outside that. The layout drop (about
2 s) is measured against one baseline run only.

**Page binary.** The isolated `presentation::case_workspace::` launch runs two tests. Run directly,
those tests finished in 1.33 s, so its nine 100 ms settles are below the stopping rule, and the rest of
that launch is startup. I did not change `run-wasm-tests.py` (out of scope). The runner's total did not
fall measurably. I did not get per-launch `finished in` lines for the main page launch.

**Name diffs.** `wasm-pack test --lib -- --list` (sorted) before against after: library 16 = 16,
layout 167 = 167, page binary 70 = 70, all `diff` empty. The crates I did not touch have no source
change, so their lists are unchanged by construction; their test counts in the three runs match the
baseline (81, 49, 68, 40, 30, 33, 16 except library, as above).

**What changed.**
- library (`web/crates/library/src/library.rs`, test modules only): `settle()` is gone. `wait_for_saved_cards`,
  `wait_for_project_cards` and `wait_for_list_error` are `wait_until`; `wait_outcome` is `wait_for_step` with
  the shared bound and keeps its timeout diagnostics. The bare gate awaits (`entered.await`) became
  `drive_pending` plus `try_recv` inside `wait_until_step`, so a gate that never opens fails at the bound.
  Predicates are the next assertion's own condition (DOM, store or Core state). The list retry waits for
  its Loading status (the test host holds the response 120 ms), and the delete dialog waits for its `open`
  attribute before confirming.
- layout outline (`outline_lifecycle_browser_tests.rs`): mount and action waits, and the perimeter and
  canvas Escape waits, pointer-capture release waits, and the coordinate Escape wait. One new wait had to
  include more than its first assertion (see Verification).
- page relation highlight (`layout_relation_highlight_tests.rs`): all 16 settles, and the 60 ms helper, are
  gone. Each wait is the highlight count or class list, or the Relations reference button, that the next
  assertion reads.
- page layers (`layout_layers_tests.rs`): six 80 ms settles are gone. Each wait is the counts the next
  assertion reads.

**Not converted (listed so the next agent can start here).**
- Layout outline file: 22 `settle_dimension()` sites (35 ms) and 16 `accept(&probe)` drains (6 x 10 ms, no
  predicate). The shared `settle(runtime, ticket)` (100 x 10 ms, returns silently at its bound instead of
  failing) is used at 5 sites. One site (the coordinate accepted value) cannot be distinguished from the
  draft (both show `1.5`), so it stays a settle.
- Layout other files, not read: `keycap_size_tests.rs` (`rendered(ms)`), `setup_guide/tests.rs` (60 ms),
  `part_placement.rs` (16 and 20 ms), `board_inspector.rs` (50 ms and 10 ms loops), `layout_findings.rs`
  (40 ms x3), `layout_viewer.rs` (30 ms and 10 ms loop), `matrix_edit_test_support.rs` and the `*_pending`
  helpers (10 ms loops).
- Page binary (snapshot before the later conversions): remaining waits are `layout_component_inspector_tests.rs` (57 calls to
  `settle_component_inspector`, plus its pending-edit drains), `layout_key_selection_actions/tests.rs` (4 waits: two fixed post-action delays,
  one shared refresh delay, and the full Editor mount), and `case_workspace.rs` (9 x 100 ms in the isolated
  launch). Fixed waits
  are removed from `inspector.rs`, `zmk_firmware_export.rs`, `keycaps_workspace_tests.rs`,
  `setup_workspace.rs`, `keycaps_workspace.rs`, `parts_workspace.rs`, and `keymap_workspace.rs`.
- ui-shared: `pending_edit_helpers.rs` (26 `settle` calls, each 12 x (drain + 30 ms); 47 bare `rendered().await`),
  `geometry_scripts.rs` (10 ms loops), `canvas_navigation.rs` (20 ms x2), `panels_scroll_tests.rs` (80 ms, 40 ms).
- keymap: `macro_controller.rs` (40 ms `rendered`/`settle`, 10 sites), `macro_editor.rs` (40 ms, 4), `panel.rs` (40 ms).
- keycaps: `keycaps_settings.rs` (15 sites, 50/20/10 ms), `keycaps_navigation.rs` (40 ms), `keycaps_fit.rs` (50 ms x2),
  `keycaps_finding_marker_tests.rs` (40 ms).
- Carried from 36, not started: case `mechanical_settings.rs` (76 `rendered()` at 60 ms, 17 at 40 ms, 3 negative-only
  settles); pcb (about 24 intermediate waits and 6 settle sites); parts (about 84 `tick()` sites at 30 ms).
- Production code, left as is: `library.rs` line 608 has a 120 ms test-only hold that keeps the Loading status on
  screen for two tests (cfg(test) only); the focus helpers' 0 ms `spawn_local` yields.

## Progress (2026-10-14, ZMK export page waits)

Replaced the fixed mount delays in `zmk_firmware_export.rs` with bounded waits for the rendered
workspace headings and export rows, then replaced the failure/retry pipeline and unavailable-row
ticks with bounded waits for dispatched effects and visible status. The isolated ZMK export module
now has no fixed delays.
`python3 scripts/run-wasm-tests.py --files web/src/presentation/zmk_firmware_export.rs` passed all
5 tests; `cargo fmt --all --check` and `git diff --check` passed. Ticket acceptance remains open.

**Verification (what I observed).**
- Mutation checks, one edit each, one filtered run, then restore with `cmp` against a saved copy:
  - library, never-true `Escape restores the accepted name`: `wait_until: Escape restores the accepted name did not
    hold within 5000 ms`, 5.22 s.
  - library, always-true `the trimmed search leaves the current keyboard`: the next assertion fails (`left: 4,
    right: 1`), so the predicate is load-bearing.
  - library, production search filter (`cards.retain(...)`) removed: the same wait fails at the bound, named, 5.15 s.
  - layout, never-true `Escape restores the accepted X coordinate`: named failure, 5.27 s.
  - layout, always-true `Edit perimeter points renders the point editor`: the next `unwrap` panics on `None`.
  - layout, production `OutlineCoordinate` Escape reset (`outline/overlays.rs`) removed: the real predicate fails at
    the bound with its name, 5.19 s.
  - page, never-true `hovering Column 4 highlights its keys`: named failure at the bound.
  - page, always-true `leaving the reference clears its highlight`: the next assertion fails (`left: 2, right: 0`).
  - page, production `relation_highlighted` branch (`web/src/presentation.rs`) disabled: named failures in four tests.
- One real failure found and fixed while converting: in the perimeter Escape test the first wait covered only the
  selection, and the nested editor's unmount and the focus move come a render later. The first layout test run failed
  `the nested Perimeter inspector is no longer mounted after returning to Board` (outline line 508). The wait now
  includes the unmount and the focused field.
- Three consecutive `python3 scripts/check.py browser` runs: all steps passed each time (137.9 s, 146.3 s, 167.0 s;
  `run-wasm-tests` executed 70, failed 0 each). Then `typecheck` (20.8 s), `lint` (7.5 s) and `repo` (0.2 s): all
  passed. The browser logs show the same 108 warnings before and after, none in a changed file.
- Stale Chromium profiles: 98 older than 30 minutes removed; none older remain.

**Residual negatives (drain-protected only, not render-proven).** These assert absence after a drain or one
event-loop turn (`drain_and_yield` in library, `yield_turn` in layout, `run_pending` in page relation), with no
render signal:
- library: delete-cancel store check; the pending-rename "Still typing" draft after the older landing; the
  delayed persist's `slot.is_none()` after release (the released completion has no observable signal);
  the retained action's no-submit after unmount; the draft-survives check after an unrelated revision.
- layout outline: the blur-with-no-commit check (one `yield_turn`).
- page relation: the revision check after the highlight clears (`run_pending` only).
A test-support render or idle signal (ticket 36 follow-up) would make these render-proven.

**Unverified.**
- GitNexus `impact` and `detect_changes` were not run: the index is stale (last indexed ef45d94) and agents do not
  refresh it. The new helpers are test-module local; callers were checked with `rg`.
- The 5 s bound was not stressed on a slower machine. Each mutation ran once; the three browser runs are the only
  repetition.
- Clippy on the wasm target was not run for the web crates (the lint step is native). Running it on the runtime crate
  alone reports 6 `await_holding_refcell_ref` errors in `boardstudio-web-runtime`. I did not check whether they predate this change; no runtime file was touched.
- The page runner's total does not show a gain (see Page binary), so the page's effect is not demonstrated by the step wall.

## Verification

Independent review by **Codex, Sol 6.1 at xhigh**, 2026-10-10, in `.worktrees/tce-matrix`,
branch `typed-core-edits/mechanical`, starting at `2c9fcd6d6`. Runtime service tier was not exposed.
**The delivered slice passes after one small test-only correction. Full ticket acceptance remains open;
status stays `implemented (awaiting verification)` and both acceptance boxes remain unchecked.**

**Reviewed scope and correction.** Read the four converted files against `f7dc7b75b` (before ticket 37),
checking every converted predicate against the next assertion or action. Library gate waits are bounded,
outline waits cover their relevant inputs, controls and pointer capture, and the two page files wait on
rendered counts/classes. The residual negative assertions documented in the Outcome still have no idle or
render proof; this review does not close them.

The current-project deletion test (`web/crates/library/src/library.rs`, wait near line 1518) waited for a
replacement accepted ID and deletion from storage, then asserted the active-project ID and closed dialog.
The dialog closes in a later effect. A temporary **50 ms delay between `saved.set(remaining)` and
`pending_delete.set(None)`** reproduced the weak wait: the existing test failed at `!dialog.open()`
(test 0.18 s, command 11.34 s). The wait now also requires the active stored ID to equal the replacement
and the dialog to be closed. With the identical delay still in place, the test passed (0.20 s,
command 9.67 s). The delay was then removed; no production change remains.

**Executed names, independently reproduced.** Ran `wasm-pack test --headless --chrome ... --locked ...
-- --list` with the touched sources from `f7dc7b75b`, then with the current sources plus the correction.
Each replacement and restoration used this same checkout; restored bytes were compared with saved copies
and sources touched. Sorted complete lists are byte-identical:

| Suite | Baseline names | Current names | Diff |
| --- | ---: | ---: | --- |
| library (`web/crates/library --lib`) | 16 | 16 | empty |
| layout (`web/crates/layout --lib`, including setup guide) | 167 | 167 | empty |
| page (`web --no-default-features --features page --bin boardstudio-web`) | 70 | 70 | empty |

The four source files' test declarations also match. Library assert macros fell from 123 to 113 and
relation-highlight macros from 18 to 16 in the implementation: the removed checks are enforced by
bounded waits for the same conditions. Outline (114) and layers (10) assert counts are unchanged.

**Mutation checks, independently reproduced.** One site at a time, one filtered test per run, then
exact source restoration and touch in this checkout. Each failure printed `wait_until: <named wait>
did not hold within 5000 ms`; none reached the runner's 120 s timeout. Times below are test / command wall.

| Representative wait | Never-true predicate | Production dependency disabled |
| --- | --- | --- |
| library, `the rename reaches the gated Core reply` | 5.10 / 15.28 s | `ProjectNameCommitAction::commit` returned early: 5.11 / 14.62 s |
| outline, `Enter commits 5.25` | 5.07 / 19.67 s | `OutlineDimension`'s commit callback suppressed: 5.06 / 18.55 s |
| page, `keyboard focus highlights Column 4's keys` | 5.63 / 25.19 s | production relation-highlight class branch suppressed: 5.70 / 24.89 s |

The tests were `pending_project_rename_preserves_typing_and_escape_does_not_commit`,
`outline_dimension_enter_commits_and_escape_restores_accepted_value`, and
`keyboard_focus_highlights_and_blur_clears`. Full passing runs below recompiled and exercised the restored
sources. All temporary mutations, including `outline/overlays.rs` and `web/src/presentation.rs`, were
byte-compared after restoration.

**Checks and repetition.** `python3 scripts/check.py repo lint typecheck` passed in 0.1 / 31.9 / 38.3 s.
Then two consecutive full `python3 scripts/check.py browser` runs passed, sequentially in the foreground.
`BOARDSTUDIO_CHECK_JOBS=1` enforced serial crate launches; Cargo used the configured 12 jobs
(`CARGO_BUILD_JOBS` unset). No other lane was allowed heavy checks during these runs. The full-step
walls below therefore are **not comparable** with the implementer's default parallel launch walls.

| Browser suite (passed count) | Verifier run 1 | Verifier run 2 |
| --- | ---: | ---: |
| host storage (10; 8 filtered) | 0.45 s | 0.45 s |
| runtime (29) | 1.34 s | 1.33 s |
| ui-model (5) | 0.01 s | 0.01 s |
| ui-shared (33) | 8.82 s | 8.82 s |
| ui-shared panels (1) | 0.13 s | 0.13 s |
| keycaps (30) | 2.54 s | 2.55 s |
| library (16) | 1.02 s | 1.01 s |
| keymap (40) | 9.67 s | 9.69 s |
| catalogue (12) | 0.25 s | 0.24 s |
| case (81) | 14.67 s | 14.87 s |
| parts (68) | 16.18 s | 16.23 s |
| pcb (49) | 18.59 s | 18.24 s |
| layout (166) | 12.93 s | 12.99 s |
| layout setup guide (1) | 0.37 s | 0.37 s |
| page runner (70, whole launch wall) | 43.8 s | 43.5 s |
| full browser step | 251.3 s | 248.3 s |

Both runs passed the CAD browser gate and reported page `executed 70, failed 0`. `uptime` immediately
before run 1: 15:25:24, load 3.87 / 7.58 / 12.56; before run 2: 15:29:49, load 4.37 / 5.87 / 10.59.
Against the implementation's single pre-edit baseline (library 11.00 s, layout 15.41 s, page launch
47.2 s), these runs reproduce the substantial library saving; smaller layout/page differences remain
indicative because load and launch policy differ. The implementer's three consecutive passing runs
(137.9 / 146.3 / 167.0 s) are retained as historical repetition evidence. There are **two**, not three,
fresh full runs after this verifier correction; a third post-correction run was not requested.
A final `repo` check after this verification note also passed (0.1 s).

**Remaining acceptance gates.** The unconverted scope in the Outcome remains outstanding: ui-shared,
keymap helpers, keycaps, most layout/page files, and the case/pcb/parts work carried from 36. The original
per-suite conversion or justified stopping rule has not been met for that scope. The pcb unrelated-edit
recovery defect and residual negative assertions carried from 36 also remain open. They need their
listed implementation and mutation plans, not a resolution based on this slice passing. In particular,
no idle signal was added, no slower-machine bound validation was run, no wasm Clippy gate was added,
and the page's speed gain is not established by comparable measurements.

**Graph and hygiene.** Bound GitNexus calls used `boardstudio-tce-matrix`; its index remains at
`011f39a1c`, 37 commits behind the reviewed HEAD. `context`, compare change analysis, and pre-edit
`impact` were run. Test entry points and `Library`, `OutlineDimension`, and `Editor` returned UNKNOWN
with no resolved callers; source searches confirmed wasm-test registration, component uses and handlers.
`ProjectNameCommitAction::commit` returned LOW with one retained-action test caller and one missing
receiver-typed call; the production blur handler was confirmed by source search. File impact was MEDIUM
for library; ticket-document impact was HIGH (5 direct / 126 total document dependents, no processes),
reported before appending this evidence. These answers are provisional because the index is stale;
no refresh was performed. Final bound `detect_changes(scope="all")` reported four changed symbols across
the two intended files, zero affected processes and LOW risk, without partial/truncated flags. Its zero
process result remains provisional under this stale index; the test is browser-registered and its
production dependencies were exercised by the probes above. Removed 62 stale Chromium profiles older
than 30 minutes after the runs,
checking live process arguments and lock PIDs first; no active profile was removed.

Raw logs and the three empty name diffs are retained locally under
`/home/chris/.cache/boardstudio-verification/ticket37-20261010-codex/`.

## Incremental follow-up (2026-10-14)

Replaced the 60 ms settle after the local-X edit in
`objects::matrix_transform_inspector::mounted_tests::toggle_removal_keeps_the_inspector_and_commands_aimed_at_remaining_keys`
with `wait_until` on the accepted cell value. Commit `9e4cd640`. The focused mounted test passed
(1 passed); `cargo fmt --all --check` and `git diff --check` passed. A full `python3 scripts/check.py
browser` rerun passed all browser steps, including 89 page mounted tests. This is one converted wait;
the broader acceptance items above remain open (including the three-run and mutation requirements).

Converted four additional 60 ms settles in the same mounted test: disable, re-enable, Undo and Redo
now wait for the matching accepted enabled state. Commit `ce5c397c`; the exact mounted test passed
again (1 passed), with format and diff checks clean. Five settle waits in this test are now replaced
by state waits across the two commits; the rest of #37 remains open.

Converted `macro_controller.rs`'s 15-cycle settle helper to bounded state waits for accepted values,
Undo, and inline failures. The keymap WASM suite passed all 40 tests (`wasm-pack test --headless
--chrome web/crates/keymap --lib`, 1.95 s reported by the test harness); test declarations are
unchanged. `cargo fmt --all --check` and `git diff --check` passed. The remaining direct 40 ms
render yields in this file are under the ticket's per-suite one-second stopping threshold; other
keymap helper files and the broader #37 acceptance gates remain open.

Converted all 17 `pending_edit_helpers.rs` settlement sites from 12 fixed 30 ms drains to bounded
waits on their observed Runtime operation outcomes. The ui-shared WASM suite passed 34/34 in 2.90 s
with `wasm-pack test --headless --chrome web/crates/ui-shared --lib -- --nocapture`; this is not a
comparable before/after measurement against the older full-browser run. Formatting and diff checks
passed. The file's remaining bare render yields and the other #37 suites remain open.

Three consecutive full `python3 scripts/check.py browser` runs passed on the current tree. The latest
two each completed in 118.8 s; each included 15 CAD browser checks and 96 page WASM tests with zero
failures. This satisfies the repetition count only; the remaining conversion, mutation, baseline,
and acceptance work above is still open.

## Progress (2026-10-14, keymap workspace state)

The keymap workspace state test now waits for its controls and selected-layer label instead of
sleeping. The filtered wasm test passed (1/1), with `cargo fmt --all --check` and `git diff --check`
passing. Remaining #37 suites and acceptance evidence stay open.

## Progress (2026-10-14, setup workspace state)

The setup workspace state test now waits for its controls, stage update, and dismissal state rather
than using three fixed delays. Its filtered wasm test passed (1/1), with `cargo fmt --all --check`
and `git diff --check` passing. Remaining #37 suites and acceptance evidence stay open.

## Progress (2026-10-14, keycaps workspace state)

The keycaps workspace state test now waits for its control and both navigation identity updates
instead of sleeping. The filtered wasm modules passed (3/3), with `cargo fmt --all --check` and
`git diff --check` passing. Remaining #37 suites and acceptance evidence stay open.

## Progress (2026-10-14, parts workspace state)

The mounted module selection test now waits for its selector and the accepted module, generation,
and preview activation values instead of sleeping. Its filtered wasm test passed (1/1), with
formatting and diff checks clean. Remaining #37 suites and acceptance evidence stay open.

## Progress (2026-10-14, Layout workspace waits)

The workspace-state test and origin-pick/relationship Escape regressions now wait for their
rendered state changes instead of sleeping. All three filtered Layout workspace wasm modules passed
(3 tests), with format and diff checks clean. No fixed delays remain in this file.

## Progress (2026-10-14, Keycaps Inspector state)

Converted the Keycaps Inspector's fixed settle calls to bounded waits on its assigned count,
selected-key disclosure, and selection updates. The filtered wasm tests passed (2/2), with format
and diff checks clean. Remaining #37 suites and acceptance evidence stay open.

## Progress (2026-10-14, Layout edit settlement)

Replaced `layout_remainder_tests.rs`'s 100-iteration polling loop, which could silently return while an
edit was still pending, with the shared bounded `wait_until_step` helper. All 10 tests in that module
passed; formatting and diff checks are clean. Remaining #37 suites and acceptance evidence stay open.

## Progress (2026-10-14, cross-matrix key actions)

Replaced the mounted action toolbar's initial 40 ms render delay with a bounded wait for the
selected-key action control, and the post-disable delay with a wait for Enable to render. The
filtered action module passed 7/7 tests. Two later 40 ms waits remain around Enable/Delete acceptance:
replacing Delete's delay with direct accepted-state polling timed out, so the correct completion
signal still needs investigation. Formatting and diff checks pass; #37 remains open.

## Progress (2026-10-14, position Inspector waits)

Converted the mounted position Inspector's mount, pending, landing, failed-save, deselection, and
reselection waits to bounded DOM/Core state predicates. All 13 filtered Inspector modules passed;
formatting and diff checks are clean. No fixed delays remain in this file's position draft tests.

## Progress (2026-10-14, full gate on current tree)

`python3 scripts/check.py` passed all steps in 203.8 s (repo 0.4, tooling 3.8, lint 3.7, build 49.4,
test 29.1, browser 117.3); all 97 page WASM tests passed. This is one current-tree full browser
pass. It does not complete the required three consecutive browser runs or the remaining wait audit,
mutation checks, and before/after suite evidence.

## Progress (2026-10-14, post-ZMK full gate)

`python3 scripts/check.py` passed all steps in 206.6 s (repo 0.4, tooling 3.8, lint 3.6, build 48.8,
test 28.7, browser 121.2); all 97 page WASM tests passed. This is one current-tree full browser
pass. The remaining wait audit, mutation checks, three-run requirement, and baseline evidence remain.

## Progress (2026-10-14, remaining Layout remainder waits)

Replaced the last four fixed waits in `layout_remainder_tests.rs`: mounted input waits now check the
Inspector, and queued/Apply edits wait for the accepted positions they must produce. All 10 tests in
the module passed with formatting and diff checks clean. This file has no fixed sleeps remaining.

## Progress (2026-10-14, current Layout wait conversions)

`python3 scripts/check.py` passed all steps in 201.6 s (repo 0.4, tooling 3.8, lint 3.7, build 49.5,
test 27.9, browser 116.2); all 97 page WASM tests passed. This is one current-tree full browser
pass. The remaining wait audit, mutation checks, three-run requirement, and baseline evidence remain.


## Progress (2026-10-14, mirrored-half disclosure test)

Replaced the two fixed waits in `mounted_mirrored_half_disclosure_dispatches_the_selected_one_shot_action`
with bounded checks for the mounted component Inspector and linked disclosure. The file's focused wasm
modules passed 40/40 tests; formatting and diff checks passed. Remaining Inspector test waits stay open.

## Progress (2026-10-14, component Inspector waits)

Replaced the remaining 56 fixed Inspector settles and the 1 ms queued-edit polling loop in
`layout_component_inspector_tests.rs` with bounded render-generation and accepted-outcome waits. The
matrix-relations fixture now waits for its host to mount before driving the same render-generation
gate. The focused file run passed all 40 tests; `cargo fmt --all -- --check`, `git diff --check`,
and a search for fixed `TimeoutFuture`/settle calls in the file passed. The broader #37 wait audit,
mutation checks, three consecutive browser runs, and before/after suite evidence remain outstanding.

`python3 scripts/check.py` then passed all steps in **195.1 s** (repo 0.4, tooling 3.8, lint 3.5,
build 46.4, test 28.8, browser 112.3); all 97 page WASM tests passed. This is one full browser
pass after the Inspector conversion, not the required three consecutive runs.

## Progress (2026-10-14, shared pending-edit helper waits)

Replaced the mounted pending-edit helper's 30 ms render delays with a bounded wait on a dedicated
render-generation signal. The probe is present in all four fixture hosts, including typed drafts,
retention, and unmount/remount cases. `wasm-pack test --headless --chrome web/crates/ui-shared --locked
--lib` passed all 34 tests. The geometry-script, canvas-navigation, and scroll waits in ui-shared
remain under audit; the full gate and acceptance evidence have not yet been rerun for this change.

The geometry-script waits are also converted: editor renders use the host's requested render
generation, and ticket/Core progress uses bounded state waits instead of 10 ms loops. The same
ui-shared command passed all 34 tests in 1.13 s. The four remaining canvas-navigation and panel-scroll
delays account for less than one second in this suite, below the ticket's per-suite stopping threshold;
they are intentionally retained. A full repository gate has not yet been rerun after these changes.

`python3 scripts/check.py browser` passed after the ui-shared changes in **134.5 s**; the page
runner executed all 97 selected tests with no failures. This is the first full browser pass after
these changes. The recorded 195.1 s `check.py` gate predates the ui-shared conversion, so #22's full
gate must be refreshed before closing.

`python3 scripts/check.py` passed all steps after the conversion in **207.3 s** (repo 0.4, tooling
4.0, lint 4.6, build 54.3, test 31.7, browser 112.3); the page runner again executed all 97 tests.
The full gate is current, but the remaining above-threshold case/pcb/parts waits, mutation checks,
name-diff evidence, and two more consecutive browser passes remain.

## Progress (2026-10-14, selected-key action waits)

In `layout_key_selection_actions/tests.rs`, the Editor-mount delay now waits for the selection
adapter, and the shared refresh helper waits for its render generation. The Enable action waits for
both accepted matrix cells, then confirms the Inspector render before the next action. That filtered
accepted-state predicate timed out for Delete, so its 40 ms settle remains with that observed reason.
The focused module passed 7/7 tests; formatting and diff checks passed. No full repository gate has
been run after this change.

## Progress (2026-10-14, Case contextual layer waits)

Replaced the 76 contextual-layer fixture render delays (60 ms each) with a bounded wait on an
explicit render-generation signal. All 12 contextual-layer pages mount through the probe host.
`wasm-pack test --headless --chrome web/crates/case --locked --lib` passed all 81 tests; the
case test list remains the same size. The 17 waits in `mounted_settlement_tests` total under one
second and remain below the suite cutoff. The three negative-only `settle_fields` waits remain
because they need an idle signal or sentinel edit. Formatting and diff checks pass; no full gate
has been run after this change.

## Progress (2026-10-14, Parts new-component settlement)

Replaced the four 30 ms delays in `parts_new_component::mounted_tests::create_selects_only_after_the_exact_ticket_lands` with bounded waits for the action mount, Core entry, and accepted selection. The focused test passed, and `wasm-pack test --headless --chrome web/crates/parts --locked --lib` passed all 70 tests in 16.44 s with unchanged test names. This is a focused cleanup only; the remaining Parts and PCB waits, mutation checks, name-list comparison, and browser-run acceptance gates remain open.

## Progress (2026-10-14, Parts import and creation waits)

The mounted Parts import-selection test now waits for control mount, persistence entry, and the accepted selection instead of fixed sleeps and a 100-iteration timer loop. Together with the new-component selection conversion, the full Parts WASM suite passed **70/70** in **16.30 s**; the focused import and creation tests also passed. The suite's test names and count are unchanged. Remaining Parts waits and the ticket-wide mutation, name-diff and repeated-browser gates are still open.

## Progress (2026-10-14, consecutive current-tree browser gates)

After the Parts new-component and footprint-import wait conversions, three consecutive `python3 scripts/check.py browser` runs passed: **130.1 s**, **110.6 s**, and **111.0 s**. Each included 15 CAD browser checks and 97 selected presentation/case tests with zero failures. Raw logs for runs 2 and 3 are in ignored `.evidence/ticket37-browser/`; run 1's aggregate result was captured directly in the terminal. This meets the repeated-browser gate for the current tree, but not #37's remaining conversion, mutation, or test-name audit requirements.


## Progress (2026-10-14, current full repository gate)

After the Parts wait changes and the three consecutive browser passes, `python3 scripts/check.py` passed all steps in **202.4 s** (repo 0.4, tooling 3.8, lint 4.2, build 53.9, test 29.2, browser 110.8). The native suite ran 1,976 baselined tests plus 6 additions; the browser step passed. The wait-site audit, remaining high-gain Parts/PCB conversions, mutation checks, and test-name diff are not complete, so #37 remains open.

## Progress (2026-10-14, matrix transform Inspector waits)

Replaced all 50 fixed 60 ms settles in `matrix_transform_inspector_tests.rs` with a bounded wait
that requests a render through the test host's existing version signal and observes its render
counter. The 17 mounted matrix-transform tests passed. The configured Layout browser checks also
passed: 169 tests with `setup_guide` skipped, then the isolated desktop setup-guide test (1 test).
An unsplit crate run failed only the setup-guide viewport-composition test, which the repository
already runs separately; the configured split passed both portions. Formatting and diff checks
passed. Remaining #37 wait audit, mutation checks, test-name comparison, and three current-tree
browser gates are still open.

## Progress (2026-10-14, Parts placement chooser waits)

Replaced the Parts Inspector's 50 ms mount delay and the Add Object chooser's 100 × 10 ms polling
loop with bounded waits for their actual controls. Removed four 20 ms post-click delays: the
callbacks update the test probes synchronously. The Parts browser crate passed all 70 tests in
16.11 s with test names and count unchanged; formatting passed. The remaining wait audit, mutation
checks, baseline name comparison, and three repeated browser gates remain open.

## Progress (2026-10-14, bounded-wait mutations)

Ran three representative wait mutations with `scripts/mutation-check.py`: the Layout render wait,
Parts catalogue-loading wait, and Parts placement-action mount wait. Each never-true predicate
failed with its named `wait_until` message at the 5,000 ms bound, and the mutation tool confirmed
all three files were restored byte-for-byte. Logs and hash results are in ignored
`.evidence/ticket37-mutation-results/`. This verifies timeout behavior; production-action removal
mutations and the rest of #37 acceptance remain open.

## Progress (2026-10-14, Parts catalogue and browse waits)

Converted the Parts search/browse UI and catalogue-state tests from fixed 10/30 ms sleeps and
100-iteration polling loops to bounded checks for the search draft, browse state, loading text, and
error alert. The Parts browser crate again passed all 70 tests (16.07 s) with unchanged test names
and count; formatting passed. The remaining broad Parts audit, mutation checks, baseline name
comparison, and three repeated browser gates remain open.

## Progress (2026-10-14, PCB physical setup waits)

Replaced the six 12-round fixed-settle loops in `pcb_physical_setup/tests.rs` with bounded
predicates for silent retirement, project replacement, accepted reversible layout/topology, and
Undo restoration. The topology waits also require the updated Inspector projection to render idle
before the next action, preventing a stale mount from dropping that action. The focused split/Undo
test passed, then all 49 PCB browser tests passed. Formatting passed. Other low-gain PCB render
settles and the cross-ticket mutation, name-list, repeated-gate and full-gate evidence remain open.

## Progress (2026-10-14, Parts mechanical-profile waits)

The mounted mechanical-profile tests now wait for the entry/editor controls, Core gate entry,
save landing, persistence error, and silent owner retirement. Their 30 ms ticks, 100-round Core
poll, and eight-round drain loop are removed. The three focused mounted tests passed, and the full
Parts suite passed 70/70 in 14.96 s before the final one-line oneshot-result unwrap; those same
three tests passed again after that final edit. The broader Parts wait audit, production-action
mutations, executed-name comparison, and current-tree repeated/full gates remain open.

The production Save fit profile action was then renamed temporarily with
`scripts/mutation-check.py`; the mounted editor test failed at its named 5,000 ms wait, and the
tool restored the production file byte-for-byte (`restored_exact: true`). Evidence is in ignored
`.evidence/ticket37-mutation-results/parts-mechanical-save-action-missing.log` and `results.json`.
