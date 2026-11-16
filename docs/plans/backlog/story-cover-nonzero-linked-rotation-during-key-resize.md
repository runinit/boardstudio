---
id: 4
type: story
title: "Cover nonzero linked rotation during key resize"
parent: none
sources:
  - docs/plans/initiative-layout-test-strength/tickets/epic-audit-harness/epic-audit-harness-retrospective.md
companions: []
after: [3]
refined: false
hitl: false
risk: low
---

# Cover nonzero linked rotation during key resize

## Description

Add a synthetic linked-half resize scenario whose accepted nonzero rotation distinguishes reflection from copying the source angle. This schedules retrospective A2 alongside, not instead of, the existing zero-angle reflow case and builds on A1's metadata oracle.

## Requirements

1. **Nonzero linked rotation** (retrospective A2)
   **Given** a Core-accepted synthetic linked pair with a nonzero source angle that is not reflection-equivalent to itself,
   **When** the real planner and typed key-size resize commit execute,
   **Then** the linked partner has the independently expected reflected rotation and translated pose, with the requested sizes, stable identities and preserved metadata.
2. **Rotation regression detection** (retrospective A2)
   **Given** the unchanged nonzero-angle assertions,
   **When** a temporary test-act control copies source rotation to the partner instead of reflecting it while retaining otherwise valid state,
   **Then** the named test fails at its rotation oracle, and restoring the control produces a named pass with zero ignored tests.
3. **Retain paired history and the original case** (retrospective A2)
   **Given** the accepted and committed nonzero states plus the existing zero-angle scenario,
   **When** Undo/Redo and the named/module tests run,
   **Then** both halves restore the expected rotation and relevant state, and the existing numeric size/reflow/identity/metadata coverage continues to pass under applicable normal project verification.

## Boundaries

- Test-only; retain the working zero-angle fixture and A1's independent axis/metadata checks.
- No production changes, external archive dependency, ignored replacement, target-mapping changes or audit.
- This is end-to-end planner/Core linked coverage, not isolated proof of Core source-only propagation. Independent moves/offsets or size-only controls remain outside this ticket.
- A new test failure against unchanged production is reported for separate bug investigation; temporary controls are never committed. Protected files remain untouched.

## References

- retrospective — docs/plans/initiative-layout-test-strength/tickets/epic-audit-harness/epic-audit-harness-retrospective.md, Action items A2 and adversarial report 3
- accepted coverage — docs/plans/initiative-layout-test-strength/tickets/epic-audit-harness/tickets/story-make-linked-key-resize-coverage-reproducible-plan.md, Intent and Verification
- prerequisite — docs/plans/backlog/story-preserve-linked-resize-axis-and-history-metadata.md, Requirements

## Notes

- Decision: 2026-11-16 — user requested scheduling A1–A3. This ticket waits on backlog story 3 (A1), whose metadata oracle it must retain; they share the linked-resize test lane. A3 is independent and has working priority before either.
- Owner: Layout test maintainer (Amelia); risk low because no product behavior changes.
- Location: linked-resize coverage in `web/crates/layout/src/objects/keycap_resize.rs`; a neighboring single-matrix rotated test is not a substitute for this explicit linked pair.
