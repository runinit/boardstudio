# 38: Merge the duplicate browser tests the audit named

Status: claimed
Type: build
Blocked by: [37](37-sleeps-layout-library-ui-page.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Audit: [test-audit.md](../audit/test-audit.md)

## Request

Merge only the exact duplicate browser tests identified in audit items M2 and M3. Keep separate
tests where different owners, mounting paths, failure boundaries, or assertions protect distinct
behavior; record those dispositions in the audit.

## What to do

- Audit M2: merge the keymap keycode/delay twin tests into table-driven tests. Audit M3: merge the probable duplicate position-draft tests in the presentation inspector tests. Re-read both bodies first and merge ONLY exact duplicates; list what each merge preserves; skip any that are not exact. Update the audit with a phase 2 results section and close ticket [24](24-test-audit.md) together with [35](35-test-maintenance-cuts.md).

## Acceptance

- [ ] The exact keymap twins are table-driven; non-exact candidates are retained with reasons.
- [ ] Focused browser suites and full repository gate pass; test names are compared before and after, with only documented merges removed.

## Verification

Run the affected WASM suites, `cargo fmt --all --check`, and `python3 scripts/check.py`.

## Progress (2026-10-14, exact-duplicate review)

The four Keycode/Delay pairs in `macro_controller::mounted_step_tests` share the same helper
and assertions, differing only in field label and raw/canonical values. They are now four
table-driven tests. The Macro-name/tap-time newer-draft pair in `queued_edit_tests` shares one
helper and is now one table-driven test. Layer-name newer-draft/failure tests and the layer-name
versus macro-number failed-unchanged tests use distinct owner/controller paths, so they are retained.

The two position-draft candidates in `layout_component_inspector_tests` and
`inspector::tests` use different controls and commit paths: one submits typed edits through the
Layout component Inspector; the other uses the legacy explicit Apply form and checks pending
disablement and the clean axis. Their failure cases also inject different failure boundaries.
They are not exact duplicates and remain separate. Focused Keymap verification passed 35/35 after
the merges; full-tree verification and final name reconciliation remain open.
