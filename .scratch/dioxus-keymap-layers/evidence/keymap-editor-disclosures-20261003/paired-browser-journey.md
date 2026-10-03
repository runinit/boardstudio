# Keymap editor disclosure paired journey

## Before repair: 34751

- Pinned reference: React `5a472a9426e6e38993361da402cd4ec730feb369`, `http://127.0.0.1:5173/`.
- Candidate: `http://127.0.0.1:34751/`, frozen source `ab532f275cb14385db3e4e52e5adcb5fb7c3ee18` (contains disclosure packet `64115c132d5d7f3f5533a9388797da7dea913b56`).
- Both named Keymap browser profiles were set to 1280 × 577 and opened Sofle v2 copy, Keymap, Base. No key binding, layer, project document, or history action was changed.
- The candidate's `details.m1-keymap-editor-section` summary click returned to `open: true` on immediate DOM readback. The static `open: true` attribute was being reapplied; Keys could not remain collapsed. React's `details.wb-inspector-section` changed to `open: false` and stayed closed while switching to other tab buttons. React Macros/Encoders panes each mounted open, could be collapsed, and reopened expanded when switching away and back.
- On the candidate, Macros, Encoders · Base, and Macros again mounted expanded after tab switches; clicking each summary also returned `open: true`, so their collapse behavior failed the same way.
- The Keys summary in React updates per selected key/layer. The candidate's failed collapse prevented the intended collapsed-summary refresh check; it remains for the green rerun.

## Captures

- Candidate before repair: `dioxus-34751-before-repair.png`, SHA-256 `687af19730f01e1be5a8013d0aa5d321f01a8f1d8c01c956fb0396b6c6dfa51e`.
- Reference: `react-reference-before-repair.png`, SHA-256 `787fa780c7e9cd8a0bc4d7abc19d744166f3b0a7bf7247bed7be5812e11adeaa`.

## Repair and next journey

Candidate 34754, source `9c16db41155a7b72be187789d225f78c2fbfe9af`, confirmed the per-pane toggle and tab-remount behavior: Keys, Macros, and Encoders all collapsed, while switching tabs mounted the next pane expanded. It exposed one remaining owner mismatch: after changing from SW2 to SW1 while Keys was collapsed, Dioxus stayed closed while React reopened. React `Workbench.tsx:1399` keys Inspector content by `activePart.id`; the new accepted owner remounts the section expanded. The source now resets Keys open on a selected-key/scope-owner change, while layer-only changes retain the user's choice. No editor callbacks or domain state changed. The final paired replay needs to confirm this exact reset plus tab toggles and no document revision or Undo/Redo change from disclosure actions.
