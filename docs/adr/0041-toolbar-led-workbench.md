---
status: accepted
---

# Adopt a toolbar-led shared workbench without document-owner changes

The user chose the saved toolbar-led direction to keep the canvas available while making
Objects and precise editing deliberate, temporary requests. Across Layout, PCB, Keymap,
Keycaps, Case and Parts, replace side docks and the narrow dock Sheet with one 42px toolbar
and mutually exclusive anchored Objects / unpinned Inspect surfaces; share chrome, not
domain selection, drafts, edit admission or Undo ownership.

This partially supersedes [ADR-0035](0035-rebuild-the-workbench-shell-on-rust-ui-patterns.md):
its pinned/collapsed/resizable docks, saved dock widths/modes, narrow Sheet/tabs, Layout
two-strip/wrapping arrangement and crowded header composition. Keep semantic Themes,
rust-ui control behaviour, stationary canvas controls, project/save/search/history/settings/
Export reachability and the footer camera controls. The quieter header applies globally;
project and Export pages do not acquire irrelevant canvas panels. The 42px strip applies
to touch too; this is not a waiver of accessibility or other controls' target-size floors.

Objects has no permanent sidebar or minimized icon rail. In Layout, Outline comes first;
choosing it or the canvas boundary selects the current active outline without activating
another version or opening Inspect. Picking a half uses its actual name and changes
view-only focus, never construction-source identity, linking or history; saved links and
explicit exceptions still govern edits. “Placed from …” describes a key-anchor association,
not physical-side correctness or clearance. Findings and existing outline lifecycle stay
in Inspect. There is no duplicate Position editor, pinned alternative, backdrop or panel-
induced canvas resize/reframe. Drafts survive panel and selection switches; hidden content
leaves focus order. Measure final label fitting before solid-unit translation under a
stationary clip, favour action labels over captions, use no opacity/stagger/pop-in and
switch instantly for reduced motion.

## Consequences and alternatives

The former dock/pin/rail/Sheet alternatives are rejected because they preserve competing
shells and permanently spend canvas space. Retain all existing capabilities through named
destinations before retiring old paths; remap show/hide commands and stop reading obsolete
dock preferences without clearing unrelated browser preferences. Workspace-specific tools
and existing geometry, linked-half, export and fabrication contracts do not change.

The [planning transfer](../plans/toolbar-led-workbench/map.md) proposes exact overflow,
responsive and draft-lifetime details for approval. This ADR records direction, not shipped
UI or production implementation authorization; the frozen HTML references remain unchanged.
Legacy scripting access on the base is retained pending separately approved retirement,
not revived as product scope (deprecated [ADR-0029](0029-constrain-geometry-scripts-and-apply-results-atomically.md)).
