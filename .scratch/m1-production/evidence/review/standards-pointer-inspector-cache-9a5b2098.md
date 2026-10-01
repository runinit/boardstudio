# Standards review: Inspector keyboard item cache

- Base: `9a5b20985a9db1f4c0f0d635c43e1dd5439bbfd`
- Candidate: `e7d29ce6dd703a4d9b064cb842366538987cf442`
- Reviewed production path: `web/src/presentation.rs`
- Exact production diff SHA-256 (`git diff --binary BASE...CANDIDATE -- web/src/presentation.rs`): `06587ec484b38965efc0880fca493d3de1b4f24d5bf77ca41f3867946c785ed7`

## Findings

No findings.

The patch retains the existing filtered, ordered list of active-board components and wraps it in `Rc<Vec<_>>`. Rendering iterates with `.iter().cloned()`, and each keyboard callback clones the `Rc` rather than cloning the full vector. Arrow, Home, End, selection, and focus behavior still index the same list and use the same item IDs. The change narrows repeated allocation while preserving callback ownership and list order; it does not alter public API or domain behavior.

This is a source-only Standards review of the production diff. I did not rerun the provider's checks or the focused timing run.
