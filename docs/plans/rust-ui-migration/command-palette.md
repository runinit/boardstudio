# Command palette

Ticket: [012](issues/012-command-palette-contents.md). Decided with the user, 2026-10-21 (all six proposals accepted). Part of the shell
([ADR-0040](../../adr/0040-rebuild-the-workbench-shell-on-rust-ui-patterns.md)); the term is in the [glossary](../../../GLOSSARY.md).

## Rules

- **Opens with Ctrl/Cmd+K** from the header's search button or the keyboard, from anywhere, including inside a field. A field blurs
  exactly as on a click-away and commits or cancels by its own rule, so no new edit-loss case appears.
- **Existing actions only.** Every command dispatches the same event or UI change as the control it mirrors (`Navigate`, `SelectParts`,
  `SelectMatrixCell`, `Undo`, `Redo`, `StartGeneration`, `CancelGeneration`, `StartExport`, `Open`, or workspace and dock state). It
  adds no editing ability.
- **Opens tools, never performs destructive or parameterized edits.** No Delete matrix, Unlink halves, removing parts or numeric
  transforms; "Align" opens the Align tool and aligns nothing.
- **Context.** Commands belonging to other Workspaces are hidden. A command that is unavailable now in the current Workspace is shown
  disabled with the reason the UI already gives (for example the Export blocker).
- **Shortcuts.** Ctrl/Cmd+K is the only new shortcut; undo and redo show their existing hints (Ctrl/Cmd+Z, Ctrl/Cmd+Shift+Z or Y).
  Single-key tool shortcuts are not added. The pointer modifiers (Alt bypasses snapping, Shift gives coarser steps, Ctrl quick offset)
  are documented as `Kbd` hints in tooltips on the controls they affect, not as palette commands.
- **Build.** Look from rust-ui `Command`; behaviour written by us in Rust on `Dialog` and `Combobox` behaviour (both pass the
  [accessibility floor](accessibility-floor.md)), without upstream's script. Escape closes the palette first.

## Contents

| Group | Commands |
| --- | --- |
| Go to | Layout, PCB, Keymap, Keycaps, Case, Parts; Export; project library; Setup guide |
| Project | New keyboard, import project, open demo keyboard, rename project, save a copy, Workspace settings |
| Edit | Undo, Redo |
| View | Show or hide Objects and Inspector, fit board to view, zoom in and out, 2D / 3D / Footprints (Layout) |
| Current Workspace | Layout: selection scope (Matrix, Row, Column, Key, Part), Add (Layouts, Parts, Board geometry, Encoders…), Snap and Geometry snap, Align, Transform, Position / Stagger / Offset / Splay when relevant. Case: Generate, Cancel, Retry |
| Objects | typing searches parts, matrices, keys and outlines by name and selects the match through the existing selection events |

The exact labels and ordering come from the controls they mirror; the build lane lists them against the shipped UI.
