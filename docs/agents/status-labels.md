# Status labels

> Retired workflow — 2026-11-15. The body below is preserved only to interpret historical records, not as operating instructions. Installed BMad skills now govern planning, ticketing, orchestration, review and verification; these roles, statuses, routes and process gates impose no additional requirements.

Tickets use five status roles. Each role's label is the string below, recorded as the
ticket's `Status:` line.

| Role | Label | Meaning |
|---|---|---|
| `unstatusd` | `unstatusd` | A maintainer needs to evaluate this ticket |
| `awaiting-info` | `awaiting-info` | Waiting on the reporter for more information |
| `ready` | `ready` | Fully specified, ready for an agent to start |
| `awaiting-decision` | `awaiting-decision` | Waits on a decision or human work |
| `declined` | `declined` | Will not be actioned |

`claimed` and `resolved` are workflow states, not status roles: a ticket is `claimed` while
someone works it and `resolved` once it is done.
