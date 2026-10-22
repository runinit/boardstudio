---
status: accepted
---

# Rebuild the workbench shell on rust-ui patterns

The workbench's overall structure is rebuilt to follow rust-ui's own site, not only its controls
([ADR-0034](0034-adopt-rust-ui-components-with-two-themes.md)). The user chose this on 2026-10-21 so the
Header, Docks and menus share one visual and interaction language instead of a re-skinned old shell.

- **Header.** A sticky 56px bar with a bottom border and a solid background, composed like rust-ui.com's:
  logo, Project menu and Workspace links on the left; on the right a Command search button (Ctrl/Cmd+K),
  save status, Undo and Redo, one theme control for Theme and Appearance, and Export as the primary
  button. The Project and Settings menus become Dropdown Menus.
- **Docks.** Pinned and collapsed docks use Sidenav's layout and classes, Objects on the left and
  Inspector on the right. Resizing (pointer and arrow keys) and one persisted width per side are ours,
  because Sidenav's resize handle is mouse-only, finds only the first handle on a page and shares one
  storage key. Auto-hide is dropped; saved auto-hide preferences need a fallback in the build.
- **Narrow screens.** Below 980px every Workspace shows its Docks as one Sheet over the lower part of the
  screen, with Objects and Inspector tabs, as Layout does today. The Sheet is non-modal: no backdrop and
  no scroll lock, so the canvas stays visible and interactive.
- **Commands and feedback.** A Command palette lists existing actions only; shortcut hints use Kbd.
  Transient results become Toasts; inline Alert and Field text stays for blocking states and field errors.
- **Layout strips.** Layout keeps its two fixed strips and the Stationary Controls Rule, rebuilt on Toolbar,
  Toggle Group and Dropdown Menu, with Snap staying in the canvas toolbar.

Earlier layout commitments (canvas always visible, Stationary Controls, 44px coarse-pointer targets, keyboard
reach, Export as its own page) are defaults that the migration spec may relax with a stated reason.
Canvas and 3D surfaces are unchanged. Workspace and Dock are defined in the glossary; DESIGN.md is
rewritten as a build step once the structure is implemented.
