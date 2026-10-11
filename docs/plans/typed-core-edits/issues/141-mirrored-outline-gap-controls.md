# 141 — Diagnose mirrored outline gap controls

Status: claimed
Type: task
Blocked by: none

## Report and scope

The user reports two sets of gaps in the Inspector and needing to uncheck both a left and right gap before they are ignored. Establish the actual control identities, linked-outline ownership and generated effect before changing shared behavior. Whether the report involves one outline spanning both halves or separate outlines is awaiting clarification; this does not block observational reproduction.

L3/E owns diagnosis and any subsequently confirmed Inspector/adapter fix; coordinate a Core reconciliation defect with L1/B [108](108-reconcile-mirrored-outlines-assemblies.md). Preserve independent per-outline settings unless the linked contract proves they should propagate. Do not collapse controls merely because they look similar. Unchecking protection permits outline repair and does not necessarily remove support geometry.

## Acceptance

- Reproduce with an independently specified minimal synthetic linked pair; no demo board fixtures.
- Identify each displayed control by board, outline feature and gap identity, including correspondence between halves.
- Exercise the real Inspector edit/resolver route, one side at a time from fresh baselines. Record protection state and generated contour effects on both halves.
- Distinguish duplicate rendering, lost linked propagation, legitimate independent settings and geometry-generation behavior with actual evidence.
- If a defect is confirmed, retain an expected-failing regression before the smallest coherent fix, then affected native and actual mounted WASM checks plus one fresh Sol6.1 medium review. A product ownership question is escalated with the concrete scenario; do not invent coupling.

## Execution authority

E may prepare a minimal observation/regression fixture in `web/crates/layout/src/outline_lifecycle_tests.rs` and `web/crates/layout/src/outline_lifecycle_browser_tests.rs`. No Core edits or API/schema expansion are granted by this ticket. Report any different required paths before editing. Source-bound impact and complete change-analysis gates apply. Runtime checks await a GLOBAL HEAVY grant with four build jobs, four native test threads and one suite at a time; no full lane suite.

## Evidence so far

L3 preparation: `/home/chris/.cache/boardstudio-baselines/mirrored-outline-gaps/repro-preparation.json`, binding eight source hashes and existing native/mounted seams. Inspector rows follow selected board projection gaps and edit individual gap IDs; creation mirrors gap identity/points and `keep_gaps`, but this does not prove ongoing propagation. No runtime reproduction, root cause or fix is claimed yet.
