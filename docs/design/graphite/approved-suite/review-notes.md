# Graphite suite review

## Direction carried across the suite

One application header, neutral charcoal surfaces, periwinkle selection, docked navigation and inspectors, and opaque contextual toolbars with labels. Project browser and setup use a simpler header. Objects, case assembly, parts catalogue and keymap layers each have a distinct navigation job. Export uses a handoff list rather than a canvas or a spreadsheet.

## Onboarding sequence

Projects → New keyboard → Project & hardware → Layout & assemblies → Create layout & open editor. The guide remains optional and resumable, with Controller & wiring, optional Case, and Review & export available later. The first setup preview was corrected to show empty board silhouettes and remove premature export actions. Setup is separate from Objects.

## Details to resolve before implementation

- Generated keyboard key counts and example references vary. Use actual document geometry and stable IDs, not image counts, when implementing. The first-layout preview intentionally has 36 positions before any thumb keys, while Aster 42 is a different example state.
- PCB: the illustration contains too many footprint rows for the named example. Pin values are examples, not a verified controller mapping. The Wiring tab currently includes placement fields; separate tab content consistently.
- Keymap: the model drew a Transparent key checkbox under Layer. Transparency is a selected-key behavior, not a layer-wide toggle. Keep behavior and layer semantics explicit. Do not import that checkbox literally.
- Parts: footprint pads, courtyard and dimensions are illustrative and not a valid manufacturing footprint. The generated Cherry (OEM) label conflates distinct profiles and must not be retained. Collapse specialist connection and geometry controls by default.
- Export: Save .boardstudio project downloads a portable file. The generated note Saves to this browser under that action is misleading; local autosave and explicit download must be separate. Avoid duplicating Export in header navigation and the action area.
- Project browser: remove irrelevant undo/redo/Saved controls in the browser shell. Retain plain local-storage guidance without literal negative prompt constraints.
- Onboarding: direct stagger drag should use a vertical handle. The first-layout image drew horizontal arrows. Use real board outlines derived after layout creation; the starting silhouettes are schematic only.
- Case direct thickness handles and redesigned guide placement are proposed interaction work. Verify them with keyboard alternatives and undo behavior.
- All palettes need measured contrast, consistent toolbar heights, disabled/focus/hover states and responsive verification in implementation. Generated slight gradients are not a new token requirement.

No application UI was implemented in this mockup batch. These proposals translate the selected visual direction; other surfaces await user review.
