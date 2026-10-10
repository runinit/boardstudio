# 27: Mirror modules in CreateMirroredHalf

Status: implemented (awaiting verification)
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

Implemented by Haiku 5.5 (claim commit `04175f20f`). Not verified; status is for the verifier.
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

### Derived, not tested (for the verifier)

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

Core tests, boundary parity native and wasm, `repo lint test typecheck browser`.
