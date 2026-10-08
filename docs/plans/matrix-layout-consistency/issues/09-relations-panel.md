# 09: Relations shows structure, departures and authored constraints

Status: ready-for-agent
Type: build
Blocked by: 07, 08
Spec: [spec.md](../spec.md#relations) · Map: [map.md](../map.md)

## What to build

Render ticket 08's projection in the matrix-context Relations tab
(`web/src/presentation/layout_workspace.rs`, the `show_matrix_relations` branch). The
tab has three sections: **Structure**, **Local adjustments** (only when present) and
**Authored constraints** (only when present).

- Replace the workspace inputs `matrix_relationship_summary` and
  `matrix_relationship_target` with the projection. Delete
  `matrix_context_relationship_summary` and `matrix_context_relationship_target` in
  `web/src/presentation.rs`. Keep `layout_relationship_summary`, which the standalone
  component inspector uses.
- References are buttons that select their `TreeContext` through
  `on_select_context`. Reuse `matrix_relationship_escape_target`, so Escape returns
  as it does today.
- A departure's Reset uses the Inspector field path (`transform_edit_resolver`,
  `…Reset`). Convert uses ticket 07's one-shot action, with the same
  disabled-with-reason state.
- Departed targets: before selecting, check `selection::context_is_current`. If the
  target has gone, do nothing. A new selection re-projects, so no rows go stale.
- Remove the static "Matrix rows and columns share pitch, stagger and splay" note.
  Use existing Inspector tokens and classes. The `impeccable` pass refines the
  visuals later.

## Impact

`matrix_context_relationship_summary` and `layout_relationship_summary`: LOW, five
`Editor` processes. Rerun `impact` before deleting.

## Tests (mounted, `web/src/presentation/layout_component_inspector_tests.rs`, `web` bin browser batch)

1. A blank matrix created through the real Session shows Structure for the matrix, a
   row, a column and a key, and never shows the old fallback sentence.
2. A column offset departure: Reset lands, the departure disappears, and Undo
   restores it. Convert lands and Structure shows the new stagger.
3. A constrained companion: the constraint's button selects the source, and Escape
   returns.
4. Changing the selection from Column 2 to a key shows no stale rows.
5. A referenced key removed by a landed edit: its row disappears, and a click queued
   before the removal does nothing and raises no error.
6. The existing "Relations tab survives an unrelated accepted revision" (~line 1271)
   still passes.

## Acceptance

`lint`, `typecheck` and `browser` pass (the `web` bin and layout batches).

## Files

The Relations branch of `layout_workspace.rs`, the `matrix_context_relationship_*`
region of `presentation.rs`, and additions to `layout_component_inspector_tests.rs`.
