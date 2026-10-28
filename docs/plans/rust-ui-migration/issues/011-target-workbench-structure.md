# 011: Target structure of the workbench around rust-ui patterns

Status: resolved
Type: grilling
Blocked by: [003](003-inventory-existing-ui.md)
Parent: [Map](../map.md)
Related: [009](009-close-rust-ui-behaviour-gaps.md), [PRODUCT.md](../../../../PRODUCT.md), [DESIGN.md](../../../../DESIGN.md)

## Question

The user put restructuring in scope: beyond swapping components, the app's structure may change
(workspaces, panel layout, navigation, command surfaces). What structure do we want, using
rust-ui patterns where they fit (Sidenav, Menubar, Command palette, Toolbar, Sheet, Toast,
Empty, Data Grid)? Which present structures stay (Layout, Case, Parts, PCB, Keymap, Keycaps,
Library, Export workspaces and their inspectors), what is rearranged, and what is the
fastest-Layout-editing priority from the UI design lane's plan asking of it? Canvases stay out of scope.

## Comments

**2026-10-21, progress (grilling in progress; ticket stays claimed).**

Decided with the user so far:

- Add a Command palette (Ctrl/Cmd+K) listing existing actions only; shortcut hints as Kbd.
- Layout keeps its two fixed strips and the Stationary Controls Rule, rebuilt on Toolbar, Toggle Group
  and Dropdown Menu, with Snap staying in the canvas toolbar.
- One narrow-screen rule for every workspace: below 980px side panels become Sheets with Objects and
  Inspector tabs (Layout's current behaviour).
- Transient results become Toasts; inline Alert and Field text stays for blocking states and field errors.
- Restructure freely toward the rust-ui.com look: the header copies rust-ui.com's, and the docks move
  to rust-ui's Sheet or Sidenav. The existing invariants (canvas always visible, Stationary Controls,
  persisted dock modes, 44px targets, keyboard reach) are defaults the spec may relax with a stated reason.

Facts read from rust-ui at `7120636e` (not run in a browser):

- **rust-ui.com header** (`src/components/navigation/app_header.rs`, docs app, not in the registry): sticky
  56px bar, `border-b border-border/40`, blurred translucent background; left: logo and desktop nav (a
  `NavigationMenu` with mega-menu panels, script-driven) or a mobile menu button; right: Command search
  (⌘K), vertical Separators, GitHub stars, theme toggle (sun/moon, writes `.dark` on `<html>` and
  localStorage `darkmode` through `eval`), a small rounded primary Button.
- **Sheet** is modal: full-screen `bg-black/50` backdrop, scroll lock, fixed 400px size, script-driven
  open and close. It cannot be a persistent dock without customising away the backdrop and scroll lock.
- **Sidenav** is a dock layout: `side` Left/Right, `variant` Sidenav/Floating/Inset, `collapsible`
  Offcanvas/Icon/None, `state` Expanded/Collapsed, trigger, resize handle, inset content area. Its resize
  handle is an `eval` script, mouse-only, finds only the first handle on the page, and stores one shared
  `sidenav-width` key, so two docks would collide; no keyboard resize. Take its markup and classes, write
  the behaviour ourselves.
- Our vocabulary: GLOSSARY.md has no "workspace" or "dock"; PRODUCT.md names the workspaces Design / PCB /
  Case / Parts / Export while the header shows Layout / PCB / Keymap / Keycaps / Case / Parts; DESIGN.md
  says in one section that workspace links are header tabs and in another that they are Objects-dock links.

## Answer

Decided with the user, 2026-10-21. Recorded in
[ADR-0040](../../../adr/0040-rebuild-the-workbench-shell-on-rust-ui-patterns.md); terms **Workspace** and
**Dock** added to the glossary; PRODUCT.md renamed Design to Layout.

- **Header:** copies rust-ui.com's composition (sticky 56px, bordered, solid), with a ⌘K Command search,
  one theme control for Theme and Appearance (built by [005](005-theme-structure.md)), and Export as the
  primary button. Project and Settings become Dropdown Menus; Workspace links stay in the header.
- **Docks:** Sidenav layout and classes for pinned and collapsed docks; our own resize and per-side
  persisted widths. **Auto-hide is dropped.** Narrow screens: one non-modal Sheet for every Workspace.
- **Added capabilities:** Command palette (existing actions only) and Toasts for transient results.
- **Kept:** Layout's two strips and the Stationary Controls Rule, rebuilt on Toolbar, Toggle Group and
  Dropdown Menu. Old layout commitments are defaults that the spec may relax with a stated reason.
- **Follow-on:** the palette's action list becomes [012](012-command-palette-contents.md); DESIGN.md is
  rewritten as a build step.
