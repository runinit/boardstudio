# 17: Mirrored-half schema and format version 5

Status: implemented (awaiting verification)
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

## Outcome

Implemented by Haiku 5.5 (the implementer role). Not verified by a separate verifier, so this
is not `resolved`. Ticket claimed in commit d39e085da.

### What changed

- **Schema** (`core/src/model.rs`): `MechanicalMirrorPair` gains optional `constructionSource`
  and `halves`, both absent when unset. New types: `MirroredHalves` (`sourceBoardId`,
  `mirrorBoardId`, `correspondences`, `exceptions`), `MirroredCorrespondence` (`kind`, `sourceId`,
  `mirrorId`), `MirroredException` (`kind`, `mirrorId`, `state`), `MirroredItemKind` (`matrix`,
  `layout`, `part`, `outline`, `net`, `assembly`, `mechanical-feature`) and `MirroredExceptionState`
  (`local`, `omitted`). Field names are camelCase; enum values are kebab-case, as in
  `MechanicalFeatureKind`.
- **link-mirror-pair** (`core/src/mechanical/edit.rs`) records `constructionSource`; `halves` is
  left absent.
- **Closure rule** (`link_pair_closure_proposals`) uses `constructionSource`, and falls back to
  the first member when it is absent. For a pair that records a source, the other half's stored
  closure list is also emptied (see open points).
- **Migration** (`core/src/migrate.rs`): `CURRENT_VERSION` is 5. `version_4_to_5` changes nothing
  but the version. ADR-0004 has a follow-up paragraph.
- **Validation** (new `core/src/mechanical/halves.rs`, `validate_document`, called from
  `features::validate`): the construction source must be a pair member; each board may be a half
  of one record only; both boards must exist; correspondence ids must exist on their board
  (part, outline and net are board-scoped; matrix, layout and assembly are document-scoped;
  mechanical-feature is checked against the pair's features); no repeated source or mirror per
  kind; exceptions name an existing mirror item that no correspondence links, and no repeats.
  Errors name the pair and the index (`halves.correspondences[1]`).
- **Literal fixtures**: `archive.rs` (test fixture and assertion), `demo_projects.rs`,
  `storage_lifetime_tests.rs`, `core/tests/outlines.rs` inline document,
  `core/tests/fixtures/reviung41-outline-original.json` and
  `web/crates/layout/tests/fixtures/sofle-v2.json` (top-level `formatVersion` 4 to 5 only).
  `Open` refuses any document not at `CURRENT_VERSION`, which is why these had to change.
- **Pair literals** in five web and core sites gain `constructionSource: None, halves: None`.
- **Boundary transcript** re-blessed (`BOUNDARY_BLESS=1 cargo test -p boardstudio_core --test
  integration -- boundary_parity`). Diff: 53 `formatVersion` values 4 to 5; 4 archive buffer
  digests and 3 embedded `projectJson` strings. The decoded projects differ only in
  `formatVersion`. The buffer lengths change by one byte, consistent with deflate output of a
  one-character change. No other lines changed.

### Tests (TDD)

- Added: `halves.rs` (11 unit tests), `migrate.rs` (`version_4_to_5_changes_only_the_format_version`,
  `version_5_documents_with_mirrored_halves_keep_their_text`), `current_schema.rs` (3 serde tests),
  `mechanical_integration.rs` (`closure_proposals_come_from_the_recorded_construction_source`,
  `closure_proposals_fall_back_to_the_first_member_without_a_recorded_source`), `archive.rs`
  (`version_5_mirrored_halves_cross_the_archive_boundary_unchanged`), and wasm
  `storage_lifetime_tests.rs` (`mirrored_halves_survive_browser_storage_at_format_five`).
- Replaced the pinned `closure_proposals_come_from_the_first_member_even_when_the_second_is_the_source`.
- Red, observed: before implementation the new tests did not compile (no `validate_document`, no
  `construction_source` or `halves` fields). With a temporary accept-everything stub, 17 lib tests
  failed (the refusals, the version literals and the v4 to 5 test), and the closure test failed on
  `a stores no private closure mount: left Some(4)`. The stub was then removed.
- Green: the targeted groups passed. Then the steps below.

### Commands (sequential, foreground, one at a time)

- `python3 scripts/check.py repo`: passed (158 Markdown files).
- `python3 scripts/check.py lint`: passed (`cargo fmt --all --check`, clippy `-D warnings`), first and final run.
- `python3 scripts/check.py test`: first run **failed**, for environmental and fixture reasons,
  and was fixed before the passing run. (a) The footprints test binaries were stale: they were built
  from `/tmp/claude-1000/boardstudio-tce-verify` through the shared `target` directory, so
  `context_vectors.json` was not found. I touched `footprints/tests/*.rs` (mtime only, no content
  change) to force a rebuild. (b) 29 `outlines` tests and the firmware projection test opened
  version 4 documents, which `Open` refuses; fixed by the fixture bumps above. (c) The transcript,
  re-blessed. Final run after the last test additions: passed, 94.5 s.
- `python3 scripts/check.py typecheck`: passed, first and final run.
- `python3 scripts/check.py browser` (sandbox lifted for wasm-pack, Chrome and the podman CAD test):
  passed on the final tree, 187.9 s, `run-wasm-tests: executed 70, failed 0` (an earlier run, before the
  last test additions, took 273.8 s and also passed).
- The `browser` step does not run the wasm boundary test: its host command filters to
  `host::storage::`. I ran it explicitly with `wasm-pack test --headless --chrome web/crates/host
  --locked --features page --lib -- boundary_parity`, which passed. The new browser storage test
  passed with the same command shape (filter `mirrored_halves`).
- `build`: not run. No demo project or generated file changed, and the demo baseline pins no
  `formatVersion`. The demo test asserts 5 and passes. `release`: never run.

### GitNexus

- `impact` before editing (CLI, because the MCP name lookup failed for two symbols):
  `migrate_value` CRITICAL (17 impacted, shared axes MEDIUM); `MechanicalMirrorPair` HIGH (18
  impacted, through `link_mirror_pair`, `mirror_pair` and the feature tests); `link_mirror_pair` LOW
  (17 impacted). `link_pair_closure_proposals` is UNKNOWN: it is a private helper that the index
  did not resolve, so I confirmed its only caller by reading `edit.rs`.
- The index is stale (last indexed 4b372b2; `analyze` was not run). `detect_changes(all)` reported 23
  changed symbols, 0 affected processes, risk low. It also lists symbols I did not edit (such as
  `EditOperation` fields and `FloatShapes`), so the result is provisional.

### Open points for verification

1. **Closure rule beyond the literal ticket.** For a pair that records `constructionSource`, the
   other half's stored closure list is also emptied. Stored closure lists are read as local
   hardware (`inputs.rs` `local_conflict`). A list proposed by the first member before linking
   would otherwise duplicate the linked mounts. Legacy pairs (no source) are unchanged. The verifier
   should confirm this is wanted.
2. **Load-time refusal not wired.** The archive unpack, browser decode and `Open` paths do not run
   `features::validate`. A malformed stored `halves` record is refused when an edit is applied,
   not when the file is loaded. Loading validation would also refuse existing malformed documents,
   so that is a separate decision.
3. **Refused records and mutation.** The validator takes `&ProjectDoc`. No edit in this ticket creates
   `halves`, so no test drives a refused record through `apply` to check that the document is left
   unchanged.
4. **Reuse-board (ticket 18).** Source and mirror on the same board are accepted within one record,
   but two pairs that both reuse one board are refused as "a half of two pairs". Ticket 18 must
   decide whether a board can be a half of several records.
5. **Not done here.** Mutation checks (the verifier's job). Generator-parameters 06 still states the
   "first member" rule in prose; that resolved ticket is historical, so I did not edit it. Keys are
   covered by `matrix` (matrix cells), and wiring by `net`.
