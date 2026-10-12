# Issue tracker: committed Markdown tickets

Tickets are Markdown files in a plan's `issues/` directory, one per ticket, each named in that
plan's `map.md`. The active plan is [typed-core-edits](../plans/typed-core-edits/map.md), whose
tickets live in `docs/plans/typed-core-edits/issues/`.

## Conventions

- File `NNN-short-name.md`, where `NNN` is the next free number; a letter suffix marks a sibling
  of an existing ticket (`109b`). The title line is `# NNN: Title`.
- Header lines at the top (the first twelve lines): `Status:`, `Type:` (`build`, `research`,
  ...), `Blocked by:`, and `Parent:`, `Related:` or `Decision:` when they apply.
- `Status:` is one of `ready-for-agent` (an agent can start), `ready-for-human` (waits on a
  decision), `claimed`, `resolved`, `wontfix`.
- Name every new ticket in the plan's `map.md`.
- `scripts/check-tickets.py`, part of `python3 scripts/check.py repo`, enforces `Status` and the
  map listing for typed-core-edits; `scripts/check-tickets-baseline.json` holds the older
  exceptions.

## Building and verifying a ticket

The ticket's acceptance list names what to run.

## When a skill says "publish to the issue tracker"

Create the next numbered ticket as the Conventions describe: `Status: ready-for-agent` when an
agent can start, `ready-for-human` when it waits on a decision.

## When a skill says "fetch the relevant ticket"

Read `issues/<NNN>-*.md`; its `Blocked by`, `Parent` and `Related` lines link the neighbours.
