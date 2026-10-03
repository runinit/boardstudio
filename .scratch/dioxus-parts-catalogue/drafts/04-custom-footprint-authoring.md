# Draft: Create and edit a custom component definition

**Parent:** F4.2 — Create, import and edit footprint definitions (`.scratch/dioxus-frontend-v1/issues/04-parts.md`).

**What to build:** From Parts, a designer can immediately create the reference's default custom component and then edit its supported footprint geometry and electrical pad details through the same per-action edit behavior as React.

**Start gate:** The coordinator has mounted the Parts inspector and proven immutable current-project identity, selected-definition inputs, and a private save callback into the existing edit/history path. This gate requires only the selected-definition/edit capability; it does not wait for all F4.1 catalogue families or module loading/error qualification. The parent F4.2 start and acceptance joins stay unchanged.

**Blocked by:** The concrete private Parts selection/save contract described above. No separate Rust service or F3 placement behavior is needed for custom definition authoring.

**Status:** spec-clear; standards-review-pending

- [ ] The reference Create action immediately inserts and selects the same default custom definition; do not require a separate Save/Apply action before it exists in the document.
- [ ] Edit the reference-visible supported fields (name, Definition kind, courtyard, pad identity/number/shape/dimensions/position and drills); the kind control offers the reference's Switch/Controller/Connector/Encoder/Passive/Custom values. Do not add a per-pad rotation control that React does not expose. Preserve DraftInput local text state, Escape reset, Enter-to-blur, validation and commit-on-blur/action boundaries.
- [ ] Each accepted reference action creates its own normal edit/Undo step; do not batch the full form into a new whole-form transaction. Verify selection/details, save/reopen, archive round-trip and undo/redo at those boundaries.
- [ ] Validate only the rules applicable to authored definitions: unique component/pad IDs, unique nonempty authored electrical pad numbers, positive finite dimensions and finite coordinates. Invalid input stays local/recoverable and does not overwrite accepted values.
- [ ] With an existing placed-part fixture, prove pad-ID rename remaps that part's matching net pins and pad removal removes the corresponding pin references. Preserve unrelated nets and placements; required remapping is in scope and must not be treated as an unrelated project mutation.
- [ ] Keep source-owned imported KiCad pads read-only here; KiCad import and model-file asset attachment belong to separate children of F4.2. Do not add placement controls: F3 owns placement.
- [ ] Run the same create/edit/invalid-input/save/Undo/reopen journey in pinned React and Dioxus with desktop and compact captures, keyboard/focus checks, accepted document/revision comparison, and no page errors.
- [ ] Record a refactoring handoff under existing RF-001/RF-002/RF-006/RF-009 as applicable, or explicitly state no new takeaway was observed; inherited independent Spec/Standards and shared acceptance gates remain open.

**Completion joins:** Preserve F4.2's INT.2 parent join and the F4/F9 shared browser, build, accessibility and RF gates. This child does not close F4.1, F4.2, INT.2, F4 or F9.
