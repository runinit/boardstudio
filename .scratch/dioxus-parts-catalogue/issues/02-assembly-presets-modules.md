# 02: Choose bundled key assemblies

**Current first-release scope:** the active issue covers bundled key assembly presets. VIK module catalogue rows, variants, previews and new placement are deferred; saved module data remains reopenable for compatibility. Existing VIK evidence is retained below and in its evidence files, with no dispatch.

**Blocked by:** 01: component-catalogue (the Parts query, result selection, and selected-detail surface).

**Status:** active for bundled key assembly presets; VIK work is historical/deferred.

## Bounded current slice

The eight existing `matrixPresetDefinitions` rows are mounted in the Parts Objects panel. Selecting a row selects its lead switch definition and hands the stable preset ID to Issue 03 for read-only companion composition. Names and member behavior follow `app/src/ui/assemblyCatalog.ts` and `app/src/ui/assemblyPresets.ts`, including project reversible-layout construction. Placing an assembly, editing a recipe, and full Issue 02/parent acceptance remain separate criteria.

Historical VIK slice record: the prior row/variant browser proposal and its evidence are deferred from first release. Do not dispatch or treat its unfinished criteria as active; retain the existing evidence files for future work.

- [ ] Show and select the eight existing `matrixPresetDefinitions` entries with their reference names and behavior; selecting one chooses its lead switch and activates its source-backed recipe preview. Do not limit this area to saved project assemblies.
- [ ] Selected preset details identify the actual definition/source and available readiness data; no new catalogue content or readiness/fabrication claims are introduced.
- [ ] Preserve reference loading/error/no-match behavior, including its existing re-entry retry lifecycle; do not add a Retry button or ordinary-definition loading/error UI absent from the reference.
- [ ] Query and selection remain read-only. Paired public browser evidence covers all bundled presets; record RF observations.

- [ ] Complete inherited shared acceptance, independent Standards/Spec review and RF handoff; parent acceptance/joins remain open.
