# 17: Mirrored-half schema and format version 5

Status: resolved
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

- [x] v4 documents open unchanged apart from `formatVersion`; v5 round-trips.
- [x] Malformed pairs are refused with a located message and no mutation.
- [x] `constructionSource` is recorded and used by the closure rule from 06.

## Verification

`core/tests/current_schema.rs`, `legacy_archive_migration.rs`, boundary parity native and
WASM; `repo`, `lint`, `test`, `typecheck`, `browser`. GitNexus `impact` on `migrate_value`
(CRITICAL when last measured) before editing. Never run `check.py release`.

## Outcome

Implemented by Haiku 5.5 (claimed in `d39e085da`, built in `1dcb6a764`). Verified and
integrated by Sonnet 5.5 (commit `87ad564e6` and this record); see "Verification" below, which
corrects three statements of the implementer's report (marked below). Resolved 2026-10-10.

### What changed

- **Schema** (`core/src/model.rs`): `MechanicalMirrorPair` gains optional `constructionSource`
  and `halves`, both absent when unset. New types: `MirroredHalves` (`sourceBoardId`,
  `mirrorBoardId`, `correspondences`, `exceptions`), `MirroredCorrespondence` (`kind`, `sourceId`,
  `mirrorId`), `MirroredException` (`kind`, `mirrorId`, `state`), `MirroredItemKind` (`matrix`,
  `layout`, `part`, `outline`, `net`, `assembly`, `module` (added in verification),
  `mechanical-feature`) and `MirroredExceptionState`
  (`local`, `omitted`). Field names are camelCase; enum values are kebab-case, as in
  `MechanicalFeatureKind`.
- **link-mirror-pair** (`core/src/mechanical/edit.rs`) records `constructionSource`; `halves` is
  left absent.
- **Closure rule** (`link_pair_closure_proposals`) uses `constructionSource`, and falls back to
  the first member when it is absent. The implementer also emptied the other half's stored closure
  list for a pair that records a source; **verification removed that** (it could delete authored
  hardware; see Verification, finding 1).
- **Migration** (`core/src/migrate.rs`): `CURRENT_VERSION` is 5. `version_4_to_5` changes nothing
  but the version. ADR-0004 has a follow-up paragraph.
- **Validation** (new `core/src/mechanical/halves.rs`, `validate_document`, called from
  `features::validate`): the construction source must be a pair member; each board may be a half
  of one record only; both boards must exist; correspondence ids must exist on their board
  (part, outline and net are listed by the board; assembly is document-scoped;
  mechanical-feature is checked against the pair's features; the implementer treated matrix and
  layout as document-scoped, which verification corrected to board-scoped); no repeated source
  or mirror per kind; exceptions name an existing mirror item that no correspondence links, and
  no repeats. Errors name the pair and the index (`halves.correspondences[1]`).
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

### Open points the implementer raised, and how verification settled them

1. Closure rule beyond the ticket: settled by removal (finding 1).
2. Load-time refusal: `Open` now validates the record (finding 3). Archive unpack and the browser
   decode still do not, by decision.
3. Refused records and mutation: tested through the engine (finding 3).
4. Reuse-board: recorded for ticket 18 (finding 5).
5. Generator-parameters 06 still states the "first member" rule in prose; it is a resolved
   historical ticket and was left alone. Keys are covered by `matrix` and `part`, wiring by `net`.

## Verification

Verified by Sonnet 5.5 on 2026-10-10 in the ticket worktree (`.worktrees/tce-matrix`, branch
`typed-core-edits/mechanical`). The implementer's report was not trusted; each claim was
reproduced or corrected.

### Findings

1. **Closure emptying could delete authored hardware (defect, fixed).** The implementer emptied the
   non-source half's stored closure list whenever the pair recorded a source. A half may carry
   closure mounts it authored or accepted before linking; `local_conflict` reads such a list as
   local hardware, and a reviewed correspondence is the way to merge hardware into the pair. The
   same duplicate would arise for a first-member source and the legacy rule never deleted it. The
   emptying is removed: the pair's proposals still come from the recorded source, the source stores
   an empty list (the mounts moved to the pair, as in 06), and the other half keeps whatever it
   stored. `closure_proposals_come_from_the_recorded_construction_source` now asserts the other half
   keeps all four mounts it proposed before linking (red before the fix: `Some([])`). Legacy pairs
   are unchanged: `closure_proposals_fall_back_to_the_first_member_without_a_recorded_source`,
   `included_keyboards::closure_proposals_on_a_paired_half_are_made_once_from_the_construction_source`
   and the demo baseline test pass untouched.
2. **`MirroredItemKind` had a gap (fixed).** ADR-0010 reflects modules, which had no kind. `module`
   is added (hosted on the board: `host_board_id`). Matrix and layout were wrongly document-scoped;
   a layout names its board and a matrix may; they are now checked against it (a matrix that names no
   board matches either half). Recorded for ticket 18, not added: embedded circuits, board outline
   versions (`BoardOutline` is per board), the electrical board configuration and keycap
   configuration have no kind. Adding a kind later is additive but must land before any v5 document
   with `halves` is saved, so ticket 18 should add them with its first writer.
3. **Records were only validated on mechanical edits (fixed).** `features::validate` ran in
   mechanical edits only. `ReplaceDocument` was already covered because the topology
   reconciliation ends with `features::validate`, which the implementer's report missed. `Open` was
   not, so a malformed stored v5 record would have loaded. `Open` now runs
   `halves::validate_document` (halves rules only, not the whole pair validation, so it cannot
   refuse a document for a legacy pair defect). It cannot reject a legitimate document: no document
   has `halves` yet, a pair without `halves` is checked only for a source that is a member, and the
   only writer (`link-mirror-pair`) records a member. Archive unpack and the browser decode
   deliberately do not validate: they must not hide a stored project; the user meets the located
   refusal when opening it, and the project stays in storage. Two rules were added that the ticket
   did not list but a record needs to mean anything: `halves` requires a `constructionSource`, and
   `sourceBoardId` / `mirrorBoardId` are the boards of the source and the other member (a member
   that is not an instance is left to the pair checks). Errors never mutate: edits run on a clone
   of the accepted document, and the engine test refuses a replacement and then accepts the same
   revision again.
4. **Schema and naming hold.** Both fields are optional with `default` and `skip_serializing_if`;
   field names are camelCase and enum values kebab-case like `MechanicalFeatureKind`. Unknown kinds
   and a `linked` exception state are refused (test).
5. **Reuse-board semantic, for ticket 18.** Source and mirror on the same board are accepted in
   one record (a reused board). Two records that both name one board are refused ("already a half
   of pair ..."), because a board is a half of one record only. Ticket 18 must decide whether a
   board used by several pairs (for example a reversible board in two splits) needs relaxing this
   rule. Ticket 19 must also re-validate after every accepted edit and keep correspondences in
   step with deletions: an unrelated edit that removes a corresponded part would leave a record
   that every later mechanical edit then refuses.
6. **Migration.** `version_4_to_5` changes only `formatVersion` (unit test), chains from versions
   1 to 4 (existing chain tests now end at 5), refuses version 6 (`rejects_newer_or_malformed_versions`),
   and `Open` refuses version 4 and 6 (new `open_refuses_documents_that_are_not_at_the_current_version`).
   Every shipped demo project, taken back to its version 4 form (`formatVersion` 4, no
   `constructionSource`), migrates with only `formatVersion` changed, and its pairs encode back
   identically with no new field (added to the demo acceptance test, 20 projects). The two version 4
   fixture files differ from their version 5 form in the `formatVersion` line only (checked
   against `74812d038`; neither contains a pair). Browser write-back and archive parity:
   `version_3_closure_project...`, `legacy_generator_project...` and the new
   `mirrored_halves_survive_browser_storage_at_format_five` pass in Chrome (`host::storage::`, 10
   tests); the archive test passes natively.
7. **Boundary transcript.** Compared against `74812d038` field by field: 53 `formatVersion` 4 to 5,
   4 archive buffer digests (each length +1 byte) and 3 embedded `projectJson` strings whose
   decoded content differs only in `formatVersion`. Nothing else changed. Native and WASM parity
   pass.

### Mutation checks

Disabled one site at a time, confirmed a test failed, restored and touched the source:

- `halves.rs` (16 mutations, all killed): construction-source membership, board existence, board
  claimed twice, correspondence source exists, mirror exists, repeated source, repeated mirror,
  exception exists, exception on a linked mirror, repeated exception, missing `constructionSource`,
  board mismatch, module host board, layout board, matrix board, part on board.
- `migrate.rs`: `version_4_to_5` rewriting data (killed); `CURRENT_VERSION` 6 (killed).
- `lib.rs`: `Open` validation removed (killed); `Open` accepting older versions (killed).
- `edit.rs`: reconciliation's `features::validate` removed (killed by the Replace case);
  `features::validate` skipping halves (killed); closure rule ignoring the recorded source
  (killed); `link-mirror-pair` not recording the source (killed); `constructionSource` not
  persisted (killed by `current_schema`).
- Survived, by design: `halves` not persisted (`#[serde(skip)]`) survives the archive unit test
  because the archive boundary carries the JSON text without decoding it; `current_schema` and the
  browser storage test cover persistence. A redundant `validate_document` call I first added to
  `ReplaceDocument` survived because reconciliation already validates; it was removed.
- Not mutation-checked: the wasm `mirrored_halves_survive_browser_storage_at_format_five` assertions.

### Commands (sequential, foreground, one at a time, in this worktree)

- `python3 scripts/check.py lint test` on the final code: lint 8.3 s, test 87.8 s, passed.
- Earlier on the same code (the only later change was the new `current_schema` test):
  `lint` 13.3 s, `test` 109.9 s, `typecheck` 13.7 s, `browser` 223.3 s (sandbox lifted;
  `run-wasm-tests: executed 70, failed 0`), all passed.
- `wasm-pack test --headless --chrome web/crates/host --locked --features page --lib --
  boundary_parity::` passed (`wasm_exports_reproduce_the_native_transcript`); the `browser` step
  does not run it (it filters to `host::storage::`).
- `python3 scripts/check.py repo` on the final docs: passed (0.1 s, 158 Markdown files).
- `build` was not run: no demo, generated file or fixture used by `build` changed. `release` never
  run. The implementer's first `test` failure came from stale footprint binaries built through a
  shared `target` directory from another checkout; it did not recur here.

### GitNexus

Index stale (4b372b2; `analyze` not run). `impact` on `link_pair_closure_proposals`,
`validate_document` and `CoreEngine` returned UNKNOWN or not found, so the callers were confirmed
by text search: `link_pair_closure_proposals` has one caller (`initialize_target_closures`),
`validate_document` one (`features::validate`) plus the new `Open` call, and `Open` is an additive
refusal that only a document with `halves` can reach. `detect-changes` (all) reported 10 symbols
and risk high, mapped through stale line numbers (`handle` is the `Open` arm; the rest are tests and
unrelated neighbours of edited lines), so it is provisional, not a clean check.

### Unverified

Typecheck and browser were not re-run after the last test-only commit. The behaviour of a half that
keeps its own closure list beside the pair's linked mounts (visible duplicates and overlap findings)
is not exercised on a real board.
