# Issue tracker: committed Markdown plans

Specs, wayfinder maps and tickets live as committed Markdown under `docs/plans/`,
so every checkout (and every agent) sees the same plan. `.scratch/` is gitignored
and is only for throwaway local notes.

## Conventions

- One effort per directory: `docs/plans/<effort-slug>/`.
- The spec is `docs/plans/<effort-slug>/spec.md`; the wayfinder map is `map.md`.
- Tickets are one file each at `docs/plans/<effort-slug>/issues/<NN>-<slug>.md`,
  numbered from `01` in dependency order. Never combine tickets in one file.
- Each ticket has `Status:`, `Type:` and `Blocked by:` lines near the top.
  Status values: `ready-for-agent`, `ready-for-human`, `needs-info`, `claimed`,
  `implemented (awaiting verification)`, `resolved`, `wontfix`. Type values: `build` (implementation slice), or the
  wayfinder types `research`, `prototype`, `grilling`, `task`.
- A ticket is unblocked when every ticket in its `Blocked by` line is `resolved`.
  The frontier is the open, unblocked, unclaimed tickets; lowest number first.
- Claim a ticket by setting `Status: claimed` and committing that change before
  any other work. Resolve it by appending `## Answer` (decisions) or `## Outcome`
  (builds: commits, checks run, follow-ups), setting `Status: resolved`, and adding
  a one-line pointer to the map's "Decisions so far" or "Progress" section.
- `implemented (awaiting verification)` is the implementer's last status: the change and its
  Outcome are written but a separate verifier has not reproduced them. It counts as open (not
  `resolved`), so it does not unblock dependents. Only the verifier sets `resolved`; see
  [verification-process.md](verification-process.md).
- Comments append under a `## Comments` heading at the bottom of the file.
- Markdown here is covered by `python3 scripts/check-doc-links.py`; keep relative
  links valid.

## Building and verifying a ticket

Implementation and verification roles, which check steps to run for which change, and the
one-run-at-a-time rule are in [verification-process.md](verification-process.md).

## When a skill says "publish to the issue tracker"

Create the file under `docs/plans/<effort-slug>/` and commit it.

## When a skill says "fetch the relevant ticket"

Read the file at the referenced path.
