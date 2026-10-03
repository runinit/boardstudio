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

The private panel now owns one default-open signal per pane. Summary clicks update that signal; selecting another tab resets the newly mounted pane to expanded, while ordinary rerenders preserve the current pane's open state. No editor callbacks or domain state changed. Re-run the same bounded paired journey on the next verified candidate, including Keys collapsed while changing the selected key and checking the refreshed summary, each tab's collapse/reopen, and no document revision or Undo/Redo change from disclosure actions.
