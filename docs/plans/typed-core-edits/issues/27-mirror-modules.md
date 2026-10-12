# 27: Mirror modules in CreateMirroredHalf

Status: resolved
Type: build
Blocked by: [18](18-core-create-mirrored-half.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

Ticket 18 refuses a source board that carries modules (a named, located refusal), although
ADR-0010 says modules are reflected. No shipped split half has a module (only
`vik-module-review`, a single board). User, 2026-10-10: follow-up before [19](19-reconcile-mirrored-halves.md),
because 19 requires "a module edit moves both halves".

## What to do

- Reflect each module: `at` through the pair map, `rotation` through the reflected heading;
  keep the definition, faces, gap, attachment and supports. Map the module id,
  `host_connector_part_id`, `bus_id`, the upstream ids and `host_instance_id` (to the mirror
  instance). Add `module` correspondences (kind exists since 17).
- Chirality is decided (see Decision): module geometry mirrors
  only when the host is `flipped` (`placement.rs` ~:347), so the same definition at a mirrored
  pose is not a mirror image. For `reuse-board`, `resolve` already reflects modules for a flipped
  host (`placement.rs` ~:545-552); only modules pinned by `host_instance_id` need mapping.
- Never change module or generator parameters.

## Decision

User, 2026-10-10 ([ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md#amendment-the-heading-rule-in-practice-2026-10-10)):
a module takes the same heading rule as any part, `180 - r` over the full 360 degrees; the module
is not flipped. Core adds a finding when the result is uncertain (a handed module), asking a
person to confirm; it neither refuses nor stays silent.

## Acceptance

- [ ] A source board with modules mirrors; ids and correspondences exact; one Undo step.
- [ ] Chirality behaviour as answered, with a test per case.
- [ ] `vik-module-review`-style fixture used for the tests.

## Outcome

Implemented by Haiku 5.5 (claim commit `04175f20f`, implementation `8504531bc`). Verified by Sonnet 5.5; see
[Verification](#verification), which settles the on-grid claim below.
The brief named a Sonnet 5.5 trailer; the commit carries `Co-Authored-By: Claude Haiku 5.5`
because Haiku 5.5 wrote it.

### What changed

- `core/src/mirrored_half.rs`
  - `new_board` reflects mounted modules on the source board instead of refusing them. `mirror_module`
    maps `at` through the pair map and takes the heading through `mirror_pose` (the ticket 30 rule,
    `rotation + 180 - r` over the full circle). It keeps `definitionId`, `hostFace`, `facingFace`,
    `gap`, `attachment`, `detached`, `serviceClearance`, `mountSupports`, and the connection's
    `modulePortId`, `cableType`, `assignments`, `railVoltages` and `upstreamPortId`.
  - Ids: the module `id` and `busId` take the board-prefix rule; `upstreamModuleId` goes through the
    module map; `hostConnectorPartId` goes through the part map (an empty connector stays empty);
    `hostInstanceId` maps the source instance to the mirror instance. Module ids are minted with the
    other new ids, and `taken_ids` now reserves existing module ids, so a collision refuses the record.
  - Each mirrored module adds a `module` correspondence (`halves::item_exists` already checks it) and
    appears in `changedIds`.
  - `hosted_items` names only embedded circuits, which are still refused by name (ticket 28).
  - `reuse_board` copies no module. A module pinned to the source instance is refused by name, because
    the flipped instance would not show it. An unpinned module is not copied: `resolve` reflects it on a
    flipped host.
  - `module_findings` (new): one Warning per mirrored module, `mirror/<module id>/handed-module`, scope
    Pcb, targeting the mirror module. Core neither refuses nor turns it.
- `core/src/validate.rs`: one added line, `findings.extend(crate::mirrored_half::module_findings(doc));`.
- `core/tests/mirrored_half.rs`: four new tests and one rename. `hosted_modules_and_embedded_circuits_are_refused_by_name`
  became `hosted_embedded_circuits_are_refused_by_name` (its module part was removed in the change).
  - `two_hosted_modules_mirror_with_exact_ids_poses_and_correspondences`: two chained catalogue
    `vik-splitter` modules on the Corne left half, under an axis map (300, 0) and a rotated map (310, 42,
    rotation 27). It checks ids, module correspondences, positions to 1e-9 (`reflected_point`), headings
    (source 0 and 135, so 180 and 45 on the axis map and 207 and 72 on the rotated map), unchanged fields,
    connector, bus, upstream, host instance, changed ids and one Undo step.
  - `a_module_the_mirror_cannot_place_refuses_the_record_and_changes_nothing`: a pin to another instance,
    a connector off the board, and an upstream module off the board are each refused by name, with the
    document byte-identical (`refused`).
  - `reuse_board_copies_no_module_and_refuses_one_pinned_to_the_source`.
  - `every_mirrored_module_raises_one_handed_module_warning_and_a_source_module_raises_none`.
- Fixture: `add_two_splitters` injects two catalogue definitions and modules into the seed as JSON. The
  brief allowed this as the module fixture route; `set-mounted-module` would also need a connector
  definition.

### Decisions

1. Heading: a module uses the same function as a part (`mirror_pose`). The module is not flipped:
   `hostFace` and `facingFace` are kept. `flipped` is not stored; `resolve` derives it from the two faces
   (`placement.rs` ~:654), so it stays as in the source.
2. Handed-module finding: no `ModuleDefinition` field marks symmetry (checked in `core/src/modules.rs`),
   and the board outline does not show components or connectors. So every mirrored module is flagged,
   the brief's fallback. No geometric check was written.
3. Refusals, not silent keeps, for the three references the mirror cannot follow (a pin to another instance,
   a connector and an upstream module off the source board). A kept id would point the mirror at the
   source's hardware.

### Derived, not tested (for the verifier; confirmed by the verifier, see Verification)

Modules and parts keep their own geometry; only the pose is reflected. The reflected pose gives the mirror
image of that geometry only if the geometry is symmetric about its own heading axis (`y -> -y` in its
frame). Worked example, by hand: a single pad at local (1, 0.5) on a footprint at heading 0, on the axis
map at x = 300. The source pad maps to (299, 0.5). The mirror part sits at (300, 0) with heading 180, so the
pad lands at (299, -0.5). This is independent of the 90-degree grid. So ticket 30's verification statement
(on-grid placement is certain) holds only for footprints symmetric about their heading axis. Nothing was
changed for this: it is for the user to decide (ADR-0010 amendment) and for tickets 20, 25 and 30 to review.

### Commands (sequential, foreground)

- Red, before the implementation: `cargo test -p boardstudio_core --test integration mirrored_half::`,
  22 passed and 4 failed (the new module tests, with the refusal "hosts module ... cannot be mirrored yet"),
  50.0 s. A helper bug in the first draft (`correspondences` is omitted when empty) made the reuse test
  panic on an unwrap; that was fixed in the test, not the product.
- After the implementation: 25 passed, 1 failed. The failure was my test's own Undo assertion (an empty
  `pairs` list is omitted from the document). Fixed in the test. 51.2 s.
- Green: 26 passed, 53.8 s.
- Mutations: `cargo test -p boardstudio_core --test integration module` (58 tests matched, about 52 s
  each). The source was restored from a saved copy and checked with `cmp` after every run (all ten
  restored). Each mutation was caught:
  - A bus id not mapped: `two_hosted_modules...` fails.
  - B host instance dropped (`None`): `two_hosted_modules...` fails.
  - C heading not turned (`module.rotation`): `two_hosted_modules...` fails.
  - D heading `-r`: fails.
  - E heading modulo 180: fails. The fixture headings were set to 0 and 135 before the sweep, because
    under 90 and 180 the modulo-180 mutation gives the same results (checked by arithmetic, not run).
  - F upstream not mapped: `two_hosted_modules...` and `a_module_the_mirror_cannot_place...` fail.
  - G connector not mapped: the same two tests fail.
  - H module id not mapped: three tests fail.
  - I handed finding disabled: `every_mirrored_module...` fails.
  - J reuse-board pin refusal disabled: `reuse_board_copies_no_module...` fails.
  Ten of ten caught. The sweep ran for about 10 minutes wall time.
- `python3 scripts/check.py repo`: passed, 0.1 s (165 Markdown files). (Repeated after the ticket edit; see below.)
- `python3 scripts/check.py lint`: the first run failed on two clippy errors in the new test code
  (`type_complexity`, `redundant_closure`), 7.6 s. Fixed; rerun passed, 5.9 s. `rustfmt --edition 2024`
  was run on the three Rust files first, and it changed nothing else.
- `python3 scripts/check.py typecheck`: passed, 10.0 s.
- `python3 scripts/check.py test`: passed, 106.1 s. Integration: 542 passed, 0 failed, including the 26
  mirrored-half tests and `boundary_parity::native_transport_reproduces_the_golden_transcript`.
- Not run: wasm boundary parity (the transcript and its refusal entries are unchanged, and the change is
  not in the transcript), `browser`, `build`, `deny`, `release`.

### GitNexus

- The index is stale (last indexed `4b372b2`). `impact` on `validate` (`core/src/validate.rs`): CRITICAL, 29
  impacted; I warned before the one-line edit. `new_board`, `hosted_items` and `reuse_board` are not in the
  index ("Target not found", risk UNKNOWN). Their callers were checked by text search: `new_board` and
  `reuse_board` only from `create`, and `hosted_items` only from `new_board`.
- `detect-changes --scope all`: 3 files, 1 symbol (`validate`), 0 processes, risk low. This is not a clean
  check, because the index does not contain the new or edited private functions.

### Unverified and follow-ups

- Reflection of an unpinned module on a flipped ReuseBoard instance is not tested here; the test checks only
  that nothing is copied.
- ReuseBoard still does not refuse embedded circuits (unchanged). Ticket 28 should decide.
- A module's own geometry is not checked: every mirrored module is flagged. A real discriminator needs a
  symmetry mark on `ModuleDefinition` (a format change under ADR-0004) or a geometry check that includes
  components.
- The on-grid claim in the derivation above is a follow-up for the user's decision and for tickets 20, 25 and 30.
- A mapped `busId` is not checked for uniqueness against other buses.
- Modules pinned to another instance on the same board are refused, not mirrored, because the mirror has no
  instance for them.
- The test uses the Corne `left/TRRS` part as the module connector. The test does not check that it is a
  valid VIK host connector; any finding that says so is not asserted.

## Verification

Sonnet 5.5, 2026-10-10, `tce-matrix` worktree on top of `8504531bc`. Reviewer pass, not a second
implementation. Acceptance holds (ids, correspondences, poses, refusals and one Undo step are
asserted and die under mutation), so the ticket is resolved. One finding changes the picture of ticket
30 and is for the user: see [Pad layout](#pad-layout-is-rotated-not-mirrored-the-on-grid-claim-is-wrong).

### Claims reproduced

- **Semantics read in `core/src/modules.rs` and `modules/placement.rs`.** `flipped` is not stored: `resolve`
  derives it from the two faces (`host_sign * face_sign > 0`, ~:654) and then mirrors the module's local `x`
  (`xy`, ~:347; footprints and volumes follow). Keeping `hostFace` and `facingFace` therefore keeps the
  module unflipped exactly as the source is, which is the decision (ADR-0010, "the module is not flipped").
  `host_instance_id` pins a module to one physical instance (`active_instances` skips a module whose pin is not
  the selected instance; `set` requires the instance to exist on the module's board), so the mirror pin must be
  the mirror instance on the mirror board, as the code does. The reuse-board view mirrors a module for a flipped
  instance itself (`host.flipped`, ~:545), so copying would double it; copying nothing is right.
- **Id-bearing fields.** `MountedModule` ids: `id`, `host_board_id`, `host_instance_id`; `ModuleConnection`:
  `host_connector_part_id`, `bus_id`, `upstream_module_id`. All are mapped. `ModuleSupport.mount_id` names a
  mount of the (shared) definition and is correctly unchanged; `assignments` are MCU terminal names, which the
  mirror keeps (the mirror keeps the source numbering). Nothing else in the document refers to a module id (a
  search of `core/src` for module id references found only `modules/`, `firmware.rs` uniqueness and
  `mechanical` lookups by the module's own resolved id). `taken_ids` now reserves module ids, so a collision
  refuses the record.
- **`busId` uniqueness: no change.** Core never reads `bus_id` (only the model field and the two web places
  that set it to `vik/<module id>`); there is no uniqueness rule for it on a single board, so a check only in the
  mirror would invent one. Two source modules on one bus keep one shared bus label on the mirror (the same
  deterministic map). Recorded, not changed.
- **Mutations (3 of Haiku's 10, redone by me; `cargo test -p boardstudio_core --test integration mirrored_half::`,
  about 59 s wall each, source restored with `cmp` against a copy and touched):**
  heading unturned (`mirrored.rotation = module.rotation`): `two_hosted_modules_mirror_with_exact_ids_poses_and_correspondences`
  failed (25 passed, 1 failed). Host instance kept as the source instance (instead of the mirror instance): the same
  test failed. Upstream module id not mapped: `two_hosted_modules...` and
  `a_module_the_mirror_cannot_place_refuses_the_record_and_changes_nothing` failed (24 passed, 2 failed).
  The other seven were not rerun.
- **Handed-module finding.** A Warning (`Severity::Warning`), scope Pcb, id `mirror/<mirror module id>/handed-module`,
  `target_ids` is the mirror module, message names it. Only `Severity::Error` blocks an output
  (`artifact/mechanical_plate.rs` is the only gate found), so it does not block export.

### Handed-module finding: keep, noisy by design

It fires for every mirrored module, because `ModuleDefinition` has no symmetry field and the data that would
decide it is not all there: the board contours and mounts say nothing about the connector, and the constituent
parts exist only when a `circuit` is embedded. A cheap reliable discriminator does not exist, so the fallback the
ADR allows (a finding, neither a refusal nor silence) is acceptable. The noise is real: there is no way to
acknowledge it, so it stays for the life of the pair. Follow-up (not done here, a format decision under ADR-0004):
a symmetry mark on `ModuleDefinition`, or a check of the module's circuit parts and mounts against the heading axis
(the same test as below), and an acknowledgement for the finding.

### Pad layout is rotated, not mirrored: the on-grid claim is wrong

Haiku's claim reproduces and is exact. The rule turns an independent part by `r_map + 180 - r` and does not flip
it. For a pad at part-frame `(x, y)` (front side; a back-side part reverses `x`, which changes nothing below):

- the true mirror of the source pad is at `F(W) + R(r_map - r) * (-x, y)`;
- the rule places the pad at `F(W) + R(r_map - r) * (-x, -y)`.

So the local `y` is reversed, and the placed pad is off by the vector `R(r_map - r) * (0, -2y)`, magnitude `2 |y|`,
zero exactly when the pad is on the heading axis (`y == 0`). This does not depend on the heading, so it holds at
0, 90, 180 and 270. Haiku's example: a pad at local (1, 0.5) at heading 0 on an axis map at x = 300 lands at
(299, -0.5), the mirror would be (299, 0.5). Ticket 30's verification statement that the on-grid rule is "certain"
holds for the heading and the part's position, not for its pads.

- **Test added (no behaviour change):** lib `a_turned_part_puts_a_pad_on_the_true_mirror_only_when_it_sits_on_the_heading_axis`
  (`core/src/mirrored_half.rs`). Headings 0, 90, 180, 270, front and back side, an axis map and a 27 degree map,
  pad offsets (1, 0), (-2, 0), (0, 0) [agree] and (0, 1), (0, -1), (1, 0.5), (2, -3) [differ]: the error equals
  `R(r_map - r) * (0, -2y)` to 1e-9, and is below 1e-9 exactly when `y == 0`. It passes (lib `mirrored_half::`,
  8 passed, about 10 s). It pins the arithmetic, not a decision; it documents the rule's actual behaviour.
  Anchored outline geometry is already compensated (`(x, -y)`, ticket 30), so this concerns only a part's own pads.
- **Which shipped footprints this matters for** (a scratch report, removed; pad count off the heading axis and the
  largest `|y|`, from the shipped `left` fixtures of each split project that has one):
  `demo/mcu_nice_nano` (U1: 25 of 27 pads, up to 15.24 mm from the axis, so the two ends of the pin rows swap and a
  pad can land 30.5 mm from where the mirror puts it) and `demo/trrs_pj320a` (all 6 pads, up to 11.3 mm) in Cantor, Chocofi, Corne, Klor and Lily58;
  in the Sofle splits `sofle/mcu_nice_nano` (25 of 27, 15.24), `sofle/trrs_pj320a` (6 of 6, 11.3),
  `sofle/display_ssd1306` (4 of 4, 16.7), `sofle/rotary_encoder_ec11_ec12` (5 of 7, 7.5) and, in Sofle RGB only,
  `sofle/led_sk6812mini-e` (4 of 4, 0.7). Not affected: both reset switches (`demo/reset_switch_tht_top`,
  `sofle/reset_switch_tht_top`: two pads on the axis) and the mounting holes (one pad on the axis). The key and
  diode definitions are not independent parts (switch rule, diode side is ticket 29's open decision).
  The shipped right halves keep U1 and the TRRS at the source heading (ticket 30's list), which differs from the
  rule for exactly these footprints.
- **Option proposed, not implemented (the user decides; the rule is unchanged):** replace the off-grid-only trigger
  of `mirror/<part>/heading-uncertain` by a Core finding for any independent part whose pad layout is not
  symmetric about its heading axis, computed from the footprint's pads: a pad with `|y|` above a tolerance and no
  pad of the same number, shape and size at `(x, -y)`. It would flag U1, the TRRS jack, the display and the encoder
  on every shipped split, and stay silent for resets and holes. Larger options for the same problem are a per-part
  `mirrored` footprint (flip, not turn) or a hand heading, both of which touch the rule itself.

### Not done, and why

- `busId` uniqueness: no rule exists to check against (above).
- Reflection of an unpinned module on a flipped reuse-board instance is the existing `resolve` path; still untested
  here (Haiku's note stands).
- Haiku's other seven mutations were not rerun.
- A module whose circuit or mounts are off the heading axis is the module analogue of the pad finding; follow-up.

### Commands (sequential, foreground)

- `cargo test -p boardstudio_core --lib mirrored_half::`: 8 passed, 10 s.
- Three mutation runs of `cargo test -p boardstudio_core --test integration mirrored_half::`: each 51 s test time,
  about 59 s wall; each failed as stated above and was restored (`cmp` clean) and touched.
- Scratch pad report (an ignored integration test, removed): 61 s and 49 s.
- `python3 scripts/check.py lint`: passed, 9.7 s. `test`: passed, 94.2 s (includes the native boundary parity).
  `typecheck`: passed, 9.4 s. `repo`: passed, 0.1 s (165 Markdown files).
- Not run: wasm `boundary_parity::` (the transcript is unchanged: no fixture, bless or golden edit, and the finding
  is raised only for a mirrored module), `browser`, `build`, `deny`, `release`.
- GitNexus: the index is stale. Edits are test-only and docs; no existing symbol was edited, so `impact` was not
  required. `detect-changes --scope all`: 5 files, only map sections as symbols, risk low, 0 processes; provisional
  because the index is stale (it does not know the new test).
