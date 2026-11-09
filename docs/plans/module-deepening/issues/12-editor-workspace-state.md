# 12: Editor composes per-workspace state

Status: resolved
Type: build
Blocked by: 11
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md)

## What to build

Apply ticket 11's pattern to the rest of `Editor`'s state (signal names at
`915d305c0`):

- **Case**: `case_body_selection`, `case_layer_selection`, `case_display`,
  `case_body_edit_dispatch`, `case_body_editable`, `case_tree_expanded`.
- **Layout tools**: `layout_selection_kind`, `layout_snap_settings`,
  `layout_context_tab`, `layout_command_menu`, `layout_transform_tool`,
  `layout_assembly_3d`, `matrix_splay_affect`, `pending_splay_origin_pick`,
  `pair_created_selection`, the component Inspector edits.
- **Parts**: `parts_query`, `parts_selection`, `parts_assembly_selection`,
  `parts_selection_generation`, `parts_preview_activation`, `parts_generator_draft`,
  `parts_assembly_orientation`.
- **Keymap and Keycaps**: `keymap_layer_id`, `focused_keycaps_finding`, the pending
  layout fit, the keymap and keycaps projections.
- **Setup guide**: `guide_preferences`, `instance_preference`, `geometry_scripts_open`
  and their persistence effects.

One commit per workspace, each passing the checks below.

## Acceptance criteria

- [x] `Editor` composes state handles and lays out workspaces; it owns only
  cross-workspace state (active workspace, selection routing).
- [x] `WorkspaceCallbackSlots` holds only handlers that cross workspaces, or is gone.
- [x] Every extracted module has tests at its handle's interface.
- [x] Mounted page tests pass unchanged.
- [x] The backlog's "Feature ownership" entry is updated or removed.

## Verification

```sh
python3 scripts/check.py lint typecheck test browser
```

## Outcome

Merged `deepening/12-workspace-state` through `784dcb079`, rebased onto `1b1534e70`.
Each workspace extraction includes its Editor wiring: Case `719ccbb56`, Layout
`9946a3c1d`, Parts `54198e862`, Setup `7bbe745e1`, Keymap `fc4ab9163`, Keycaps
`4bc3c7521`; the final commit composes the common routing. The final source preserves
the native Runtime's direct placement consumer and the constructor sweep.

Use a per-workspace hook returning a private-state handle. Cohesive state, effects,
projections and actions live there; narrow methods/accessors support cross-workspace
arbitration. Editor composes handles and retains only the select_tree/navigate routing
slots. Every handle has mounted interface coverage; the Feature ownership backlog
entry is updated.

Intermediate workspace WASM checks and the corrected Layout lint/check passed;
final lint/typecheck and native workspace/footprints tests passed. The complete browser
gate passed, including Keymap 21 and page 43. After constructor integration, lint,
typecheck and page 43 passed again; unchanged module browser evidence was reused.
The native test gate retains the unchanged CAD baseline (49 passed, 1 failed,
4 ignored; rotated-concave/bottom volume expected 80481.2399, actual
80579.55733514718, tolerance 0.1). The earlier full-Keymap timeout is intermittent,
not a permanently blocked gate.

Final parallel Standards/Spec reviews and the constructor conflict-delta reviews found
no actionable issues. Duplicated mounted-test setup and a repeated pending-pick reset
were recorded as nonblocking judgment calls. The final claim-doc rebase changed no
implementation files.
