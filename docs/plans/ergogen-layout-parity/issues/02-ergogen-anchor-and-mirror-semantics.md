# 02: Pin ergogen's anchor and mirror semantics

Status: ready-for-agent
Type: research
Blocked by: none
Map: [map.md](../map.md) · Decision: [ADR-0034](../../../adr/0034-matrices-follow-ergogen-point-layout.md)

## Question

Ticket 07 (anchors) and the parity fixtures need ergogen's exact rules for:

- zone anchors: `ref`, `shift`, `rotate`, `orient`, `affect` (including partial x/y/r), and the
  order these apply in;
- `resist`, and how relative shifts flip on mirrored (`mirror_`) points;
- referencing a key of an earlier zone, and of a mirrored zone;
- the zone-level and global `rotate` relative to the anchor's rotation;
- how a zone's `mirror` axis is resolved from an anchor (`ref` + `distance`);
- `asym` (`source`/`clone`/`both`) and `skip`, and their effect on later keys and anchors;
- columns with their own row sets (`columns.<c>.rows`) and a missing first row;
- which ergogen key-level fields are not yet covered by the comparison (for example
  `width`/`height`, `bind`/`autobind`), with a note on whether layout parity needs them.

Use upstream ergogen at `8286e18f` (`src/points.js`, `anchor.js`, `point.js`, `prepare.js`) as the
primary source. Confirm behaviour by running it with the comparison harness
(`.evidence/ergogen-layout-comparison/compare.js`). List the scenarios the parity fixtures
should include.
