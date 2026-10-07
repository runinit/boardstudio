# 17: Cleanup: one settlement path and a record of what still replaces the document

Status: resolved
Type: build
Blocked by: 08, 09, 10, 11, 12, 13, 14, 15, 16
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md)

## What to build

The contract step of the migration. After the cluster tickets, every product edit
should be an intent. Confirm that, delete whatever settlement code and test
interception is left, and write down which resolvers still produce
`ReplaceDocument`, so decision tickets 18 and 19 rest on evidence.

## Background you need

- `grep -rn "struct Pending" web` lists both edit settlement structs and unrelated
  ones. These are **not** edit settlement and stay: the Core and CAD worker request
  maps (`host/core_client.rs`, `host/cad_worker.rs`), `PendingLayoutFit`
  (keycaps navigation), `PendingPreview` (layout viewer), `PendingNewKeyboard`
  (ui-model state) and `PendingProtectedRemap` (strict electrical remap).
- `grep -rn "Event::Edit" web` should find only previews and anything a cluster
  ticket deliberately left (each should be named in that ticket's Outcome).
- Runtime test interception: tickets 01 and 07 removed the project-name and
  definition-name modes; tickets 11/12 remove the layout-inspector mode (deferred
  from ticket 05). If it is still present, delete it here. The
  `firmware_export_test_*` fields (context, effects, events, deliveries) are a further
  mode, flagged by ticket 07: move its tests onto ticket 01's adapter and delete it,
  unless that needs export-path changes (strict paths are out of scope), in which case
  record why it stays. Check for any other `#[cfg(test)]` fields or `submit` branches.
- Shared "landed" helpers to delete if unused: Std-landed variants,
  `pending_settlement_gate`, `matrix_transform_lifecycle` settlement helpers,
  standard failure strings outside the edit ticket
  (`grep -rn "did not save\|Retry after recovery" web`).

## Approach

1. Inventory what's left with the greps above and each cluster ticket's Outcome.
2. Delete unused settlement structs, helpers, strings and interception.
3. Write `docs/plans/edit-settlement/replace-document-resolvers.md`: one row per
   resolver that still produces `ReplaceDocument`, with the concept it changes
   (board name, wiring mode, outline feature…), the panel, how often a user hits it,
   and whether a typed Core operation already exists. Group rows by concept.
4. Update `docs/architecture.md` (edit submission now goes through intents and the
   edit ticket; one sentence plus a pointer) and `docs/backlog.md` (close
   "Layout queued coordinate edits" if ticket 06 didn't; keep the stale-preview
   entry).
5. Add the final Progress line to the map.

## Acceptance criteria

- [x] No edit-settlement `Pending*` struct remains; the unrelated ones above are untouched.
- [x] No product commit sends `Event::Edit` directly; remaining `Event::Edit` uses are previews or listed with a reason.
- [x] Runtime has no feature-named test interception.
- [x] `replace-document-resolvers.md` exists and is linked from the map.
- [x] Architecture and backlog updated; doc links pass.

## Verification

```sh
cargo test -p boardstudio-application --locked
cargo test -p boardstudio-web-runtime --locked
python3 scripts/check.py repo typecheck test
python3 scripts/check-wasm-tests.py
python3 scripts/check.py browser
python3 scripts/check-doc-links.py
```

## Out of scope

- Restricting `Event::Edit` (decision ticket 18) and typed Core edits (decision ticket 19).


## Comments

2026-10-07: Prepared a [source cleanup audit](../../../investigations/edit-settlement-cleanup-audit.md) and [provisional resolver inventory](../replace-document-resolvers.md). Refresh them after ticket 15 lands; no cleanup implementation or final acceptance claim is made here.

2026-10-07: Claimed after ticket 15 merged. Migrate remaining Runtime feature
interception to the shared adapter, retain strict export/remap/gesture behavior,
refresh the resolver inventory, and leave decisions 18/19 to the user.

## Outcome

Implemented in `95eb2e04`, `a2645667` and `05692c8f` after ticket 15 merged.
All product commits now use intents and `EditTicket`. The three remaining direct
product `Event::Edit` paths are old position Inspector, transform toolbar and
outline perimeter previews; direct commit fixtures remain in tests. Session's
strict captured gesture, export and protected-remap semantics are unchanged.

Deleted the firmware context/effects/events/deliveries mode and its fabricated
Runtime read model, scope and executor identity. Export tests now use a real
Session/Core electrical fixture, selective provider gates/failures, scripted
firmware generation and real Core ZIP packaging. Actual Open and executor restart
exercise obsolete-completion guards. Generic delivery capture lives in the adapter.
Archive import's result override and store/open synchronization also moved into
that adapter; async observers are captured per task so overlapping opens remain
independent. Neutral fixtures no longer depend on firmware support.

Retained ticket/owner records have `*Submission`/`KeyEditOwner` names; the separate
worker, preview, navigation and strict-remap pending states remain. No dead shared
settlement helper was found to delete. The source audit explains why live
selection-retention helpers, preview provider ports and `PendingPart` remain.

The [resolver inventory](../replace-document-resolvers.md) now has one row per
replacement-producing resolver, grouped by concept with panel, inferred frequency
and existing typed-operation coverage. Architecture names the intent/ticket path;
backlog closes queued coordinates and preserves stale previews. The
[decision brief](../decision-brief.md) and
[direct-event options](../../../investigations/edit-event-restriction-options.md)
are current evidence, not answers to human tickets 18 and 19.

Validation:

- Astra high Standards: **0 actionable findings**. Astra high Spec: **0 actionable
  findings**. Both reviewed the fixed ticket range independently.
- `check.py repo typecheck test`: repository and WASM typecheck passed; all native
  Rust workspace and footprint suites passed, including Application **29** and
  Runtime **102**. The CAD STEP-oracle build then encountered the known nested
  worktree workspace-discovery failure. Ticket 15 separately established the
  unrelated CAD gasket-volume baseline failure in a clean checkout; neither CAD
  issue was changed here.
- Runtime browser: **33 passed**, including real export failure/retry, executor
  replacement at each provider await, owner replacement, concurrent exports and
  import/open supersession. Mounted firmware page subset: **5 passed**.
- `check.py browser`: **passed**. Workspace browser suites passed **366** tests,
  the page runner passed **42**, and CAD browser smoke passed **15**. The CAD package was copied from the
  existing local build after verifying identical tracked CAD source and no dirty
  CAD files; no source build was substituted silently.
- WASM test ownership, documentation links and whitespace checks passed.

No restriction of `Event::Edit`, new typed Core edit, or product choice for
18/19 was made. The user can now choose enforcement scope and the first typed Core
concept using the linked evidence.
