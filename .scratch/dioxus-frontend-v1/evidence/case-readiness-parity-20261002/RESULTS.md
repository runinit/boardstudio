# Case workbench model preview gap

## Paired setup

- Reference: TypeScript app at `http://127.0.0.1:5175/`.
- Candidate: Dioxus build at `http://127.0.0.1:34695/`, source `fc5e17e3526afe8a2c55efe9afdbc0f0faff3efa`, build `frontend-case-keycaps-pcb-reviewgreen-20261002`.
- Both used the same public `.boardstudio` import, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`, at document revision 9, Left PCB selected, and physical instance `left`.
- The imported document has no authored case bodies, has global mechanical configuration for board `left`, and has no per-instance mechanical configuration or shared construction.

## Journey and observed result

On each app, open Case and add the default “Left PCB plate” body. The reference immediately displays the assembly model preview, labelled “90 / 90 models · 1.6 mm PCB”. The candidate shows a blank canvas. Its tree and Inspector do expose the new plate and its type, thickness, clearance and Z controls. This establishes the missing baseline physical-model viewer in Dioxus, independently of whether a generated case-body scene is available.

Before adding a body, the separate generation controls do not have matching readiness: React's `Update preview` is disabled with `Waiting to update preview`; Dioxus `Generate case` is enabled, but clicking it reports `Case generation blocked: selected board is not ready for case generation`. The diagnosis in [readiness-mismatch.md](/tmp/frontend-parity-reset-20261002/case/readiness-mismatch.md) explains why this fixture is ineligible for generation in both apps and why Dioxus's enabled action is inconsistent with its own lower-level admission check. The earlier reported React generation-success action was not reproduced and is not counted as evidence.

The candidate’s current `CaseViewer` is created only from a generated `CadScene`; its case preparation gate requires per-board readiness for a configured mechanical stack or a case-ready board with authored bodies. That does not provide the reference’s baseline model assembly view, which is driven independently of generated case-body geometry. Earlier RF-003 evidence also found that the Dioxus Case shared-viewer projection sends empty board surfaces, holes and model arrays. The separate enabled-button mismatch is a readiness-predicate inconsistency: the UI gate is weaker than `preparation_request`.

## Qualification

This is a paired public behavior gap on the preserved imported fixture. It blocks F7 Case visual and interaction acceptance. The Case tree/Inspector mount can compile and be structurally reviewed while this separate viewer workflow remains absent. Do not treat the successful strict WASM Clippy check at this source revision as Case acceptance. Treat baseline model projection and generation-action readiness as two distinct child slices. Do not claim the archive generates successfully in React; the paired click was disabled there.

The exact screenshots are `reference-case-body.png`, `candidate-case-body.png`, `reference-case-before-body.png`, and `candidate-case-before-body.png`. Browser sessions use isolated temporary profiles; no production storage or saved user project was changed.

## Required next slice

Refine the existing F7 Case viewer work into a reviewed child that transports the accepted board contours, physical component/model projections and matching preview identity into the Dioxus shared viewer before any generated case body exists. Preserve the existing Rust CAD authority for authored/generated case geometry, and prove separately how the ordinary physical assembly preview is assembled. Paired acceptance must show the 90/90 assembly view before and after adding the default plate, then exercise generation readiness and recovery in both apps.
