# 15: Inspect all mounted-module PCB layers

**Parent:** F5.4; refines PCB02's remaining module layer criteria.

**What to build:** Show and independently toggle source module outlines,
clearance/service geometry, mounting holes, selected supports, front/back artwork
and module error regions from the TypeScript PCB Layers menu.

**Blocked by:** None for this bounded start. PCB14's accepted selected-board source
scene and transient module visibility owner are mounted and paired-proven on34748.
PCB14's module-free/board-retention/owning-navigation criteria and full F5.4 joins
remain acceptance work and are not silently closed.

**Status:** bounded public geometry/control packet verified; remaining finding visibility and full joins open

**Spec:** [module layer inventory](../drafts/F5.4b-mounted-module-layer-inventory.md).

- [x] Ten module labels/order/defaults match TypeScript, including on empty categories.
- [ ] Real source board, hole/support, clearance and artwork geometry uses accepted host pose/side once; missing data stays empty.
- [x] Front/back silk and fabrication controls independently affect the same source artwork as TypeScript.
- [ ] Module findings visibility filters current-host module error marker regions without changing other geometry or validation data.
- [x] Workspace navigation retains module visibility and toggles preserve revision/history/saved ProjectDoc.
- [ ] One affected compile/check, changed paired browser receipt and consolidated candidate review are retained with honest limits and RF handoff.

Owning-module navigation, shared finding focus and complete parent visual/user
journey criteria remain open. No source presence or build is parent acceptance.

Public evidence: [paired receipt](../evidence/15-mounted-module-layers-20261003/public-receipt.md). All435 source leaves and28 corresponding marker contours match. Default/toggle marker counts differ and remain explicitly open; nonempty clearance is not exercised by this fixture.

## Follow-up implementation boundary

Pinned `Workbench.tsx` renders every current-board error marker in PCB. Its
`moduleFindingIds` set is derived from findings whose `targetIds` intersect the
current host board's mounted-module instance IDs; turning off **Findings &
clearances** suppresses only those IDs. Other host error markers remain visible.
The current Dioxus marker component rendered only module-targeted errors and
returned no markers at all while that layer was hidden, which explains the
recorded `0→28` versus React's `47→72` counts without requiring an ID-string
filter. A focused finding is an additional React visibility override and remains
outside this bounded correction because PCB has no equivalent focused-finding
owner yet.

The 34769 paired comparison is retained in [the route and findings receipt](../evidence/24-module-source-route-20261003/receipt.md), with raw marker arrays and exact archive identity. All-visible unique marker IDs match (72 each). The captured default DOM differs by three `gate/source-caveat` IDs, and the all-visible raw DOM count differs by one instance per such ID. Direct inspection of React's accepted findings and current module IDs confirms the pinned visibility predicate classifies those three IDs as module findings, matching Dioxus; its JSX excludes all such marker IDs when the layer is hidden. React's browser also reports duplicate keys for these IDs, and the live DOM retained stale groups after the predicate excluded them. Treat the recorded count as an observed React rendering artifact, not expected behavior for Dioxus to reproduce. The changed layer-toggle result preserves the accepted project archive and revision. Module finding focus, every geometry criterion, changed candidate layer-toggle qualification, Issue15/F5.4/F5.5 and parent joins remain open.
