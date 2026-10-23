# Ergogen anchor and mirror semantics

Date: 2026-10-22. Upstream: ergogen `8286e18f` (v4.2.1). Primary sources are the pinned
[`points.js`](https://github.com/ergogen/ergogen/blob/8286e18f/src/points.js),
[`anchor.js`](https://github.com/ergogen/ergogen/blob/8286e18f/src/anchor.js),
[`point.js`](https://github.com/ergogen/ergogen/blob/8286e18f/src/point.js), and
[`prepare.js`](https://github.com/ergogen/ergogen/blob/8286e18f/src/prepare.js). Line references
below refer to those files at that commit. Behavioural numbers were executed against the local
read-only clone using Node from `/tmp`, `NODE_PATH=/home/chris/.cache/ergogen-ref/pkg/node_modules`,
and temporary `/home/chris/.cache/ergogen-ref/harness/anchor-semantics.js`; the existing comparison
harness was also run against `core/tests/fixtures/matrix-projections.json` (54-cell port check,
maximum error `7.11e-15`; S1–S15 output). These are direct upstream observations, not BoardStudio
runtime evidence.

## Findings

### Anchor fields and order

An anchor can be a point name, an object, or an array of anchor steps. Object fields are processed
in the fixed order `ref`/`aggregate`, then `orient`, `shift`, `rotate`, then `affect`; source key
ordering does not change it. `affect` copies only the selected candidate `x`, `y`, and/or `r` onto
the incoming start point. Numeric `orient` and `rotate` add a heading; nonnumeric forms resolve a
target anchor and point toward it. `shift` is in the current point's heading frame. [anchor.js:11-19, 43-115](https://github.com/ergogen/ergogen/blob/8286e18f/src/anchor.js)

**Observed:** `base_a_r0` at `(10,20,30)`, anchored `partial_b_r0` with `shift:[5,0]`,
`rotate:10`, `affect:'x'` resolved to `(14.330127,0,0)`: shift first produced `(14.330127,22.5)`,
then the affect retained only x. The same anchor without affect resolved to
`(14.330127,22.5,40)`. This establishes fixed order and partial-field behavior. [anchor.js:82-115](https://github.com/ergogen/ergogen/blob/8286e18f/src/anchor.js)

### `resist` and mirrored relative shifts

Mirroring reflects x about the axis and negates point rotation; the mirrored point is tagged
`meta.mirrored`. A relative shift negates its local x component on a mirrored point unless
`resist:true`; numeric anchor rotations similarly negate their angle unless resisted. A relative
shift is then rotated by the current heading. [point.js:12-30, 33-37](https://github.com/ergogen/ergogen/blob/8286e18f/src/point.js), [anchor.js:82-107](https://github.com/ergogen/ergogen/blob/8286e18f/src/anchor.js)

**Observed:** from mirrored ref `(-10,20,-30)`, relative `[5,0]` plus `rotate:10` produced
`free_b_r0=(-14.330127,22.5,40)`; with `resist:true` it produced
`resisted_c_r0=(-5.669873,17.5,20)`. This confirms both shift-x and angle sign reversal, and
that resistance restores their positive signs. [point.js:12-30](https://github.com/ergogen/ergogen/blob/8286e18f/src/point.js)

### References, zone transforms, and mirror axis

Zones are rendered in declaration order. A zone anchor sees points from earlier zones. The new
zone's points are then rendered, given zone rotation, inserted into the available points, and
mirrored; global rotation follows all zones, then global mirroring. Thus a later zone can reference
an earlier `mirror_<name>` point, but a zone cannot reference the mirrored copy it is only about to
create. `anchor.js` toggles `mirror_` on references when its `mirror` argument is true; direct
zone anchors default that argument to false. [points.js:312-390](https://github.com/ergogen/ergogen/blob/8286e18f/src/points.js), [anchor.js:5-9, 53-60](https://github.com/ergogen/ergogen/blob/8286e18f/src/anchor.js)

Zone anchor rotation is carried as the first rotation about the anchor point during zone layout.
By contrast, `zone.rotate` and global `points.rotate` rotate each finished point around world
`(0,0)`, in that order; rotations also add to heading. [points.js:27-42, 349-376](https://github.com/ergogen/ergogen/blob/8286e18f/src/points.js), [point.js:22-30](https://github.com/ergogen/ergogen/blob/8286e18f/src/point.js)

An object-valued `mirror` is parsed as an anchor (its x coordinate is the axis) plus
`distance / 2`; the copy uses `x' = 2*axis - x` and `r' = -r`. `distance` is therefore the
full separation between a point and its reflected counterpart along x. Numeric mirror means the
axis directly. [points.js:195-203, 206-215](https://github.com/ergogen/ergogen/blob/8286e18f/src/points.js), [point.js:33-37](https://github.com/ergogen/ergogen/blob/8286e18f/src/point.js)

**Observed:** anchor `(10,0,0)`, mirror `{ref:'base_a_r0', distance:20}` produced `copy_a_r0`
at `(10,0,0)` and `mirror_copy_a_r0` at `(30,0,0)`, proving axis `20` and 20 mm pair
separation. With an anchor shift to `(10,0)` and anchor rotate `20`, then zone rotate `40` and
global rotate `30`, the final point was `(3.420201,9.396926,90)`: anchor rotation affects local
layout heading; zone/global rotation are world-origin transforms. [points.js:27-42, 349-376](https://github.com/ergogen/ergogen/blob/8286e18f/src/points.js)

### Cumulative key transforms, row sets, `asym`, and `skip`

For each column, the row names are the insertion-ordered union of zone-wide and column-specific
row objects. Each column lays out its own resulting row list from the same column anchor; a missing
row is not synthesized to align later row indices. Zone-level row settings apply to every column
that has that row. [points.js:61-80, 104-111, 143-187](https://github.com/ergogen/ergogen/blob/8286e18f/src/points.js), [prepare.js:5-23](https://github.com/ergogen/ergogen/blob/8286e18f/src/prepare.js)

For every key, `orient`, `shift`, and `rotate` update the running point before that point is
committed as the next row's starting anchor. `adjust` is then applied only to the current emitted
point, after the running anchor was copied, and the row advances by that key's padding along its
current heading. A skipped point is still laid out and advances the running anchor; filtering
happens at the very end. [points.js:166-187, 395](https://github.com/ergogen/ergogen/blob/8286e18f/src/points.js)

`asym:'source'` suppresses its mirror copy. `asym:'clone'` emits the mirror copy and marks the
original skipped, so the original is removed only during final filtering. `both` retains both.
Mirroring merges `meta.mirror` overrides into the clone before setting its generated name and
mirrored marker. [points.js:206-229, 395](https://github.com/ergogen/ergogen/blob/8286e18f/src/points.js), [assert.js asym validation](https://github.com/ergogen/ergogen/blob/8286e18f/src/assert.js)

**Observed:** the runner retained `first_a_r1` with metadata `source` but emitted no mirrored r1;
it emitted `mirror_first_b_r1` with metadata `clone` (the original clone key was absent); default
`both` emitted `mirror_first_a_r0`. A later zone anchored successfully to that mirrored r0 at
`(200,0,0)`. The `skip:true` r2 was absent from output. [points.js:206-229, 395](https://github.com/ergogen/ergogen/blob/8286e18f/src/points.js)

**Observed:** with `shift:[2,0]` and `rotate:10` configured on column `a`, its rows were
`a/r0=(2,0,10)`, `a/r1=(0.670300,19.058644,20)`, `a/r2=(-3.948697,37.596844,30)`.
Column `b` remained straight at `(19,0,0)`, `(19,19,0)`, `(19,38,0)`. The transform carries
up rows of that column only. Existing S7/S8/S10 measurements also show a padding increase on
`r1` moves `r2` by 3 mm, a `rotate:10` on `b/r1` moves `b/r2` by 3.312 mm and rotates it 10°,
and `shift:[2,0]` moves `b/r2` by 2 mm. [points.js:166-187](https://github.com/ergogen/ergogen/blob/8286e18f/src/points.js)

### Unaddressed fields

`width` and `height` specify generated point/keycap dimensions in visualization, not center
placement; `autobind` is consumed after point generation for electrical autobinding. They do not
belong in the layout-coordinate parity kernel, though dimension behavior may need separate
fixtures. `bind` is not an accepted per-key field here; `autobind` is. `skip` does affect the
final emitted point set, so it needs parity coverage even though it does not alter coordinates.
[points.js:85-100, 395-399](https://github.com/ergogen/ergogen/blob/8286e18f/src/points.js), [points.js autobind implementation](https://github.com/ergogen/ergogen/blob/8286e18f/src/points.js)

Recommendation for the parity model: represent cross-matrix anchor position and heading as the
starting frame; apply relative shift in that heading, numeric rotation in the anchor's handedness,
and explicit x/y/r affect projection; distinguish anchor-frame rotation from world-origin zone
and global rotations. Keep mirror reflection, asymmetric source/clone/both membership, and skip
membership as fixture-tested behavior. The standing ADR intentionally excludes ergogen's
`orient`, but the anchor and mirror contract still needs these fixtures.

## Parity fixture scenarios

Each sketch is YAML-like ergogen config. Compare emitted point `x`, `y`, `r`, name, and membership
against the pinned runner.

1. **Anchor field order and partial affect:** `base` at `anchor:{shift:[10,20],rotate:30}`; `next`
   at `anchor:{ref:'base_a_r0',shift:[5,0],rotate:10,affect:'x'}`. Proves `(14.330127,0,0)` and
   fixed `orient → shift → rotate → affect` ordering.
2. **Mirrored anchor relative shift / resist:** make `base` mirrored about x=0; anchor `next` to
   `mirror_base_a_r0` with `shift:[5,0],rotate:10`; repeat with `resist:true`. Proves heading-frame
   shift, sign inversion, and resistance independently.
3. **Earlier mirrored-zone reference:** zone `base` has `mirror:100`; later zone anchors to
   `mirror_base_a_r0`. Proves availability after a zone's mirror pass and rejects forward/self
   mirror references.
4. **Zone-anchor rotation versus zone/global rotation:** use nonzero anchor position/rotation,
   `zone.rotate:40`, and global `points.rotate:30`. Proves anchor-pivot layout rotation then
   world-origin zone and global rotations.
5. **Mirror axis from anchor and distance:** `mirror:{ref:'base_a_r0',distance:20}` with base x=10.
   Proves axis x=20 and reflected partner x=30; include a nonzero heading to assert `r'=-r`.
6. **Column-local carry:** put `shift:[2,0],rotate:10` on column `a` with three rows and a straight
   column `b`. Proves cumulative carry only within the configured column; pin observed coordinates
   `(2,0,10)`, `(0.670300,19.058644,20)`, `(-3.948697,37.596844,30)`.
7. **Ragged per-column rows and missing first row:** column `a` has rows `r0,r2`, column `b` only
   `r1`, with distinct row padding/shift. Proves declaration-order row set per column, independent
   row anchor progression, and no synthetic missing row.
8. **Skip advances anchor:** mark `r1` skipped and give it a nonzero shift or altered padding;
   assert `r1` absent while `r2` retains the displacement caused by it.
9. **Asym membership:** test one point each for `source`, `clone`, and `both` under a mirror axis;
   assert source emits only original, clone emits only mirror, both emits both. Include a later
   zone anchoring to a visible mirrored point.
10. **Adjust does not carry:** place `adjust:{shift:[3,1],rotate:15}` on a middle key; assert only
    that point changes, while a following key uses the unadjusted running anchor.
11. **Width/height/autobind scope:** set custom dimensions and autobind; assert coordinates are
    unchanged, and keep dimension/binding expectations in separate non-layout checks.
