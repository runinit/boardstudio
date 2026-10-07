# 20: The direct edit event carries previews only

Status: resolved
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

- [x] No `Event` variant can carry a commit-phase command; `Event::Edit` is gone.
- [x] The three production preview paths still show and clear their previews (existing
  mounted tests pass; add one Session test that a preview edit displays and does not
  change the accepted document or history).
- [x] Every former direct test commit goes through `ResolveEdit`; no escape hatch exists.
- [x] Gesture commit, export and remap strictness are unchanged (their tests pass
  untouched).
- [x] `CONTEXT.md` and `docs/architecture.md` describe the preview event and that
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

## Outcome

Merged to `dev` in `a479152fd` (implementation commits `320e82a34`,
`a327f4d4e`, `4f6702ef7`, `d908780c4`, `d8467b3cd`, `a9e42f92`).
`PreviewEdit` carries no phase; Session constructs previews, and former direct test
commits use `ResolveEdit`. The three production preview paths and strict captured
routes retain their behaviour. Standards and Spec reviews found no issues.

Checks: lint, WASM typecheck, documentation links and WASM test inventory passed;
Application 29/29 and runtime native 102/102 passed. All Rust workspace tests passed.
Affected browser suites passed: runtime 33, shared UI 8+1, Case 60, Parts 50, PCB 38,
Layout 92 plus setup guide 1, and mounted page runner 42. CAD browser 15, host 6,
UI model 4, Keycaps 27 and Library 13 also passed. The browser gate caught an omitted
Layout WASM test-helper migration; `a9e42f92` fixes it and both reviews were refreshed.

Full gates are not green: the full Keymap browser suite times out after 17/21 at
`failed_binding_restores_accepted_field_and_explains_failure`; the same timeout
reproduced on the exact starting commit `19598aea0`, whose isolated test passed 1/1.
The nested worktree cannot discover the separate CAD oracle workspace. Running the
unchanged CAD gate from canonical `dev` instead yielded 49 passed, 1 failed, 4 ignored:
`core_internal_gasket_fixtures_export_connected_positive_regions`, rotated-concave
bottom volume expected 80481.2399 versus 80579.55733514718 (tolerance 0.1). Core, CAD
and contracts sources were identical to `dev`; these baseline gates remain follow-ups.
