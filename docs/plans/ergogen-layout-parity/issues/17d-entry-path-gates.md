# 17d: Entry-path version gates

Status: claimed
Type: build
Blocked by: [17a](17a-grouped-model-and-core-callers.md)
Parent: [17](17-cut-over-format-and-model.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)

## Outcome

Gate every entry path on the controlled version-7 check before migration or replacement effects.

## Acceptance

- Gate browser list/load, archive unpack and direct Core Open before migration or replacement effects.
- Keep `boardstudio/v2`, DB version 1 and archive metadata.
- Missing/older/newer/malformed versions fail through the specified controlled paths.

## Checks

Source anchors: `core/src/migrate.rs`, `web/crates/host/src/host/storage.rs`. Use focused direct-Open refusal/history, archive and BrowserStore mixed-version tests, and `check.py quick --files`.

## Progress

- [x] Gap audit (base d45f30271). Already gated before migration/replacement effects:
  direct Core Open `core/src/lib.rs:269` (+ raw-JSON preflight `lib.rs:~63`, also for
  replace-document `lib.rs:2259`); browser list/load `web/crates/host/src/host/storage.rs:690`
  (`decode_document` -> `migrate_value`; `storage/startup.rs:186`). NOT gated first:
  archive unpack read the asset table (`asset_references`, `core/src/archive.rs`) before
  the version check, so an old/malformed-version archive got an unrelated
  "Unsupported project format"/asset error. Fixed (`da63ee9a8`): `migrate_json` runs first.
- [x] Acceptance, proving tests:
  - Archive gate before effects: `archive::tests::unpacking_refuses_every_version_before_reading_the_asset_table`
    (RED confirmed: got "Unsupported project format") and
    `unpacking_an_unrecognized_format_is_unsupported_without_conversion`.
  - Direct Core Open/replace, history untouched: `incompatible_format_refusals_explain_recovery_and_preserve_history`
    (existing) and new `malformed_missing_and_foreign_format_inputs_refuse_before_any_state_change`
    (added coverage; passed immediately, path already gated).
  - Browser list/load, raw record preserved, healthy projects listed:
    `mixed_format_records_are_listed_without_conversion_and_raw_json_is_preserved` (existing) and new
    `foreign_format_identifier_is_unsupported_and_preserved_on_list_and_load`.
  - `boardstudio/v2`, DB version 1, archive metadata unchanged (no edits to them).
- New test names (baseline for integrator, not edited): native
  `archive::tests::unpacking_refuses_every_version_before_reading_the_asset_table`,
  `archive::tests::unpacking_an_unrecognized_format_is_unsupported_without_conversion`,
  `document_integrity::malformed_missing_and_foreign_format_inputs_refuse_before_any_state_change`;
  wasm `host::storage::lifetime_tests::foreign_format_identifier_is_unsupported_and_preserved_on_list_and_load`.
  No tests renamed or removed.
