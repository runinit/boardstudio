# Consolidated Parts assembly search review — 2026-10-03

Verdict: the filter/count source, corrected aggregate feedback and supplied focused test/package gates pass. The final browser replay passes for the changed search behavior. No remaining actionable source finding was identified. F4.1-C02's broader loading/source-error requirements remain unqualified; this batch accepts no parent.

## Scope and source

Reviewed the `parts.rs` application change at `fa275d839ad4b1ddad182895b185f9cf76e0c18e` and the feedback correction through final source `b2bb813cdb75340a2725ac4488dfde6752104484`. This is one consolidated review of that candidate batch. The reviewer performed targeted reads only, ran no builds/tests/browser actions, and wrote only this review. The finalized attachment/BND review remains unchanged at SHA-256 `283af977b4ffb4242fe28c686571072faadb6185036cc59cea1cedb320baabcc`.

`matching_assembly_presets` applies the existing trimmed, case-insensitive name/category search to the eight presets. Its results now supply the rendered list, displayed count and aggregate no-match predicate. Empty query retains all eight in their existing order; filtering does not mutate the selected assembly, definition, project or placement callbacks.

The initial public replay found another rendered feedback defect: the component-only `groups.is_empty()` branch displayed the aggregate `No parts match this search.` message while assemblies matched, and duplicated it for a true no-match query. The reviewer initially missed that adjacent branch; the browser RED established the gap. The final correction removes that local message and retains the existing aggregate `no_matches` branch as the sole query no-match status. Populated component groups and actual catalogue-empty/error/loading behavior retain their existing branches. No public API, wire or saved-document change is added.

## Supplied verification

The focused WASM Chrome regression `key_assembly_search_hides_nonmatching_presets_without_losing_categories` failed before the filter fix at `none.is_empty()` for the **nonmatching** `no-such-assembly` query, then one test passed. Empty query already returned all eight before and after the change. The test also checks default ordering, the four MX matches and all eight category-query matches. It verifies the filter, while the retained browser RED/GREEN verifies rendered message behavior that the pure helper test does not cover. Logs: `/home/chris/.local/share/boardstudio/retained-tmp/20261003/assembly-search-{red,green}.log`. No broader suite was rerun by this review.

| Candidate | Exact source | Package result |
| --- | --- | --- |
| `frontend-parts-assembly-search-20261003`, port 34783 | `fa275d839ad4b1ddad182895b185f9cf76e0c18e` | 135.957746 seconds; correct filtering, rendered feedback RED |
| `frontend-parts-search-feedback-20261003`, port 34784 | `b2bb813cdb75340a2725ac4488dfde6752104484` | 135.329291 seconds; corrected feedback GREEN |

The [initial package proof](../frontend-parts-assembly-search-20261003/package-proof.json) and [final package proof](../frontend-parts-search-feedback-20261003/package-proof.json) each identify their source commit and report 8 fresh / 22 inherited commands, zero source/asset mismatches, HTTP 200 with isolation headers at root and `/boardstudio/`, and zero release warnings. Compiler/package/served checks were supplied by the root; package identity is separate from functional qualification.

## Browser verdict and limits

The [existing appended journey](../functional-delivery-20261003/journey.md) retains the paired pinned TS 5175 baseline and both candidate replays on the public VIK review fixture. On 34783, empty query showed eight assemblies, `MX` showed the same four choices as TS, and `key assembly` showed eight. The feedback mismatch described above remained. Selecting MX Hotswap, hiding its row with a true no-match query, then clearing the query retained the Inspector selection and restored the selected row. No assembly was placed or project data changed.

On final 34784, `key assembly` shows eight options and **zero exact no-match statuses**; `MX` shows four; the true no-match query shows zero assembly options and **exactly one** no-match status. The receipt distinguishes that status from the unrelated saved-state live region. The unchanged selection behavior reuses the 34783 public selection-retention proof. Screenshots and hashes remain in the existing journey; owned sessions were closed.

The changed query/filter/count/feedback behavior is qualified. The catalogue was already loaded, and no source/network failure was induced. Thus loading transitions and source-error behavior do not become verified by this replay, and F4.1-C02 remains broader than this successful search slice. Existing parent gates, accepted history and unrelated criteria are preserved. No additional source defect or refactoring requirement is established by this bounded change.
