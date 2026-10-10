# 17: Mirrored-half schema and format version 5

Status: claimed
Type: build
Blocked by: [16](16-native-mirrored-halves.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md) · [16](16-native-mirrored-halves.md)

## Request

Persist the pair record ADR-0010 describes. `MechanicalMirrorPair` (`core/src/model.rs:1025`)
has a `Pose2` map and linked features but no construction source and no record of a mirrored
half. Generator-parameters 06 uses "first member" as the source because the pair does not
say which half is.

## What to do

- Extend the pair with `constructionSource` and `halves {sourceBoardId, mirrorBoardId,
  correspondences[{kind, sourceId, mirrorId}], exceptions[{kind, mirrorId, state}]}`, all
  optional with serde defaults (absent means unpaired). Set `constructionSource` from
  `link-mirror-pair`, and replace 06's "first member" rule with it.
- Add a v4→v5 migration that changes only `formatVersion` (`core/src/migrate.rs`), so an
  older build refuses a v5 document ([ADR-0004](../../../adr/0004-document-format-version-and-id-rename.md)).
- Validation: unique correspondences, existing ids, an exception only on a mirror id,
  one half not paired twice. Errors do not mutate.
- Archive and browser-persistence parity; update the literal-version fixtures and re-bless
  the boundary transcript (append-only unless a version line must change).

## Acceptance

- [ ] v4 documents open unchanged apart from `formatVersion`; v5 round-trips.
- [ ] Malformed pairs are refused with a located message and no mutation.
- [ ] `constructionSource` is recorded and used by the closure rule from 06.

## Verification

`core/tests/current_schema.rs`, `legacy_archive_migration.rs`, boundary parity native and
WASM; `repo`, `lint`, `test`, `typecheck`, `browser`. GitNexus `impact` on `migrate_value`
(CRITICAL when last measured) before editing. Never run `check.py release`.
