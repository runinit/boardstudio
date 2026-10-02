# F6C.2 specification — Selected-key Inspector composition

## Outcome

The Keycaps Inspector matches the pinned React Selected key disclosure: title, open/collapsed behavior, search, picker and override-only body. It removes two Dioxus-only regions that React does not render, while retaining existing key selection and Editor-owned update paths.

## Source-backed gap

React `KeycapPanel.tsx` wraps the key search, Selected key picker and current key's legend/color/profile/mount/row/units fields in one `InspectorSection` titled `<reference> · key` (or `Select a key`) and initially open. Dioxus `keycaps_workspace::inspector` renders `KeycapsKeyList`, `KeycapsSelectedSummary`, and `KeycapsSettingsEditor` as separate always-visible regions, and also inserts a read-only Matrices list absent from React. This changes contextual grouping/collapse behavior and adds two extra regions.

## Requirements

- [ ] Use one default-open native disclosure for the React Selected key section; title is exactly `<reference> · key` or `Select a key` with no selected key.
- [ ] Keep only Find a key, Selected key, and current per-key override fields inside that section, preserving React labels and available choices. Remove the Dioxus-only separate `KeycapsMatrixList` and `KeycapsSelectedSummary`; matrix configuration remains in Matrix profiles and selected values remain visible through the React-equivalent inputs/physical view.
- [ ] Selection from the physical canvas, searchable list, or dropdown updates the title and selected editor without switching the editor's existing operation owner or discarding unrelated fields.
- [ ] Collapse/reopen works with pointer and native keyboard disclosure behavior. Changing selection while open follows current React behavior; do not reset the user's disclosure choice unnecessarily.
- [ ] Preserve per-key editor's existing pending/accepted/error/currentness, React input event timing, clear/default pair semantics and normal undo/save behavior. This ticket is presentation composition only.
- [ ] Paired browser journey compares the same fixture and selection path in pinned React and Dioxus, including the absence of a standalone Matrices list and selected summary, no-selection title, selected-key title/fields, search, dropdown/canvas selection, pointer/keyboard collapse and reopen, one per-key edit, accepted feedback, Undo/Redo and reload. Record source/build/fixture hashes; this child does not close the full F6C.2/F6C.4 joins.

## Start and acceptance

Start after canonical F6C.1 accepted key projection/selection and the existing per-key editor mount are proven. This child remains within F6C.2; Board colors and Matrix profiles tickets 01/02 keep their own gates. F6C.2 parent status and all F6 criteria remain open until their listed acceptance passes.

## Refactor notes

Carry RF-001 contextual Inspector composition. No new architecture finding is asserted. Do not create a second selected-key state owner or move edit/history authority into the disclosure component.
