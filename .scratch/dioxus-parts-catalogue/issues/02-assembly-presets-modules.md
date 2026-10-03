# 02: Choose bundled key assemblies and VIK module variants

**What to build:** Parts includes the reference’s bundled key assembly presets and source-backed VIK module catalogue. A designer can select a preset or module row, switch among its variants, and inspect the selected item’s existing details without editing it.

**Blocked by:** 01: component-catalogue (the Parts query, result selection, and selected-detail surface).

**Status:** implementing the bundled VIK module row/variant browser; parent acceptance remains open.

## Bounded current slice

The eight existing `matrixPresetDefinitions` rows are mounted in the Parts Objects panel. Selecting a row selects its lead switch definition and hands the stable preset ID to Issue 03 for read-only companion composition. Names and member behavior follow `app/src/ui/assemblyCatalog.ts` and `app/src/ui/assemblyPresets.ts`, including project reversible-layout construction. The current bounded slice adds source-backed VIK row/variant browsing and read-only provenance/readiness details. Neither slice implements VIK placement or editing, placing an assembly, editing a recipe, or full Issue 02/parent acceptance.

## Current bounded slice: VIK module rows and variants

Use the existing `app/src/modules/imported-modules.json` source, not hand-authored module metadata. Load it lazily while Parts is mounted, retain successful package data for re-entry, and allow a failed load to retry after leaving and re-entering Parts. Merge the accepted document's `moduleDefinitions` by stable ID with project precedence, preserving the bundled row identity for a matching override. Group by that reviewed row identity (falling back to module ID only for project-only snapshots), filter the actual name/row/family/variant fields with the shared Parts query, and select one exact module snapshot. The Inspector exposes only the existing variant, pinned source, provenance and hardware-gate summaries; it does not edit the module or claim placement, wiring, fabrication, or model alignment.

- [ ] Show source-backed VIK module rows grouped by `catalogueRow`/reviewed row, including duplicate-name disambiguation and exact variant count; selecting a row selects its first source snapshot.
- [ ] Switch among only the selected row's stable module identities and display that snapshot's source URL, license/revision and recorded readiness gates.
- [ ] Project snapshots override bundled snapshots by ID without changing row membership; project-only snapshots use their existing row metadata or ID fallback.
- [ ] Loading, failure, re-entry retry and no-match states remain scoped to the mounted Parts catalogue. Query and selection remain read-only.
- [ ] Pin one actual React row/variant journey and replay only the changed behavior on the packaged Dioxus candidate. This bounded evidence does not cover every module row, override or lifecycle failure.
- [ ] Preserve RF-001/RF-009 observations and the inherited shared acceptance/review gates; parent acceptance remains open.

- [ ] Show and select the eight existing `matrixPresetDefinitions` entries with their reference names and behavior; selecting one chooses its lead switch and activates its source-backed recipe preview. Do not limit this area to saved project assemblies.
- [ ] Load bundled module entries from the existing imported module catalogue, merge project snapshots by ID with project precedence, group by reviewed catalogue row, and provide the reference variant selector.
- [ ] Selected preset/module details identify the actual definition/source and available readiness data; no new catalogue content or readiness/fabrication claims are introduced.
- [ ] Preserve reference loading/error/no-match behavior, including its existing re-entry retry lifecycle; do not add a Retry button or ordinary-definition loading/error UI absent from the reference.
- [ ] Query and selection remain read-only. Paired public browser evidence covers all bundled presets, a duplicate-name/multi-variant module row, project override precedence, loading/error and no-match; record RF observations.

- [ ] Complete inherited shared acceptance, independent Standards/Spec review and RF handoff; parent acceptance/joins remain open. This bounded slice leaves VIK module variants and the remaining Issue 02 criteria unchecked.
