# Native custom-component editor — bounded 34768 journey

## Candidate and reference

- Dioxus candidate: `http://127.0.0.1:34768/boardstudio/`
- Candidate source: `9e6f9b62e33d7dc22412cf5793a3f4b857e219c3`
- Build: `frontend-contextual-panes-followup-20261003`
- Provenance SHA-256: `0d04905a8dbd66e961595fbae3702124e2f7d2784cfbe5bc3e8dab85dc81d290`
- React reference: `http://127.0.0.1:5173/`, source `5a472a9426e6e38993361da402cd4ec730feb369`
- Browser profiles: `parts-custom-editor-34768` and `parts-sofle-variants-20261003`.

The candidate source includes the already-integrated private Parts editor and its current-owner/pad-row corrections. The candidate server and provenance were supplied by the coordinator; this browser author ran no build or test suite.

## Paired user journey

On each app, start a fresh Sofle v2 copy, open Parts, create `Custom component 15`, add the default pad, and change courtyard width from 10 to 12 mm using the editor's field commit boundary.

| Check | React | Dioxus candidate |
|---|---|---|
| Sofle v2 project ID | `88cf4f2a-05a3-4474-b70f-cc423b27b8a9` | `81b8fcfb-ad7f-4c23-866d-eb4731c8b5c9` |
| Custom definition ID | `ui-6abb6e39-5239-4895-a632-771c5d5e6654` | `ui-3571751d-7d21-4c78-9b64-6291ed953a02` |
| Default pad | `ui-7590905e-5f9f-4ec6-8bf3-0bf30522fb67`, number `1`, circle, 2 × 2 mm at (0, 0) | `pad-22`, number `1`, circle, 2 × 2 mm at (0, 0) |
| Changed courtyard | 12 × 6 mm, centered at origin | 12 × 6 mm, centered at origin |
| Accepted revision after width edit | 5 | 6 |
| Undo / Redo | Undo restored width 10; Redo restored width 12 | Undo restored width 10; Redo restored width 12 |
| Saved reopen | reopened Sofle v2 retained definition, pad and 12 × 6 mm courtyard at revision 7 | reopened Sofle v2 retained definition, pad and 12 × 6 mm courtyard at revision 8 |

Read-only IndexedDB inspection supported the visible controls and saved values. Each create, Add pad, field edit, Undo and Redo advanced one accepted revision. Dioxus reopened the saved Sofle v2 with the catalogue selection reset to MX switch; selecting `Custom component 15` again displayed the persisted fields. The ticket does not require selection state to survive project reopen, so this is recorded as a behavior detail rather than a claimed defect.

The React reference also exposed the expected Kind choices, courtyard dimensions, pad ID/number/X/Y/width/height/shape/drill and Remove pad controls. Dioxus exposed the same control family. This journey exercised only the default pad and courtyard width, not every control or validation branch.

## Evidence files

- `react-after-reopen.png`, SHA-256 `79ecec2694fe007cf4366f0736769f5624b0787718c36a175ad8f637d44707cf`
- `dioxus-after-reopen.png`, SHA-256 `6d28ba091b25a59ed019dc6fa305f0580d7fe6c439c6478017719cfcbbafd5a0`

## Remaining limits

This receipt does not cover blank/invalid input recovery, pad-number collision repair, pad-ID remap/removal across placed instances and nets, multiple pad rows/draft retention, all kind/shape choices, KiCad-owned read-only behavior, compact/dark/accessibility states, or injected stale-scope/rejected-operation behavior. It does not close Issue 04's inherited F4.2, F4 or F9 joins.
