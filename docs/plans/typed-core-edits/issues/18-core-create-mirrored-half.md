# 18: Core CreateMirroredHalf

Status: resolved
Type: build
Blocked by: [17](17-mirrored-half-schema-and-format-5.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md) · [16](16-native-mirrored-halves.md)

## Request

Create the second half from the first in Core. Today `CreateMirroredPair`
(`core/src/lib.rs:1512`) works only for two layouts on one board and maps by a vertical axis.

## What to do

- TDD at the Session/Core seam. `EditOperation::CreateMirroredHalf {pairId,
  sourceInstanceId, mode: new-board{boardId, instanceId} | reuse-board{instanceId},
  mapping, idRule}`. Validate fresh ids, an existing source, a finite map, an unpaired source.
- Generalize `layout::reflected` (`core/src/matrix/layout.rs:93`) to the pair's `Pose2` map
  with the single heading rule and source numbering from ADR-0010. Copy and reflect
  matrices, layouts, parts, the board, fixed outline versions, modules, nets and the
  electrical board config; never copy `case-closure/*` (regenerated). Link the mechanical
  pair and fill the correspondence table.
- Report every new id plus the source ids; one Undo step. `reuse-board` mode makes a
  flipped instance and copies nothing, matching what physical setup produces today.

## Acceptance

- [ ] A one-half Corne seed plus the operation reproduces the current right-half key
      positions within 1e-9 by correspondence.
- [ ] Changed ids reported exactly; one Undo step; fresh-id and double-pair errors do not mutate.
- [ ] Reuse-board equals today's physical-setup result.

## Verification

Core native tests; boundary transcript appended and passing native and WASM.
GitNexus `impact` on `layout::reflected` and the edit apply path first.

## Carried from 17's verification

- Add the item kinds the record still lacks when this ticket first writes them: embedded circuits,
  the electrical board configuration, fixed outline versions (`BoardOutline` is per board) and keycap
  configuration. `MirroredItemKind` has `module` already. Validation must learn each kind's owner
  (`core/src/mechanical/halves.rs` `item_exists`).
- Validation allows source and mirror on one board (reuse-board) but refuses a second record that
  names a board already in a record. Decide whether a reused or reversible board can be a half of
  several records, and relax the rule with a test if so.
- `halves` requires `constructionSource`, and its boards must be the boards of the two members, so
  create the pair and its instances in one accepted edit.
- Linking a pair never empties a half's stored closure list; a created half starts with none.

## Outcome

Implemented by Haiku 5.5 (claimed in `584b4ea22`). Not verified; status is for the verifier.

### What changed

- **Schema** (`core/src/model.rs`): `EditOperation::CreateMirroredHalf { pairId, sourceInstanceId, mode, mapping: Pose2, idRule }`.
  `mode` is `{kind: "new-board", boardId, instanceId}` or `{kind: "reuse-board", instanceId}`; `idRule` is `"board-prefix"`.
  `MirroredItemKind` gains `electrical-board` and `outline-version`, validated in `halves.rs` `item_exists`.
- **Operation** (`core/src/mirrored_half.rs`, new): built on a clone of the document and committed only after
  `layout::validate` and `halves::validate_document` pass. Wired in `lib.rs` `apply` and `affects_outline`;
  a preview of this operation is refused (commit only).
- **Reflection** (`core/src/matrix/layout.rs`): `reflected(source, target, mapping: Pose2, local_companions)`
  replaces the axis argument. `axis_mapping(axis_x)` gives the old map, so `sync` and `CreateMirroredPair`
  pass the same numbers as before. The assembly rule in `sync` now also goes through `pair::forward_vector` and
  `pair::forward_angle`, so there is one reflection rule. `local_companions` is `false` for sync (it keeps the
  other half's local companions, as before) and `true` for a new half (it has none to keep).
- **Tests**: `core/src/mirrored_half.rs` unit tests (id rule, heading rule, non-finite map refused with no
  change); `core/tests/mirrored_half.rs` (7 integration tests, registered in `core/tests/integration.rs`);
  two refusal requests appended to `core/tests/support/boundary.rs`, and the transcript re-blessed.

### Design decisions (the verifier should check these)

1. **Reflected heading.** One rule: `forward_angle` (`pair rotation - angle`). For a footprint with no rotation
   on a zero-rotation map this equals the recipes' `-r`. On a non-zero map it is the pair's own rule; there is
   no `180 - r` or `-r mod 180`. Chiral footprints keep their side. A part that carries a `reversible`
   generator parameter gets `reversible: true` on its mirror. Matrix members come from the matrix definition,
   which is shared, so key switches stay on the same definition (its `reversible` is `false`). The
   reversible-variant rule is therefore only partly implemented.
2. **Numbering.** The mirror keeps the source's matrix numbering and the mirror flag toggles, as `reflected` does.
   Column 0 is the same physical column on both halves.
3. **Ids.** `board-prefix`: every whole token equal to the source board id (split at `/` and `-`) becomes the mirror
   board id. An id with no such token gets `<mirror>/` in front. Non-matrix reference tokens use the same rule
   (`left-U1` becomes `right-U1`), with `<mirror>-` in front when there is no token. Matrix members, which
   `set_matrix` generates, match the mapped source ids (`matrix/left-keys/r0c0` becomes `matrix/right-keys/r0c0`).
4. **Mirror configuration.** The mirror's stored mechanical configuration is the source's effective configuration
   with `openings` set to `Some(vec![])` and `mounts` cleared. Closure mounts are never copied, and linked features
   wait for reconcile (ticket 19). The source instance keeps its configuration and takes `constructionOwnership: pair`
   (materialised first if it had none), as `link_mirror_pair` does.
5. **Mirror instance.** Half is the opposite of left/right (other labels are kept). Role is `peripheral` for
   right and `central` otherwise, matching the recipes. Name is `Left half`/`Right half` or `<name> mirror`.
   Not flipped. Controller is the mapped source controller.
6. **Board name** is `<source name> (mirror)`.
7. **One record per board** (ticket 18 open question). I did not relax the rule. A reused board is one record,
   and a board already named by a record refuses a second one, including a reused board.
8. **Reuse-board** copies nothing. It creates a flipped instance (`half: right`, `role: peripheral`,
   `name: Right half`, no controller) and the pair with the mapping given. It refuses an already-flipped source.
9. **Changed ids** are the pair, the source instance, the new instance, the new board, and every correspondence's
   source and mirror id. The electrical correspondence is board-level, so it adds the source and mirror board ids.
10. **Refusals**: a board that hosts a mounted module or an embedded circuit is refused, and so is a
   fixed outline version with protected gaps. These are refused, not copied.

### Deviations from the brief

- **Modules and embedded circuits are not copied.** They carry attachment, connection and support geometry
  with no reflection rule yet, so a source board that hosts any is refused, with no change.
- **Protected outline gaps are not copied**, for the same reason. A version that has any is refused.
- **Reuse-board's mechanical configuration** differs from physical setup. Physical setup clones the retained
  configuration (openings and mounts kept when eligible) and creates no `halves` record. This operation stores the
  cleared configuration and a `halves` record with the same board on both sides.
- **Keycap configuration** is copied by its board, matrix and key keys, with no correspondence kind of its own.
  A setting that already names a mirror id refuses.
- **Traces and vias** are not copied, as ADR-0010 requires. **Assemblies** (document-level definitions) are shared
  and not copied.
- **Layouts** are copied as independent layouts (`mirrorLink` absent).

### Commands (sequential, foreground, one at a time)

- `cargo test -p boardstudio_core --lib mirrored_half`: red first, with a stub operation: 2 failed (`not implemented`,
  id rule). After implementation: 3 passed.
- `cargo test -p boardstudio_core --test integration mirrored_half`: red first with the stub: 7 failed with
  `CreateMirroredHalf is not implemented`. Then two rounds of real failures were fixed: the member assemblies (the
  recipe marks them cell-local, so the reflection skipped them, which led to the `local_companions` flag), and the
  test's undo comparison (undo and redo take a new revision). Final: all 7 pass.
- `python3 scripts/check.py repo`: passed (158 Markdown files).
- `python3 scripts/check.py lint`: first run failed on one clippy `clone_on_copy` (fixed). Final run passed.
- `python3 scripts/check.py test`: passed, 120.2 s.
- `python3 scripts/check.py typecheck`: passed, 31.4 s.
- `python3 scripts/check.py browser` (sandbox lifted for wasm-pack and Chrome): passed, 273.6 s,
  `run-wasm-tests: executed 70, failed 0`.
- The browser step does not run the boundary test, so I ran it explicitly:
  `wasm-pack test --headless --chrome web/crates/host --locked --features page --lib -- boundary_parity::`
  (sandbox lifted): `1 passed; 0 failed`.
- Boundary transcript: `BOUNDARY_BLESS=1 cargo test -p boardstudio_core --test integration boundary_parity`. Diff
  against the previous transcript: 12 lines added, 0 removed (the two new refusal entries).
- Not run: `build` (no demo fixture or generated file changed), `deny` (no dependency change), `release` (never).

### GitNexus

The index is stale (last indexed 4b372b2), so every result below is provisional.
- `impact` before editing: `layout::reflected` CRITICAL (16 impacted, 6 processes). `layout::sync` CRITICAL
  (41 impacted) and was edited after the check, so it is disclosed here. `apply` (lib.rs) MEDIUM (32 impacted).
  `MechanicalMirrorPair` HIGH (18). `EditOperation` UNKNOWN (the index does not resolve the enum; its callers are
  `apply`, `affects_outline` and `edit`, confirmed by text search). `item_exists` and `affects_outline` UNKNOWN
  (private helpers the index does not resolve; callers confirmed by text search). `kind_name` LOW (1 impacted).
- `detect-changes --scope all` (after the final code): 7 files, 24 symbols, 6 processes, risk high. The high
  rating comes from stale line mapping (it lists `MechanicalFeature` and similar, which I did not edit), and it
  does not include the three new untracked files. It is not a clean check.

### Unverified

- Reuse-board equality with physical setup is checked against field values read from `physical_setup.rs`, not
  against physical setup's output. The mechanical configuration differs as described above.
- No test covers keycap copies, fixed outline versions (geometry, provenance), the `reversible` rule, the layout
  copies, or electrical config beyond its reopen. Electrical `locks`, `assignments` and `keyBindings` are reflected
  by key or value only when they name a copied part or net. Their meaning is not verified.
- The mirror's outline and key geometry are not compared with the shipped right outline. Ticket 06's KLOR gap-repair
  mismatch is unaddressed here.
- The boundary transcript records only the two refusal paths, because its fixture has no physical hardware. The
  success path is native-only in this ticket.
- Reconcile (19), recipes (20) and the web adapter (21) are untouched.


## Verification

Sonnet 5.5 verifier (stopped by the user after running over an hour; its work was checked and
committed by the integrating session on 2026-10-10). Haiku 5.5 implemented the first version.

**What the verification changed** (one commit on top of `af4c329fd`):

- Mirror mechanical features are linked pair features in both modes (not cleared, not cloned);
  `ReuseBoard` creates the pair record and linked features; the source's board id,
  stabilizers, hardware and critical fits are not copied; `case-closure/*` parts are not copied.
- A part's side and every generator parameter, including `reversible`, are copied unchanged
  (test `parts_keep_their_side_and_every_generator_parameter_exactly`).
- Outline geometry anchored to a part is reflected in the part's frame, protected gaps and
  keep-gaps are mapped, and outline identities stay unique.
- Layouts, keycaps (legends copied, key bindings not), net names and electrical configuration
  follow the mapped ids; `ReuseBoard` refuses a map other than the flip.
- The mirrored-half behaviour tests number 19 in `core/tests/mirrored_half.rs`.

**Commands observed (final tree):** `lint` passed (13.7 s); native `test` passed (106.8 s);
`typecheck` passed (14.3 s); wasm `boundary_parity::` passed (36 s); `cargo test ... mirrored_half`
19 passed (50 s). Mutations: 7 of 8 killed (fixed `case-closure` filter, key-binding copy, board id,
outline X reflection, side, `reversible` write, reuse-board map check); one survived (see below).
The mutated sources were restored and compared to the saved good copies. `browser` and `build`
were not run (batch boundary); `release` was never run.

**Gaps carried to follow-up tickets (not done in 18):**

- Mutation 7 survived: `layout.rs` `cell.rotation = -angle` replaced by `pair::forward_angle` is not
  caught by `a_rotated_pair_map_reflects_...`. Ticket [29](29-mirror-switch-and-companion-headings.md)
  must add a switch-heading test under a rotated map that kills it.
- The uncertain-heading finding (non-symmetric footprint at a non-multiple of 90 degrees) is not
  implemented: ticket 29.
- No equivalence test against physical setup's real output: ticket 26 (the comparison belongs in
  `web/crates/catalogue`, which can call both).
- Modules and embedded circuits on the source board are refused by name: tickets 27 and 28.
- The new test file is large (about 1,500 lines); the test audit (24) should look at it.
- GitNexus index is stale; `detect-changes` listed `sync`, `SceneDelta` and others, no clean check.
