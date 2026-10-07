# 20: The direct edit event carries previews only

Status: ready-for-agent
Type: build
Blocked by: 18
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0005 amendment](../../../adr/0005-resolve-queued-edits-at-execution.md#amendment-commits-are-intents-the-direct-edit-event-carries-previews-only-2026-10-07)

## What to build

Make a direct commit unrepresentable. Replace `Event::Edit { operation_id, command }`
in `application/src/session.rs` with a preview-only event (for example
`Event::PreviewEdit { operation_id, transaction_id, target_ids, operation }`) that has
no phase field. Session builds the `EditCommand` with `EditPhase::Preview` and keeps
today's preview behaviour: base revision refreshed from the accepted document,
display preview, `ClearPreview`. Every commit goes through `Event::ResolveEdit`.

The internal `IntentKind::Edit` stays: resolved intents and the strict gesture route
still enqueue commands. Session's captured `GestureCommit`, export commits and
`ReviewElectricalRemap` keep their strict captured-revision checks; do not touch them.

## Call sites

Production (all previews today; line numbers at `a3b41ae8a`, orientation only):

- old position Inspector, `submit_position` (`web/src/presentation/inspector.rs:199`);
- outline perimeter live preview (`web/crates/layout/src/outline_lifecycle.rs:~1308`);
- Layout transform toolbar, `submit_transform_edit`
  (`web/crates/layout/src/objects/layout_transform_toolbar.rs:1364`).

Tests: 26 direct commits in 14 files, including the shared
`project_name_test_support::replace_document` helper (`web/crates/runtime/src/runtime.rs`,
12 callers). Add a test helper that submits a fixed command through `ResolveEdit` (a
resolver that returns it) and move every direct test commit to it or to an existing
resolver. Tests that send direct previews move to the new event.
`docs/investigations/layout-part-editing-repro.rs` reproduces the pre-ADR-0005 bug and
is documentation, not compiled code; leave it, adding a one-line note that it targets
the removed event.

## Rules

- Shared file: the `application/` change goes in its own small commit first.
- No `#[cfg(test)]` or `test-support` direct-commit variant.
- `queued_discrete_edits_use_each_preceding_durable_revision`
  (`application/tests/durable_session.rs`) tests direct commits refreshing their
  revision. Rewrite it for two queued resolvers (each runs against the document the
  previous one produced), or delete it if an existing Application test already covers
  that.

## Acceptance criteria

- [ ] No `Event` variant can carry a commit-phase command; `Event::Edit` is gone.
- [ ] The three production preview paths still show and clear their previews (existing
  mounted tests pass; add one Session test that a preview edit displays and does not
  change the accepted document or history).
- [ ] Every former direct test commit goes through `ResolveEdit`; no escape hatch exists.
- [ ] Gesture commit, export and remap strictness are unchanged (their tests pass
  untouched).
- [ ] `CONTEXT.md` and `docs/architecture.md` describe the preview event and that
  commits are intents.

## Verification

```sh
cargo test -p boardstudio-application --locked
cargo test -p boardstudio-web-runtime --locked
python3 scripts/check.py typecheck test
python3 scripts/check-wasm-tests.py
python3 scripts/check.py browser
python3 scripts/check-doc-links.py
```

## Out of scope

- Constraining what resolvers return (typed Core edits, ADR-0006).
- Stale previews drawn after a queued commit (`docs/backlog.md`).
