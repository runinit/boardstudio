# F6C.2 specification — Keycaps Inspector summary

## Problem Statement

The pinned React Keycaps Inspector identifies the workbench with a `Keycaps` heading and shows the number of assigned keys out of the physical key count. Dioxus has the accepted projection data for that count, but the Inspector omits the summary. This removes useful scope and completion information from the workbench and makes the right pane visibly different from TypeScript.

## Solution

Render the reference Keycaps title and assigned-key summary at the start of the Keycaps Inspector, sourced directly from the active board's accepted `KeycapsView`. Keep it a read-only projection: do not add saved fields, duplicate state, or another key-binding edit owner.

## User Stories

1. As a keyboard designer, I want to see the `Keycaps` heading in the Inspector so that I can tell which workbench's settings I am viewing.
2. As a keyboard designer, I want to see assigned keys divided by supported physical keys so that I can quickly assess the active board's keymap coverage.
3. As a keyboard designer, I want the count to use the same assignment definition as TypeScript so that transparent bindings count as assigned while unassigned bindings do not.
4. As a keyboard designer, I want the denominator to match the physical Keycaps projection so that unsupported objects are not mistaken for editable keycaps.
5. As a keyboard designer, I want the summary to follow the active board so that switching boards cannot display another board's assignments.
6. As a keyboard designer, I want the count to refresh from accepted keymap changes, Undo, and Redo so that the status never describes an unaccepted draft or stale projection.
7. As a keyboard designer, I want the count to survive save and reopen because it is derived from the same saved key bindings, not a separate counter.
8. As a keyboard designer, I want the Keycaps editor and its existing empty/unavailable state to remain understandable when the board has no supported physical keys.

## Implementation Decisions

- Reuse the existing accepted board-scoped `KeycapsView` and its `assigned_count` / `keys` projection; do not compute a second count from selection, canvas, or panel-local state.
- Match the pinned React title and summary wording/placement: `Keycaps` and `<assigned>/<total> assigned` at the top of the contextual Inspector.
- Preserve TypeScript assignment semantics: count every projected key whose effective binding is not `&none`; a transparent binding remains assigned. The denominator is the number of supported keys in the active board's Keycaps view.
- Read the value from the currently accepted snapshot. The Keycaps panel must update when the existing accepted Keymap binding path, Undo/Redo, or board scope changes; merely selecting a key does not alter the count.
- Add no new edit operation, persisted field, public API, projection owner, global shell behavior, or layout control.

## Testing Decisions

- Exercise the public workbench against the pinned React source and Dioxus candidate on the same retained project and active board.
- Compare the exact heading/count before and after an existing binding edit, then Undo, Redo, board switch, and save/reopen. Include an unassigned binding and a transparent assignment to verify the counting rule.
- Use a mounted Inspector/workbench regression that observes the displayed count following accepted projection updates, not a unit test of the string-formatting helper alone.
- Capture the source/build and fixture hashes, project/board identity, count values, and screenshots at the same viewport. Existing Keycaps projection and keymap edit/history tests remain supporting coverage.

## Out of Scope

- Editing key bindings, legends, colors, profiles, sockets, or sizes; these retain their existing workbench owners and tickets.
- Changing the set of supported physical keys or the keycap resolver.
- Reworking the shared Objects tree or global Inspector shell.
- Keycaps finding navigation, 3D preview, STEP export, or any F6 parent acceptance join.

## Further Notes

The current Keycaps projection already calculates an active-board `assigned_count`; the missing behavior is its visible Inspector composition. Carry the existing RF-001 contextual Inspector finding. This child does not complete F6C.2, F6C.4, or any parent join.
