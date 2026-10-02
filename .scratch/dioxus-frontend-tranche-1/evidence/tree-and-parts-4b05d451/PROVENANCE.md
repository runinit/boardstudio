# Tree and Parts evidence provenance

This folder preserves bounded evidence for the tree projection, the later hierarchy repair, and the Parts catalogue correction. `SHA256SUMS` covers every copied artifact except itself. No build targets, binaries, browser profiles, or production source files are included.

## Source checkpoints

- **8ea71a84 projection baseline:** the private tree native harness includes the exact `web/src/presentation/objects/tree.rs` snapshot with SHA-256 `cabf98db27bea8e52f562268f017c7ed4185e4a1bbdd6d59710e32649c2510ec`. That five-test harness opens actual REVIUNG41/Sofle ProjectDocs through Session and Core, then checks the private projection/resolver. It ran from worktree HEAD `96811f92`, where this file's bytes still matched 8ea. It does not test the later hierarchy edits.
- **515f390d hierarchy integration:** `objects/tree.rs` now has SHA-256 `84cb7cbae6d37c24d90ffbd26db745b646197cdb0adf57a33f90cd3557563ea0`. The hierarchy repair is the b51353bb change carried in 515f390d: initialize disclosure, render owned matrices under layout groups, and retain owned/unowned components at the intended levels. The exact source snapshot is included as `native/tree-contract-tests/tree.rs.source-515f390d`. The paired browser red/reference/green JSON and the `tree-515f390d-*` snapshots record public hierarchy behavior. The worktree `parts-f41a` still has the same tree source hash. The final independent Spec review clears those three bounded hierarchy findings and explicitly leaves full T1-10 open.
- **4b05d451 Parts checkpoint:** `objects/tree.rs` remains byte-identical to 515f390d. `parts/catalogue.rs` is SHA-256 `1ba3b7088ac9d613812cd389e4bc953d97b42a1c4dffb7e01cd939613900cb46`; it replaces the invalid digest `LowerHex` formatting from the 515 source with per-byte lowercase hex and adds an exact imported-data digest test. The imported JSON is unchanged at SHA-256 `000f4ba13114305c33e1378806c25840d903fa335da559b88c9d9404731acd7f`.

## Executed proof and limits

- The tree native result is five passing tests against the 8ea projection/resolver snapshot, using accepted `SceneDelta` values produced by the real Core engine and ReadModels accepted by Session. The actual fixture JSON inputs and harness snapshots are included under `native/tree-contract-tests/`.
- The initial Parts harness used the exact 515 catalogue source/data. Compilation stopped before tests because that source formatted sha2 0.11.0's hybrid-array digest with `{:x}`. That failure is retained in `native/parts-contract-tests/RESULTS.md`.
- The corrected 4b05 source and exact imported JSON passed all eight embedded native tests. One decodes all nine imported static definitions as the existing Core `PartDefinition` type. Tests also cover source digest equality, merge precedence/order, `Rc` identity, category/search aliases, parameter JSON shape, line terminators, and catalogue filtering.
- Parts tests include the complete catalogue module in its private module path. The harness has a clearly labeled fail-closed `runtime::resource_url` stub only to satisfy compilation; no test calls it. No browser URL, JS dynamic import/provider, loader cache, offline mode, or browser `serde-wasm-bindgen` path is proven here.
- The hierarchy browser captures compare public controls and accepted fixture documents, as described in the retained diagnosis. These are specific to the captured artifacts and do not establish every T1-10 acceptance item, keyboard selection parity, or loader behavior. Review reports retain their own scopes and caveats.

`native/*/Cargo.lock` and harness sources are copied for audit. Their original commands and source paths are recorded in the adjacent `RESULTS.md`; compiled targets were deliberately excluded.
