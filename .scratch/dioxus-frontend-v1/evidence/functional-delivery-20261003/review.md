# Functional candidate 34776 consolidated review

Finalized 2026-10-03; requested profile Sol 6.1 High. **PASS for the assessed selection correction and scoped qualification transition. No actionable finding.** This is one Standards/Spec candidate pass. No parent is accepted, and the historical 34774/34775 reviews remain unchanged.

Scope: base `ac3a3614`, application/control source `6f8ee398386e1a6ffb6b08346fd85cdd0dacbf7a`, record commit and immutable packaged source `c85160c70c09c69289c6c9fd330dc5f357552c32`. Build `frontend-functional-selection-20261003`, root `http://127.0.0.1:34776/` and `/boardstudio/`. React reference remains `5a472a9426e6e38993361da402cd4ec730feb369` at port 5175. The updated `CONSTRAINTS.md` prioritizes working workflows; exact cosmetic layout is deferred without waiving missing actions, incorrect edits or unusable controls.

## Source assessment

`web/src/presentation.rs:1889–1913` maps Matrix, Row, Column, Key and Component tree contexts to the corresponding existing canvas selection kind. Board, Outline, OutlineVersion, Bridge, MountedModule and LayoutGroup retain the prior kind. The update remains in the existing tree callback at line 3674, after scope/generation/current-context admission. It writes the same root signal already consumed by canvas hit-selection and the toolbar, introducing no second selection or document authority. The regression covers matrix/row/column/key, matrix-member and standalone parts, and excluded contexts.

`.scratch/dioxus-frontend-v1/qualification.py:41–177` honors an explicit `next_scope`, retaining the legacy Layout record until a new scope is selected. It validates the source ancestry, nonempty scope/stream names and unique journey IDs, matching page compiler evidence and only the selected streams' queued packets. Starting a scope preserves the prior scope, review and results in history; new journeys strip old result fields and begin pending. Replacing a candidate also archives its prior results when the new candidate is still integrating. Already-started same-candidate results are retained. This changes qualification timing, not parent acceptance or canonical criteria.

Actual candidate recording confirms active scope `functional-selection-20261003`, pending `objects-selection-mode`, no unmet start conditions, cleared `next_scope` and retained qualification history. No source correctness or standards defect was identified in this bounded delta.

## Verification and evidence

The settled adjacent `journey.md` reports the expected selection-regression RED followed by 1/1 WASM GREEN, with 180 tests filtered. Qualification controls passed 12/12 after the two new scope cases failed for the expected reason; `py_compile` also passed. These are reused author/coordinator results; this reviewer ran no test, browser, Cargo or build job.

The new public replay demonstrates the corrected Matrix/Column/Key/Part tree modes, Row mode after grouping changes, and Outline retaining Part. Trusted clicks on the observed row-2/column-3 hit area selected Column 3 in Column mode, the entire matrix in Matrix mode and Row 2 in Row mode. This establishes changed hit-selection behavior as well as labels. Browser errors were empty and the owned session closed. An automation selector mismatch over a decorative rectangle is disclosed and was resolved using the actual hit area; it is not a product failure.

Package proof records eight fresh and 22 inherited successful commands, 1,385 source hashes, 190 assets per route, no source/asset mismatches, no release warnings, and both routes at HTTP 200 with isolation headers. Duration: 147.557471 seconds. Provenance SHA-256: `057ac248598153797ad950c434fbed9d3f5762e2d8998857363d041a392a7fac`. Existing local/served verification is reused; no duplicate asset sweep was performed.

The receipt also retains representative 34775 functionality across exports/outline characterization, Parts/Project, Case, Keycaps, Keymap and PCB. Those earlier results remain attributed to their actual source. The settled same-history key-size roundtrip yields identical React/Dioxus SVG bytes; it resolves the earlier mismatched-history comparison without treating the normalization itself as an approved invariant.

## Limits

This review does not re-audit the spec inventory or all 62 parents. The changed selection journey and scope controls pass; representative six-stream results do not establish complete workbench, history/reopen, conflict, manufacturing, accessibility, performance, retirement or cutover qualification. Existing exact visual findings remain deferred design inputs under the user's priority. No parent, canonical record or implementation was changed by this reviewer; this file is the only new review output.

Evidence pins: `journey.md` SHA-256 `fc9be35d93fd8a9dc107ae32b28c9480a50d393ca7db2e78e528fbd390ed2123`; `package-proof.json` SHA-256 `ee4b93cb5a2a4c878726adc59af2099f57b3c8b1cb50699df37dbc1eefb66a4b`.
