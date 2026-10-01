# Spec review: pointer range-ID sharing at `a3172ee6`

**Result: no blocking Spec findings.** The only candidate change is in
`web/src/presentation.rs`: it wraps the visible-part ID vector in `Rc`, clones
the `Rc` into each rendered part handler, then materializes the existing
`Vec<String>` only when a Shift+pointer selection submits
`Event::SelectParts`.

This preserves the M1 selection and pointer contracts (`spec.md`, user stories
5–9) and the ticket 06 interaction/performance acceptance. The visible parts
are still filtered to the active board and kept in document order; their ID
values and order are unchanged. Range selection still sends the same ordered
IDs through the existing by-value event field, while Replace and Toggle still
send an empty range list. `Session` selection-anchor, valid-ID filtering and
range slicing are unchanged. The change removes per-part deep copies during
render; it retains one O(n) vector allocation per render and an O(n) value copy
when a range gesture actually needs the existing event payload.

The diff does not change pointer capture, drag/preview/commit behavior, hit
testing, selection history, public APIs, serialized data, or frozen budgets.
The existing `SelectionMode::Range` session coverage remains applicable.
This narrow review does not establish the required real-browser selection
parity or a performance gate pass; those remain subject to focused candidate
verification and the unchanged acceptance gates.

**Review basis:** compared the unstaged `web/src/presentation.rs` diff in
`codex/t06-pointer-range-cache-20261001` against `a3172ee6`; inspected the M1
specification, acceptance ledger, ticket 06 acceptance, migration constraints,
ADR 0003, and `Session` range-selection handling. `git diff --check a3172ee6`
passed. No build or timing run was performed during this review.
