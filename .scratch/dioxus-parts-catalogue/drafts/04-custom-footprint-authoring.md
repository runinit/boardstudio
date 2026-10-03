# Draft: Create and edit a custom component definition

**Parent:** F4.2 — Create, import and edit footprint definitions (`.scratch/dioxus-frontend-v1/issues/04-parts.md`).

**What to build:** From Parts, a designer can immediately create the reference's default custom component and then edit its supported footprint geometry and electrical pad details through the same per-action edit behavior as React.

**Start gate:** The accepted current source already exposes the needed feature-level entry points: Parts can create and select an empty custom definition through one normal accepted edit, and its selected project definition can commit a scoped Name edit through the existing accepted edit/history path. The authoring slice can reuse these proven capabilities to add the remaining fields; it does not need a new host callback or a complete F4.1 catalogue. The parent F4.2 start edge and INT.2 acceptance join stay unchanged.

**Blocked by:** None among the local F4 child tickets. Reuse the existing New custom component and scoped Definition Name capabilities; their presence is an implementation start proof, not a claim that their full paired-browser acceptance or the F4.1 parent is complete. No separate Rust service or F3 placement behavior is needed for custom definition authoring.

**Status:** source-refined; independent Spec and Standards re-review pending before dispatch.

- [ ] The reference Create action immediately inserts and selects the same default custom definition; do not require a separate Save/Apply action before it exists in the document.
- [ ] Edit the reference-visible supported fields (name, Definition kind, courtyard, pad identity/number/shape/dimensions/position and drills); the kind control offers the reference's Switch/Controller/Connector/Encoder/Passive/Custom values. Do not add a per-pad rotation control that React does not expose. Preserve DraftInput local text state, Escape reset, Enter-to-blur, validation and commit-on-blur/action boundaries. Name remains a raw-string commit as in React; its “Name is required” definition issue is displayed but is not silently converted into a new name-trimming or commit-rejection rule.
- [ ] Each accepted reference action creates its own normal edit/Undo step; do not batch the full form into a new whole-form transaction. Verify selection/details, save/reopen, archive round-trip and undo/redo at those boundaries.
- [ ] Validate only the rules applicable to authored definitions: unique component/pad IDs, unique nonempty authored electrical pad numbers, positive finite courtyard/pad dimensions and drills, and finite pad coordinates. Invalid courtyard or pad commits stay recoverable, show the reference error/issue, and leave the prior accepted field unchanged; preserve the reference Name behavior described above.
- [ ] With a fixture containing multiple placed instances of the edited definition, prove pad-ID rename remaps each matching instance's pin references from the old pad ID to the new ID, and pad removal removes only those instances' references to the removed pad. Preserve unrelated pins, nets, definitions, assets, and placements; these coupled changes are required definition-edit behavior, not unrelated project mutation.
- [ ] Keep source-owned imported KiCad pads read-only here; KiCad import and model-file asset attachment belong to separate children of F4.2. Do not add placement controls: F3 owns placement.
- [ ] Run the same create/edit/invalid-input/save/Undo/reopen journey in pinned React and Dioxus with desktop and compact captures, keyboard/focus checks, accepted document/revision and durability-state comparison, and no page errors. Use a separate accepted fixture with placed instances and nets for pad-ID remap/removal; prove unrelated data survives.
- [ ] Record a refactoring handoff under existing RF-001/RF-002/RF-006/RF-009 as applicable, or explicitly state no new takeaway was observed; inherited independent Spec/Standards and shared acceptance gates remain open.

**Completion joins:** Preserve F4.2's INT.2 parent join and the F4/F9 shared browser, build, accessibility and RF gates. This child does not close F4.1, F4.2, INT.2, F4 or F9.
