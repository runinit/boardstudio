# 002: Diagnose MCU rotation

Status: ready-for-agent
Type: research
Blocked by: none
Parent: [001: Recovery specification](001-ui-integration-recovery-spec.md)
Related: [Plan map](../map.md), [Audit A1](../../../research/2026-11-06-dev-ui-integration-audit.md#a1--mcu-fix-integrated-observed-symptom-unresolved)

## What to build

Establish why the user's mirrored MCU appears incorrectly oriented in the actual
affected project and loaded build. Deliver a repeatable reproduction, a minimized
fixture and demonstrated cause that define the bounded correction for
[003](003-resolve-mcu-orientation.md). This ticket diagnoses the symptom; it does
not assume a missing cherry-pick or implement a speculative repair.

## Acceptance criteria

- [ ] Follow `diagnosing-bugs`: establish a red-capable command for the user's
  exact symptom before developing a cause. Record an actual failing invocation
  and its output, with enough fixture/setup information to repeat it.
- [ ] Identify the affected project and loaded artifact, and record the relevant
  part definition kind, mirror linkage, stored pose, evaluated pose and rendered
  transform for both halves. Distinguish stored half-frame values from evaluated
  board-space values.
- [ ] Reproduce repeatedly and minimize the scenario without losing the symptom.
  If the necessary project/build cannot be obtained or the symptom cannot be
  reproduced, leave this ticket open with the specific missing input or blocker.
- [ ] After reproduction, test ranked, falsifiable hypotheses and demonstrate the
  cause with targeted observations. The integrated heading helper's presence or
  an old commit's reachability is not sufficient evidence either way.
- [ ] Determine whether the discrepancy affects accepted geometry, rendering,
  persistence or the loaded artifact, and identify which output checks are
  relevant. Record uncertainty rather than inferring export corruption.
- [ ] Define the smallest evidence-backed correction and regression seam for 003.
  If multiple independent causes or new product decisions emerge, record the
  necessary re-scoping before implementation.
- [ ] Preserve the reproduction for 003 and remove disposable instrumentation
  that is not needed to repeat it. Record the controller-heading documentation
  discrepancy and the evidence needed to reconcile it under A8.

## Verification and completion

Run and record the original reproduction, minimized reproduction and the probes
that distinguish the demonstrated cause. No broad product gate is required for
this research ticket. Diagnosis completion does not close A1; 003 owns the
verified outcome. Keep the original audit and parent specification unchanged.

## Comments

- 2026-11-06: Published from the accepted breakdown. Reproduction and diagnosis
  have not been performed by this planning step.
