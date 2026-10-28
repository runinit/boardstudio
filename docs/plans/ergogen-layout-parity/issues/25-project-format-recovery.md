# 25: Deliver safe incompatible-project recovery and project switching

Status: resolved
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

## Answer

Integrated the reviewed source pin `5424d2010512899a694ddaca0708f84e2bed50d6` with merge
commit `879ac9d603cf56d0da7540079e1c0e5b63419e5d`. The native retention update is
`d2bb6b49a3051c31508ba644ef1b6ce42b3e1e71`. Fresh merged-code verification ran at
`d2bb6b49a3051c31508ba644ef1b6ce42b3e1e71`, before this documentation-only closure.

- The combined independent Spec and Standards review passed with zero blockers or
  should-fix findings. It reviewed the full source change and connected callers.
- The native baseline records exactly four additions and no removals or renames:
  `durable_session::owned_gesture_commit_finishes_before_project_open`,
  `durable_session::invalid_open_preserves_the_current_gesture_and_accepted_session`,
  `durable_session::invalid_recovery_keeps_the_exact_failed_result_for_storage_only_retry`,
  and
  `durable_session::failed_replacement_retries_the_exact_candidate_without_core_or_early_publication`.
  The supported updater passed all 2,154 native tests; the baseline moves from 2,150 to
  2,154.
- Fresh quick passed 48/48 commands: 1,998 native tests passed, 9 were ignored, no
  retained names were missing; all 14 selected WASM launches passed (598 tests),
  including Host, Runtime and Library. Receipt: `.evidence/ergogen-25-integration/quick.json`.
- Fresh full passed 55/55 commands: 2,154 native tests passed, 12 were ignored, and
  all 2,154 baseline names matched. All 14 WASM launches passed (598 tests), including
  Host 47/47, Runtime 50/50 and Library 17/17. The page suite passed 77/77; packaged
  trusted-drag, CAD-provider and CAD-browser smoke checks all passed. Receipt:
  `.evidence/ergogen-25-integration/full.json`.
- The change validates candidates before discarding current work, lets owned gesture
  commits settle before Open, and publishes replacements only after persistence. A
  conservative confirmation can also appear when an editor is already open or a result
  is retained; this preserves drafts without inventing a shared dirty-state registry.

This closes ticket 25 in the unshipped format-cutover lane. It does not authorize release
or user-data writers; ticket 27 remains the release gate.
