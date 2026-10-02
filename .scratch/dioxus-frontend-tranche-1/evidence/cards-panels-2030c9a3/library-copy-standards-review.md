# Library copy correction: independent Standards verification

Reviewed the dirty `web/src/presentation/library.rs` correction against fixed candidate `2030c9a3` in the migration integration worktree. Source-only review; no application edits or build/browser execution. `git diff --check -- web/src/presentation/library.rs` passed. Root reports strict WASM Clippy passed; that check was not independently rerun here.

**The deep-document-copy finding is resolved by the reviewed correction.**

- `KeyboardCard` receives `Arc<ProjectDoc>` (`128`) rather than an owned document.
- The accepted snapshot's existing Arc is cloned (`217–221`); its underlying ProjectDoc is not copied.
- Saved state is `Vec<Arc<ProjectDoc>>` (`223`). Successfully listed, already-owned documents are sorted and moved into individual Arcs once (`263–265`).
- Card construction borrows `saved.read()` and clones Arc handles (`273–289`); it no longer clones the saved vector's underlying documents on Runtime repaints. The current card also clones only an Arc handle.

Name sorting, preview projection, open IDs, request guards and read-only document semantics are preserved in this focused diff. No new public API or writable document owner appears. This satisfies the specific CONSTRAINTS.md Rust/Dioxus unnecessary-document-copy concern previously reported.

This source conclusion does not prove repaint allocation or latency budgets: card-vector construction, string/preview work and framework prop comparisons may still have cost. Future allocation measurement remains unperformed. No new refactoring takeaway beyond the previously recorded immutable presentation ownership observation.

Panel responsive findings and full panel acceptance remain open. T1-02's open-supersession and malformed-record listing joins remain open. This review closes only the specific source copy finding, not either complete ticket or its integrated acceptance gates.

Reviewed file identity: Git blob `edd674ea829a94ae70e19e076f7fa5c28ad0086c`; SHA-256 `6f36b1ff0c6a3a3f27fc9aa480598be2f89bcd529015a578d45fe0cb33c15a5b`.
