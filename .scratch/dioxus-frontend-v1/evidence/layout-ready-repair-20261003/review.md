# Layout-ready repair delta review

Finalized 2026-10-03. **PASS for the assessed repairs and two focused behavior journeys; full Layout acceptance remains open.** No actionable defect was found in this repair delta. The frozen candidate-34774 consolidated review/audit remains unchanged; this review resolves its L1 against the new source and evidence without repeating the five workflow reviews or accepting any parent.

Source: application repair `cbce292b771b6a4b513e359fdbcefd695f01c8cf`, standalone-test ownership repair `2caace52ee2bd9a813030586bf88a1de7aa88a0c`. The immutable candidate is `frontend-layout-ready-repair-20261003`, served at `http://127.0.0.1:34775/` and `/boardstudio/`, packaged at the latter commit. Warning-only follow-up `e0e899243b2f172686a3467ea2e5456e95ce25ab` is source-only. React remains pinned at `5a472a9426e6e38993361da402cd4ec730feb369`.

## Correctness and lifecycle

L1 is repaired: `web/src/presentation.rs:7094–7109` retains the current-owner guard and selects the shared Properties tab before opening the Inspector. `objects/layout_transform_toolbar.rs:1264–1282` invokes that route before Column/Row selection changes the owner generation. Position keeps its property-availability gate; the two selection shortcuts retain their own availability gates. No edit authority or document state is added.

The newly reproduced startup freeze is repaired at `presentation/board_reference_effect.rs:14–17` and its call in `pcb_board_reference.rs:672`: the missing-reference effect increments its invalidation epoch using `peek()` instead of subscribing to the epoch it writes. It still clears discovery/error state, respects the existing busy branch and preserves asynchronous epoch/current-owner settlement. The mounted regression imports the actual production helper; its retained RED shows eight effect runs versus one expected, and GREEN shows settlement after one run. The narrow test-callsite updates preserve existing assertions.

The packaging follow-up adds top-level `web/tests/*.rs` roots to the test graph. Page/provider release graphs remain strict and are subtracted before test-only eligibility. Opaque-macro tolerance is enabled only for standalone test traversal; it does not relax release/provider ownership. The negative regression makes the same test path reachable from core-worker and requires rejection without reserving candidate output. No unsafe reuse exemption was identified.

## Evidence reused and assessed

- Missing-reference mounted regression: expected RED then GREEN, 1/1 native; retained logs were read. Production toolbar regression: expected `false:1` versus `true:1` RED then GREEN, 1/1 browser with 179 filtered; command/results are recorded in the journey receipt, with stdout retained only in agent tool results.
- Ownership checks: coordinator reports 34 reuse and 3 source checks passed, including positive/negative classification and actual retained-baseline preflight. Combined compiler and package passed.
- Package proof: eight fresh/22 inherited commands, 1,385 frozen sources and 190 assets per route, zero source/asset mismatches, HTTP 200 and required isolation headers on both routes. Provenance SHA-256: `656a1f2493d7c3ba092df350068bc976252ddbf16b53d25ec5f83224d608509f`. Two unused-`mut` release warnings belong to this exact package. The reviewed follow-up removes only those two qualifiers; its clean compiler check passed in 6.46 seconds and is not attributed to the preview package.
- The completed paired receipt establishes responsive Sofle startup, all three Relations → Properties shortcuts, key width `1→1.25`, Undo→`1`, Redo→`1.25`, and retained `1.25` after reload. Outline copy, point X→`-5`, Undo/Redo and retained `-5` after reload also passed. These are coordinator browser results; this reviewer ran no browser, Cargo, build or suite.

## Remaining parity and limits

The receipt retains real visual/context gaps: Matrix Inspector order/headings and typography; Select pill remaining Key after matrix tree selection; general toolbar during perimeter editing instead of Outline points/grid/Snap/Done; oversized point handles and contextual outline presentation. These remain existing parity work, not repaired or accepted by this delta.

Initial outline X differs (`-3.97` candidate versus approximately `-3.82` React). The runs used different key-size edit/copy ordering, so they establish the listed operations and persistence, not geometry equivalence. Matched pristine-fixture/size-roundtrip characterization remains necessary. This pass does not qualify all constraints, drawing, snapping, pointer cancellation, accessibility, performance or parent criteria.

Evidence pins: `journey.md` SHA-256 `8cd9f266b7c1798e7b63d79bd3c1c9f91bcea12b3a3ea204aef644a43a07fc23`; `package-proof.json` SHA-256 `92934f96a6b32effac62d37e13ebae9d19cf698785aac9a602d21b3047a8e447`. Both are in this directory. No implementation or canonical record was changed by this reviewer; this file is the only new review output.
