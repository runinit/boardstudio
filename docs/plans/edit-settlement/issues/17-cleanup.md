# 17: Cleanup: one settlement path and a record of what still replaces the document

Status: ready-for-agent
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

- [ ] No edit-settlement `Pending*` struct remains; the unrelated ones above are untouched.
- [ ] No product commit sends `Event::Edit` directly; remaining `Event::Edit` uses are previews or listed with a reason.
- [ ] Runtime has no feature-named test interception.
- [ ] `replace-document-resolvers.md` exists and is linked from the map.
- [ ] Architecture and backlog updated; doc links pass.

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
