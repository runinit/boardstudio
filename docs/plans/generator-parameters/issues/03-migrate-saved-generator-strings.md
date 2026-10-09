# 03: Repair saved numeric strings with a v3 to v4 migration

Status: ready-for-agent
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

- [ ] A saved v3 closure project loads, is written back as v4 and exports.
- [ ] `legacy_archive_migration` and `outline_persistence` pass with the new version.
- [ ] ADR-0004 follow-up line and the investigation's deviations table updated.

## Verification

Core tests, boundary parity native and WASM, `browser` (IndexedDB migration checks),
`quick`.

## Risks / ADR

Another lane bumping the version at the same time (none found on current branches);
coordinate the number. No new ADR, since this follows ADR-0004. If coerce-on-load is
preferred instead, it needs an ADR amendment and explicit `changed` plumbing in
`migrate_json`.
