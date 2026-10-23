# 25: Deliver safe incompatible-project recovery and project switching

Status: claimed
Type: build
Blocked by: [17a](17a-grouped-model-and-core-callers.md), [17d](17d-entry-path-gates.md) (parent: [17](17-cut-over-format-and-model.md))
Parent: [10](10-slice-the-build.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)

## Outcome

Finish ticket 13's browser/runtime/session behavior on top of the strict version gate.
This is not a reset command: no user store is cleared and no legacy converter is added.
Use isolated browser stores for tests throughout the unshipped cutover lane.

## First RED / GREEN slice

Fail a real saved-project list/load scenario containing healthy v7 and unsupported v6
records: both remain, attempted open writes nothing and current accepted work/history
survives. Pass through actual BrowserStore/Runtime paths, then add incompatible import.

## Subsequent acceptance

- Keep older/unversioned/newer/invalid classifications truthful, with found/supported
  version details and no conversion/tag-repair claim. Package validation stays intact.
- Reuse JSON-only raw saved-data download, fresh New project and named, cancelable,
  guarded Delete. Do not imply raw JSON contains assets. Preserve unreadable records,
  unrelated/live assets, stale-tab protection and retryable failures.
- Reject incompatible/invalid imports before replacement work without partial project/
  asset writes, losing drafts, clearing history or mutating the original archive.
- Validate targets before abandoning current work; confirm draft discard; settle
  owned saves/queued edits rather than canceling them to switch. Cancellation does
  not discard work. Known save failure retains the exact result for storage-only retry;
  unknown outcomes retain explicit recovery, with no Core replay or guard bypass.
- Publish replacement only after storage commits. Successful New/Open/reopen resets
  Editor session/history; mere list/refusal/cancel does not. Preserve offline assets
  and delayed worker activation; no forced update or global wipe.

## Checks

Sources: Core gate/archive, Host `web/crates/host/src/host/storage.rs`/`storage_lifetime_tests.rs` (same directory), Runtime and
Session, library action/recovery controls and current mounted delete/list tests.
Exercise invalid direct Open, storage races/failures and draft/pending-save ordering
through public interfaces. Run affected native checks and browser `host runtime library`
subsets plus page coverage as needed; partial subsets are not full acceptance.

## Progress

### Parallel implementation claim — 2026-10-23

Claimed for an isolated implementation lane. Acceptance, TDD, review and gates are
unchanged; integration owns shared baselines, test aggregation, map and closure.
