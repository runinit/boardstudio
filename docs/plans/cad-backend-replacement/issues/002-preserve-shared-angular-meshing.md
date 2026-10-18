# 002: Enforce shared-edge angular sampling without whole-model regressions

Status: resolved
Type: build
Blocked by: None
Owner: /root

## What to build

STEP preview tessellation honors the requested topology-edge angular limit while incident faces retain the same shared samples and correct trim charts.

## Acceptance criteria

- [ ] At the public tessellation APIs, refine shared edges once, preserve original samples, and propagate bounded sampling failures to every incident face.
- [ ] Reject invalid angular options consistently for nonempty inputs, including edge-free faces; preserve the documented empty-shell behavior where the API has no error channel.
- [ ] Retain expected-red controls and verify the actual failing STEP edge meets its requested turn limit after the fix.
- [ ] Compare the full affected shell set against its frozen baseline: topology, source-edge coverage, empty/dropped faces and independent source-domain accuracy. Explain substantial changes using source geometry rather than assuming mesh volume is trustworthy on open shells.
- [ ] Keep edge-turn, face-normal and linear accuracy checks distinct; document remaining diagnostics and do not silently change their contracts.
- [ ] Pass focused and full affected meshing tests, native/WASM checks, independent review and combined-source verification.

## Existing work

V1 was rejected by whole-model regressions. V2 restores topology classes and passes native/WASM checks, but domain review and invalid-option consistency work remain. Continue the current isolated assignment; do not restart it. Evidence: astra-surface-refinement/angular-edge-candidate (`../../../../.evidence/monstertruck-qualification/astra-surface-refinement/angular-edge-candidate/`). The [qualification record](../../../research/cad/monstertruck-qualification-2026-10-16.md) identifies shared source and broader acceptance limits.

## CURRENT — 2026-10-18

Delivered: angular shared-edge sampling (meshing-v21 onward) and fallback mesher caching; every acceptance box is open and the ticket stays `claimed`. Remaining exits: (1) confirm invalid-option consistency for edge-free faces and the frozen full-shell-set baseline comparison; (2) run it through 007's 34-asset replay with 001; (3) one combined closure review with native/WASM verification. Close then, not on status alone.

## Comments

2026-10-17: Under the approved vendor-asset contract, distinguish requested tessellation settings from certified source/mesh/repair error bounds. Keep shared-edge consistency, face completeness and source attribution; bounded vendor mesh repair may proceed under 007 without proving an exact imported solid. Generated CAD/export remains exact. Vendor collision targets 0.01 mm clearance accuracy with an uncertain result when the combined error budget cannot support it; a 0.1 mm display mesh is not a tighter collision certificate.

2026-10-17: Reviewed Circle/Processor fixes are integrated as circle-v34 and pass 549 serialized native tests/19 inherited ignores plus WASM, but the fresh 15-file replay is 29/30. Checked/angular horizontal VIK connector regresses to 71 invalid edges and two unmeshed faces (all 17 parts retained); owner is isolating the source defect without rolling back correct inverse behavior. Surface-v33 remains the last 30/30 checkpoint. Full Choc still has 40 invalid edge keys. See `.evidence/monstertruck-qualification/vendor-integration/circle-v34/REPORT.md`.

2026-10-17: Shared surface-v33 passes 526 combined native tests (18 inherited ignores), WASM compilation, and all 30 prior-PASS STEP corpus modes. Parts, bounds and loss accounting remain unchanged. Full checked Choc retains all eight parts and has zero missing/empty faces or degenerates but still fails with 40 invalid edge keys. The source-pole candidate remains private while an independent spherical coverage oracle distinguishes near-zero radial-area meridional facets from missing/overlapping source regions. An initial out-of-active-range classification hypothesis was not established for Choc and is withdrawn; it remains a separate bounded-wrapper negative control. A mesh-gate PASS alone is not acceptance. See `.evidence/monstertruck-qualification/vendor-integration/surface-v33/REPORT.md`.

2026-10-17: Reviewed source-identity repair is integrated as identity-dish-v32 with unchanged shared samples and stable inverse. Fresh shared native279/2 inherited ignores and WASM pass. The fresh pinned full 15-file prior-PASS replay restores all 30 default/checked-angular runs to PASS, with zero invalid/degenerate edges or missing/empty faces and unchanged parts/assembly/bounds/losses. Only six previously failing mode/file rows change triangle counts. Independent retained-triangle sample evidence reports no new missing references; existing exact-coordinate membership gaps remain explicit. See `.evidence/monstertruck-qualification/vendor-integration/identity-dish-v32/REPORT.md`.

2026-10-17: Reviewed derivative-v31 is integrated on chart-v30. The continuous active-upper derivative correction preserves rank-zero geometry and original linear samples on all nine affected Choc curves; public source/angular regressions and two killed mutations support the change. Fresh shared release IO/meshing/qualification checks pass 476 tests (18 inherited ignores), and WASM compilation passes. Exact-runtime geometry evidence retains two separately reproduced baseline Circle failures explicitly filtered; no general angular or whole-file validity is inferred. See `.evidence/monstertruck-qualification/vendor-integration/derivative-v31/REPORT.md`.

2026-10-17: Current15 prior-PASS comparison yields14 default/10 checked-angular passes. Four checked-only regressions (2inch,1.47inch,1.28inch-round,horizontal connector) retain all geometry/occurrences but gain invalid/degenerate edges; `/root/sol_step_bounded_trim` owns the isolated angular root fix. Crenshaw fails both routes and is separately assigned to `/root/sol_cylinder_oracle_review`. No completion or tolerance relaxation is authorized by lower overall defect counts. Receipt: `.evidence/monstertruck-qualification/step-v27-passing-corpus/RESULT.md`.

2026-10-17: The former Astra owner is terminal; root owns remaining combined acceptance. The reviewed angular implementation remains integrated from meshing-v21. The active checked STEP adapter lane now exercises its public source-certified route; no new mesher change is selected merely from a passing dish experiment.

2026-10-17: Combined reviewed closure/angular changes integrated into shared qualification meshing-v21. Source-domain review approves six repaired closed faces with exact no-interior-overlap checks and omitted boundary bands <=2.442e-7 model units. Shared checks:170 meshing /94 IO library /50 IO input passes; checked combined-source WASM reused after code/manifest parity verification. Whole-file refusals, broader source-edge incidence and production acceptance remain explicit; no blanket feature completion is claimed. See `.evidence/monstertruck-qualification/vendor-integration/meshing-v21/REPORT.md`.

2026-10-17: V5 permits near-UV closure compaction only for exactly equal XYZ points, preserving distinct shared samples. Owner reports 21 passing focused tests and restoration of #30304 distance/area and #11945 triangle count to v3 values. `/root/sol_angular_spec_v3` reviews the changed files; `/root/luna_angular_v5_gates` runs independent native/WASM checks. No shared integration until those results and combined closure-source checks pass.

2026-10-17: V4 fixes the reviewed shared-sample loss and passes 20 focused tests, but is rejected for integration pending downstream repair. Owner probes report a newly empty face #11945 and face #30304 sampled facet distance increasing from 0.08880475 mm to 0.78041218 mm, with XYZ area increasing from 10.93587 to 40.7109977 despite nearly unchanged UV area. Source-domain/constraint diagnosis continues in isolation; passing edge-turn tests do not establish whole-face acceptance.

2026-10-17: User approved this slice. Existing ownership and evidence carried forward; no implementation was moved, discarded or overwritten.

2026-10-17: Frozen angular v3 patch is under separate Standards and Spec reviews. Root inspected overlap with 001: tessellation entry points, triangulation routing and boundary projection. Preserve both reviewed behaviors and newer pole-v18 changes through an explicit combined-source merge and affected checks; do not replace vendor trees. No shared integration or ticket completion is claimed.

2026-10-17: Standards review (`../../../../.evidence/monstertruck-qualification/angular-v3-standards-review/REPORT.md`) of frozen v3 found no hard documented violations and one low-priority duplication concern in native/serial paths. Specification review remains active, including a scaled-surface counterexample for possible loss of distinct shared samples. No acceptance checkbox is closed from standards/build results alone.

2026-10-17: Spec review confirmed a public compressed-tessellation failure: a scaled plane retains original shared XYZ sample (0.5, 0, 0) on the topology edge but omits it from both incident face meshes because distinct UV coordinates are near-deduplicated. Public repro log (`../../../../.evidence/monstertruck-qualification/angular-v3-spec-review/public-close-uv.log`) blocks v3 integration despite passing tests. Existing owner is addressing the same ticket in a new isolated revision; frozen v3 is preserved.

- 2026-10-17 root: Integrated approved source-pole/XYZ retention and Processor endpoint transport as endpoint-v38, preserving prior test supplements. Combined verification: 559 native tests pass (19 inherited ignores), WASM passes, all 430 source hashes verify, and all 30 prior-PASS corpus modes pass with parts/assembly/bounds/loss accounting retained. Full Choc checked and checked/angular pass with zero mesh defects and eight parts; default still has 32 invalid edges. Remaining 19-file refresh and broader acceptance stay open. Receipt: `.evidence/monstertruck-qualification/vendor-integration/endpoint-v38/REPORT.md`.

- 2026-10-17 root resumed after app interruption: v39 qualification-only Polyline Range repair passes 598 native tests, WASM and 30 prior-PASS corpus modes with unchanged mesh counts/parts/bounds. Choc checked-only passes; checked/angular hit the recorded 90-second timeout and was reaped. Performance diagnosis compares pinned v38/v39 binaries under matched conditions; no passing angular result is claimed. Current source and all 451 files are retained under `.evidence/monstertruck-qualification/vendor-integration/mesh-range-v39/`.

- 2026-10-17 root: Shared Choc-v41 combined replay is complete: 612 native passes (19 inherited ignores), WASM release library compilation and normal gate build pass. Separate full Choc default, checked-only and checked/angular are all strict PASS with eight parts, zero mesh defects/missing/empty faces and no recorded source loss. All 30 prior-PASS corpus modes retain counts/parts/bounds/losses; original corpus remains 17/34 PASS per mode. All 71 invocations exit naturally/reaped without timeout. Four still-failing default meshes recover faces; their substantial volume changes remain unqualified. Root verifies 461 actual source files and copies, 98 remaining-corpus artifacts and every raw process/output/hash invariant. Broader trim/angular/source accuracy and production acceptance remain open; no acceptance checkbox is closed from strict mesh success. Receipt: `.evidence/monstertruck-qualification/vendor-integration/choc-v41/REPORT.md`.
