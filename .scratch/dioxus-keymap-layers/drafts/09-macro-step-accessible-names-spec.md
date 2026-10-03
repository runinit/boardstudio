# Source specification: F6K.3a macro step accessible names

## Problem and intended result

The saved Keymap macro editor exposes generated internal macro IDs in step-control accessible names even though the user sees a friendly macro name. On the same `Macro 1` fixture, React exposes the step-kind select as `Macro 1 step 1`; Dioxus exposes `keymap-macro-11-131081 step 1`. The keycode field also leaks the internal ID in Dioxus. Make the controls identify themselves using the same user-facing labels as the pinned React editor, while preserving stable IDs for editing and operation correlation.

## Source and evidence

- Pinned React source: commit `5a472a9426e6e38993361da402cd4ec730feb369`, `app/src/ui/KeymapPanel.tsx`, `MacroEditor` lines around 49–58. The step-kind select has `aria-label={`${macro.name} step ${index + 1}`}`; ordinary keycode and wait fields are associated with visible `Keycode` and `Delay (ms)` labels.
- Browser candidate actually tested: `frontend-command-icons-reuse-20261002`, source `8cfd6bb79e9e10b788e007fd428145b1e37095d1` at port 34733. Its `web/src/presentation/keymap/macro_editor.rs` is byte-identical to that file at the later full candidate `f261a327858f51a1de4928374bc65b669e2792a3` (SHA-256 `a44edc316d3c737afc8493a3c74d4be328fe5f03b96701ae11a8fdfc06bae2d2`). `MacroCard` has the accepted macro display name and stable macro ID; `StepEditor` currently derives accessible text from the ID. The full `f261…` candidate was served separately at port 34734 and was not used for the retained browser snapshots.
- Paired browser evidence: `.scratch/dioxus-frontend-v1/evidence/keymap-parity-audit-20261002/RESULTS.md` and the paired snapshots/screenshots in that directory. Both fresh profiles imported the identical layered Sofle archive (SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`).

## Bounded behavior

For every macro step, use accepted display text for accessibility:

- Step-kind control: `<accepted macro name> step <1-based index>`, matching React.
- Non-wait keycode input: `Keycode`, supplied by its visible label as in React.
- Wait duration input: `Delay (ms)`, supplied by its visible label as in React.

These labels must not expose a generated macro ID or other storage identity. They update after an accepted macro rename and after step insertion/removal changes the visible one-based index. Draft text before acceptance does not rename the accessible control. Keep the stable macro ID, step target, scope, token/revision, edit intent and async feedback identity unchanged.

This is an accessibility-label correction only. It does not change macro defaults, visible copy, step semantics, validation, history, keymap references, export behavior, macro identity, or the F6K.3 acceptance boundary. No public API/schema, Core operation, new control, general key behavior editor, or task-graph edge is introduced.

## Verification

Use the same imported layered Sofle archive in fresh React and Dioxus profiles. Compare accessible snapshots for a new `Macro 1` with a default tap-A step, after accepted rename, after adding/removing a step, and after changing a step to wait and back. Assert exact step-kind labels, exact field labels, stable visible editor behavior, and absence of generated-ID text in accessible names. Confirm normal edit intents still address the original stable macro ID and target step. Run affected native/WASM checks, formatting, and the paired browser route on the changed source.

F6K.3 remains open for its existing broader acceptance and joins; this child does not close F6K.3, F6K.1, F6K.2, F3.1, INT.2 or RF-009 reconciliation.
