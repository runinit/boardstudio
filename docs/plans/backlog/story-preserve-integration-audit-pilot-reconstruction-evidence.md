---
id: 2
type: story
title: "Preserve integration-audit pilot reconstruction evidence"
parent: none
sources:
  - docs/plans/initiative-layout-test-strength/tickets/epic-audit-harness/epic-audit-harness-retrospective.md
companions: []
after: []
refined: false
hitl: false
risk: low
---

# Preserve integration-audit pilot reconstruction evidence

## Description

Preserve the completed pilot's effective exclusions and Cargo-forwarding behavior as inert, reproducible evidence before its disposable checkout is removed. This schedules retrospective A3; it does not authorize rerunning or expanding the audit.

## Requirements

1. **Effective candidate boundary** (retrospective A3)
   **Given** the recorded pilot revision and retained disposable configuration,
   **When** the evidence is documented,
   **Then** the existing pilot plan preserves the exact exclusion text, tool versions, temporary target mapping and invocation/reconstruction details, including the three excluded closure-bearing tests and two zero-candidate helper-only tests.
2. **Faithful forwarding reconstruction** (retrospective A3)
   **Given** the retained forwarding implementation and execution logs,
   **When** its exact text or equivalent complete reconstruction specification is recorded as inert evidence,
   **Then** a reader can reconstruct argument/environment handling, working directory, stdout/stderr, normal and signal exit-status mapping, and FD9 inheritance without guessing or requiring the original disposable checkout.
3. **Evidence and authorization boundaries** (retrospective A3)
   **Given** the completed run's evidence,
   **When** the documentation change is checked,
   **Then** its claims match the retained files/logs and documentation checks pass; unavailable evidence is reported as a blocker rather than reconstructed speculatively.

## Boundaries

- Must not change: completed pilot outcomes, exclusion limitations, source revision, recorded approvals or frozen Build intent.
- Documentation only: no new audit, mutation, build, checkout/database creation, cleanup or wider-audit authorization.
- Deliver no executable scratch tool, operational temporary mapping/configuration, or database; reconstruction text belongs in the existing plan, not a parallel validation record.
- The existing parent database and protected repair retrospective remain unread and untouched.

## References

- retrospective — docs/plans/initiative-layout-test-strength/tickets/epic-audit-harness/epic-audit-harness-retrospective.md, Action items A3 and adversarial report 7
- evidence — docs/plans/initiative-layout-test-strength/tickets/epic-audit-harness/tickets/spike-prove-and-calibrate-the-integration-audit-pilot-plan.md, Pilot evidence and Verification

## Notes

- Decision: 2026-11-16 — user requested scheduling A1–A3; prioritize A3 before disposable cleanup. This independent backlog follow-up does not reopen the accepted epic's requirements or start implementation.
- Owner: audit evidence maintainer; risk low because only evidence documentation changes.
- Location: preserve evidence in the existing spike plan; read only the retained configuration/forwarder and relevant logs in the recorded disposable checkout, never either database.
