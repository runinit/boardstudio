# Standards review: pointer range-cache patch

- Baseline: `a3172ee64f2fcea0bda7795ed2c8654947e17808` (`HEAD` when reviewed).
- Candidate: working-tree-only production diff in `web/src/presentation.rs` (three insertions, two deletions); no implementation commit was present at review time.
- Exact production diff hash: `3c23452f508d48ef4cfd7c89488204bce831b5afcfa008bafb5447a92a673cc4` (`git diff --binary a3172ee6 -- web/src/presentation.rs`).
- Standards inspected: `docs/agents/domain.md`, `docs/adr/0003-rust-application-ownership.md` (especially the Dioxus presentation lifecycle guidance); no separate coding-style document was present in this checkout.

## Findings

No documented-standard violations or actionable smell-baseline findings.

The patch stores the already-computed visible part IDs behind `Rc<Vec<String>>`, so each rendered part closure clones the shared pointer instead of cloning the full ID vector. The Shift-range path clones the underlying vector only when it constructs the existing `SelectParts` event. Non-range selection still sends an empty vector. This keeps ownership inside the presentation boundary, preserves the event payload and ordering, and does not add a speculative abstraction or broaden an API. Existing `Rc` usage in this module and the ADR's presentation ownership guidance are consistent with the change.

No additional checks were run for this review; source/build/test claims should be taken from the candidate's recorded gates, not inferred from this standards pass.
