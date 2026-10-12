# 30: Independent parts take the full 360-degree heading rule

Status: resolved
Type: build
Blocked by: [29](29-mirror-switch-and-companion-headings.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md#amendment-the-heading-rule-in-practice-2026-10-10)

## Request

Ticket 29 found that `mirror_pose` (`core/src/mirrored_half.rs` ~:203) gives independent parts
(everything that is not a switch or a switch companion) the heading `-r`, while ADR-0010's amendment
says the general rule is `180 - r` over the full 360 degrees: a TRRS facing left must face right on
the mirror. The shipped Sofle right half has the TRRS and reset at `-r`, which matches the code and
not the ADR, so the shipped data is the thing to question, not only the code.

## What to do

- Make independent parts follow `180 - r` through the pair map's `forward_angle` over the full 360
  degrees (never modulo 180). Test exactly: TRRS facing left (180) becomes 0, facing right (0)
  becomes 180, facing up (90) stays 90, facing down (270) stays 270; an off-axis LED mirrors by the
  rule. Mutation-check against `-r`.
- Keep switches and their companions as in ticket 29 (switches `-r`, no half turn).
- Report which shipped parts differ (recipes' hand placement, measured fixtures, Sofle TRRS and
  reset). Do not change the recipes (ticket 20) or the shipped data (ticket 25's review); list the
  differences in the Outcome so those tickets can decide.
- Never change a generator parameter, `reversible` or `side`.

## Acceptance

- [ ] The four-heading test and the off-axis LED test pass and are mutation-checked.
- [ ] A list of shipped parts whose hand-placed heading disagrees with the rule.
- [ ] Switch behaviour from 29 unchanged.

## Outcome

Implemented by Haiku 5.5 (claim commit `1c8e0d882`, implementation `ff41858f9`). Both commits carry
`Co-Authored-By: Claude Haiku 5.5` and were left as they are. Verified by Sonnet 5.5; see
[Verification](#verification), which corrects two claims below (the on-grid finding is not a gap, and
the `forward_angle` doc is now fixed).

### What changed

- `core/src/mirrored_half.rs`
  - `mirror_pose` (the only non-test caller is the independent-part loop in `new_board`; matrix members
    still go through `layout::reflected`, unchanged): heading is `(forward_angle(map, r) + 180)` taken
    modulo 360 into `[0, 360)`. The doc comment derives it: the map's linear part is a reflection about
    the axis at `90 + rotation/2`; `forward_angle` is the orientation image (only meaningful modulo 180),
    and a heading is a direction, so it is turned by 180. On an axis map this is `180 - r`.
  - `Renames` gains `full_rule_part`. `anchored_local(point, full_rule)`: a point or cutout anchored to an
    independent part is reflected in that part's frame as `(x, -y)` instead of `(-x, y)`, because the
    mirror part is turned by 180. Anchors on keys and companions keep `(-x, y)`. Used by the anchored
    polygon and rect centre in `mirror_feature` and by `mirror_control` (attached connection points).
    Needed by this change: without it, outline geometry anchored to `U1` or `RST` would no longer sit at the
    mirrored world position.
  - Unit tests: `a_mirrored_pose_takes_the_pair_heading_rule` now expects 190 (was 10, the old
    `forward_angle`). New: `a_turned_part_faces_outward_on_the_mirror_over_the_full_circle` (180 to 0,
    0 to 180, 90 to 90, 270 to 270, and 135 to 45) and `every_heading_mirrors_to_the_mapped_direction_under_axis_and_rotated_maps`
    (headings 0 to 315 under rotation 0 and 27; the oracle is the mapped vector from `pair::forward_vector`
    and `atan2`, not `forward_angle`; under rotation 27 the LED at 135 goes to 72).
- `core/tests/mirrored_half.rs`
  - `assert_reflects_every_part`: a corresponded part that is a matrix member expects `rotation - r`
    (switch rule, unchanged); every other part expects `rotation + 180 - r`.
  - `assert_part_matches`: compares position, reference, side and footprint with the shipped part, but no
    longer the heading. The shipped right halves hand-place some independent parts at `-r`, so the heading
    is checked by the rule instead (see the list below).
  - `outlines_reflect_anchored_geometry_in_the_part_frame_and_keep_unique_identities`: two literal local-frame
    expectations changed for the independent anchor `right/U1`: the attached connection point
    `(-4, 2)` to `(4, -2)`, and the first polygon point `(-5, 7)` to `(5, -7)`. The world-frame assertions
    that follow are unchanged and are the ones that matter.
- Not changed: `core/src/matrix/layout.rs` (switch rule), recipes, shipped data, every generator parameter,
  `reversible`, `side`, ticket 29's switch and companion tests (`key_switches_mirror_their_tilt...` passes untouched).

### Decisions

1. Generalisation: `forward_angle(map, r) + 180`. Checked by the vector oracle for rotation 0 and 27.
2. Stored headings are normalised to `[0, 360)`, so "facing down stays 270" holds as a literal.
3. The anchored frame `(x, -y)` for independent parts comes from the algebra (`p' = -F p` for both sides)
   and is confirmed by the world-frame outline tests.
4. The heading-uncertain finding is unchanged (still off-grid only, see Follow-ups).

### Shipped parts whose hand-placed heading differs from the rule (for tickets 20 and 25)

Scratch test, removed before commit: one-half mirror built from each shipped `left` fixture with the shipped
map, each `right/...` independent part compared with the shipped right half of the same id (heading
modulo 360; shipped to rule).

- **Corne** (3 of 3): `right/U1` (`demo/mcu_nice_nano`, front) 0 to 180; `right/RST`
  (`demo/reset_switch_tht_top`, front) 0 to 180; `right/TRRS` (`demo/trrs_pj320a`, front) 0 to 180.
- **Sofle RGB** (19 of 19): `right/U1` (`sofle/mcu_nice_nano`) 0 to 180; `right/SW25`
  (`sofle/rotary_encoder_ec11_ec12`) 0 to 180; `right/J3` (`sofle/display_ssd1306`) 0 to 180;
  `right/J2` (`sofle/trrs_pj320a`, the TRRS) 90 to 270; `right/RSW1` (`sofle/reset_switch_tht_top`)
  90 to 270; `right/TH1` to `TH7` (`sofle/mounting_hole_npth`) 0 to 180 (round holes, no geometric effect);
  `right/D31`, `D33` (`sofle/led_sk6812mini-e`, back) 180 to 0; `right/D32`, `D34` to `D37` (same
  definition, back) 0 to 180.
- **Sofle v2** and **Sofle choc** (12 of 12 each): `U1`, `SW25`, `J3`, `J2`, `RSW1`, `TH1` to `TH7`, as
  for Sofle RGB, with the same shipped and rule headings.
- **Matrix members** (keys, diodes, LEDs), matched by physical position and member kind (the mirror keeps the
  source's numbering, so ids cannot be compared): every key matches the shipped right half in position and
  heading (Corne 21, Sofle RGB 29, Sofle v2 29, Sofle choc 29). Every RGB LED matches in position and heading
  (Sofle RGB and choc). Every diode is at a different position from the shipped right diode (Corne 21,
  Sofle 29 each): the ticket 29 side decision (decision 4), unchanged here. Diode headings were not compared
  because their positions differ.

No recipe or shipped file was edited.

### Commands (sequential, foreground)

- GitNexus: the index is stale (last indexed 4b372b2). `impact` for `mirror_pose`, `mirror_feature`,
  `mirror_snapshot` and `Renames` returned "not found", so risk is UNKNOWN. I checked callers by text search:
  `mirror_pose` has one non-test caller (`new_board`) and one unit test; `mirror_feature` is called from the
  board outline loop and `mirror_snapshot`; `Renames` is built once, in `new_board`. `detect-changes --scope all`
  said "Diff touched 2 file(s) but no indexed symbols overlap those hunks", which is not a clean check.
- Red, before the implementation: `cargo test -p boardstudio_core --lib mirrored_half::` 3 failed, 2 passed
  (10 s; the failures showed the old `-r`, e.g. `left: -180.0 right: 0.0`). `cargo test -p boardstudio_core --test
  integration mirrored_half::` 6 failed, 15 passed (49.8 s); the switch and companion tests passed on the
  unchanged rule.
- Green: lib 5 passed (8 s); integration 21 passed (50.1 s), after the outline and placement changes.
- Mutations (each restored, and the source checked against a backup with `cmp`):
  - `-r` (no +180): lib 3 failed; integration 7 failed (49.9 s).
  - Modulo 180 (`rem_euclid(180.0)`): lib 3 failed (`a_turned...`, `every_heading...`, `a_mirrored_pose...`).
  - Anchored parts use the switch-local transform: integration `outlines_reflect_anchored...` failed (50.0 s).
- `python3 scripts/check.py lint`: the first run failed on rustfmt in my new test code. `cargo fmt --all`
  changed only the two files; rerun passed (clippy `-D warnings` clean), 9.8 s.
- `python3 scripts/check.py test`: exit 0, 96.9 s, all native targets. The boundary golden
  (`native_transport_reproduces_the_golden_transcript`) passed without `BOUNDARY_BLESS`.
- `python3 scripts/check.py typecheck`: exit 0, 9.3 s.
- Wasm boundary parity: not run. The golden transcript is unchanged (no fixture or bless change; `git status` shows
  only the two source files).
- Not run, per the brief: `repo`, `build`, `browser`, `deny`, `release`.

### Unverified and follow-ups

- **The heading-uncertain finding does not fire on-grid.** Not a gap: see Verification (ADR-0010 makes the
  rule certain on the 90-degree grid).
- **Shipped data disagrees with the rule** in the list above; tickets 20 and 25 decide. Nothing shipped changed.
- Stabilisers: none in the shipped splits (ticket 29). If one is a matrix member it keeps the switch rule through
  `layout::reflected`, which I did not test.
- Single-colour LED: no fixture; unchanged.
- No pipeline-level TRRS test. The unit test checks the four headings exactly, and
  `assert_reflects_every_part` checks every independent part produced by `create` against the rule.
- `pair::forward_angle` doc said a projected vector carries this angle; that holds only modulo 180. Fixed in
  the verification pass. The other caller, `features.rs:1301`, already compares directions modulo 180.
- `inverse_angle` is not used for part headings. The new rule is an involution (applying it twice is the
  identity), so an inverse-mapped mirror edit can reuse `mirror_pose` when that path lands.
- The GitNexus index is stale; impact and detect-changes are provisional (above).
- Diode side (ticket 29, decision 4) is still open.

## Verification

Sonnet 5.5, 2026-10-10, `tce-matrix` worktree on top of `ff41858f9`. Reviewer pass, not a second
implementation. Acceptance holds: the four-heading and off-axis LED tests pass and die under mutation, the
shipped differences are listed and spot-checked, and the switch behaviour from 29 is unchanged. Resolved.

### Claims reproduced

- **The rule.** Derived independently: the pair map's linear part is `R(r_map) * diag(-1, 1)`. A direction
  at `a` goes to `180 - a` under `diag(-1, 1)` and then to `r_map + 180 - a`. So the mirrored heading is
  `r_map + 180 - r` (`180 - r` on an axis map, `207 - r` at a 27 degree map), which equals
  `forward_angle(map, r) + 180`. The implementation matches, normalised to `[0, 360)`.
- **Exact headings.** Under the axis map the existing tests assert 180 to 0, 0 to 180, 90 to 90, 270 to 270
  and the 135 LED to 45 exactly. Only 135 to 72 was asserted exactly under the rotated map. Added
  `the_four_edge_headings_and_an_off_axis_led_are_exact_under_a_rotated_map` (27 degree map: 180 to 27,
  0 to 207, 90 to 117, 270 to 297, 135 to 72).
- **Mutations of the heading rule** (lib `mirrored_half::`, each restored, `cmp` against a backup): dropping the
  `+ 180` (5 of 7 failed), `rem_euclid(180.0)` (5 failed), the literal `-r` (5 failed). All die.
- **The `(x, -y)` anchor frame.** Algebra: a part-frame point `p` is at `W + R(r) p`; the map sends it to
  `M W + R(r_map) diag(-1, 1) R(r) p = M W + R(r_map - r) diag(-1, 1) p`. The mirror part at heading
  `r_map + 180 - r` has `R(r_m) = -R(r_map - r)`, so the local point is `-diag(-1, 1) p = (x, -y)`. For a switch
  at `r_map - r` it is `(-x, y)`. Both correct, including for back-side parts (the side flip commutes with
  `diag`). Two tests were added, since the one existing check used a single heading (0) and a single map:
  - lib `anchored_points_land_on_the_mapped_world_point_at_every_heading`: headings 0, 90, 180, 270 and 37
    under an axis and a 27 degree map, the world point of the mirrored anchor equals the map applied to the
    source anchor within 1e-9, for the independent frame and the switch frame.
  - integration `anchored_geometry_lands_on_the_mapped_world_point_at_every_heading_and_map`: the same grid
    through `create` on the Corne, with subtract rects anchored to `left/U1`, `left/RST` (headings set per
    case), the key `matrix/left-keys/r0c0` and its companion; checks the world centre and, for independent
    parts, the heading `r_map + 180 - r`. Keys and companions keep the old transform and hold.
  - Mutations: `anchored_local` always the old transform: the lib test fails. `full_rule_part` always `false`
    fails the integration test on `left/U1`; always `true` fails it on the key. All restored.
- **Shipped differences, spot-checked against the fixtures** (a scratch test, removed): Corne `U1`, `RST`, `TRRS`
  are at 0 on the left and 0 in the shipped right half; the rule gives 180 (3 of 3, axis map). Sofle RGB:
  `J2` (TRRS) and `RSW1` are at -90 left and 90 shipped, rule 270; `U1`, `SW25`, `J3`, `TH1` to `TH7` are at 0
  and shipped 0, rule 180; `D31`, `D33` are at 180 and shipped 180, rule 0; `D32`, `D34` to `D37` at 0 and
  shipped 0, rule 180. The count (12 non-LED parts plus 7 LEDs) is 19. The Outcome list is accurate.

### Findings

- **The on-grid finding is correct as it is.** ADR-0010's amendment makes the uncertain case "a non-symmetric
  footprint at a heading that is not a multiple of 90 degrees"; on the grid the rule is certain, so a part
  turned by 180 degrees raises no finding. The Outcome's "real gap" is not one, and the finding scope is not
  widened. (Correction, ticket 27 verification: this holds for the heading and the position only. A pad off the
  heading axis lands with its local `y` reversed even on the grid; see
  [ticket 27](27-mirror-modules.md#verification).) What it does mean: every independent part at heading 0 is turned half a turn, including the MCU,
  encoder, display and mounting holes. That is the rule as written (the shipped halves keep these at 0), so
  whether heading 0 means "facing outward" in a footprint's own frame is a data question for tickets 20 and 25.
- **Stabilisers** are matrix members (ticket 29), so they keep the switch rule through `layout::reflected`
  and do not take the full rule; the Outcome's "not tested" stands, and the integration test covers a matrix
  companion rather than a stabiliser.
- **Doc fix.** `pair::forward_angle` no longer says a projected vector carries the angle; it says the vector
  lies on that line and its heading is the angle plus 180 (comment only).
- **Diode side** on the mirrored half is still ticket 29's open user decision; unchanged.

### Commands (sequential, foreground)

- lib `mirrored_half::` 7 passed (about 7 s); integration `anchored_geometry_lands...` passed, 58.6 s.
  Mutation runs: lib about 7 s each (4); integration 2 x 49 s.
- `python3 scripts/check.py lint` passed, 9.6 s. `test` passed, 90.8 s. `typecheck` passed, 9.5 s.
  `repo` ran last, see the commit. Wasm `boundary_parity::` not run: no fixture, bless or transcript change.
  `build`, `browser`, `deny`, `release` not run.
- GitNexus: the index is stale. `impact mirror_pose` returned "not found" (UNKNOWN, checked by text search:
  one non-test caller); `impact forward_angle` LOW (comment-only edit). `detect-changes` is provisional.
