# F6C.2 bounded settings receipt — 2026-10-03

This is a bounded paired owner workflow for the remaining matrix/color/legend-history questions in [`f6c2-criteria-reconciliation.md`](f6c2-criteria-reconciliation.md). It does not accept F6C.2 or the broader Keycaps parent.

- React reference: `http://127.0.0.1:5173/`, source `5a472a9426e6e38993361da402cd4ec730feb369`.
- Dioxus candidate: `http://127.0.0.1:34759/`, source `04c85b88eae2894b416979f785a1f0060d2eb27d`; root reports the candidate build and affected strict WASM check passed.
- Viewport: 1280×577. Both use the retained c6 fixture `/home/chris/.local/share/boardstudio/retained-tmp/20261002/keycaps-findings-f6c4-c6.boardstudio`, SHA-256 `9027125846d2878176f127ec39b60c9ab605a0eb2a4b9019e10dbba846fe87a6`.
- Reused older sufficient history receipts: `.scratch/dioxus-keycaps-workflow/evidence/settings-20261002/source-review-manifest.json` (clearance/profile history and reopen) and `.scratch/dioxus-keycaps-workflow/evidence/keycaps06-paired-browser-20261002.md` (per-key DSA/width history and reopen). No neighboring profile matrix was repeated.

On each app, the `keys` matrix was set to first profile row 2, wall thickness 1.3 mm, and explicit MX cross. Board keycap and legend colors were set to `#add8e6` and `#284080`. React’s standard DOM color input/change events were used; this verifies the browser control’s accepted change path, while physical native-picker interaction remains unverified. SW1 was made explicitly blank; the 2D canvas showed a dash, Undo showed inherited `A`, and Redo returned to the blank state in the Dioxus workflow. After reload, React retained the two board colors, row, wall, socket, and SW1’s blank legend (input empty with placeholder `A`). Dioxus retained the same values and its reported owner-level blank/Undo/Redo/reopen sequence. The React attempt then surfaced `Error: History is empty` after Redo; its action/setup did not prove that the legend edit had entered the history owner, so this is an inconclusive observation, not a defect finding.

The invalid matrix wall value 5 mm was rejected in both apps with the 0.8–2 mm constraint feedback; the last accepted value remained 1.3 mm and the accepted revision did not advance. Switching from SW1 to SW2 showed inherited board color and matrix sizing instead of SW1’s key overrides. Switching from Left PCB to Right PCB showed Right PCB’s default colors and its unconfigured matrix profile, confirming board ownership isolation. The current React session was reloaded after accepted edits; Dioxus and React source state was not edited during this journey.

React captures:

- [`f6c2-react-blank.png`](f6c2-react-blank.png), SHA-256 `e92d107bdc152959560d234c6b3b12e7aab2c9e013052ad8f28b6f1ff00df9a2`.
- [`f6c2-react-inherited.png`](f6c2-react-inherited.png), SHA-256 `5d29f3dd9b7475d1439acb2ecbb3d96600cb7cbf205631cf92385c300ef81456`.
- [`f6c2-react-redo.png`](f6c2-react-redo.png), SHA-256 `72acb557ad4c63a0b7d9e4b3b73b16b1757d052477bc5dc2cd63ed8f6d77ad16`; includes the history-empty message.
- [`f6c2-react-reopen-blank.png`](f6c2-react-reopen-blank.png), SHA-256 `a669b28003f4029702e3687d4633ce0293354595423626f0f6fe28baeeaa82c3`.
- [`f6c2-react-pre-reload.png`](f6c2-react-pre-reload.png), SHA-256 `d208e75ea3420c8775d524be1780c15087d6ad4a89b1716ad4911507df043ea4`.

The remaining meaningful gaps are the owner-valid React null/empty legend history sequence, remaining per-key row/socket/depth behavior, and native host picker interaction. The parent criteria stay open.

## Current paired fit refresh on the same saved fixture — 2026-10-04

This short continuation supplies a current-source profile/fit check for F6.6-C03 and reuses the existing linked-size, legend/color, 3D, and STEP journeys above and in the referenced receipts. The browser command opened Dioxus at `http://127.0.0.1:34803/`; its verified served identity is build `frontend-findings-panels-20261004`, source `f399d8c2e8ef1b409bbadfd17bfe59907fb8a272` (package proof `.scratch/dioxus-frontend-v1/evidence/frontend-findings-panels-20261004/package-proof.json`). React 5175 used pinned source `5a472a9426e6e38993361da402cd4ec730feb369`. Both imported the same unmodified archive `/home/chris/.local/share/boardstudio/retained-tmp/20261002/keycaps-findings-f6c4-c6.boardstudio`, SHA-256 `9027125846d2878176f127ec39b60c9ab605a0eb2a4b9019e10dbba846fe87a6`; both opened Sofle v2, Left PCB, Keycaps.

The starting `keys` matrix profile was SA and minimum clearance was 0.5 mm. On both apps, I changed the profile to DCS and clearance to 0.6 mm through their mounted controls. The accepted inputs settled to DCS and 0.6; both fit views retained 27 findings and showed the same three key-clearance messages for SW1↔SW2 (-18.00 mm), SW1↔SW3 (-9.20 mm), and SW1↔SW4 (-6.70 mm), each requiring 0.60 mm. One Undo on each app restored clearance 0.5 and the displayed required clearance to 0.50 mm; Redo restored 0.6 and 0.60 mm. The DCS profile remained accepted through that history sequence. No mismatch was observed, and the imported archive was not overwritten.

Current-run captures are in [`f66c03-current-paired-20261004/`](../f66c03-current-paired-20261004/): React before `b74519dc5baeb7a47b0636c9feee81beefacfd61e0e420a7a4cd5af005c28477`, React after `a3b5e9158e7b461f73bab3dd992646b578a67a8a01edfc42816f42f6279375db`; Dioxus before `c076a01127645943705280de3ffca0f8fac41be88fd3ae3c17c14703e4a6dd7b`, Dioxus after `09cf98be0c94f2878393cd8022bc704b290994aad32f417075a707ea307a1f74`. The React before image hash matches the prior c6 entry capture; its after hash above is the newly captured 0.6 result.

The run reuses the paired legend/color/matrix-settings result in this receipt, the linked-size result in `keycaps-size-reflow-20261003/paired-linked-resize.md`, and the accepted cap/legend preview and byte-identical STEP result in `keycaps-f6c5-functional-20261003/` and `functional-delivery-20261003/journey.md`. Size, 3D, and STEP were not repeated. This bounded continuation does not claim the remaining F6.6 joins or full Keycaps acceptance.
