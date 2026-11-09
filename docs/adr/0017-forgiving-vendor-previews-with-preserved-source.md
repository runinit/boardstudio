---
status: accepted
---

# Allow forgiving vendor previews while preserving the original

Vendor STEP import should prioritize useful display over requiring mathematical repair certification: show a repaired preview with explicit warnings when its geometric error cannot be certified, retain the untouched original file separately, and keep collision accuracy unknown wherever evidence is insufficient. The user chose this trade-off on 2026-10-19 because the strict importer had become too complicated. A successful preview is not proof of exact geometry, fabrication readiness or a collision-clearance guarantee.

If some geometry cannot be imported or repaired, show the remaining geometry with an explicit incomplete-model error rather than refusing the entire preview. Missing geometry must not be silent, and the partial preview cannot establish clearance for the complete model; that conclusion remains unknown. The user confirmed this partial-display policy on 2026-10-19.

This is a new policy, not a reconstruction or an implementation-completion claim. It replaces certification and completeness as prerequisites for preview admission in the earlier [strict-import plan](../plans/cad-backend-replacement/issues/007-strict-step-import.md#closure-rules-shared-by-every-package). It does not remove input/resource safety checks, authorize silently discarding geometry, or change [ADR-0015](0015-exact-generated-cad-and-mesh-preview-scope.md)'s exact generated case/PCB output boundary.
