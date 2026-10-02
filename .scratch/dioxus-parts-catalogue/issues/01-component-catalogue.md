# 01: Browse and search component footprint entries

**What to build:** The Parts workspace shows the reference component catalogue in its established categories. A designer can search and select a real definition and inspect its selected details without changing the project.

**Blocked by:** INT.1 is the parent start edge. Before implementing, the coordinator must prove and own the actual Parts mount, immutable inputs, scope and callback path in the page crate; INT.1 alone does not provide a Parts contract.

**Status:** ready-for-agent

- [ ] Build the catalogue from the existing Ergogen catalogue, imported static definitions, and current project overrides with documented precedence; do not list only definitions in the active project.
- [ ] Preserve the reference categories, preferred labels, aliases, assembly-snapshot exclusion and legacy `infused-kim/nice_nano_pretty` exclusion while retaining a currently assigned choice where the reference does.
- [ ] Search names, source/generator terms, category and supported aliases; selecting by pointer or keyboard opens the selected definition’s meaningful existing details, not only a heading.
- [ ] Query/selection/detail viewing is read-only and leaves project revision/history unchanged; empty and no-match feedback match the reference.
- [ ] Compare public Dioxus browser behavior with the pinned React source using representative bundled, imported and project-override entries; record RF observations.

- [ ] Complete inherited shared acceptance, independent Standards/Spec review and RF handoff; parent acceptance/joins remain open.
