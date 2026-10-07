# 12: Editor composes per-workspace state

Status: claimed
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

- [ ] `Editor` composes state handles and lays out workspaces; it owns only
  cross-workspace state (active workspace, selection routing).
- [ ] `WorkspaceCallbackSlots` holds only handlers that cross workspaces, or is gone.
- [ ] Every extracted module has tests at its handle's interface.
- [ ] Mounted page tests pass unchanged.
- [ ] The backlog's "Feature ownership" entry is updated or removed.

## Verification

```sh
python3 scripts/check.py lint typecheck test browser
```
