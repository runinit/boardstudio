# 08: Parts and PCB panels settle through `PendingEdits`

Status: claimed
Type: build
Blocked by: 05, 19
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md)

## Execution split

Run the [Parts half](../handoff-08-parts-pcb.md) and
[PCB half](../handoff-08-pcb.md) concurrently with disjoint crate ownership.
Acceptance criteria remain shared; close this parent only after both halves are
reviewed and integrated.

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

## Comments

### 2026-10-08: PCB final-source review and browser gate

Root confirmed the PCB-only branch is clean at
`05e1b7026e983afd1148581fc94b148e4ab5eaac`, 11 commits after `8ce860a09`.
Protected pcb_wiring/remap.rs is unchanged. Fresh pinned root review covered all
17 changed files: Standards found zero violations/smells; Spec found two remaining
PCB implementation gaps and the previously reported typed-placement contract gap.
See the [focused final follow-up](../handoff-08-pcb.md#final-follow-up-at-05e1b7026)
for the fixes, required evidence and preserved-history decision.

Create Net still implements its submitted one-shot state outside the shared helper.
Circuit-removal keys accumulate per copied circuit while sharing one disabled Signal;
a retained sibling handler can submit another removal and an older terminal can
re-enable the control while that other request is pending. Use the existing helper
and bounded action keys; neither correction needs a shared API extension.

The oneshot preparation-fixture fix, dispatcher refusal, parent-owned circuit
Signal and narrowed visibility have static review support. The agent reports native
PCB/lint/typecheck passing at that HEAD. The user's latest browser report starts 45
tests but ends in driver SIGKILL with filtered output; an isolated run has no output.
Final mounted behavior remains unverified. Use an explicit 120-second batch budget,
unfiltered output and preserved process exit status before diagnosing another cause.

The branch-specific graph comparison saw all 17 changed files, 35 indexed symbols
and one affected flow, medium aggregate risk, with no partial/truncated flags. Branch
symbols unresolved by the canonical dev graph were inspected directly; this is not
complete graph coverage. The root embedding refresh also failed on duplicate primary
key `:0`; current-source review is the fallback, with the integrated-tree gate retained.

Keep the parent claimed until both Parts and PCB are reviewed and integrated. The
remaining common helper/spec decision stays with root; the other panel stream remains
held. Preserve all PCB commits, including the non-building intermediate `63e051463`,
and record that limitation rather than rewriting history.
