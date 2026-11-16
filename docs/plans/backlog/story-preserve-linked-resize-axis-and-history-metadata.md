---
id: 3
type: story
title: "Preserve linked-resize axis and history metadata"
parent: none
sources:
  - docs/plans/initiative-layout-test-strength/tickets/epic-audit-harness/epic-audit-harness-retrospective.md
companions: []
after: []
refined: false
hitl: false
risk: low
---

# Preserve linked-resize axis and history metadata

## Description

Strengthen the accepted linked-resize test with independent mirror-axis expectations and history checks for layout ownership and link correspondence. This schedules retrospective A1 as test hardening, not as an established production defect or a change to story 2.1's accepted baseline.

## Requirements

1. **Independent accepted metadata** (retrospective A1)
   **Given** a Core-accepted synthetic linked pair and its pre-edit mirror axis, layout ownership and correspondence,
   **When** the real planner and typed resize commit run,
   **Then** reflected poses are checked against the captured accepted axis, and the expected ownership/link metadata remain unchanged rather than being inferred solely from post-edit state.
2. **Both-halves metadata history** (retrospective A1)
   **Given** the accepted and committed relevant-state snapshots,
   **When** Undo and Redo run,
   **Then** each restores both halves' expected ownership/link metadata as well as the existing identities, poses, sizes and offsets; revision counters are not treated as stable state.
3. **Counter-example sensitivity and restoration** (retrospective A1)
   **Given** unchanged metadata assertions,
   **When** temporary test-act controls introduce an incorrect axis with correspondingly reflected partner placement, or corrupt ownership/correspondence while retaining otherwise correct member geometry,
   **Then** the named test fails at the relevant assertion; restoring each control returns it to a named pass with zero ignored tests.
4. **Preserve existing coverage** (retrospective A1)
   **Given** the restored final test,
   **When** the named/module tests and applicable normal project verification run,
   **Then** existing numeric reflow, widths, partner sizes, unique member identities and both-halves history checks still pass without external fixtures or ignore annotations.

## Boundaries

- Test-only hardening; preserve the real planner/Core seam and the working zero-angle fixture.
- No production change, unrelated refactor, delivered target-mapping change or audit. A new failure against unchanged production is reported for a separate bug investigation, not ignored or silently fixed here.
- Temporary controls are never committed. Preserve accepted story/epic requirements and the existing protected files.

## References

- retrospective — docs/plans/initiative-layout-test-strength/tickets/epic-audit-harness/epic-audit-harness-retrospective.md, Action items A1 and adversarial reports 1–2
- accepted coverage — docs/plans/initiative-layout-test-strength/tickets/epic-audit-harness/tickets/story-make-linked-key-resize-coverage-reproducible-plan.md, Intent and Tasks & Acceptance

## Notes

- Decision: 2026-11-16 — user requested scheduling A1–A3. Queue A1 after the evidence-retention priority in working order, without inventing a technical dependency on A3. Schedule A2 after A1 because both extend the same test.
- Owner: Layout test maintainer (Amelia); risk low because no product behavior changes.
- Location: existing linked-resize test and local helpers in `web/crates/layout/src/objects/keycap_resize.rs`; do not change its accepted historical Build plan except separately authorized evidence labeling.
