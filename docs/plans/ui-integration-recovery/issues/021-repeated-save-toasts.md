# 021: Stop repeated save Toasts during routine interactions

Status: needs-triage
Type: bug
Blocked by: none
Parent: [001: Recovery specification](001-ui-integration-recovery-spec.md)
Related: [Plan map](../map.md)

## Report

The user reports that the save notification appears far too often: even zooming
can produce dozens of save Toasts. Repeated success notifications flood the
interface during ordinary use and obscure useful feedback.

This is a user-observed symptom, not an agent-reproduced failure. The affected
build, exact interaction sequence and cause have not been established. Do not
assume zoom should never persist view state or that the durable save is faulty.

## Desired outcome

Routine interaction and automatic saving provide calm, accurate save feedback
without repeatedly stacking save-success Toasts. Keep meaningful failure and
recovery feedback visible and preserve the intended persistence behavior.

## Acceptance criteria

- [ ] Reproduce the reported Toast flood on an identified build, including
  repeated zooming. Trace whether it comes from distinct saves, replayed
  notifications, or presentation lifecycle, and record the demonstrated cause.
- [ ] Repeated zooming and representative routine interactions do not generate
  a flood of save-success Toasts. Record the intended success-notification policy
  before implementing its regression coverage.
- [ ] Save-status feedback remains accurate and durable saves still complete;
  suppressing Toasts must not hide failed saves or disable required persistence.
- [ ] Preserve meaningful failure/retry/recovery feedback and unrelated result
  notifications, including exports and the waiting-update notice.
- [ ] Demonstrate RED/GREEN at the real notification boundary and verify the
  reported interaction in the packaged application. Update relevant records
  with actual tested source and scope.

## Verification

Use `diagnosing-bugs` to establish the actual reproduction before choosing a
repair. Prefer existing mounted production notification and real Runtime/Session
workflow seams; use a packaged browser scenario for the reported zoom behavior.
Record the notification count and relevant save outcomes without substituting
notification suppression for persistence proof.

## Comments

- 2026-11-06: Filed at the user's request: “the save notification pops up way to
  many times. even when doing things like zooming I will see it dozens of times
  via the toast”. No investigation or implementation performed by filing it.
