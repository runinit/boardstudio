# Keymap paired visual refresh

Reference: TypeScript app at `http://127.0.0.1:5175/`, reference source `5a472a9426e6e38993361da402cd4ec730feb369`.
Candidate: Dioxus build at `http://127.0.0.1:34695/`, source `fc5e17e3526afe8a2c55efe9afdbc0f0faff3efa`, build `frontend-case-keycaps-pcb-reviewgreen-20261002`.

Both sessions started the Sofle v2 demo in fresh isolated browser profiles, opened Keymap, left the Base layer selected and did not select a key. Screenshots: [TypeScript](typescript.png), [Dioxus](dioxus.png).

## Observed parity

- Both render the 29-key Sofle contour, Base layer, layer name, and Keys/Macros/Encoders contextual tabs. This current Dioxus build does show the board contour; older evidence claiming no outline is stale for this build.
- The TypeScript Objects tree shows the Outline entry and expands `keys` into Columns 1–6. Dioxus's shared tree shows only `keys` and `thumbs`; it omits the Outline/version row and column grouping in this Keymap context. This remains a shared tree/Outline navigation gap owned by T1-11/F3.4, not a missing Keymap canvas contour.
- Layout differs: the reference places a Keymap / Layers & key behaviors header above the canvas and has no physical-instance/grouping controls in Objects; Dioxus retains generic board/instance/grouping controls and reserves a wider left rail. The right contextual editor controls are broadly present in both, though labels and explanatory content differ and need interaction-level comparison.

## Limits and next acceptance

This was an initial visual/control inventory, not a complete ticket acceptance journey: no key binding, macro, or encoder edits, history, or save/reopen were performed. Both profiles were isolated; the browser's local demo projects were not the user's saved workspace. Continue with identical imported fixtures for edits and durable reopen checks.
