# Map: Split board editing

Label: wayfinder:map

## Destination

This planning map is finished when split boards created in BoardStudio behave as one
editable board during layout work. That means:

- Split boards never collapse into one merged outline when an MCU is added during
  onboarding (adaptive gap vs board size);
- The mirrored half does not read as secondary during layout editing until clicked;
- Anchor/relationship structure is visible at a glance in Inspector/canvas now that
  Relations is being retired.

## Notes

- Builds on ergogen-layout-parity (resolved), [ADR-0034](../../adr/0034-matrices-follow-ergogen-point-layout.md),
  and tickets [09](../ergogen-layout-parity/issues/09-inspector-and-toolbar-prototype.md) /
  [12](../ergogen-layout-parity/issues/12-relations-and-highlight.md) /
  [24](../ergogen-layout-parity/issues/24-relations-and-highlights.md) decisions.
- Primary source for the lost highlighting: local branch
  `prototype/ergogen-parity-inspector`.
- Use `grilling` + `domain-modeling` for grilling tickets, `prototype` for the prototype
  ticket, `research` for research tickets.
- Tracker: [docs/agents/issue-tracker.md](../../agents/issue-tracker.md), Wayfinding
  operations.

## Tickets

```text
01 Split gap and outline merge (research) ────────────────┐
02 Recover prototype anchor highlighting (research) ───────┤
03 Dim mirrored-half editing focus (research) ──────────────┤
├── 04 Decide adaptive split gap (grilling, after 01) ──────┤
├── 05 Decide dim and focus interaction (grilling, after 03) ┤
├── 06 Decide relationship highlighting home (grilling, after 02)
└── 07 Prototype split editing view (prototype, after 04, 05, 06) ── 08 Slice the build
```

- [01: Split gap and outline merge](issues/01-split-gap-and-outline-merge.md)
- [02: Recover prototype anchor highlighting](issues/02-recover-prototype-anchor-highlighting.md)
- [03: Dim mirrored-half editing focus](issues/03-dim-mirrored-half-editing-focus.md)
- [04: Decide adaptive split gap](issues/04-decide-adaptive-split-gap.md)
- [05: Decide dim and focus interaction](issues/05-decide-dim-and-focus-interaction.md)
- [06: Decide relationship highlighting home](issues/06-decide-relationship-highlighting-home.md)
- [07: Prototype split editing view](issues/07-prototype-split-editing-view.md)
- [08: Slice the build](issues/08-slice-the-build.md)

## Decisions so far

(none yet)

## Not yet specified (fog)

- Exact clearance formula (stored gap vs derived).
- Whether existing saved boards get re-spaced or keep their gap.
- Dimming in PCB/Case modes vs Layout only.
- MCU placement snap/clearance during onboarding drags.
- Whether relationship highlighting needs canvas handles or Inspector-only.

## Out of scope

- Ergogen YAML import/export.
- Changing the 19.05mm default pitch.
- Removing Relations data/facts (only its Inspector presentation may move).
- Aggregate-anchor changes.
