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

- [ ] 
