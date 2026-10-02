# Spec review: library document-sharing correction

**Pass for this exact private correction; no new Spec findings.** Reviewed the dirty `web/src/presentation/library.rs` diff against `2030c9a3` (16 insertions, 10 deletions), T1-02 and the previously reviewed corrected library contract. No source edits, builds or browser execution.

The change preserves T1-02’s “current keyboard once, followed by saved keyboards in reference name order.” It reuses the accepted `Arc<ProjectDoc>`, sorts freshly loaded owned documents before wrapping them once, borrows the saved-list signal during card assembly, and clones only shared document handles. Current-ID filtering, card keys, original nonblank names, locale sorting, summaries, geometry and fallback projection remain identical. `KeyboardCard` reads through the Arc and still opens the same captured saved ID through the existing Runtime action.

“Late list responses cannot overwrite newer project/list state” retains the same generation, mounted-lifetime and accepted-ID checks for success and error. Current accepted snapshots remain independently refreshed by the shared version subscription; accepted-ID changes continue to drive listing. Saved-list borrows end before rendering and introduce no document writes or competing state owner. Recovery labels, loading/failed/retry/prior-card retention, entry/menu reuse and menu-close behavior are untouched. No public API, visibility, persistence or domain contract changes occur.

Strict WASM Clippy passing was reported by the coordinator; this review did not rerun it and does not imply browser acceptance. Full T1-02 remains open for malformed stored-record isolation and post-submit open supersession. Both original panel findings remain open and outside this correction.

No new refactoring takeaway observed; this addresses the reviewed document-copy issue without changing the behavioral contract.

Reviewed file SHA-256: `6f36b1ff0c6a3a3f27fc9aa480598be2f89bcd529015a578d45fe0cb33c15a5b`.
