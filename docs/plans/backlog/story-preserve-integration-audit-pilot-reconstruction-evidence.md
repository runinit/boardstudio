---
id: 2
type: story
title: "Preserve integration-audit pilot reconstruction evidence"
parent: none
sources:
  - docs/plans/initiative-layout-test-strength/tickets/epic-audit-harness/epic-audit-harness-retrospective.md
companions: []
after: []
refined: true
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
   **Then** the existing pilot plan preserves the exact two-line effective exclusion configuration as inert text, identifies the retained default assertion exclusions, and names the six excluded helpers, three excluded closure-bearing tests and two zero-candidate helper-only tests.
   **And** it preserves the recorded tool versions, audited source revision, temporary target name/path and invocation/reconstruction details without implying that excluded tests were audited for removals.
2. **Faithful forwarding reconstruction** (retrospective A3)
   **Given** the retained forwarding implementation and execution logs,
   **When** its exact text or equivalent complete reconstruction specification is recorded as inert evidence,
   **Then** a reader can reconstruct forwarding to the recorded Cargo executable with unchanged arguments and inherited environment/working directory, separate stdout/stderr capture and replay, and the log destination, naming and record format without requiring the original disposable checkout.
   **And** the evidence specifies normal exit-code forwarding, signal mapping to 128 plus the signal number, and FD9 inheritance only when the recorded lock environment variable equals "9"; it does not claim live output streaming or preserved cross-stream ordering.
3. **Evidence and authorization boundaries** (retrospective A3)
   **Given** the completed run's evidence,
   **When** the documentation change is checked by read-only comparison with the retained configuration, mapping, forwarder and relevant logs,
   **Then** each reconstruction claim agrees with that evidence without executing the reconstructed material, and any required documentation-only checks pass.
   **And** unavailable or contradictory evidence is reported as a blocker rather than reconstructed speculatively.

## Boundaries

- Must not change: completed pilot outcomes, exclusion limitations, source revision, recorded approvals, frozen Build intent or existing ticket/epic statuses.
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
- Location: retained disposable checkout `/home/chris/.local/share/opencode/worktree/3ca6f9/integration-audit-pilot-2-2`; inspect `necessist.toml`, `core/Cargo.toml` and `.scratch/pilot-bin/cargo` read-only. Logs are under parent `.evidence/integration-audit-pilot-2-2/`; the destination is the existing spike plan named in References.
- Provenance: completed pilot commit `086fa1f21725e3868460a66b328328b9ad5fdb13`; audited source revision `b8d7011a259736ec4838161a095fc6aee0779706`. These identify different stages and must not be substituted for each other.
- Decision: 2026-11-16 — user approved this refinement and recording it in a ticket-only commit, not Build or evidence implementation. Exclusions describe the completed pilot, not a new exclusion policy. A3 remains independent with retention priority; A1/A2 are outside this ticket.
