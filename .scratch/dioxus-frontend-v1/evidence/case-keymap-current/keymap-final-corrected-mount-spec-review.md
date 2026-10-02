Decision: all three prior Keymap mount source findings cleared. No new material Spec finding in this bounded re-review; public acceptance remains open.

Reviewed integration presentation.rs SHA256 897d8542f20a2f90843fb956f3a181570616f520e7e7cdd1c1ce245748d639a9 and CSS b643ea167c6f8b74ef7f12cb9853943eab795b051c65a0a7e8b6ffde8857d28b against the c827 contract and earlier report.

1. Inspector reachability: the outer InspectorPanel condition now includes Keymap (1761), making layer selection, searchable selected-key details and the unavailable state reachable through the existing panel owner.
2. Panel handoff: workspace entry closes Objects and opens Inspect (1402-1418). Nonempty successful key activation does the same only after shared selection submission and fresh scope/generation, semantic context and supported-membership checks (755-842). Stale/failed callbacks return first; empty placeholder selection retains its separate clear-selection behavior.
3. Geometry: keymap_bounds (533-552) includes all four rotated projected rectangle corners. This matches the canvas translate/rotate geometry and accounts for wide keys and accepted-scene poses. Existing Y reflection, camera transform and padding remain intact.

The corrected code preserves the earlier accepted memo/selection/layer/read-only contract. No binding edits, schema additions, alternate selection authority or public API expansion were introduced by these repairs. Compact tree behavior remains separately qualified.

No compiler/browser was run for this review. Public compact entry/activation, wide/rotated geometry, first activation after Layout, scope/history/storage, keyboard/axe/theme and F3.1 acceptance remain open; source clearance does not close them. Refactoring: no new takeaway.
