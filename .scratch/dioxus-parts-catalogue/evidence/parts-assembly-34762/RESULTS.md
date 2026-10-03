# Parts assembly preview and empty-surface journey on 34762

This changed-only paired journey exercises one LED assembly preset, its South/North recipe transforms, the shared 2D/3D selection, return to 2D, and the `utility_text` zero-geometry surface. Earlier import, generator Apply/Undo/reopen, and standalone 3D journeys are reused; none was repeated here.

## Candidate and reference

- Dioxus: <http://127.0.0.1:34762/>; served source `900068a0df413059732f9f365abd598734b7f86c`; package `frontend-footer-empty-assemblies-shell-20261003`; provenance SHA-256 `319f378ba0d2eb0855059d2368e1e8028c0c6267e0a50a22b9ed8e8a195e1251`.
- React reference: <http://127.0.0.1:5173/>; pinned source `5a472a9426e6e38993361da402cd4ec730feb369`.
- Both sessions opened the same retained Sofle fixture, `layered-sofle-export.boardstudio`, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.
- Candidate browser session: `parts-assembly-34762-2318ea1a1376`; React session: `parts-assembly-react-34762-2318ea1a1376` (agent-owned).

## Observed journey

The Parts owner selected **MX Hotswap RGB**. Its Dioxus 2D SVG contains three ordered recipe members: the switch at `(0, 0)`, diode at `(7.4, -1.5)` with 90° rotation, and LED at `(0, -4.75)` with 180° rotation in South orientation. The resulting viewBox is `-12 -12 24 24`; there are no preview alerts. Changing to North flips the member positions and rotations together, and the same three members remain in the preview. Captures show both orientations. React displays the same preset and three visible geometry groups in its source-backed 2D preview.

Switching the candidate to 3D reaches **3D preview ready** in the existing read-only sample viewer. Returning to 2D restores the three-member South recipe with the same viewBox and no alert. React's same-preset viewer reports `4 / 4 models`; its ready capture is retained. Candidate active-project state remained `Sofle v2 · Revision 3 · Saved` throughout the candidate journey.

Selecting `utility_text` after the assembly clears the preset selection. Dioxus renders its selected-definition context with an SVG viewBox exactly `0 0 0 0` and zero alerts; React displays the matching blank surface with `0.0 × 0.0 mm`. No fallback point, courtyard, or synthetic geometry was added.

## Captures

All PNGs are stored alongside this receipt. SHA-256 hashes:

| Capture | SHA-256 |
| --- | --- |
| Candidate MX South 2D | `36851482d0267d8bab2cbac5428f1f57e7b4ed0eedc1586ebfd36c20f90ab5c0` |
| Candidate MX North 2D | `b3b4daad8fe61c42571b463729712e239bfbc187a7525b68993744b214197579` |
| Candidate MX South 3D ready | `3d3853632b7775cbe55a3617563797644c60fff16fde96ada0fe8db0b9f9ab36` |
| Candidate return to MX South 2D | `36851482d0267d8bab2cbac5428f1f57e7b4ed0eedc1586ebfd36c20f90ab5c0` |
| Candidate `utility_text` empty 2D | `c7dd72c3b37c4f6dfde8095e9ddff3d6dd012ece6135436fd93936e0eac5edfe` |
| React MX South 2D | `8477d15006e15c3c0c0f897caa3685053ae95d2fa3562cd9ada19391a64e6fd4` |
| React MX South 3D ready | `7f22c59da8c89eaeab74f2209ec1b30e47c68657d8521d420e5a684ae5149970` |
| React `utility_text` empty 2D | `f2df8338f8a201cbcd19999dbacbb0cabf1d571b1affdcc674b5ef5105cbb97f` |

## Limits and RF

This verifies one representative MX LED preset, two orientations, 2D/3D return, and the empty `utility_text` surface. It does not establish all eight preset parameter combinations, VIK modules, rapid supersession/cancel/error branches, responsive/theme combinations, or the inherited F4.4/F7.3/INT.2 acceptance joins. The candidate 3D ready state was confirmed in the DOM; the React viewer also reported all four models ready. The complete assembly source behavior remains narrower than the Issue 03 parent acceptance matrix.

No new refactoring takeaway observed. Existing RF-006 (isolated sample versus canonical project scope), RF-009 (parity/acceptance accounting), and RF-012 (renderer-host capability wrapper) remain the relevant RF owners; no new RF item is asserted by this journey.
