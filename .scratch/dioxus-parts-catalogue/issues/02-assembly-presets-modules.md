# 02: Choose bundled key assemblies and VIK module variants

**What to build:** Parts includes the reference’s bundled key assembly presets and source-backed VIK module catalogue. A designer can select a preset or module row, switch among its variants, and inspect the selected item’s existing details without editing it.

**Blocked by:** 01: component-catalogue (the Parts query, result selection, and selected-detail surface).

**Status:** ready-for-agent

- [ ] Show the eight existing `matrixPresetDefinitions` entries with their reference names and behavior; do not limit this area to saved project assemblies.
- [ ] Load bundled module entries from the existing imported module catalogue, merge project snapshots by ID with project precedence, group by reviewed catalogue row, and provide the reference variant selector.
- [ ] Selected preset/module details identify the actual definition/source and available readiness data; no new catalogue content or readiness/fabrication claims are introduced.
- [ ] Preserve reference loading/error/no-match behavior, including its existing re-entry retry lifecycle; do not add a Retry button or ordinary-definition loading/error UI absent from the reference.
- [ ] Query and selection remain read-only. Paired public browser evidence covers all bundled presets, a duplicate-name/multi-variant module row, project override precedence, loading/error and no-match; record RF observations.

- [ ] Complete inherited shared acceptance, independent Standards/Spec review and RF handoff; parent acceptance/joins remain open.
