---
status: accepted
---

# A part-axes offset is its own constraint kind

An Offset constraint places its target at `source.at + offset` on the board's axes and turns it
relative to the source. A companion held to a splayed key therefore turns with the key but
drifts off it. On 2026-10-12 the user decided that Core gains a second persisted kind,
`part-offset`, whose offset runs along the source part's own axes, and leaves `offset` exactly as
it is.

The representation is the decision. A property on Offset was the obvious shape, but `Constraint`
has no `deny_unknown_fields` and `CURRENT_VERSION` is 5, so an older build would ignore the
property, solve the target on board axes, and drop the property when it saved: a silent
misplacement and a permanent loss. [ADR-0004](0004-document-format-version-and-id-rename.md)
sets the rule that a build refuses what it cannot read. A format bump satisfies the rule only by
refusing every document the new build saves; a new tagged kind refuses only the documents that
use it, and needs no bump.

## Considered options

- **A property on Offset, no bump.** Rejected: silent misplacement in older builds.
- **A property on Offset plus format 6.** Rejected: every saved document becomes unreadable to
  older builds, and format 6 is the step ticket 139 planned to own.
- **Every Offset becomes part-axes.** Rejected: the stored numbers are identical in both
  meanings, so intent cannot be recovered, and the board-axis placement disappears.
- **Meaning chosen by whether the source is a matrix key.** Rejected: the meaning would flip when
  the part left its matrix.

## Consequences

- Every exhaustive `match` on `Constraint` gains an arm; the compiler lists them.
- Converting between the kinds is one Core operation that preserves geometry, so a user never
  moves a part by changing the kind.
- `part-offset` is the Attach half of the Attach/Align end state and `offset` is the Align half;
  a later rename is mechanical.
- Ticket 139's framed relations must compile `part-offset` when they compile legacy records.
