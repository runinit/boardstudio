# 29: Mirror switches on X only and choose companion headings

Status: implemented (awaiting verification)
Type: build
Blocked by: [18](18-core-create-mirrored-half.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md#amendment-key-switches-and-their-companions-2026-10-10)

## Request

User, 2026-10-10: a mirrored key switch only mirrors on X. It must not take the 180-degree
turn of the general rule, or its LED would land on the wrong side. The diode and LED that
belong to the switch can be rotated, per variant (an RGB LED variant may be turned).

## What to do

- In `CreateMirroredHalf` (and, with 19, the reconcile step) give a key switch the mirrored
  position and the mirrored tilt `-r`, with no 180-degree turn, for switches in any matrix
  cell. Tests: a switch at 0, 15 and -20 degrees keeps its LED side and mirrors its tilt;
  mutation-check against the general `180 - r` rule.
- Companion diode and LED: mirrored position; heading chosen per variant (plain diode,
  single-colour LED, RGB LED). Start from the shipped recipes' placement of the right half
  and report any difference. Core adds a finding when the heading is uncertain.
- Never change a generator parameter, `reversible` or `side`.
- The uncertain-heading finding for non-symmetric footprints at a non-multiple of 90
  degrees (from the earlier amendment) must not fire for switches.

## Carried from ticket 18

- A mutation survived: `layout.rs` `cell.rotation = -angle` replaced by `pair::forward_angle(mapping, angle)`
  is not caught. Add a switch-heading test under a rotated pair map that kills it.
- The uncertain-heading finding is not implemented yet.

## Acceptance

- [ ] Switch headings follow the rule for every shipped split keyboard's mirrored half, by correspondence, within 1e-9.
- [ ] Diode and LED headings per variant, tested; uncertain cases produce a finding.
- [ ] The earlier general-rule tests (TRRS left, right, up, down; off-axis LED) still pass.

## Verification

Core tests, `repo lint test typecheck`, wasm boundary parity when the transcript changes.

## Outcome

Implemented by Haiku 5.5 (claimed in `fd3af2efe`). Not verified; status is for the verifier.

### What changed

- `core/src/mirrored_half.rs`: `heading_findings` (the uncertain-heading finding) and
  `SYMMETRIC_FOOTPRINTS` (generator sources `ceoloide/switch_mx`, `switch_choc_v1_v2`,
  `diode_tht_sod123`, `mounting_hole_npth`).
- `core/src/validate.rs`: one line, `findings.extend(crate::mirrored_half::heading_findings(doc))`.
- `core/src/matrix/layout.rs`: doc comment on `reflected` that states the switch and companion rule.
  No code change there.
- `core/tests/mirrored_half.rs`: three tests (`key_switches_mirror_their_tilt_without_a_half_turn_and_keep_their_companion_side`,
  `every_shipped_split_mirror_follows_the_switch_and_companion_rules`,
  `a_handed_mirrored_footprint_off_the_right_angle_grid_is_a_finding`) and helpers (`with_cell_turns`,
  `mapped_pose`, `in_switch_frame`, `find_part`, `assert_close_pair`). The tilted source is built through
  Core's `set-matrix` operation, so the parts are projected the same way `layout::sync` writes them.

No generator parameter, `reversible`, `side`, recipe, web file or other ticket changed.

### Decisions

1. **Switch rule.** The existing code already implements it, and I did not change it. The matrix frame
   carries the pair heading of the matrix rotation, the mirror negates column splays, and the cell turn is
   `-r`. The world heading is therefore `forward_angle(map, heading)`: `-r` on an axis map, `rotation - r`
   under a rotated map. No `180 - r` turn. The rule is written in the doc comment on `layout::reflected`.
2. **Companions.** Position is mirrored (the offset's x flips) and the heading is negated. This is the
   existing code; I verified it below and left it alone.
3. **RGB LED: no extra turn.** The preset LED (`core/examples/demo_projects/physical.rs`, `append`) sits at
   offset (0, -4.75) with heading 180 on the back side, with `reverse_mount: true` (the generator's own
   default). The shipped Sofle RGB right-half LED is at local (0, -4.75) with heading -180, which is
   equal to the mirror heading modulo 360. No parameter was changed.
4. **Diode: deviation from the shipped recipe.** The shipped right diodes sit on the same side as the
   left one: local (+7.40, -1.50), heading +90. The mirror puts them at local (-7.40, -1.50), heading -90
   (ADR-0010 "mirrored position"). The footprint is symmetric, so the 180-degree difference in heading
   does not show, but the diode moves 14.8 mm to the other side of its switch. I followed the ADR. **The
   user needs to decide** whether the recipe's same-side diode is intended.
5. **Single-colour LED.** No shipped instance exists. It uses the same reflected rule as the RGB LED
   (same code path). Not tested: there is no fixture.
6. **Finding.** A Warning, scope `Pcb`, id `mirror/<part id>/heading-uncertain`. It is raised for a mirror
   part of a pair whose footprint is not in `SYMMETRIC_FOOTPRINTS` and whose heading is off the 90-degree
   grid (tolerance 1e-9). It does not refuse the edit and does not move the part. It was small enough to
   build (about 60 lines plus one call). Stabilisers are not on the symmetric list, so a stabiliser at a
   non-90 heading would be flagged; I did not check whether stabilisers are symmetric.

### Commands (sequential, foreground, one at a time)

- Claim commit `fd3af2efe` (Status: claimed), with the Sonnet 5.5 trailer the brief asked for.
- GitNexus impact (the index is stale: the hook says last indexed 4b372b2):
  `reflected` CRITICAL (16 impacted, 6 processes); `forward_angle` LOW (1 direct, 6 processes through
  `features.rs`); the callers I saw by reading are `mirrored_half.rs` `mirror_pose` and `layout.rs`
  `reflected`, not confirmed by text search for every caller; `validate` CRITICAL (29 impacted, direct
  callers `CoreEngine.scene` and `CoreEngine.edit`). My change to `validate` is additive. It adds nothing
  unless a pair has an off-grid handed mirror part.
- Red run, before the finding existed: `cargo test -p boardstudio_core --test integration mirrored_half::`,
  49.6 s: 19 passed, 1 failed (`a_handed_mirrored_footprint_...`, no finding). The two switch and companion
  tests passed on the unchanged code, so there was no red for the switch rule. The rule already held.
- Mutations (each restored; `layout.rs` checked clean afterwards):
  - `cell.rotation = 180 - r` (the general rule on switches): `key_switches_...` FAILED (49.0 s).
  - `cell.rotation = pair::forward_angle(mapping, r)` (the ticket 18 mutant): `key_switches_...` FAILED
    (49.1 s). The existing `a_rotated_pair_map_...` test also FAILED on this mutant (`corne matrix/left-keys/r0c0:
    rotation 54.0 from 0.0`, 48.5 s). **The ticket 18 survival does not reproduce in this tree.** The two rules
    give the same world heading only when the map rotation is 0 modulo 360; otherwise they differ by it.
  - Finding, with the symmetric exclusion removed: `a_handed_...` FAILED (the switch, diode and LED of
    `r0c0` all flagged).
- `python3 scripts/check.py lint`: the first run failed only on rustfmt in my new test code. `cargo fmt --all`
  touched only my four files. The rerun passed (9.3 s; clippy `-D warnings` clean).
- `python3 scripts/check.py test`: passed, 93.1 s (all native tests, including the native boundary test).
- `python3 scripts/check.py typecheck`: passed, 11.2 s.
- Wasm boundary parity: not run. `git status` shows the transcript did not change (brief rule).
- `build`, `browser`, `deny`, `release`: not run (per the brief; no dependency, fixture or web change).
- A temporary measurement test (49.2 s) compared shipped companions with the mirror; I removed it and
  restored the file.
- `detect-changes --scope all`: 4 files, 2 symbols (`reflected`, `validate`), 0 affected processes, risk low.
  The index is stale and does not contain the new function or tests, so this is not a clean check.

### Shipped check, by correspondence

- For all ten shipped splits and Sofle variants (`corne lily58 klor sweep chocofi totem cantor sofle-v2
  sofle-rgb sofle-choc`), the mirror's key positions and headings equal the shipped other half within
  1e-9, and every corresponded part equals the pair map's image. Passed (`every_shipped_split_...`).
- Unmodified `sofle-rgb` mirror: 2 heading-uncertain findings (I did not identify which parts). `corne`: 0.

### Unverified and follow-up

- **Non-switch parts disagree with the ADR (not changed here).** `mirrored_half::mirror_pose` (line ~203, used
  for independent parts at line ~1033) gives the heading `forward_angle` = `-r`. The ADR-0010 amendment says
  the general rule is `180 - r` (a TRRS jack at an edge must face outward). The shipped Sofle right half (J2 TRRS and RSW1 at heading 90,
  `recipes.rs` `add(...)` with `angle(r)`) matches `-r`. ADR and implementation disagree; this needs a decision
  and a follow-up ticket.
- Single-colour LED: not tested (no fixture).
- Stabiliser symmetry: unknown; they are flagged if off the grid.
- The finding is deterministic. ADR-0010 says "when Core is unsure". The rule is conservative: it flags every
  handed mirror part off the grid.
- Diode placement (decision 4) needs the user.
- Trailer: the brief asked for `Co-Authored-By: Claude Sonnet 5.5` and the session's attribution says Haiku 5.5.
  The claim commit used Sonnet; the implementation commit uses Haiku 5.5.
- The GitNexus index is stale; impact and detect-changes results are provisional.
