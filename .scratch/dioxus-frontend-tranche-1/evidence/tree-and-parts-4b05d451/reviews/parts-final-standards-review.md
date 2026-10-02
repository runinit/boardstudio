## Standards follow-up: Parts immutable handles

Reviewed `7f9d5265...3db93b44` in `/home/chris/.local/share/boardstudio/worktrees/parts-f41a-20261002`, specifically the previously reported P2 copy finding and retained JavaScript data boundary. Checkout was clean at `3db93b44`. No application mutation, compiler, build or browser execution. `git diff --check 7f9d5265...3db93b44` passed.

**The specific P2 repeated-copy finding is resolved at source level.**

`CatalogEntry.definition` is now `Rc<PartDefinition>` (`catalogue.rs:21–23`). Consequently `bundled.to_vec()` during project overlay clones handles rather than complete pad/model/KiCad-source payloads. `parts.rs:120–128` likewise clones a small entry/handle for SelectedDefinition, whose local binding moves that Rc and reads the definition without another deep copy. The already memoized overlay materializes project definitions once per accepted-snapshot/bundle identity change; search/selection renders reuse those handles. Initial bundled-cache construction still makes owned copies once per cache entry, and project overrides copy from the accepted snapshot into immutable per-snapshot entries; these are distinct from the repeated full-bundle/detail copies previously flagged.

The added pointer-equality test checks unchanged bundled entries and selected handles retain the same definition allocation. It was inspected, not executed. This review does not establish allocation/latency budgets or eliminate every potential copy optimization. The prior nonblocking repeated-kind-metadata heuristic remains a later structural observation, not a new blocker.

**RF-012 boundary observation:** the retained correction in `catalogue.rs:284–300` serializes typed definitions with serde_json and JSON.parse before invoking normalizeDefinition. This provides plain JavaScript objects for generator.parameters, which retained JavaScript spreads as an object; serde-wasm-bindgen's default map representation is not equivalent. The source contract and correction are verified by inspection. Record this as a representation-boundary observation under existing RF-012, not a confirmed public application bug or public-green result until the actual packaged browser path executes.

No new public API, mutable document owner or Session path is introduced. Root composition/CSS, affected compiler checks, typed browser catalogue execution, root/subpath offline and accessibility/Spec acceptance remain pending. This follow-up closes only the source copy finding.
