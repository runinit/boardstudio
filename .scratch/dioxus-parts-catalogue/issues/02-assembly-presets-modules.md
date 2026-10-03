# 02: Choose bundled key assemblies and VIK module variants

**What to build:** Parts includes the reference’s bundled key assembly presets and source-backed VIK module catalogue. A designer can select a preset or module row, switch among its variants, and inspect the selected item’s existing details without editing it.

**Blocked by:** 01: component-catalogue (the Parts query, result selection, and selected-detail surface).

**Status:** implementing bundled key-assembly selection; VIK module variants remain open.

## Bounded current slice

Mount the eight existing `matrixPresetDefinitions` rows in the Parts Objects panel. Selecting a row selects its lead switch definition and hands the stable preset ID to Issue 03 for read-only companion composition. Names and member behavior follow `app/src/ui/assemblyCatalog.ts` and `app/src/ui/assemblyPresets.ts`, including project reversible-layout construction. This does not implement VIK module browsing, placing an assembly, editing a recipe, or full Issue 02/parent acceptance.

- [ ] Show and select the eight existing `matrixPresetDefinitions` entries with their reference names and behavior; selecting one chooses its lead switch and activates its source-backed recipe preview. Do not limit this area to saved project assemblies.
- [ ] Load bundled module entries from the existing imported module catalogue, merge project snapshots by ID with project precedence, group by reviewed catalogue row, and provide the reference variant selector.
- [ ] Selected preset/module details identify the actual definition/source and available readiness data; no new catalogue content or readiness/fabrication claims are introduced.
- [ ] Preserve reference loading/error/no-match behavior, including its existing re-entry retry lifecycle; do not add a Retry button or ordinary-definition loading/error UI absent from the reference.
- [ ] Query and selection remain read-only. Paired public browser evidence covers all bundled presets, a duplicate-name/multi-variant module row, project override precedence, loading/error and no-match; record RF observations.

- [ ] Complete inherited shared acceptance, independent Standards/Spec review and RF handoff; parent acceptance/joins remain open. This bounded slice leaves VIK module variants and the remaining Issue 02 criteria unchecked.
