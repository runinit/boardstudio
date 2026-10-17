# 003: Support contained blind and partial-depth prism cuts

Status: resolved
Type: build
Blocked by: None
Owner: /root/astra_boolean_high

## What to build

A valid contained cutter removes the requested material for blind, one-flush and partial-depth prism cuts, preserving the intended floor, openings and shared topology.

## Acceptance criteria

- [x] Reproduce unsupported blind/partial-depth behavior before implementing the new operation.
- [x] Certify containment, source surface eligibility and two-sided material states; retain source curves/surfaces and shared section identities through adjacent walls.
- [x] Validate closed oriented output, solid counts, independent material probes and volume/geometry expectations for blind, one-flush, nonflush-through and enclosed-cavity cases.
- [x] Retain through-cut regressions and explicit refusal for unsupported crossing/contact/general-surface cases; refusal must not become empty or successful output.
- [x] Pass native/WASM checks and independent review of the exact combined candidate. General Boolean parity remains a separate goal requirement.

## Existing work

The curved-face component has bounded review approval for the existing contained-through route. Blind/partial-depth behavior is the new work; the component alone does not resolve generic dispatch. Continue the current isolated assignment; do not restart it. Evidence: [astra-boolean-topology/curved-planar-cells-v1](../../../../.evidence/monstertruck-qualification/astra-boolean-topology/curved-planar-cells-v1/). The [qualification record](../../../research/cad/monstertruck-qualification-2026-10-16.md) identifies shared source and broader acceptance limits.

## Comments

2026-10-17: V2 review found no hard Standards violations (two judgment-call smells) and no bounded Spec blocker. Independent Spec probes pass 22 depth/face-order configurations. An isolated combination with current shared sources passes 22 native tests (one inherited skip) and WASM compilation; [preflight receipt](../../../../.evidence/monstertruck-qualification/blind-v2-combined-preflight/REPORT.md). Shared integration remains pending.

2026-10-17: User approved this slice. Existing ownership and evidence carried forward; no implementation was moved, discarded or overwritten.

2026-10-17: The [frozen v2 candidate](../../../../.evidence/monstertruck-qualification/astra-boolean-topology/curved-planar-cells-v2/REPORT.md) reports a genuine pre-fix blind-cut failure, 22 passing native tests (one inherited ignored baseline), and passing WASM compilation. Standards review is assigned to `/root/luna_blind_standards_v2`; independent geometry/spec review to `/root/sol_blind_spec_v2`. No shared integration or full Boolean acceptance is claimed pending review.

2026-10-17: Integrated as [blind-v19](../../../../.evidence/monstertruck-qualification/vendor-integration/blind-v19/REPORT.md); shared native/WASM checks pass. Ticket scope is complete; general Boolean and production replacement scope remain open. Whole-package formatting debt is confined to unchanged gate binaries.
