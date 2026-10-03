# F6C.4 Clearance findings disclosure: paired source read

This is a bounded, pre-fix paired read used to select the next Keycaps presentation slice. It does not claim browser acceptance of the implementation or close F6C.4/INT.2.

## Provenance

- React: `http://127.0.0.1:5173/`, pinned source `5a472a9426e6e38993361da402cd4ec730feb369`.
- Dioxus: `http://127.0.0.1:34763/`, candidate source `99ec041a2895e5ab23880be25501beb9360487db`, provenance SHA-256 `d3c03d6db107ab7a55753b5798d99444f7d0843540ef96530c00d9f44296fe08`.
- Fixture: `/home/chris/.local/share/boardstudio/retained-tmp/20261002/keycaps-findings-f6c4-c6.boardstudio`, SHA-256 `9027125846d2878176f127ec39b60c9ab605a0eb2a4b9019e10dbba846fe87a6`; embedded project `m1-sofle-v2-copy` / Sofle v2, revision 12; active board `left` / Left PCB.
- Viewport: 1280×577. Isolated profiles: `keycaps-read-react-20261003` and `keycaps-read-dioxus-20261003`.

## Observed behavior

Both Inspector projections showed `Keycaps`, `0/29 assigned`, active Left PCB and no selected key. React rendered `Clearance findings` with detail `27` in a default-open native `details`/InspectorSection. Dioxus rendered the accepted findings under an always-open `.m1-keycaps-fit` section without a disclosure summary. Its existing Board colors, Matrix profiles, and Selected key disclosures already matched the corresponding React group structure in this read.

The pinned React implementation updates its default disclosure state when asynchronous finding count changes until the user clicks its summary; afterward it preserves the user's choice. The Dioxus repair reuses the existing Keycaps details presentation and keeps fit assessment/navigation owners unchanged.

The read did not activate finding navigation, edit project data, change keycap settings, or qualify the existing board/matrix/per-key edits or full finding journey. Screenshots capture only the 2D Keycaps Inspector and canvas:

- [`react.png`](react.png), SHA-256 `b74519dc5baeb7a47b0636c9feee81beefacfd61e0e420a7a4cd5af005c28477`.
- [`dioxus.png`](dioxus.png), SHA-256 `84eeb69cd3e6ae6ac3d6a2741cd9f0575efa168d7acb145301d50526c81fa5f1`.

## Scope and refactoring

This is an existing Keycaps contextual presentation gap in F6C.4, not a new architectural finding. The correction remains within `KeycapsFitInspector` and its feature-scoped stylesheet. Carry RF-001/RF-005 through the coordinator; no new RF ID is proposed.
