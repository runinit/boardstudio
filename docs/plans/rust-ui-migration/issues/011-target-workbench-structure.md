# 011: Target structure of the workbench around rust-ui patterns

Status: claimed
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
