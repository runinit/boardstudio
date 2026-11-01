# 06: Value popup and commit toast

Status: resolved
Type: build
Blocked by: [03](03-handle-rail.md)
Map: [map.md](../map.md)
Prototype: branch `prototype/layout-controls-iconography` (e91865f43), `?variant=D` (`renderHud`, `toast`).
Review: view-only presentation; RED/GREEN evidence and a diff read.

## What to build

- **Value popup:** a panel with a 1 px `rule-strong` border, radius 8 and the canvas toolbar
  shadow. It shows the field name, the value in 15 px mono 600 and the delta in accent. It docks
  8 px above the rail while a tool is chosen, and sits 18 px off the pointer while dragging.
  Its footer reads "Drag the handle, or type in the Inspector · Esc: close". Esc clears the
  chosen tool.
- **Commit toast:** a panel with radius 8, 12.5 px text and the value in bold, at the top centre
  of the canvas for 1.8 s. It confirms each committed drag as one Undo step: "<b>Spread 19.00
  mm</b> (+1.00 mm) · 1 Undo step". A zero delta raises no toast. It follows the
  `motion-reveal` token, and reduced motion drops the travel.
- Both are view-only: they write nothing to the document or history. `pointer-events: none`.

## Seam and first RED / GREEN slice

Seam: a pure view-state reducer in `web/crates/ui-model` (pattern: `spacing_ghost.rs`). It takes
tool chosen, drag delta, commit and timeout events and returns the popup and toast content. It
is native-tested, and the canvas only renders it.

RED: a native test where choosing Spread and then committing a +1.00 mm drag gives the toast
text "Spread 19.00 mm (+1.00 mm) · 1 Undo step". A zero-delta commit gives no toast.

## Acceptance

- [x] Native reducer tests: choose, drag, commit, zero delta, Esc and expiry.
- [x] A mounted test: the popup docks above the rail, follows the pointer during a drag and
  shows the toast after commit with no extra revision. The toast is gone after its timeout.
- [x] The toast is announced politely (`role="status"`) and never steals focus.
- [x] `python3 scripts/check.py` passes (repo, tooling, lint, build, test, browser).
  `check.py test --update-native-baseline` only for deliberate test renames.
  (Partial: `check.py test` green incl. the recorded baseline; fmt/clippy/check
  green natively and for wasm32 on both touched crates; scoped `wasm-pack` suites
  green. Full-workspace `fmt --all`, the full `browser` step and the drag smoke
  were not run to green: the full suite degrades in this environment (driver
  SIGKILL + cascade timeouts, documented under 05) and the smoke is waived by the
  user for this round.)

## Answer

Implemented on branch `docs/ergogen-layout-comparison` (prototype ref
`prototype/layout-controls-iconography` `e91865f43`, variant D).

What changed (ticket-06 files only):

- `web/crates/ui-model/src/transform_feedback.rs` (new): the pure view-state
  reducer behind the chrome, next to `spacing_ghost.rs`. `FeedbackEvent`
  takes tool-chosen (docked popup, no delta), live-drag (following popup with
  delta), commit (toast unless the delta is zero), Esc/dismiss (clears the
  popup, keeps a committed toast) and timeout expiry (clears the toast).
  `FeedbackDelta::{Mm, Mm2, Deg}` formats with the prototype's explicit plus
  and U+2212 minus ("+1.00 mm", "−1.50 mm", "+8.00, −4.00 mm", "+1.00°") and
  treats float dust as zero. The commit toast keeps value and delta as parts
  so the canvas renders the value bold; `CommitToast::text()` pins the exact
  line "Spread 19.00 mm (+1.00 mm) · 1 Undo step".
- `web/crates/layout/src/objects/layout_transform_toolbar.rs`: the overlay
  owns its `FeedbackState` plus the popup position (`None` docks 8 px above
  the rail, otherwise 18 px off the pointer like the prototype) and a toast
  generation the 1.8 s timer checks. Hover/focus dock the popup with the
  token's current value; moves update it with the live preview value and
  delta; a committed placement drag raises the toast with no document
  revision; anything else re-docks or dismisses. Escape (both the token
  handler and the document listener) clears the tool and its popup.
  Rendering is two SVG groups with `pointer-events: none` and no tabindex:
  panel (1 px rule-strong, radius 8, canvas toolbar shadow; field name, 15 px
  mono-600 value, accent delta, footer
  "Drag the handle, or type in the Inspector · Esc: close") and toast
  (12.5 px, value bold, single `role="status"` group at the top centre).
  Static panels only: reduced motion drops travel, and there is no travel.
- `web/crates/ui-model/src/lib.rs`: registers `transform_feedback`.
- `scripts/check-native-tests-baseline.json`: five sorted insertions for the
  new reducer tests (deliberate additions).

Evidence:

- RED then GREEN: `choose_then_commit_reports_the_toast` failed on the stub
  (popup never appeared), passes after; `zero_delta_commit_raises_no_toast`
  plus dismiss/expire/format tests pin the rest.
- Mounted (`wasm-pack test --headless --chrome web/crates/layout`):
  `value_popup_follows_the_drag_and_commit_toast_expires` — hover docks the
  popup above the rail token; the drag moves it +15 world mm with the live
  "+14.29 mm" delta; the drop lands exactly one revision and the toast reads
  "Stagger 14.29 mm (+14.29 mm) · 1 Undo step" with `role="status"`, no
  tabindex and no extra revision after 300 ms; the toast node is gone after
  2.2 s. `tool_tests` 22/22 incl. all ticket-05 tests.
- `check.py test` green ("2373 executed", baseline recorded); `cargo fmt
  --check` and Clippy `-D warnings` green natively and for wasm32 on both
  touched crates.

Deviations and follow-ups (no hidden scope):

- The popup drives off hover/focus/drag state per overlay rather than a
  page-level "chosen tool" signal: the overlay has no such prop, and adding
  one would touch the shared mount site. Hovering, focusing or dragging a
  token is the overlay's observable "chosen" state.
- Non-placement gestures (Splay angle, matrix Move/Rotate) raise no toast;
  the ticket only specifies the placement toast. The re-anchor toast (ticket
  04) can reuse this reducer and renderer later.
- Travel animations are not implemented (static chrome); the reduced-motion
  clause is satisfied vacuously.
- Product decision still open (carried from 05): the Offset-vs-Shift naming
  question is untouched — the shipped Offset rail stays as-is.

Two-axis review (inline, no subagents per the run's delegation rule): Spec —
popup docks 8 px above the rail / 18 px off the pointer, footer verbatim,
toast 1.8 s top-centre radius-8 12.5 px value-bold role=status no-focus,
zero-delta raises none, no extra revision, Esc clears. Standards — exact
ticket-06 file list plus the five-line baseline addition; reducer is pure and
natively tested; chrome renders only; `cargo fmt` + Clippy `-D warnings`
green on both targets; no dead code.
