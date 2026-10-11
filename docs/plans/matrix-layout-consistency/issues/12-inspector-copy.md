# 12: Inspector copy matches the constraint domain and navigation action

Status: claimed
Type: build
Blocked by: 04, 09
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md)

## What to change

Make these two copy-only corrections in `web/src/presentation/inspector/layout_component_inspector.rs`:

1. Change the visible `Relationship` label for the constraint selector to `Constraint type`. This matches its existing accessible label and the explicit constraint vocabulary in `CONTEXT.md`.
2. Change the Relations navigation button label from `Edit placement relationship` to `Go to Properties`. Its handler only selects the Properties tab.

Add mounted copy assertions only in `web/src/presentation/layout_component_inspector_tests.rs`. The exact source and test paths are the only permitted implementation files. Do not change CSS, APIs, behavior, waits, helpers, or fixtures. Ticket 37's wait-conversion work remains deferred; no other assertions in its shared test file are leased.

## Acceptance

- Mounted expectations confirm both visible labels and accessible names.
- Constraint selection and submission semantics remain unchanged; the navigation action still opens Properties.
- Run the affected `typecheck` and `browser` steps, plus required `repo` check.
- A fresh independent Sol 6.1 xhigh verifier confirms the exact copy and mounted behavior.

This ticket covers two domain-accurate copy corrections only. It does not establish that Inspector visual polish is complete: isolated live-browser observation was unavailable and no rendered CSS defect was established.

## Evidence

Readiness proposal: `/home/chris/.cache/boardstudio-baselines/inspector-readiness/proposal-56de07a7.md` (accepted proposal for the exact copy scope and limitations).

## Outcome

Record changed files, checks, mounted assertions, independent verification, and remaining visual limitations.
