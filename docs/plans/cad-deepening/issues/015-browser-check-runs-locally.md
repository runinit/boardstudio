# 015: Browser check runs on the development machine

Status: ready-for-human
Type: task
Blocked by: none
Parent: [CAD deepening](../map.md)

## What to build

The full check stops before its browser step because the drag smoke test needs
`playwright-core` on `NODE_PATH`. CI installs it through its web-tools action. Decide how a
developer machine gets it, then make `check.py` run the browser step end to end.

## Acceptance criteria

- [ ] The chosen install route is documented where the README's commands are.
- [ ] `python3 scripts/check.py browser` runs to completion locally.

## Comments
