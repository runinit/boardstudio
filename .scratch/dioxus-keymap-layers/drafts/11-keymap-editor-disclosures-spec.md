# Keymap editor Inspector disclosures

## Problem statement

The Keymap Inspector already has separate Keys, Macros, and Encoders editor tabs, but their active content is always expanded. React presents each active editor inside a native Inspector disclosure so the designer can reclaim vertical space without leaving the current editor.

## Solution

Put the active Selected key, Macros, or Encoders editor inside one expanded-by-default native disclosure. Preserve the selected editor tab and its existing behavior. The summary names match React: the current key and layer or “Select a key”, “Macros”, and “Encoders · <layer>”. Collapsing affects only presentation.

## User stories

1. As a Keymap designer, I want the active editor to open expanded when I enter its tab, so its controls remain immediately available.
2. As a Keymap designer, I want to collapse the active editor and reopen it, so I can reclaim Inspector space while keeping its tab selected.
3. As a keyboard user, I want the section summary to use native disclosure interaction and expose its expanded state, so I can operate it accessibly.
4. As a designer changing the active layer for the same selected-key owner, I want the summary to update without reopening a section I collapsed; changing the selected key or scope remounts the Inspector expanded, matching React.
5. As a designer returning to a different editor tab, I want its newly mounted section expanded by default, matching React's conditional editor mounting.
6. As a designer, I want collapsing a section to leave the active layer, selected key, accepted document, revision, and undo/redo history unchanged.

## Implementation decisions

- Use the existing Keymap panel, editor tabs, and private editor elements. Add no data state, event, public API, schema, or copied editor controls.
- Keep the Layers disclosure as its existing independent section. The three editor tabs remain visible and unchanged.
- The Selected key summary follows the current reference and active-layer name; the Encoders summary follows the active layer. Existing macro, binding, encoder-row, rotation, and push controls stay in their current feature owners.
- Each active editor section starts expanded on mount. Native user open/closed state persists through ordinary accepted-projection, active-layer, and local feedback rerenders while that tab and accepted selection owner remain mounted. React keys the Inspector by accepted selection owner, so changing the selected key or scope remounts Keys expanded. Switching tabs unmounts the prior pane; returning mounts it expanded.
- Match the pinned Inspector summary, disclosure marker, focus treatment, and content spacing. Preserve dynamic section titles without resetting the native open state.
- This is a behavior-preserving presentation change. Existing feature-level validation and history remain owned by their current Keymap edit paths.

## Testing decisions

- Compare the three active editor disclosures against the pinned React Inspector on the same saved fixture. Verify each starts open, can be collapsed and reopened, and keeps its tab selected.
- While Keys remains active, change active layer with the same selected-key owner while collapsed; verify the title updates and remains collapsed. Then change selected key/scope and verify the new owner mounts expanded. Switch away and return to another tab; verify its section starts open.
- Verify disclosure changes do not alter the document revision or Undo/Redo state. No new ordinary UI test is required; run the affected combined compile and one paired browser journey.

## Out of scope

- Changes to the editor tab model, layer disclosure, key binding/macro/encoder behavior, nested encoder details, firmware output, shared shell, persistent data, or the F6K parent acceptance.
- Full F6K.1/F6K.2/F6K.3/F6K.4/F3.1 acceptance joins.

## Further notes

- Parent: existing F6K.1. The F6K parent count and dependency graph remain unchanged.
- Pinned React revision: `5a472a9426e6e38993361da402cd4ec730feb369`; source oracle is `KeymapPanel.tsx` and `InspectorSection.tsx`.
- The current mounted Keymap panel already exposes all three editor tabs and private editor surfaces; this child starts from those callable presentation capabilities without waiting for broader parent acceptance.
