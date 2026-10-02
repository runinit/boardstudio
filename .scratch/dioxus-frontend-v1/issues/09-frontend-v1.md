# F9: Frontend v1 qualification and React retirement

**Status:** planned
**Blocked by:** F1–F8; applicable carried gates
**Category:** frontend behavior-preserving port of the pinned React reference.
**Reference:** React 5a472a94; Rust base f0ac0a19.
**Authority:** [parent spec](../spec.md), [roadmap](../../../docs/migration/DIOXUS-FRONTEND-V1.md).

## Work

Complete TSX/hook/style/theme inventory, transferred UI test coverage, desktop/compact/theme/AT/performance/error matrix, review and rollback-ready entrypoint switch.

## Exit evidence

No required placeholder or React UI island; every inventory row disposed and workflow verified; explicit approval before production entrypoint/data-writer cutover.

- [ ] Port all scoped TSX/UI-hook behavior through existing service contracts.
- [ ] Match reference visuals, themes, responsive and interaction states.
- [ ] Retain affected checks/browser evidence and resolve independent reviews.
- [ ] Update inventory, integrated source, demo and remaining limits.

A visible placeholder does not complete F2–F9. Preserve original projects and
reference source. Backend implementation/format/API changes require separate
scope; never silently remove a frontend behavior when an adapter is missing.

## Reference ownership

The [file ledger](../evidence/tsx-inventory.json) pins exact reference hashes. These
are source responsibilities, not permission to mark whole shared files complete.

- All 63 production TSX entries and the 18 stylesheets in the inventory; all UI hooks/controllers and fonts/assets

Existing regression coverage to transfer or retain:

- All 15 TSX test entries and the workbench benchmark, with explicit retained/replaced coverage dispositions
