# Parts standalone 3D preview delta on 34759

This is the changed selected-definition 3D/header/readiness/return journey only. The accepted archive was opened on each origin as fixture setup; the prior import, error, Undo/reopen, and Project history journeys were not repeated.

## Candidate, reference, and fixture

- Candidate: `http://127.0.0.1:34759/`, exact served source `04c85b88eae2894b416979f785a1f0060d2eb27d`.
- React reference: `http://127.0.0.1:5173/`, pinned source `5a472a9426e6e38993361da402cd4ec730feb369`.
- Both origins opened the existing `layered-sofle-export.boardstudio`, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.
- Candidate browser session: `parts-preview-936481b4b15c`; React session: `parts-preview-react-20261003`.
- Root reports strict page WASM all-target Clippy PASS (16.58 s), eight fresh and 22 inherited package commands, 1,366 source inputs with zero drift, and 145 assets per route with zero mismatch. Provenance SHA-256: `81ee87aead500473ae22f79834a2bc921618442a081719860a08188c063e49aa`.

## Verified selected-definition journey

The candidate loaded the accepted `ergogen:ceoloide/switch_mx` definition (seven pads, Back side) in Parts. Selecting **3D model** mounted the shared viewer with the accessible name “Interactive 3D Parts sample preview. Navigate the isolated read-only sample; it cannot edit the active project.” The candidate showed **Fit sample**, Parts-specific camera/display/view control labels, and `3D preview ready.` React showed the same selected definition in its 3D sample viewer. Candidate and reference captures are retained below.

Returning to **2D footprint** restored the selected MX source preview and its layer control. The candidate footer remained `Sofle v2 · Revision 9 · Saved`; no document edit/history action was issued during preview setup, rendering, or return.

A no-linked-model fallback was also exercised with `ergogen:infused-kim/utility_text`. Candidate 3D reported `No 3D model is linked to this definition.` while the sample viewer reached `3D preview ready.` The React sample for the same definition showed the sample surface without an alert.

## Captures

Files are retained under `/home/chris/.local/share/boardstudio/retained-tmp/20261003/parts-preview-34759/`.

| Capture | SHA-256 |
| --- | --- |
| Candidate MX 3D ready | `9a12cb2e7552a284c91bd58a34f892016b5f6b6592182962f56e4d80b908d6f3` |
| React MX 3D | `8bea8c143ae6d5767e9904f68d950dc3ab9693e0a3d3bc7048d13b8cee7d5ddf` |
| Candidate MX 2D return | `b7b40e661ec1f2e88e5f473e58cddefea12ce3ec1667664732792266e4b1d4f5` |
| Candidate no-model 3D fallback | `d52238ea6468464ce657aab318630bcc91e4d3a2767f3d4b335decbb88f1228e` |
| Candidate utility-text 2D return | `2335dca4011b7a31a7fbe04217a7be068f15a3e0c4d4091dd6498767f0afd06a` |
| React utility-text 2D | `753df0b84d210d1e27bd9e91fe440773b7038f4411b839211d330d6d9ab7c3fe` |
| React utility-text 3D ready | `c4a644bbc0971adcb826ba6aaf4e6b311c2f21bf56931af266b5fc3009930883` |

## Observed limit for the next Issue 03 packet

Selecting `utility_text` and returning to 2D produced a candidate alert, `Footprint preview failed: The selected definition contains no 2D footprint geometry.`, and no canvas. React presented its empty `0.0 × 0.0 mm` surface without an alert. This is recorded as an actual empty-geometry parity failure, separate from the new 3D producer; the next Issue 03 assembly/companion packet must render the legitimate empty surface without inventing geometry. The candidate source packet does not claim this path is green.

The next source-backed assembly frontier is also recorded in the Issue 03 ticket: Issue 02 owns the eight-preset/variant selection input, while Issue 03 consumes ordered recipe members and their accepted definitions, merged generator parameters, poses, rotations, and sides. This receipt closes neither that work nor any F4.4/F7.3/INT.2 parent acceptance join. RF-006 and RF-009 remain the relevant scope/evidence records; no new RF item was observed.
