# 08: Parts and PCB panels settle through `PendingEdits`

Status: claimed
Type: build
Blocked by: 05, 19
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md)

## What to build

Move Parts and PCB settlement sites onto `PendingEdits` and delete the per-panel
layer. Files (at `915d305c0`):

- `web/crates/parts/src/`: `parts_custom_definition.rs` (`settle_ticket`,
  `apply_text_settlement`, `apply_plain_settlement`, both `owner_is_live`),
  `parts_new_component.rs`, `parts_import_footprint.rs`, and under `parts/`:
  `generator_settings`, `assembly_editor`, `component_model_editor`,
  `mechanical_profile`, `mechanical_profile_ui`, `module_profile_editor`,
  `modules_catalogue/module_attachment`;
- `web/crates/pcb/src/`: `pcb_board_reference`, `pcb_module_inspector`,
  `pcb_physical_setup/controller`, and under `pcb_wiring/`: `apply`, `controller`,
  `mode`, `pins`, `part_input_settings/owner` (`owner_is_live`).

Typed Core edits 01 also edits `pcb_wiring/mode.rs`; whichever lands second rebases.

## Test preparation dependency

[Parts and PCB native tests prepare the panel migration](19-parts-pcb-native-test-preparation.md)
runs independently on the merged Runtime/constructors in the user's third AI app.
It owns the named test drivers and PCB native owner regression coverage. Reuse its
merged tests; do not repeat or overwrite that preparation. Remaining hand-resolved or
dormant domain tests in this ticket's other files still migrate here as needed.

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

- [ ] `GeneratorSubmission`, `AssemblySubmission`, `ModelSubmission`,
  `PartInputFeedbackState` and the three remaining `owner_is_live` functions are gone.
- [ ] Exact post-landing selection (edit settlement 15) still works through `Landed`.
- [ ] The electrical remap review keeps its strict route (ADR-0005).
- [ ] Mounted Parts and PCB tests pass.

## Verification

```sh
cargo test -p boardstudio-web-parts -p boardstudio-web-pcb --locked
python3 scripts/check.py lint typecheck test browser
```
