# Ergogen layout engine vs BoardStudio matrix layout

Date: 2026-10-22. Question: does BoardStudio's matrix layout still follow ergogen's
point-layout concepts (origin, stagger, spread, splay, orient, shift, rotate, rows, columns,
mirror)? Where does it differ, and how does each parameter propagate to later keys?

Sources:

- Ergogen `src/points.js`, `src/anchor.js`, `src/point.js`, `src/prepare.js` and `src/units.js`
  at upstream commit `8286e18f` (v4.2.1, 2025-11-02), <https://github.com/ergogen/ergogen>.
  The `ergogen/` directory in this repository vendors only the footprint library, not the
  points engine.
- BoardStudio `core/src/matrix.rs` (`location`, `column_rotation`, `column_basis`), and
  `core/src/matrix/splay.rs`, `placement.rs` and `layout.rs` (`reflected`, `move_keys`,
  `local_displacement`). All read at `c112a835a`.

Method: each claim below was read from source. The behavioural claims were then measured.
Ergogen's own `points.parse` ran on paired configurations. Our side ran through a JS port of
`location()` and the key rotation. Before any comparison, the port was checked against
`core/tests/fixtures/matrix-projections.json`, which the Rust test
`projections_match_captured_typescript_behavior` asserts. All 54 parametric cells matched to
7e-15. The harness and the full output are in the gitignored
`.evidence/ergogen-layout-comparison/` (`compare.js`, `results.txt`). Scenario IDs (S1…) refer
to that output.

Both engines are y-up internally and negate y only when writing KiCad
(ergogen `pcbs.js:128-132`, our `artifact/source.rs:332`). Row 0 is the bottom row, and a
positive angle turns counter-clockwise in both. Axis and sign conventions therefore match.

## 1. How an ergogen zone is laid out

`render_zone` (`points.js:20-196`) builds one zone, which corresponds to one of our matrices.

### Configuration inheritance

Every key's settings come from a six-level `extend`, from least to most specific
(`points.js:104-111`):

`defaults → points.key → zone.key → columns.<c>.key → zone.rows.<r> → columns.<c>.rows.<r>`

`extend` replaces scalars. It merges objects key by key and arrays index by index
(`prepare.js:5-23`). A more specific level therefore **replaces** a value; it never **adds** to
it. For example, a row `adjust.shift: [0,3]` hides a column `adjust.shift: [4,0]` entirely,
because the result is `[0,3]` (S15).

Column-level values (`spread`, `stagger`, `splay`, `origin`) are read from `keys[0]`, the merged
configuration of the column's **first (bottom) row** (`points.js:143-156`). A `stagger` set
under `zone.rows.<first row>` therefore applies to every column.

### The algorithm

1. **Zone anchor.** The anchor is resolved from `ref`, `aggregate` (`average`/`intersect`),
   `orient`, `shift`, `rotate` and `affect`. These can name keys of earlier zones. The anchor's
   rotation becomes the first entry in a rotation list, about the anchor point, and is then
   cleared from the anchor (`points.js:37-42`).
2. **For each column, in declaration order:**
   1. `zone_anchor.x += spread`, skipped for the first column, then `zone_anchor.y += stagger`,
      applied to the first column too (`points.js:143-146`). Both happen in the **unrotated**
      zone frame and **accumulate**: every later column inherits them.
   2. `col_anchor` is a snapshot of that point. It is where the bottom key would sit before any
      rotation.
   3. If `splay ≠ 0`, `push_rotation` adds a rotation about `col_anchor + origin`, with
      `origin` relative to the column anchor in the unrotated frame. That pivot is first
      carried through every earlier rotation (`points.js:6-15`, `152-156`). The rotation list
      only grows, so a splay turns **this column and every later column**.
   4. A running anchor starts at `col_anchor` and is carried through the whole rotation list.
3. **For each row of the column, bottom to top** (`points.js:166-187`):
   1. `r += orient`, then `shift(shift)` along the current heading, then `r += rotate`.
   2. **The running anchor is committed here** (`running_anchor = point.clone()`). `orient`,
      `shift` and `rotate` therefore **accumulate up the column**: every later row inherits
      them.
   3. `adjust`, a full anchor, is applied to this key only. Its fields are applied in the fixed
      order `orient → shift → rotate`, whatever order they are written in (`anchor.js:144-165`).
      The adjustment does not reach the running anchor.
   4. The running anchor steps `[0, padding]` along its own heading, to the next row.
4. **After the zone.** The zone-level `rotate` turns the zone about **world (0, 0)**
   (`points.js:349-352`). Next, `zone.mirror` adds `mirror_*` copies (`perform_mirror`: `x' = 2a − x`,
   `r' = −r`, `asym` source/clone/both). Then the global `points.rotate` is applied, again
   about world (0, 0), then the global mirror, then `skip` filtering (`points.js:395`). A
   skipped key has already advanced the running anchor, so removing it moves nothing else.

### Propagation in ergogen

| Parameter | Level | The key itself | Later rows in the column | Later columns | Splay pivot |
|---|---|---|---|---|---|
| anchor `shift`/`rotate`/`ref` | zone | moves | yes | yes | carried |
| zone `rotate`, `points.rotate` | zone/global | turns about world 0,0 | yes | yes | carried |
| `spread` | column (from row 0) | moves | yes | **yes, cumulative** | moves with it |
| `stagger` | column (from row 0) | moves | yes | **yes, cumulative** | moves with it |
| `splay` | column | turns about pivot | yes | **yes, cumulative** | n/a |
| `origin` | column | sets the pivot | (turn) | (turn) | relative to `col_anchor` |
| `orient` | key | heading, before `shift` | **yes, cumulative** | no | no |
| `shift` | key | moves along heading | **yes, cumulative** | no | no |
| `rotate` | key | heading, after `shift` | **yes, cumulative** | no | no |
| `padding` | key | no | **yes, step to next row** | no | no |
| `adjust` | key | orient→shift→rotate | no | no | no |
| `width`/`height` | key | keycap size only | no | no | no |
| `skip` | key | removed | no (anchor still advances) | no | no |

## 2. How a BoardStudio matrix is laid out

`location()` (`core/src/matrix.rs:303-361`) is a closed formula per cell. The key heading is
`matrix.rotation + column_rotation + cell.rotation`, where `column_rotation` is the sum of
splays `0..=column`, negated under a mirror (`matrix.rs:363`).

1. In the unrotated matrix frame:
   `x = column·pitch.x + rowOffset.x + columnOffset.x + cellOffset.x`
   `y = row·pitch.y + rowOffset.y + columnOffset.y + cellOffset.y + Σ stagger[0..=column]`.
2. Splays `column…0` are applied, last first, each about its pivot. The default pivot is
   `(i·pitch.x, Σ stagger[0..=i])`: the column's base **before** row, column and cell
   offsets. A custom `columnOrigins[i]` is an **absolute** point in that frame.
3. Mirror `x → −x` or `y → −y` in the local frame.
4. Rotation by `matrix.rotation` about the local origin, then translation by `matrix.origin`.

Applying the splays last-first about raw pivots is algebraically the same as ergogen's
first-to-last rotation about carried pivots. S3 and S4 confirm this numerically.

### Propagation in BoardStudio

| Parameter | Level | The key itself | Later rows in the column | Later columns | Default splay pivot |
|---|---|---|---|---|---|
| `origin` + `rotation` | matrix | moves/turns | yes | yes | carried |
| `mirror` X/Y | matrix | local flip, heading negated | yes | yes | carried |
| `pitch` x / y | matrix | uniform spacing | uniform | uniform | yes |
| Column **stagger** | column | moves | yes | **yes, cumulative** | moves with it |
| Column **offset** (x, y) | column | moves | yes | **no** | **does not move** |
| Column **splay** "Following" | column | turns | yes | **yes, cumulative** | n/a |
| Column **splay** "This column" | column | turns | yes | compensated back | n/a |
| Splay **origin** | column | sets the pivot, keys held still | — | compensated | absolute, fixed |
| **Row offset** | row (all columns) | moves | **no** | n/a | no |
| **Key local offset** | key | moves (column frame) | no | no | no |
| **Key rotation** | key | turns about centre | no | no | no |
| Enabled / deleted | key | removed | no | no | no |

The row, column and key offsets **add up** at every key. Ergogen's levels replace one another.

## 3. Equivalences (measured matches)

These ergogen concepts behave identically in BoardStudio when mapped as shown. Each scenario
matched 9/9 keys in position and heading.

| Scenario | Ergogen | BoardStudio |
|---|---|---|
| S1 | grid, `spread = padding = u` | `pitch` |
| S2 | column `stagger` (cumulative) | column stagger |
| S3 | column `splay`, default origin, including the first column | column splay, "Following" |
| S4 | `splay` + `origin: [dx, dy]` | splay + custom origin at `col_anchor + [dx, dy]` |
| S5d | `columns.<c>.key.adjust.shift` | column offset |
| S6b | `rows.<r>.adjust.shift` | row offset |
| S9 | key `adjust: {shift, rotate}` | key local offset + key rotation |
| S13 | zone `anchor: {shift, rotate}` | matrix origin + rotation |
| S14 | `mirror: <axis>` copy (`mirror_*`) | linked mirrored half (`layout::reflected`) |

The BoardStudio vocabulary maps onto ergogen like this:

- **Stagger** is ergogen `stagger`.
- **Splay** in "Following" mode is ergogen `splay`.
- **Offsets** (row, column, key) are ergogen **`adjust`** (independent, not inherited). They
  are never ergogen `shift` (cumulative).
- **Key rotation** is `adjust.rotate`, never key-level `rotate`.

The glossary terms *Column stagger* and *Column offset* (`GLOSSARY.md`, "Matrix layout")
already follow this split.

## 4. Behavioural differences

Ordered roughly by how much they limit reproducing an ergogen layout.

### D1. Nothing accumulates up a column (orient, shift, rotate, padding)

Ergogen commits each key's `orient`, `shift` and `rotate` to the running anchor, so every key
above inherits them. That is how ergogen builds curved, fanned or arced columns. BoardStudio
has no cumulative per-row transform: key rotation and key offset affect only that key.

- S8: `rotate: 10` on `b/r1`. In ergogen, `b/r2` turns 10° and swings 3.31 mm. In ours,
  `b/r2` does not move.
- S10: `shift: [2,0]` on `b/r1`. In ergogen, `b/r2` also moves 2 mm. In ours, it does not.
- S7: `padding: 22` on `r1`. In ergogen, every key above moves 3 mm. We have one uniform
  `pitch.y` and no per-row step. The nearest match is a row offset on **every** higher row.
- E1: ergogen's `orient` and `rotate` differ only in whether the key's `shift` follows the new
  heading. We have no equivalent of `orient`.

### D2. No cumulative column spread; column offsets are independent

Ergogen `spread` accumulates like `stagger`. BoardStudio has no horizontal counterpart to
stagger. Column offset x moves only its own column.

- S5: `spread: 23` on `b` moves columns `b` **and** `c` by 4 mm. A column offset x of 4 on `b`
  leaves `c` where it was.

### D3. The default splay pivot ignores column offsets

Ergogen's pivot is the column anchor, which follows `spread` and `stagger`. Ours follows
stagger but **not** column offsets. A column offset on a splayed column therefore turns about
a pivot that has not moved with the keys. Even offsetting every later column by the same
amount does not reproduce an ergogen spread.

- S5b: spread on a splayed column against offset x on `b` and `c`. Six keys differ by
  0.70 mm.
- The same rule explains why `convert_offset_available` refuses "Convert to stagger" on a
  splayed column with a default origin (`model.rs`, `convert_offset_available`).

### D4. A custom splay origin is absolute and fixed; editing it keeps keys still

- **Storage.** Ergogen's `origin` is relative to the column anchor, so the pivot follows later
  stagger or spread changes. Ours (`columnOrigins`) is an absolute point in the pre-splay frame
  and stays put.
  - S4b: S4, then the stagger on `b` changes from 5 to 8. Every key in `b` and `c` ends up
    0.52 mm from where ergogen puts it.
- **Editing.** In ergogen, changing `origin` swings the column about the new pivot. Our
  `SetMatrixSplay { Origin }` moves the pivot and then writes **compensating column offsets**
  into this column and every later one, so no key moves (`splay.rs:80-104`, `116-147`;
  test `moving_origin_preserves_all_keys_through_mirror_and_rotation`). The saved document
  gains column offsets the user never typed, and those offsets behave as D3 describes when
  splay changes later.

### D5. "This column" splay mode has no ergogen counterpart

`MatrixSplayAffect::Column` subtracts the change from the **next** column's splay and adds
compensating column offsets to every later column, so only this column turns (`splay.rs:151-165`;
test `column_angle_moves_only_the_selected_column`).

- Ergogen splay is always cumulative, which matches our "Following" mode. "Following" is the
  default (`layout_workspace.rs:163`).
- Side effect: after a "This column" edit, the next column's Splay field shows a counter-angle
  the user did not enter, and its offset fields show values they did not enter either.

### D6. Row offsets are independent, not cumulative

Ergogen row-level `shift` (under `zone.rows.<r>`) is a per-key cumulative shift, so it carries
every row above. Our row offset moves that row only.

- S6: `rows.r1.shift: [0,3]` moves `r1` and `r2` in ergogen. Our row offset (0, 3) moves only
  `r1`.
- Our row offset equals ergogen `rows.<r>.adjust.shift` (S6b).

### D7. Key offset is in the column frame, not the key's own frame

Our key local offset is added before splay, so it follows the column's heading but not the
key's own rotation. That equals ergogen `adjust: {shift, rotate}`, not
`adjust: {orient, shift}`.

- S11: `adjust: {orient: 30, shift: [3,0]}` puts the key 1.55 mm from our offset (3, 0) +
  rotation 30.
- Dragging a key writes this column-frame offset (`layout.rs:1030-1063`, `local_displacement`).

### D8. Levels add up instead of overriding

Our row, column and key offsets add up at each key. Ergogen's more specific level replaces
the less specific one, index by index.

- S15: column `adjust.shift [4,0]` with row `adjust.shift [0,3]` gives `b/r1` a shift of
  (0, 3) in ergogen and (4, 3) in ours.
- Ours is arguably the clearer model. It is still a semantic difference whenever ergogen
  configurations are translated.

### D9. One free-standing frame per matrix: no anchors between zones, no world-origin rotation

- **Anchors.** Ergogen zones are anchored to other zones' keys. `ref`, `shift`/`orient`/`rotate`
  relative to that key, `aggregate` (average or intersection of several keys), and `affect`
  (copy only x, y or r) are how thumb clusters attach to the main matrix. A BoardStudio matrix
  has an absolute `origin`/`rotation` and no anchor reference (`Matrix` in `model.rs`). The
  starter thumb keys in `demo_projects.rs` are free-standing parts. A thumb matrix does not
  follow the main matrix when that matrix moves.
- **World-origin rotation.** Ergogen's zone `rotate` and global `points.rotate` turn the zone
  about world (0, 0). Ours turns only about the matrix origin. In S12 every key differs by
  14.06 mm. Translating requires rotating the origin too.
- **Row sets.** Ergogen columns can declare their own row sets (`columns.<c>.rows`). Ours is a
  rectangular `rows × columns` grid with disabled or deleted cells. Deleting a cell keeps the
  other keys in place, as ergogen `skip` does.

### D10. Mirroring

- **Plain reflection matches.** A plain vertical-axis reflection gives identical poses (S14).
- **Ergogen-only features:**
  - `asym` (`source`/`clone`/`both`): keys that exist on one side only.
  - Mirror axes defined by an anchor `ref` plus `distance`.
  - Mirror-aware anchors: `meta.mirrored` negates later relative shifts unless `resist` is
    set (`point.js:27-44`).
- **BoardStudio-only features:**
  - A **local** in-place mirror (`Mirror::X`/`Mirror::Y`), applied after splay and before
    rotation, which also negates column headings. A Y mirror has no ergogen equivalent.
  - Linked halves that stay synchronised after creation (ADR-0008/0010). Ergogen copies once
    per generation.
- **Mirrored non-key parts.** On linked halves these take `180 − r` (`layout.rs`,
  `sync_components`); keys take `−r`, as in ergogen. This only matters when comparing
  free-standing components with ergogen points.

### D11. Small defaults

- Default pitch is 19.05 mm (`catalogue/matrix_setup_operation.rs:5`, ergogen's `U`). Ergogen's
  default spread and padding are `u = 19` mm.
- Keycap size is a part property. In ergogen it is the per-key `width`/`height`.
- Ergogen's `bind`/`autobind` (outline gap filling between neighbours) is an outline concern and
  is not compared here. Our analogue is `edgeGap`.

## 5. Summary

- **Matches.** Matrix origin and rotation, cumulative column stagger, cumulative splay with
  default or translated custom pivots, row, column and key `adjust`-style offsets, key turns,
  and plain mirrored halves.
- **Differs.**
  - Nothing accumulates up a column, so curved or fanned columns are impossible (D1).
  - There is no inherited spread (D2).
  - Pivot and origin semantics differ under offsets and later edits (D3, D4).
  - "This column" splay compensation leaves stored counter-values (D5).
  - Levels add up instead of overriding (D8).
  - There are no anchors between zones (D9).

## Open questions

- Should BoardStudio adopt cumulative per-row transforms (ergogen `shift`/`rotate`/`orient`/
  `padding`), or is the independent model deliberate? No ADR records the choice.
- Should column offsets move the default splay pivot, as ergogen's spread does? This would
  change saved geometry for existing splayed matrices with offsets, which ADR-0007 says must
  keep their meaning.
- Should a custom splay origin be stored relative to the column base, so it follows stagger
  edits? The same compatibility constraint applies.
- Is anchoring one matrix to another matrix's key (thumb clusters) in scope?
