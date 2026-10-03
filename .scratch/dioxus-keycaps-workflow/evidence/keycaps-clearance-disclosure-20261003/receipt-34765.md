# F6C.4 Keycaps clearance disclosure choice — 2026-10-03

This is the changed native disclosure-choice leg for Issue05. It reuses the accepted F6C.2 settings and fit/history receipts; it does not re-run their broader workflows.

- React reference: `http://127.0.0.1:5173/`, pinned source `5a472a9426e6e38993361da402cd4ec730feb369`.
- Dioxus candidate: `http://127.0.0.1:34765/boardstudio/`, source `17ba32b984c13f3621a2145efe1dd478862ab5cd`, provenance `273ca2fa2315b32fec91b0490d2f70fb03b5426d759310bced6e934ad73947d5`. Root reports a verified package with 8 fresh + 22 inherited inputs, 1378 source hashes, 146 assets, no route drift, and the strict page WASM all-target check passing.
- Viewport: 1280×577. Each app used its own named `agent-browser` session and imported the same fixture `/home/chris/.local/share/boardstudio/retained-tmp/20261002/keycaps-findings-f6c4-c6.boardstudio`, SHA-256 `9027125846d2878176f127ec39b60c9ab605a0eb2a4b9019e10dbba846fe87a6`. The active project was Sofle v2 on Left PCB; no key was selected.

Both apps initially showed the native **Clearance findings · 27** disclosure open. After clicking the summary to collapse it, I changed Keycap clearance from 0.5 to 0.6 mm and committed by leaving the numeric input. The finding count remained 27, and the user-chosen closed state remained closed in both apps. Undo restored 0.5 mm while keeping the disclosure closed; Redo restored 0.6 mm and kept it closed. React reported revision 13 after the edit, 14 after Undo, and 15 after Redo. Dioxus exposed an enabled Undo/Redo path and reported the browser-local save state as saved after the journey. No fixture was saved over the retained input file.

The visible behavior is paired across source and candidate: default-open native details, user collapse, accepted setting refresh, Undo, and Redo all preserve the collapsed choice. This is a bounded presentation result; it does not repeat matrix, color, key-override, save/reopen, or full fit acceptance. No product source changed in this receipt commit and no new RF observation was identified.

Captures (all 1280×577):

- React initial: [`react-34765-initial.png`](react-34765-initial.png), SHA-256 `b74519dc5baeb7a47b0636c9feee81beefacfd61e0e420a7a4cd5af005c28477`.
- Dioxus initial: [`dioxus-34765-initial.png`](dioxus-34765-initial.png), SHA-256 `84eeb69cd3e6ae6ac3d6a2741cd9f0575efa168d7acb145301d50526c81fa5f1`.
- React collapsed after edit: [`react-34765-collapsed-update.png`](react-34765-collapsed-update.png), SHA-256 `b5f0d0d96ce015862be08cabd61d5d97d3a7d2a77848fad9c2033c52de0293b8`.
- Dioxus collapsed after edit: [`dioxus-34765-collapsed-update.png`](dioxus-34765-collapsed-update.png), SHA-256 `242ff5ddf4a3f8824caa73a7a56872917494687ac730e4dc3a9c677dfa5cf66f`.
- React after Undo: [`react-34765-undo.png`](react-34765-undo.png), SHA-256 `4e28f6d1c5f6bf6e5ffe61931a472a4f2b7def464e32c30a74d078f1967f7106`.
- Dioxus after Undo: [`dioxus-34765-undo.png`](dioxus-34765-undo.png), SHA-256 `a26e56cbea4fd5792a18bb2359861f6c4d755fa2968fd2e314ad099422a02ff9`.
- React after Redo: [`react-34765-redo.png`](react-34765-redo.png), SHA-256 `63e9679f662f32ccf55e37650a97eb6a29d014edcd808beb37bea5afca0d9a7b`.
- Dioxus after Redo: [`dioxus-34765-redo.png`](dioxus-34765-redo.png), SHA-256 `48f9e20316483234c1cd69cb15701d6957303756149305e119ccc9d756ef0838`.

Issue05 remains a child of the broader F6C.4 and parent acceptance joins; this receipt closes neither parent.
