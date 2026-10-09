# 03: Repair saved numeric strings with a v3 to v4 migration

Status: implemented (awaiting verification)
Type: build
Blocked by: 01
Map: [map.md](../map.md) · Decision: [ADR-0004](../../../adr/0004-document-format-version-and-id-rename.md)

## Request

`core/src/migrate.rs` coerces numeric strings only in the 1→2 step
(`version_1_to_2` → `normalize_generator_values`, lines 68-72, 174-281). The string
writer dates from `dfd217fa1` (2026-10-02), so v2/v3 documents saved since the
2026-10-05 cutover with closure mounts hold strings in
`definitions[assembly-closure/definition/*]`, and those fail at render.

## Decision (overturns "coerce on every load")

Use an ordered `version_3_to_4`, `CURRENT_VERSION = 4`:

- ADR-0004 explicitly rejects "an unversioned rewrite on each load".
- `migrate_value` sets `changed = version < CURRENT_VERSION`, and `migrate_json` returns
  current-version text unchanged without inspecting it. A per-load repair would need new
  change detection to trigger write-back. A version step reuses the storage write-back
  (`web/crates/host/src/host/storage.rs:193-200`) and archive import
  (`core/src/archive.rs:90`).
- Open refuses documents that are not at the current version (`core/src/lib.rs:230`), so
  tickets 01-04 can rely on clean data at the boundary.
- Literal-3 fixtures to update: `core/tests/outlines.rs:6`, `core/src/archive.rs:375`,
  `core/src/migrate.rs:542`, `core/tests/fixtures/reviung41-outline-original.json`,
  `web/crates/layout/tests/fixtures/sofle-v2.json`, plus 53 transcript lines; bundled
  fixtures regenerate.

## What to do

- Split `normalize_generator_values` so number/array coercion (`convert_parameters` over
  definitions, `parts[].generatorParameters`, `assemblies[].members[].parameters`) runs in
  `version_3_to_4`; the back-side `side` defaulting stays 1→2 only.
- Do not drop unknown names or values that cannot be converted; 04 reports them.
- Tests: a v3 document with string `hole_drill` migrates to numbers and renders; v4 text is
  returned unchanged (`false`); v1 documents still go 1→4; unknown fields are preserved.
- Update the literal fixtures and re-bless the transcript.

## Acceptance

- [ ] A saved v3 closure project loads, is written back as v4 and exports. (Partly: the native test
  loads, opens and exports a v3 closure project; the browser write-back test covers a v1 document,
  not a v3 closure project through IndexedDB.)
- [x] `legacy_archive_migration` and `outline_persistence` pass with the new version.
- [x] ADR-0004 follow-up line and the investigation's deviations table updated.

## Verification

Core tests, boundary parity native and WASM, `browser` (IndexedDB migration checks),
`quick`.

## Risks / ADR

Another lane bumping the version at the same time (none found on current branches);
coordinate the number. No new ADR, since this follows ADR-0004. If coerce-on-load is
preferred instead, it needs an ADR amendment and explicit `changed` plumbing in
`migrate_json`.

## Outcome

Status: implemented (awaiting verification). Code, tests, fixtures and the transcript are in
`fd7640a9a` (`Migrate saved generator strings to numbers at format version 4`). The ADR, the
investigation's deviations table, the map progress line and this Outcome are in the docs commit
that follows it.

### What changed

- `core/src/migrate.rs`: `CURRENT_VERSION` 3 to 4. `version_3_to_4` runs `convert_generator_values`,
  which coerces numeric and list text over definitions, `parts[].generatorParameters` and
  `assemblies[].members[].parameters`. `version_1_to_2` now runs only `state_back_sides` (the
  `side` default for back-side parts) before the renames.
- Names no generator declares and values that cannot be converted are kept (for example `"two"`
  for `hole_size`). Empty text for a number parameter is still removed, which is what the old
  1 to 2 step did; I kept that behaviour and did not decide it here.
- Literal version 3 moved to 4: `core/src/archive.rs` (the fixture document and the version 1
  unpack assertion), `core/tests/outlines.rs`, `core/tests/demo_projects.rs`,
  `core/tests/fixtures/reviung41-outline-original.json`,
  `web/crates/layout/tests/fixtures/sofle-v2.json` and three assertions in
  `web/crates/host/src/host/storage_lifetime_tests.rs`. The "newer than this app" test now uses 5.
- `core/tests/fixtures/boundary/transcript.json` re-blessed from HEAD's file.
- Bundled demo projects: `build` regenerates them untracked (`web/assets/fixtures`), 20 projects at
  format 4 plus `provenance.json`, which has no format field.

### Test evidence (red first)

The tests were written and run before the implementation. Red: the new migrate tests (version
assertions, `changed`, and the 1 to 2 step still coercing) failed; the closure integration test
failed; `outlines`, `demo_projects` and `outline_persistence` failed because the literal 4 was
"newer" than the old CURRENT_VERSION 3. Green after the change:

- New: `migrate::tests::version_3_saved_strings_become_numbers_and_render`,
  `version_3_to_4_keeps_unknown_names_and_unconvertible_values`,
  `the_version_1_step_states_the_back_side_and_leaves_text_for_version_4`, and
  `closure_generator_parameters::a_version_3_closure_project_with_saved_strings_migrates_opens_and_exports`.
- Updated: `migrates_a_version_1_document` (1 to 4), `current_documents_keep_their_text` (4 is
  unchanged), `rejects_newer_or_malformed_versions` (5 is newer).

### Transcript diff scope (not only version lines)

HEAD had 56 `formatVersion` lines: 53 pretty-printed `"formatVersion": 3` and 3 inside JSON strings.
The re-bless changed exactly those 56 lines to 4. It also changed four `len:hash` buffer digests
(`1100:be9ad4bcb8ba904e` to `1097:b3b004e87cf16a8c`, `974:a55ace961fc931f2` to
`971:c25698cd1975de19`, `960:dc112e3529b393d1` to `957:9fa9c377319c63ea`, and
`4196497:9b5b415e5f203b9e` to `4196497:aa472165942b7d65`). These are packed archives, which embed
`project.json` and so change with the version digit. Requests and replies show no other change.
I did not compare the archive bytes, so this rests on the transcript diff, not a byte comparison.

### Commands and results

- `cargo fmt --all --check`: clean.
- Native, in `/tmp/claude-1000/boardstudio-tce-verify` at `fd7640a9a`:
  `python3 scripts/check.py repo lint test`: all steps passed, exit 0 (`test` 128 s).
- Targeted: `cargo test -p boardstudio_core --lib migrate`: 10 passed. Integration binary filters
  (`closure_generator_parameters`, `outlines::`, `demo_projects`, `outline_persistence`,
  `legacy_archive`, `archive`): 41 passed. `boundary_parity` (native): 2 passed.
- Nested worktree: `check.py build typecheck`. The first attempt failed before reaching this code:
  `wasm-pack` could not create a temp directory for `cargo install wasm-bindgen` (read-only
  filesystem under the sandbox). Rerun with the sandbox disabled for `wasm-pack`: build and
  typecheck passed, exit 0.
- Nested worktree, sandbox disabled: `check.py browser`: all steps passed, exit 0.
  `run-wasm-tests`: executed 70, failed 0. This included
  `host::storage::lifetime_tests::legacy_generator_project_migrates_and_is_written_back_with_assets_intact`
  and `outline::browser_tests::product_context::mounted_copy_outline_on_the_sofle_demo_project_creates_one_version`.
- WASM boundary parity, sandbox disabled:
  `wasm-pack test --headless --chrome web/crates/host --locked --features page --lib -- boundary_parity::`:
  `wasm_exports_reproduce_the_native_transcript` ok.
- GitNexus: `impact` before editing: `normalize_generator_values` HIGH (split into two steps),
  `migrate_value` CRITICAL (used by archive dispatch, storage decode and Open; the change is intended),
  `version_1_to_2` and `version_2_to_3` HIGH (the first changed, the second not), `migrate_json`,
  `normalize_instance`, `convert_parameters` and `generator_definitions` LOW. `detect-changes --scope all`
  before the commit: 9 files, 17 symbols, 0 affected processes, risk low. The index is stale (GitNexus
  reports it was last indexed at `4b372b2`), so these results come from an older graph.

### Not verified

- `check.py quick`, `security` and `deny` were not run (no dependency change). `release` is not
  allowed. Acceptance item 1 is only partly covered (see above).
- No saved v3 closure project was loaded through IndexedDB in a browser.
- The archive byte comparison was not done (see the transcript scope above).
- The `formatVersion: 1` manifests under `catalogue/`, `cad/` and `scripts/` are different
  manifests and were not touched.

### For ticket 04

- Unknown names and unconvertible values now survive migration, so 04 can report them: warnings
  on Open and errors on edit.
- Open already refuses documents that are not at format 4, so 04 can rely on clean types at the
  boundary.
