# 11: Editor state tracer: canvas navigation and layout findings

Status: ready-for-agent
Type: build
Blocked by: —
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Review: [candidate 05](../../../investigations/architecture-review-2026-10-07.html#c5)

## What to build

`fn Editor` (`web/src/presentation.rs`, ~2989-9015 at `915d305c0`) owns state for every
workspace: ~37 signals, ~12 effects, and `WorkspaceCallbackSlots` (~187-224, 33
handlers). Extract the first two state modules and settle the pattern the rest follow:

- **Canvas navigation**: pan/zoom, space-pan, `zoom_surface_size`, the
  `canvas_mount`/`canvas_start_pan`/`canvas_move_pointer` handlers.
- **Layout findings**: `layout_findings_open`, `layout_finding_return_target`,
  `layout_finding_return_focus`, `pending_layout_finding`, their effects, and the
  `layout_finding_return_regression_tests` module (~1081) which moves with them.

Each module owns its signals and effects behind one `use_*` entry point returning a
small handle; `Editor` composes the handles. Use the `codebase-design` skill to choose
the handle's interface before extracting; record the chosen pattern in `## Outcome` so
ticket 12 copies it.

Where the code lives (a module of the bin, `ui-shared`, or a workspace crate) follows
the crate layering in `docs/architecture.md`: state used by one workspace goes to that
workspace's crate.

## Acceptance criteria

- [ ] `Editor` holds none of the listed signals; `WorkspaceCallbackSlots` loses the
  canvas handlers it no longer needs.
- [ ] Each module has tests at its handle's interface (native where it has no Dioxus
  runtime dependency, mounted otherwise); the findings regression tests live with the
  module.
- [ ] Mounted page tests pass unchanged.

## Verification

```sh
python3 scripts/check.py lint typecheck test browser
```
