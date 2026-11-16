---
id: 3
type: story
title: "Preserve linked-resize axis and history metadata"
parent: none
sources:
  - docs/plans/initiative-layout-test-strength/tickets/epic-audit-harness/epic-audit-harness-retrospective.md
companions: []
after: []
refined: true
hitl: false
risk: low
---

# Preserve linked-resize axis and history metadata

## Description

Strengthen the accepted linked-resize test with independent mirror-axis expectations and history checks for layout ownership and link correspondence. This schedules retrospective A1 as test hardening, not as an established production defect or a change to story 2.1's accepted baseline.

## Requirements

1. **Independent accepted metadata** (retrospective A1)
   **Given** a Core-accepted synthetic linked pair whose layout ownership and mirror-link metadata, including axis and matrix correspondence, are captured before planning,
   **When** the real planner and typed resize commit run,
   **Then** reflected poses are checked against the independently captured accepted axis, and both layouts retain their IDs, board ownership, matrix/part memberships, link presence/source, axis, complete correspondence records and exceptions; expected metadata are not inferred solely from post-edit state. The existing legacy fixture explicitly adopts the right link's `split_gap` from `None` to `Some(142.0)`, with `spacing: None` and no left link. Establish this expectation independently before the act, without invoking production adoption helpers.
2. **Both-halves metadata history** (retrospective A1)
   **Given** the accepted and committed relevant-state snapshots,
   **When** Undo and Redo run,
   **Then** Undo restores both halves' complete accepted ownership/link metadata, including legacy `split_gap: None` and `spacing: None`, and Redo restores the independently validated committed metadata, including right-link `split_gap: Some(142.0)` and `spacing: None`, as well as the existing identities, poses, sizes and offsets. Full optional links remain in both snapshots; revision counters are not treated as stable state.
3. **Counter-example sensitivity and restoration** (retrospective A1)
   **Given** unchanged metadata assertions,
   **When** temporary test-act controls introduce an incorrect axis with correspondingly reflected partner placement, corrupt ownership/correspondence while retaining otherwise correct member geometry, or supply incorrect adoption metadata or incorrect metadata restoration at Undo/Redo boundaries,
   **Then** the named test fails at the relevant assertion; record each actual assertion failure and restore each control to a named pass with zero ignored tests.
4. **Preserve existing coverage** (retrospective A1)
   **Given** the restored final test,
   **When** the named/module tests and applicable normal project verification run,
   **Then** widths 18→56, source displacements −19/0/+19, partner sizes, six unique member identities, document/scene agreement and both-halves history checks still pass without external fixtures or ignore annotations.

## Boundaries

- Test-only hardening; preserve the real planner/Core seam and the working legacy, zero-angle fixture. Do not pre-adopt or otherwise repair the fixture, exclude gap/spacing metadata, or assume whole-link invariance across adoption.
- No production change, unrelated refactor, delivered target-mapping change or audit. A new failure against unchanged production is reported for a separate bug investigation, not ignored or silently fixed here.
- Temporary controls are never committed. Preserve accepted story/epic requirements and the existing protected files.

## References

- retrospective — docs/plans/initiative-layout-test-strength/tickets/epic-audit-harness/epic-audit-harness-retrospective.md, Action items A1 and adversarial reports 1–2
- accepted coverage — docs/plans/initiative-layout-test-strength/tickets/epic-audit-harness/tickets/story-make-linked-key-resize-coverage-reproducible-plan.md, Intent and Tasks & Acceptance

## Notes

- Decision: 2026-11-16 — user requested scheduling A1–A3. Queue A1 after the evidence-retention priority in working order, without inventing a technical dependency on A3. Schedule A2 after A1 because both extend the same test.
- Decision: 2026-11-16 — user approved the refined criteria and low-risk classification. Capture accepted metadata before planning, use the accepted axis independently, and extend both-halves relevant-state history coverage without revision counters. Build remains separately authorized; A2 continues to wait after A1.
- Decision: 2026-11-16 — user approved adoption-aware A1 criteria after read-only investigation, superseding whole-link commit invariance in the prior refinement. The right link legitimately adopts `split_gap: Some(142.0)` while `spacing` stays `None`; ownership, axis and correspondence remain stable. Full accepted/committed metadata stay in Undo/Redo snapshots. The observed failure is a criteria conflict, not a confirmed product defect. User subsequently authorized updating/committing this ticket and resuming A1 through bmad-build and its checkpoints; A2 remains waiting. No A3/A4 redo, pilot acceptance, epic closure or cleanup is authorized.
- Owner: Layout test maintainer (Amelia); risk low because no product behavior changes.
- Location: existing linked-resize test and local helpers in `web/crates/layout/src/objects/keycap_resize.rs`; do not change its accepted historical Build plan except separately authorized evidence labeling.
