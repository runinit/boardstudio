# F3.5a: Inspect and edit a standalone Layout component

## Problem statement

Selecting a standalone component in the Dioxus Layout canvas currently shows a generic Position editor and the selected-context heading. The pinned React workbench gives that component a Properties/Relations Inspector with its definition, position, optional layout assignment, board-outline contribution, placement relationship, and a route to electrical connections. Without that context, designers cannot finish common component setup from the Layout selection.

## Solution

Add the reference component Inspector for one current standalone Layout selection. Keep form state in the private Dioxus Inspector and submit accepted changes through the existing Runtime/Core edit and history paths. The existing PCB workspace remains the authority for electrical editing; the Inspector action only navigates there.

## User stories

1. As a keyboard designer, I want the selected standalone component’s Properties and Relations to be named and available, so I can understand the current selection without leaving Layout.
2. As a keyboard designer, I want to set a component’s X/Y position, so I can place it precisely using the existing move command.
3. As a keyboard designer, I want to place a standalone component in a named Layout or on the board, so the saved assembly grouping matches my design.
4. As a keyboard designer, I want to include/exclude a component from the generated board outline and configure its existing margin/overhang contribution, so the component contributes correctly without editing board geometry.
5. As a keyboard designer, I want to create, edit and remove an Offset or Mirror placement relationship, so a component can follow another board part through the existing constraint authority.
6. As a keyboard designer, I want to inspect the active relationship and return to its Properties, so the relationship has a clear reference and editing path.
7. As a keyboard designer, I want to continue from a selected component to PCB electrical connections, so I can use the existing wiring workflow.
8. As a keyboard designer, I want edits to be reversible and saved with the document, so I can safely review and reopen component setup.

## Implementation decisions

- This is one vertical child of existing F3.5, not a new parent or a rewrite of the broad Inspector milestone.
- Keep one selected standalone component as the initial context. Matrix/key/row/column selection remains with existing F3.2/F3.3 ownership.
- Preserve React’s Properties/Relations labels and the selected component’s reference, definition name/kind, Position, optional Layout assignment, Board outline contribution, Layout constraint and PCB action.
- Submit position through existing MoveParts; constraint changes through existing SetConstraint/RemoveConstraint; layout membership and Part outline fields through the current accepted document replacement path. Runtime/current Layout owner must admit each mutation against its captured document, selected identity and scope.
- Restrict constraint sources to other parts on the selected board and reject stale, cross-board and self targets. Keep constraint evaluation and validation in Core.
- Preserve existing save/history behavior. The PCB action is a navigation callback, not a second electrical editor.
- Do not add public Rust API/schema/edit operations or implement board outline geometry. Root/coordinator owns shared selection-to-Inspector composition, mutation callback translation, CSS and packaging; private feature UI remains separately authored.

## Testing decisions

- Test production mounted Inspector interactions with a saved fixture containing a standalone component, at least one Layout, multiple board parts, part-envelope state and a relationship-capable selection.
- Compare the visible tabs/controls and resulting accepted document/history state against the pinned React workbench. Exercise valid and invalid forms, selection/scope changes, desktop and compact presentation, Undo/Redo and save/reopen.
- Keep a test at the highest existing seam: the mounted Dioxus Layout Inspector and existing Runtime/Core edit path. Do not treat projection-only or helper-only coverage as completion.
- Preserve full F3.5/F3.7 paired journey, finding navigation and broader parent acceptance as separate open gates.

## Out of scope

- Matrix, key, row or column structural editing; matrix transforms, align, snapping and multi-selection.
- Board-outline geometry/version editing, keycap dimensions/reflow, mirrored-component substitution and part-definition editing.
- New constraint families, geometry or constraint algorithms, schemas, public APIs, edit operations, persistence stores, electrical editing implementation or production frontend cutover.

## Further notes

- Pinned reference: React source revision `5a472a9426e6e38993361da402cd4ec730feb369`; selected `left-U1` in the imported layered Sofle fixture exposed Properties/Relations, Component layout, Position and Edit electrical connections. The source-backed component branch additionally renders Board outline contribution and Layout constraint.
- Canonical parent relation remains F3.5 under F3, whose only canonical start dependency is F3.1. This child starts on the narrower proven accepted selection/current-owner/edit-port capability rather than waiting for complete F3.1 or unrelated F3.5 children.
- F3.5/F3.7, public paired save/reopen journey, broader selection contexts and finding navigation remain open after this child.
