# F2.4 bounded slice — Project, save status, tabs, Export and Appearance

**Parent:** [F2 — Projects, panels and shared controls](../issues/02-projects-shared-ui.md), capability slice of F2.4. This packet does not close F2.4 or any parent acceptance row.

## Evidence and problem

At pinned React source `5a472a9426e6e38993361da402cd4ec730feb369`, `app/src/ui/Workbench.tsx` composes the Project trigger, a dot-only save-status disclosure, six workspace tabs, and a separate Export action. `unified-workbench.css` and `workbench.css` define the desktop 42px header and tab spacing. `Workbench.tsx` places Light/Dark/System under Project → Workspace settings → Appearance. Its mode icons are in `WorkbenchIcons.tsx`.

The Dioxus header currently places a standalone brand, text project disclosure, visible Saved label, tabs, Export, and a System/Light/Dark selector together. The Project menu does not expose Workspace settings. The paired Board2 screenshots at `1280×940` confirm the spacing and placement differences; only the visible shell/menu behavior is in this slice.

## Contract

- Keep the current project name and brand mark in one Project trigger with a chevron, accessible Project label, and existing project/library actions.
- Present the accepted local-save state as an accessible dot disclosure with status-specific text in its popover. Preserve success, saving, failure and unavailable meanings.
- Render Layout, PCB, Keymap, Keycaps, Case and Parts in pinned order with the pinned icons, icon/label gap, tab padding and selected-state semantics. Keep Export as a separate right-aligned action with its source icon and label.
- Move Light/Dark/System to Project → Workspace settings → Appearance, with the source label `Appearance`, select name `Color theme`, and existing `boardstudio:v2:theme` preference behavior.
- Preserve current navigation, session, document, and persistence authorities. No new writable project state or backend/API/schema changes.
- Limit changes to the shared header, the Project settings presentation, and scoped shell CSS. Do not absorb the separately owned footer, Objects/creation, or broad panel preferences.

## Acceptance

1. Compare the same saved archive, active board/workspace, theme, and `1280×940` viewport against pinned React. Verify topbar height and measured Project/status/tab/Export positions, tab widths and icons, selected state, and no visible `Saved` or theme selector in the topbar.
2. Open Project → Workspace settings, change Appearance among System/Light/Dark, return with Back to project menu, and confirm the preference applies and survives reload without changing the accepted document/revision or Undo history.
3. Verify Project, save-status, tab, Export and settings controls by keyboard; status and active-tab semantics remain accessible. Reuse existing evidence for unchanged behavior; do not run a broad test matrix.
4. Root owns the one combined affected build/check and integration candidate. Full F2.4 criteria, responsive/compact acceptance, panel settings and other parent gates remain open.

## Refactoring handoff

No new refactoring takeaway observed. This is bounded shared-composition wiring in the existing private presentation owner and extends RF-001 only; defer any general shell extraction until post-port review.

### Compact topbar continuation

The shared-shell continuation keeps Export as a separate topbar action at 720×640 and removes the redundant Export option from the workspace selector except while Export is the active workspace. Compact Objects/Inspect toggles live in the topbar at the source breakpoints; their drawer layout and focus behavior remain owned by the current F1 continuation in [issue 01](../issues/01-shell-theme.md). Do not move panel mode/width preferences or feature drafts into the shell.
