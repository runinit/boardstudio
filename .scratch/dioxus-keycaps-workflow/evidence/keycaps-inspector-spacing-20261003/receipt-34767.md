# Keycaps Inspector spacing comparison — 2026-10-03

This bounded visual comparison verifies the horizontal Inspector inset and selected-key action spacing fix from source commit `9afe5c35`. It reuses accepted F6C.2 setting and legend history receipts; it does not repeat edits, history, or persistence workflows.

- React reference: `http://127.0.0.1:5173/`, pinned source `5a472a9426e6e38993361da402cd4ec730feb369`.
- Dioxus candidate: `http://127.0.0.1:34767/boardstudio/`, source `3a080e0e99d0638ddc1b562b3fc5a8ac91f3fefd`, provenance `96d7bc6289aedaadd8198bb9038c67d24fc659fde847362b24c7504313cbc733`.
- Root reports the packaged candidate's strict check passed, with 1372 source hashes, 146 assets, no route mismatch, and COOP/COEP enabled.
- Viewport: 1280×577. Each app used a separate named `agent-browser` session and the same c6 archive `/home/chris/.local/share/boardstudio/retained-tmp/20261002/keycaps-findings-f6c4-c6.boardstudio`, SHA-256 `9027125846d2878176f127ec39b60c9ab605a0eb2a4b9019e10dbba846fe87a6`.
- Context: Keycaps workspace, Left PCB, selected SW1 (`matrix/left-keys/r0c0`), unassigned binding; both Inspector scroll containers were positioned at 630px so the selected-key fields and actions were visible. No document edits were made.

The before captures on candidate34765 showed the React Inspector content inset 24px from either side of its panel, while Dioxus used 12px; the React “Use binding legend” and “Blank keycap” buttons had a 6px gap while Dioxus buttons touched. On candidate34767, both apps show approximately 24px horizontal insets, matching control widths, and a 6px gap between the two actions. The correction is limited to `.m1-keycaps-inspector` horizontal padding and `.m1-keycaps-settings-actions` spacing. No remaining mismatch was found in this selected-key slice at the stated viewport.

Captures (both 1280×577):

- React: [`react-spacing-setup.png`](react-spacing-setup.png), SHA-256 `73c9e0573ec63359bd74a1ff3efd0a4cbf4179533f437e130be41fe1d9c4247a`.
- Dioxus: [`dioxus-spacing-setup.png`](dioxus-spacing-setup.png), SHA-256 `f40803b4a98b2fda15745ec29c42839218add9404d90dffda7a1fd68e4e16856`.

The F6C.2 field coverage and evidence limits remain recorded in [`F6C.2 criteria reconciliation`](../../../dioxus-frontend-v1/evidence/keycaps-3d-preview-20261003/f6c2-criteria-reconciliation.md). F6C.2 `acceptance_after=[]`; this receipt does not change parent task status or acceptance joins. F6C.1's explicit F3.1 acceptance join remains coordinator-owned.
