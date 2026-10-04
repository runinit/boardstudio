# F6.6-C02 integrated PCB quick-binding / typed Keymap comparison — 2026-10-04

This receipt joins the retained typed Keymap and PCB firmware-key-position evidence with one bounded public workflow that checks the cross-surface destination and history behavior.

## Pair and fixture

- React reference: `http://127.0.0.1:5175/`, pinned TypeScript source `5a472a9426e6e38993361da402cd4ec730feb369` (pin also recorded in the retained project receipts).
- Dioxus candidate: `http://127.0.0.1:34799/`, candidate source prefix `18ff7659` as supplied for this run.
- Isolated named browser sessions: `keymap-quickbinding-reference-ts-5175-7dd31abc1fc4` and `keymap-quickbinding-join-18ff7659-7dd31abc1fc4`.
- Both sessions publicly imported the same existing saved Sofle fixture, `binding-editor-reference/imported-layered-sofle.boardstudio`, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`. Left PCB was selected and Main was the first selected typed layer.

## Joined workflow

At baseline, the PCB `Firmware keymap` showed `0/30 assigned`; legacy `left-keys-SW3` was `&none`, and the Keymap Main option showed `left-keys-SW3 · Unassigned` in both apps.

Using the PCB Wiring surface's visible `Firmware keymap` choice for `left-keys-SW3`, selected `B` in each app. Both showed `1/30 assigned` and the legacy entry as `&kp B`; the Keymap Main editor showed the same `left-keys-SW3 · B`. This confirms the legacy action reaches the supported first typed layer on the same stable key ID in both apps.

Both public toolbars then accepted Undo and Redo for the initial edit. The paired cross-surface history check changed the PCB value from B to C; Main reflected C in both apps. Public Undo returned Main to B in both. After reload, Main still showed B, and the expanded PCB `Firmware keymap` showed `1/30 assigned` with `left-keys-SW3` at B / `&kp B` in both apps.

## Captures

- `react-keymap-after-legacy-C.png` and `dioxus-keymap-after-legacy-C.png` — typed Main destination after the PCB legacy edit to C.
- `react-keymap-after-undo.png` and `dioxus-keymap-after-undo.png` — Main after public Undo returned the legacy edit to B.
- `react-keymap-after-reload.png` and `dioxus-keymap-after-reload.png` — durable Main value B after reload.
- `react-pcb-quick-binding-after-reload.png` and `dioxus-pcb-quick-binding-after-reload.png` — expanded legacy PCB controls and accepted `&kp B` mapping after reload.

The initial B edit, Undo/Redo, and first reload were performed before the paired C→Undo check; the final reload/captures follow the C→Undo restoration to B.

## Evidence boundary

This is one cross-surface, one-key compatibility check using the supported `&kp` path. It complements, rather than replaces, the retained [typed editor reference](../binding-editor-reference/README.md), [paired F6K.4b PCB workflow](../F6K.4b-browser-20261002/RESULTS.md), and [encoder public workflow](../encoder-public-workflow/RESULTS.md). It does not close F5.2, F8.2, the F6K.4 parent, or claim other firmware-key position values / all F6.6 workflows.

No application source, TypeScript source, build/package, or canonical tracker/run file was changed in this check.
