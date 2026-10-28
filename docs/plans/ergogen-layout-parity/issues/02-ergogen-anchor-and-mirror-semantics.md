# 02: Pin ergogen's anchor and mirror semantics

Status: resolved
Type: research
Blocked by: none
Map: [map.md](../map.md) · Decision: [ADR-0034](../../../adr/0036-matrices-follow-ergogen-point-layout.md)

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

## Answer

- Anchor objects apply `orient`, relative `shift`, `rotate`, then partial `affect`; observed partial x-only result: `(14.330127,0,0)`.
- Mirrored anchors negate relative x and numeric rotation; `resist:true` restores both sign directions.
- Zone anchor rotation is around the anchor frame; zone and global rotations turn completed points around world `(0,0)`.
- Mirror `{ref, distance}` resolves its axis to `anchor.x + distance/2`; a base at x=10 and distance 20 made the partner x=30.
- Column key `shift` and `rotate` carry upward through rows; zone/column anchor references only see earlier generated points.
- Per-column row sets are independent; `skip` removes output after layout but does not stop anchor advancement.
- `asym: source` omits the clone, `clone` omits the original, and `both` keeps both; later zones can anchor to an earlier mirrored point.
- `width`/`height` and `autobind` are outside center-coordinate parity; see the fixture scenarios and measured values in the [research findings](../../../research/2026-10-22-ergogen-anchor-and-mirror-semantics.md).
- The research file gives concrete config sketches for anchor affect/resist, rotations, mirror axis, ragged rows, skip, asymmetry, and adjust carry.
