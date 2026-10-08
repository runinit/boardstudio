# 09: Case, Keymap, Keycaps and Library settle through `PendingEdits`

Status: claimed
Type: build
Blocked by: 05
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md)

## What to build

Move the remaining settlement sites onto `PendingEdits` (files at `915d305c0`):

- `web/crates/case/src/`: `case_controller.rs`, `mechanical_settings_controller.rs`
  (only its settlement and `begin_edit` port; the patch logic moves in
  [typed Core edits 02](../../typed-core-edits/issues/02-mechanical-settings-patch.md));
- `web/crates/keymap/src/keymap/`: `binding_controller`, `layer_controller`,
  `macro_controller`;
- `web/crates/keycaps/src/keycaps_settings.rs`;
- `web/crates/library/src/library.rs`.

## Migration rules (same for 07, 08 and 09)

- Every settlement site uses `PendingEdits` and the `ui-shared` helpers from
  [ticket 05](05-pending-edits-module.md); the panel keeps only its projection, its
  resolvers and its own owner predicate.
- Retirement is silent and there is no "Saved" status
  ([ADR-0005 amendment](../../../adr/0005-resolve-queued-edits-at-execution.md#amendment-a-retired-edit-is-silent-2026-10-07)).
  Update mounted tests that assert a removed message or status.
- The panel chooses where a failure message appears; default inline at the field.
- Tests that call `EditResolver::resolve` by hand move to the native Runtime
  ([ticket 03](03-native-test-runtime.md)); native tests that only restated settlement
  are deleted.

## Acceptance criteria

- [ ] No per-panel settle loop or feedback enum duplicating the module remains
  (`CaseBodyEditState` and the like). The shared modules own ticket settlement,
  latest-per-key observation and bound-field restoration/failure writes. Callers
  may project drained results for domain metadata, error placement and precise
  Landed follow-ups; those projections must not restore bound drafts or implement
  a second settlement policy.
- [ ] `MechanicalSettingsController`'s commit order across catalogue loads
  (`flush_prepared`) and its native tests are unchanged.
- [ ] Keymap and Library gain native tests through the native Runtime where they had
  none.
- [ ] Mounted tests pass.

## Verification

```sh
cargo test -p boardstudio-web-case -p boardstudio-web-keymap -p boardstudio-web-keycaps -p boardstudio-web-library --locked
python3 scripts/check.py lint typecheck test browser
```

## Orchestrator checkpoint: final reported branch

Reviewed base `8ce860a0950304ddfae34f12d5891b8c6f267f2f` through
`f8f684959b592d819796056444f6cc2a4e94e88c`. The branch is clean and the migration
uses PendingEditSignals throughout. This is a completion checkpoint, not an
Outcome: the ticket stays claimed until the remaining fixes, checks and integration.

The fresh Standards review read the complete changed macro, layer and binding
bodies and Keycaps file. It found no hard documented-standard or visibility
violation. The duplicated Case/Keymap OwnedEdits holder is a nonblocking cleanup
finding. The Spec review confirmed two macro presentation defects: field failures
also appear in the panel status, and TextDraft/NumberDraft hide an older inline
failure while a newer draft is dirty. Repair both through the real mounted fields.

Complete the focused coverage in the
[final completion pass](../handoff-09-case-keymap-keycaps-library.md#final-completion-pass):
same-field mechanical requests behind held Core, a one-shot lifecycle regression
that actually fails without the fix, and representative panel tests for submitted
draft restoration and latest-per-key observation. Preserve existing domain and
catalogue-order assertions. Generic helper coverage alone does not prove panel
bindings and accepted-projection effects are wired correctly.

Reported browser results cover all suites, with Keymap passing only in separate
module/test invocations because the whole-suite driver is killed on the clean base
as well. Earlier passing suites remain evidence for unchanged code; record their
tested commits and rerun the paths affected by the final changes. The final Keycaps
commit includes a production Signal fallback, so do not describe both last commits
as fixture-only. Root still requires complete coverage at the integrated tree.

The root worktree compare found 29 changed files, 201 indexed symbols and 28 flows
with critical aggregate risk; no partial/truncated flags were reported. New branch
symbols still require current-source confirmation because the canonical graph is
indexed from dev. Root owns the final integrated-tree analysis.
