# 11: Editor state tracer: canvas navigation and layout findings

Status: resolved
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

- [x] `Editor` holds none of the listed signals; `WorkspaceCallbackSlots` loses the
  canvas handlers it no longer needs.
- [x] Each module has tests at its handle's interface (native where it has no Dioxus
  runtime dependency, mounted otherwise); the findings regression tests live with the
  module.
- [x] Mounted page tests pass unchanged.

## Verification

```sh
python3 scripts/check.py lint typecheck test browser
```

## Outcome

Merged `86153002b`, rebased onto `dev` `e6eeb05f7`. Canvas navigation now lives in
`ui-shared`; Layout findings state lives in the Layout crate. The obsolete canvas
mount/start/move/key-up callback slots are gone. Final Standards and Spec reviews
confirmed the exact implementation worktree and committed diff, with no findings.

Pattern for ticket 12: a per-module Dioxus hook returns one private-state handle.
The handle owns cohesive signals, effects and behaviour; narrow methods and explicit
interaction accessors let Editor arbitrate cross-workspace concerns. The navigation
handle owns mount sizing, space-pan start/move/finish, pointer capture cleanup, camera
updates, wheel/keyboard zoom and key release. Editor supplies placement/selection
arbitration. The findings handle owns return/open/focus/pending state and lifecycle
effects; a compact resume callback lets the page compose navigation.

Mounted interface tests exercise actual Runtime pan and zoom camera updates, rather
than just a bag of signals. Findings regression tests moved to the handle's module.
Post-rebase lint and WASM typecheck passed; shared-UI browser tests 10/10, Layout
94/94 plus guide 1/1, and page 40/40 passed. The page count fell by two because those
findings tests moved into Layout. Earlier broad native Rust checks passed; the full
test step is blocked by the unchanged canonical CAD baseline fixture failure.
The full browser step passed its preceding suites but hit the exact-base Keymap
failed-binding timeout/SIGKILL; downstream affected tests were run separately.
See [ES-20 baseline evidence](../../edit-settlement/issues/20-preview-only-direct-edit-event.md#outcome).
