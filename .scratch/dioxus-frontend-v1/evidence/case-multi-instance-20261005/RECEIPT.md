# Same-board physical instances — 2026-10-05

Published candidate frontend-case-export-20261005/eac47f42 at34822 and pinned reference5a472a9 at5175. Fixture provenance is in input-provenance.json; a third instance right-alternate clones Right on the same board, with its own ID and openingAllowance0.2. This is an imported qualification fixture, not proof of a public create-instance action.

Both public Open project actions accepted it and showed Left, Right and Right alternate case assembly rows. Selecting Right alternate reached generated geometry. The visible common construction form displayed0.1 in both (the original Right value), not its retained per-instance0.2; the Inspector explicitly says mechanical settings/closure hardware apply to all assemblies. Do not silently call this distinct-instance form projection correct.

While Right alternate was selected, changed radial opening allowance to0.25, Enter; Undo displayed0.1, Redo0.25 in both. Both saved revision21 (input18). Actual saved archives agree: Left allowance absent, Right remains0.1, only Right alternate changes0.2→0.25. Other instance identity/board/closure data is retained. archive-comparison.json contains exact files/hashes/values. Selecting ordinary Right afterwards still displayed0.25 in both; this shared display behavior is recorded, not attributed as a Dioxus-only defect.

Candidate geometry was blocked by closure mount40 at allowance0.25. Reference generated four solids at revision21 but reported mechanical errors and blocked export. No valid exact-geometry parity is claimed for this edit. After browser reload both reopened the saved project and still offered all three assembly rows; candidate showed0.25 and Left mount44 error, reference resumed generation.

This supplies actual multi-instance public selection, edit/history, saved-scope retention and reload evidence. It does not prove per-instance form display correctness, display-preference persistence, or a pending async scope race. F7.7-C05 remains open for independent assessment of those limits.

Independent review in case-layout-repair-review-20261005.md clears F7.7-C05 for its original saved-settings/public multi-instance selection requirement. Display preferences, coherent form projections and delayed scope completion remain C04/parent obligations. No broader closure is claimed.
