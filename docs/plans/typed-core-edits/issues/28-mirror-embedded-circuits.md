# 28: Mirror embedded circuits in CreateMirroredHalf

Status: resolved
Type: build
Blocked by: [27](27-mirror-modules.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

Embedded circuits are refused by ticket 18. Their parts and nets are ordinary host-frame parts and
nets (`circuit.rs` ~:173-195), so the generic loops copy them, but the board-prefix id rule yields
`right/embedded/...`, which breaks the `embedded/<id>/` namespace (`circuit.rs` ~:57, :258), and
the `EmbeddedCircuit` record itself is not copied.

## What to do

- Give circuits their own id rule that keeps the `embedded/<id>/` namespace; copy the
  `EmbeddedCircuit` record with mapped part and net ids; reflect the parts like any others.
- Correspondences for circuits; tests with the embedded-circuit fixture; mirror stays equal
  under edits (with 19).

## Acceptance

- [x] A board with an embedded circuit mirrors with valid namespaces and circuit records.
- [x] Mutation checks on the id rule.

## Verification plan

Core tests, boundary parity, `repo lint test typecheck browser`.

## Outcome

Implemented by Haiku 5.5 (claim commit `a261c25d8`; the implementation is the commit that carries this
Outcome). The brief named a Sonnet 5.5 trailer; the commit carries `Co-Authored-By: Claude Haiku 5.5`,
because Haiku 5.5 wrote it, as ticket 27 did. Verification is for the verifier; no step below was run as a
browser or build check.

### What changed

- `core/src/mirrored_half.rs`
  - `new_board` no longer refuses a source board with embedded circuits (`hosted_items` is removed).
  - `circuit_mirror` (new). A circuit's id takes the board-prefix rule: `left-haptic` mirrors as
    `right-haptic`, `review/haptic` as `right/review/haptic`. Its members and private nets move from
    `embedded/<source id>/` to `embedded/<mirror id>/`. A host net a port joins keeps the board rule and is
    the mirror host net. Its component definitions are copied under `embedded/<mirror id>/definition/`; the
    content is the source's, with only the ids changed (`mechanicalProfile.definitionId` follows).
  - A mirror circuit id that is already a circuit (or that another hosted circuit takes), or a mirror
    namespace that already holds a part, definition or net, refuses the record by name.
  - Members take the reflected pose through `mirror_pose` (the ticket 30 rule), as every independent part.
  - Circuit members allocate their mirror reference after the board's own parts (`free_reference`).
  - Each circuit becomes an `EmbeddedCircuit` record on the mirror (`hostBoardId` = mirror board, mapped
    `partIds`, `netIds` and `ports`) and an `embedded-circuit` correspondence. The copied definitions are in
    `changedIds`.
  - `reuse_board`: code unchanged; a comment says why circuits copy nothing.
- `core/src/model.rs`: `MirroredItemKind::EmbeddedCircuit` (serialized `embedded-circuit`). No existing kind
  fit; the ticket 17 note asked for it. `rg` finds no match of `MirroredItemKind` outside Core, so no web
  match breaks.
- `core/src/mechanical/halves.rs`: `item_exists` (a circuit is on its host board) and `kind_name`.
- `core/tests/mirrored_half.rs`: `hosted_embedded_circuits_are_refused_by_name` is replaced by four tests
  (below). Helpers: `imported_haptic` (a copy of the import in `module_circuit_electrical.rs`), `embed_haptic`
  (the `embed-module-circuit` edit), `board_nets`, `record`, `definition`, `net`, `pairs_of_kind`.

### Tests (`core/tests/mirrored_half.rs`)

- `a_board_with_embedded_circuits_mirrors_with_valid_namespaces_and_records`: the Corne left half with two
  embedded haptic circuits (`left-haptic`, joined to its first host net; `review/haptic`, unjoined), under the
  axis map 300 and the rotated map (310, 42, 27). It checks the mirror ids, the `partIds`, `netIds` and
  `ports`, each copied definition, side and generator parameters unchanged, positions to 1e-9 (`reflected_point`)
  and headings to `rotation + 180 - r`, the pins of private nets and of the joined net, the `part` and
  `embedded-circuit` correspondences, that no id contains `right/embedded/`, the changed ids, one Undo step
  back to the snapshot, and that the result reopens.
- `a_circuit_and_a_module_on_one_board_mirror_together`: the module pair of ticket 27 and a circuit on the
  same board mirror in one edit, with the module's `right/TRRS` connector kept.
- `a_mirror_circuit_id_already_in_use_refuses_the_record_and_changes_nothing`: a source circuit `right-haptic`
  next to `left-haptic`. The refusal names `"right-haptic" already exists` and the document is byte-identical.
- `reuse_board_copies_no_embedded_circuit_because_it_belongs_to_the_board`: the flipped instance gets no
  circuit or circuit part and no circuit correspondence.

Red, before the Core change (three runs, as tests were written): the module test failed on the old refusal
(`Board "left" hosts embedded circuit left-haptic, which cannot be mirrored yet`). The namespace test first
stopped at a missing net name in my fixture, which I fixed; in the next run it failed at its first `create`
(`core/tests/mirrored_half.rs:1659`, message not captured). The collision test passed in the first run for the
wrong reason: its first assertion matched the old refusal's text, which lists the circuit ids. After I
tightened it to the new message, it failed at its assertion (`:1870`). The reuse test passed before the change,
because that behaviour already held; it is a guard, not a red test.

Green: `cargo test -p boardstudio_core --test integration mirrored_half::` reports 29 passed, 0 failed (about
52 s test time, 57 to 78 s wall with compile).

### Mutation checks (each restored byte-identical, checked with `cmp`)

- Id rule: the circuit map is replaced by the board-prefix rule (`map_owned = |id| map(id)`). The namespace
  test fails at `core/tests/mirrored_half.rs:1691` (a member is outside its mirror namespace).
- Heading, whole pose: circuit members keep their source pose. Fails at `:1254`, the position check
  (`embedded/right-haptic/C1: (111, 46) vs (189, 46)`).
- Heading, rotation only: circuit members keep the source heading. Fails at `:1746`, the heading check
  (`embedded/right-haptic/H1: 0 from 0 under map 0`), so the heading assertion catches it on its own.

### Decisions

1. Circuit id: the board-prefix rule applies to the circuit's own id; the members and private nets are
   re-rooted at the mirror circuit id. The board rule on a member gives `right/embedded/...`, which
   `modules::circuit` does not read back as a circuit's private net (`electrical.rs:444`), so the id rule is
   the namespace fix.
2. Collisions refuse by name, and the mirror never renames an existing circuit.
3. Member references: a circuit allocates its references on its host board only, so a member `U1` can share
   a mirror reference with a host part (`left-U1` and `U1` both mirror to `right-U1`). The test found this as
   a refusal (`The mirrored reference "right-U1" is already in use.`). Circuit members now take the next free
   reference (`right-U2`), after the board's own parts. Host parts keep the refusal.
4. Definitions are copied, not shared: a mirror's members must not depend on the source circuit's namespace,
   which `remove_circuit` on the source would clear.
5. `reuse-board`: copies nothing, and does not refuse. An `EmbeddedCircuit` has no instance pin (its fields are
   `id`, `definitionId`, `hostBoardId`, `partIds`, `netIds`, `ports`), so the flipped instance shows the
   circuit the way it shows the board's parts. The refusal for a pinned module (ticket 27) has no circuit
   counterpart.
6. Part order: host parts are mirrored before circuit members. This changes the order of `parts` and of
   `correspondences` only when a circuit is present.

### Deviations from the brief

- Paths: the brief's `core/src/circuit.rs` is `core/src/modules/circuit.rs`, and `core/src/halves.rs` is
  `core/src/mechanical/halves.rs`.
- Fixture: `web/assets/fixtures/vik-module-review.json` hosts its circuit `review/embedded-haptic` on
  `main-board`, not in a module, and it has no `hardware` block, so no mirror pair can be made from it. The
  integration test uses the Corne left half instead. The model hosts circuits on boards only, so "a circuit
  embedded in a module" does not exist; the test covers a circuit and a module on one board.
- The Corne left board has no net named GND; the test joins the first host net of the board.
- GitNexus: the index is stale, and `impact` returned `Target 'new_board' not found` (also `hosted_items`,
  `item_exists`). I used a text search instead: `new_board` is reached only from `create`, `hosted_items`
  only from `new_board` (removed), `item_exists` and `kind_name` only from `halves::validate_halves`.
  `detect-changes --scope all` reported "Diff touched 4 file(s) but no indexed symbols overlap those hunks",
  which is not a clean check.
- `docs/adr/0010` is read-only here. Its "details of creation" amendment still says embedded circuits are
  refused, and the ticket 27 note says the refusal "now applies to embedded circuits only". Both are stale
  after this change and need a decision by the user.

### Verification run by the implementer

| Command | Result | Time |
| --- | --- | --- |
| `cargo test -p boardstudio_core --test integration mirrored_half::` (red, before Core change) | the new tests failed as above; reuse passed | 57 s wall |
| same, after Core change | 29 passed, 0 failed | 78 s wall (52 s test) |
| three mutation runs (id rule, heading whole pose, heading rotation only) | each fails as listed above | about 50 to 60 s each |
| `rustfmt --edition 2024` on the two changed Rust files; `cargo fmt --all --check` | clean | seconds |
| `cargo clippy --workspace --locked --all-targets --no-deps -- -D warnings` | exit 0 (the vendored `cgmath` warnings are not denied under `--no-deps`) | 11 s; cached rerun 0.4 s |
| `cargo test --workspace --locked --exclude boardstudio-footprints` | exit 0, 1271 passed, 0 failed (boundary golden transcript test included) | 108 s |
| `cargo check -p boardstudio-web --locked --target wasm32-unknown-unknown --no-default-features --features page --bin boardstudio-web` | exit 0 | 14 s |
| `python3 -B scripts/test-check-doc-links.py`; `python3 scripts/check-doc-links.py` | exit 0; 165 Markdown files | under 1 s |
| `node .gitnexus/run.cjs detect-changes --scope all --repo .` | "no indexed symbols overlap" (not a clean check, index stale) | seconds |

Not run: the wasm boundary parity run (the native golden transcript test passed, so the transcript did not
change, and the brief limits the run to a changed transcript); `browser`; `build`; `deny` (no dependency
changed); `cargo test -p boardstudio-footprints` and `cad/scripts/test-cadrum.py` (footprints is not touched;
the CAD step needs the container, so the sandbox must be lifted for it).

### Unverified, for the verifier

- The mirror board's `embedded_findings` (for example the existing "Copied components need measured body
  heights" Error) are computed per board from the record. I did not inspect them on the mirror.
- The heading finding (`heading_findings`) for circuit members is not tested.
- A source `remove-embedded-circuit` after mirroring, and the mirror under edits (ticket 19), are not tested.
- Acceptance boxes are left for the verifier; both mutation checks are done.

## Verification

Sonnet 5.5, 2026-10-10, `tce-matrix` worktree on top of `7a2b05537`. Reviewer pass, not a second implementation.
Acceptance holds (namespaces, records, correspondences, headings and the id rule are asserted and die under
mutation), so the ticket is resolved. Part of Batch A; its full check is recorded in the [map](../map.md).

### Claims reproduced

- **Id rule, read in `core/src/mirrored_half.rs` (`circuit_mirror`, `new_board`) against `core/src/modules/circuit.rs`.**
  `embed` writes members as `embedded/<id>/<member>`, private nets as `embedded/<id>/<net>`, definitions as
  `embedded/<id>/definition/<def>` (`circuit.rs` ~:52, :104-:118), and `remove_circuit` reads the same prefix back
  (~:262). The mirror does the same with the mirror circuit id (`left-haptic` to `right-haptic`, `review/haptic` to
  `right/review/haptic`, the board-prefix rule on the id alone). It refuses by name when the mirror circuit id is
  already a circuit or another hosted circuit takes it, and when the mirror namespace already holds a part, net or
  definition (the same three checks `embed` makes). Definitions are copied with only `id` and
  `mechanicalProfile.definitionId` changed; the test compares each copy with the source field for field. A host net
  a port joins keeps the board rule. Net names are copied by the generic net loop.
- **Mirror validity.** Reopening the mirror document works (the test reopens it). The circuit findings of the mirror,
  which `embedded_findings` computes per circuit, were not asserted by the implementer. I added
  `a_mirrored_circuit_reports_the_findings_of_its_source_and_no_reference_repeats`: for the Corne left half with two
  haptic circuits, the set of `embedded/<id>/...` findings (id, severity, message) on the mirror equals the source's
  with the circuit ids renamed, so the mirror has no error the source lacks (the Errors that remain, such as
  "Copied components need measured body heights", are the circuit's own and also stand on the source), and every
  target id of those findings exists on the mirror.
- **References.** A circuit member `U1` mirrors to `right-U1`, as does a host `left-U1`, so the collision is real in
  the fixture. The implementer's tests did not notice a duplicate: with `free_reference` replaced by a plain insert
  (so two parts keep one reference) all 29 of its tests still passed, and my new test failed at the uniqueness
  assertion. Host parts reserve their references first, then circuit members take the next free one, in source part
  order; two builds from fresh engines give identical references (asserted). A second mirror of the same source is
  not possible (`ensure_board_unclaimed`), so repeated mirrors cannot collide on a circuit reference.
- **Poses.** Circuit members use the same `mirror_pose` as every independent part (ticket 30 rule), and side and
  generator parameters are copied unchanged (asserted for both circuits under an axis map and a rotated map).
- **Mutations (2 of Haiku's 3, redone by me; `cargo test -p boardstudio_core --test integration mirrored_half::`,
  about 60 s wall each, source restored with `cmp` and touched):** id rule disabled (`map_owned = |id| map(id)`):
  `a_board_with_embedded_circuits_mirrors_with_valid_namespaces_and_records` failed at `:1699` (29 passed, 1 failed).
  Circuit member rotation kept (`p.rotation = part.pose.rotation` for circuit members): the same test failed at
  `:1776`. A third, mine, for the reference allocation, is described above. The whole-pose mutation was not rerun.
- **Reuse-board.** `EmbeddedCircuit` has `id`, `definitionId`, `hostBoardId`, `partIds`, `netIds` and `ports`: no
  instance pin. Its members are ordinary parts of the board, so a flipped instance of the board shows them as it
  shows the board's own parts. Copying nothing and not refusing is therefore right against the model, and the test
  pins it (no new circuit record, no new circuit part, no circuit correspondence).
- **Heading finding.** Circuit members are `part` correspondences, so they pass through `heading_findings` like any
  other part: a handed member off the 90-degree grid raises `mirror/<member>/heading-uncertain`. Settled by reading;
  not asserted by a test.

### Other changes in this pass

- ADR-0010: the "details of creation" line and the final ticket 27 note said embedded circuits are refused; both now
  say modules and circuits are mirrored (tickets 27 and 28). Tickets 19, 20 and 21 contain no refusal claim about
  circuits. Tickets 18 and 27 keep their historical text (their Outcome records what was true then).
- `core/tests/mirrored_half.rs`: the one new test above (no product change).

### Not done, follow-ups

- Mirror under edits (a circuit edited on one half, `remove-embedded-circuit` on the source after mirroring) belongs
  to ticket 19 and is not tested; the mirror circuit carries no reconcile behaviour yet.
- A circuit's parts have the same pad-layout limit as any independent part (ticket 27 Verification: the rule turns
  rather than mirrors; the haptic IC is not symmetric about its heading axis). The user's decision on that finding
  covers circuit members too.
- Prefix ambiguity that already exists in `embed`: circuits `a` and `a/b` share the textual prefix `embedded/a/`.
  The mirror inherits it only through definition and collision checks; not reachable with the shipped fixtures.

### Commands (sequential, foreground)

- `cargo test -p boardstudio_core --test integration mirrored_half::a_mirrored_circuit`: 1 passed, 59.9 s wall (about
  50 s test time with compile).
- Three mutation runs of `mirrored_half::`: each 29 passed, 1 failed, about 60 s wall.
- `python3 scripts/check.py repo`: passed, 0.1 s.
- GitNexus: the index is stale. `detect-changes --scope all`: 2 files, no indexed symbols overlap the hunks; not a
  clean check. No product symbol was edited in this pass (mutations were restored byte-identically).
