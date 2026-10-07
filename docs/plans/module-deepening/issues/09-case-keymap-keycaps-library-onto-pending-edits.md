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
  (`CaseBodyEditState` and the like).
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
