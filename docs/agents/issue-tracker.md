# Issue tracker: committed Markdown tickets

Tickets are Markdown files in [`docs/plans/typed-core-edits/issues/`](../plans/typed-core-edits/issues/),
one per ticket, and each is named in [map.md](../plans/typed-core-edits/map.md). Model
assignments, orchestration roles, lane leases, mandatory tool/review gates and command-window rules
were retired at the user's request on 2026-10-11 and do not apply. Technical tickets and
implementation evidence remain available separately.

## Conventions

- File `NNN-short-name.md`, where `NNN` is the next free number; a letter suffix marks a sibling
  of an existing ticket (`109b`). The title line is `# NNN: Title`.
- Header lines directly under the title: `Status:`, `Type:` (`build`, `research`, ...),
  `Blocked by:`, and `Parent:`, `Related:` or `Decision:` when they apply.
- `Status:` is one of `ready-for-agent` (an agent can start), `ready-for-human` (waits on a
  decision), `claimed`, `resolved`, `wontfix`.
- Name every new ticket in `map.md`. `scripts/check-tickets.py`, part of
  `python3 scripts/check.py repo`, fails for an unknown `Status` or an unlisted ticket;
  `scripts/check-tickets-baseline.json` holds the older exceptions.

## Building and verifying a ticket

Commands are in [README.md](../../README.md); the ticket's own acceptance list says what to run.

## When a skill says "publish to the issue tracker"

Create the next numbered file in the issues directory with the header above, set `Status:
ready-for-agent` for work an agent can start or `ready-for-human` when a decision is needed, and
add it to `map.md`.

## When a skill says "fetch the relevant ticket"

Read `docs/plans/typed-core-edits/issues/<NNN>-*.md`. Its `Blocked by`, `Parent` and `Related`
lines link the neighbouring tickets.
