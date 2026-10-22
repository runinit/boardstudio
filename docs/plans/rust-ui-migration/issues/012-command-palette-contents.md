# 012: Command palette contents and shortcut hints

Status: resolved
Type: grilling
Blocked by: None
Parent: [Map](../map.md)
Related: [011](011-target-workbench-structure.md)

## Question

The Command palette (Ctrl/Cmd+K) lists existing actions only. Which actions and objects does it
list and how are they grouped (go to Workspace, Undo and Redo, panel and Snap toggles, Add object,
Export, project actions, jump to a part or key)? Which carry shortcut hints, and does it add any
shortcut beyond Undo and Redo today? Where does each action's behaviour stay owned (the palette only
dispatches existing events)?

## Comments

**2026-10-21, findings (grilling in progress; ticket claimed).**

- **Shortcuts today:** the only global shortcuts are Ctrl/Cmd+Z (undo), Ctrl/Cmd+Shift+Z and Ctrl/Cmd+Y (redo), ignored while typing in
  an input (`web/src/presentation/workbench_shortcuts.rs`). No single-key tool or workspace shortcuts exist. Other modifier keys are
  pointer-gesture modifiers: Alt bypasses snapping (outline and transform drags), Shift gives coarser steps (10x in outline nudges, 1 vs 0.1
  in transform nudges, 5 vs 1 degree in rotation), Ctrl enables quick offset.
- **Events a palette could dispatch** (`application/src/session.rs`, 23 variants): `Navigate`, `SelectParts`, `SelectMatrixCell`, `Undo`,
  `Redo`, `StartGeneration`, `CancelGeneration`, `StartExport`, `ExportCommit`, `Open`, `ResolveEdit`, camera and gesture events.
  Workspace choice, dock visibility and the project menu pages (`Project`, `Settings`) are UI state, not events.
- **Existing action surface:** six Workspace links plus Export in the header; project actions (new keyboard, import project, open demo,
  rename, delete, save a copy, Setup guide, Workspace settings); Layout selection scopes (Matrix, Row, Column, Key, Part), contextual tools
  (Position, Stagger, Offset, Splay, Origin, Align, Transform), Snap and Geometry snap options, Add sections (Layouts, Parts, Board geometry,
  Encoders and others), Case Generate / Cancel / Retry, footer zoom and fit, 2D / 3D / Footprints.

## Answer

Decided with the user, 2026-10-21; all six proposals accepted. Written in [command-palette.md](../command-palette.md):

- Ctrl/Cmd+K opens it from anywhere, including inside a field (which blurs by its normal click-away rule).
- Groups: Go to, Project, Edit, View, Current Workspace, and Objects (search and select parts, matrices, keys, outlines by name).
- Existing actions only, dispatching the same events as the controls they mirror; it opens tools and never performs destructive or
  parameterized edits.
- Other Workspaces' commands are hidden; unavailable ones are shown disabled with the UI's existing reason.
- Ctrl/Cmd+K is the only new shortcut; pointer modifiers (Alt, Shift, Ctrl) become `Kbd` hints in tooltips, not commands.
