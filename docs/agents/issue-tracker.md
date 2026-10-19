# Issue tracker: committed Markdown tickets

Tickets are Markdown files in a plan's `issues/` directory, one per ticket, each named in that
plan's `map.md`. The active plan is [typed-core-edits](../plans/typed-core-edits/map.md), whose
tickets live in `docs/plans/typed-core-edits/issues/`.

## Conventions

- File `NNN-short-name.md`, where `NNN` is the next free number; a letter suffix marks a sibling
  of an existing ticket (`109b`). The title line is `# NNN: Title`.
- Header lines at the top (the first twelve lines): `Status:`, `Type:` (`build`, `research`,
  ...), `Blocked by:`, and `Parent:`, `Related:` or `Decision:` when they apply.
- `Status:` is one of the five triage roles in [triage-labels.md](triage-labels.md)
  (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`) or a workflow
  state: `claimed` while someone works it, `resolved` once it is done.
- Comments and conversation append at the bottom of the ticket under `## Comments`.
- Name every new ticket in the plan's `map.md` (`scripts/check-doc-links.py` in `python3 scripts/check.py repo` verifies every Markdown link).

## Building and verifying a ticket

The ticket's acceptance list names what to run.

## When a skill says "publish to the issue tracker"

Create the next numbered ticket as the Conventions describe: `Status: ready-for-agent` when an
agent can start, `ready-for-human` when it waits on a decision.

## When a skill says "fetch the relevant ticket"

Read `issues/<NNN>-*.md`; its `Blocked by`, `Parent` and `Related` lines link the neighbours.

## Wayfinding operations

Used by `/wayfinder`. The **map** is a file with one **child** ticket per question.

- **Map**: `docs/plans/<effort>/map.md`, with its notes, decisions so far and open fog.
- **Child ticket**: `docs/plans/<effort>/issues/NNN-slug.md`, with the question in the body, a
  `Type:` line (`research`, `prototype`, `grilling`, `task`) and a `Status:` line.
- **Blocking**: a `Blocked by:` line. A ticket is unblocked when every ticket it lists is
  `resolved`.
- **Frontier**: the tickets that are open, unblocked and unclaimed; the lowest number goes first.
- **Claim**: set `Status: claimed` and save before any work.
- **Resolve**: append the answer under an `## Answer` heading, set `Status: resolved`, then add a
  pointer (gist and link) to the map's decisions so far.
