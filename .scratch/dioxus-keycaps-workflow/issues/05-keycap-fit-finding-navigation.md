# 05: Navigate from Keycaps fit findings

**Parent:** F6C.4 — Fit resolution, finding list/navigation, and stale-case state.

**Outcome:** A target-bearing Keycaps fit finding exposes the React-equivalent action and routes through the existing accepted navigation owner to its exact current target.

**Specification:** [Keycaps fit finding navigation](../keycaps-fit-navigation-spec.md). **Source inventory:** [Keycaps workbench parity inventory](../evidence/keycaps-workbench-inventory-20261002.md).

**Start after:** the canonical F6C.2 start prerequisite for F6C.4, with accepted `ResolveKeycaps` result/presentation proven. The root's general finding callback is a known missing private capability to be implemented in coordinator-owned root composition; its absence does not require a public API or prevent the Keycaps-owned presentation work from starting. Coordinate root-owned wiring serially. All canonical parent gates remain open.

**Acceptance joins:** F6C.4 `INT.2`; unchanged.

## Acceptance criteria

- [ ] Action labels and visibility follow React `FindingList` target resolution: `Show outline` or `Select affected geometry`; unresolved/targetless findings remain readable without a dead action.
- [ ] Navigation reuses root accepted target resolution/selection and matches active board, workbench, selected geometry, camera fit, focused finding and Inspector focus for each supported target kind.
- [ ] Current finding/document identity is revalidated at click time. Deleted, replaced or stale targets do not navigate to another object; displayed stale/current/error state remains accurate.
- [ ] Navigation does not edit the document or create a history entry. Dedupe/grouping retains all target IDs required for action parity.
- [ ] Meaningful regressions cover target types, cross-board routing, removed targets, keyboard activation, stale result and no history mutation at the production composition seam.
- [ ] Paired pinned-React/Dioxus same-fixture browser journey records fixture/build/source hashes, route and selection, action labels, camera/Inspector behavior, stale target behavior, and no page errors. Full F6C.4 edit/Undo/Redo/save/reopen criteria remain required.
- [ ] Preserve RF-001/RF-005 carry-forward and the original F6C.4/INT.2 parent joins. The shared RF ledger and task graph remain coordinator-owned.

**Status:** planning packet prepared; independent planning review and source/browser acceptance remain open. No implementation or parent closure implied.
