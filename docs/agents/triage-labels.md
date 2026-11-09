# Triage labels

The skills speak in five triage roles. Each role's label is the string below, recorded as the
ticket's `Status:` line.

| Role | Label | Meaning |
|---|---|---|
| `needs-triage` | `needs-triage` | A maintainer needs to evaluate this ticket |
| `needs-info` | `needs-info` | Waiting on the reporter for more information |
| `ready-for-agent` | `ready-for-agent` | Fully specified, ready for an agent to start |
| `ready-for-human` | `ready-for-human` | Waits on a decision or human work |
| `wontfix` | `wontfix` | Will not be actioned |

`claimed` and `resolved` are workflow states, not triage roles: a ticket is `claimed` while
someone works it and `resolved` once it is done.
