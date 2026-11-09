---
status: accepted
---

# Keep export ownership separate from preview currency

An export owns captured accepted inputs, scope, editor-session identity and executor identity; disposable preview refreshes neither authorize nor revoke that ownership. Source, scope or executor changes invalidate stale output. An export's own authorized, accepted preparation commit may advance its capture without relaxing its strict origin checks; it must not adopt unrelated edits. Active work requires Session ownership, but final delivery checks the retained export capture and current source without requiring the already-settled operation to remain active. The user confirmed this reconstructed policy on 2026-10-19.

This preserves export lifetimes independently of preview scheduling while preventing stale delivery. One active-operation predicate cannot safely serve both phases: Session releases active ownership before delivery, and applying that predicate to delivery previously suppressed valid downloads. This extends [ADR-0005](0005-resolve-queued-edits-at-execution.md)'s strict export route and [ADR-0014](0014-document-authority-and-captured-lifetimes.md)'s operation-specific lifetimes, rather than replacing them.

Evidence: [lease capture, advancement and regression tests](../../web/crates/runtime/src/export_lease.rs), [Runtime active-work and delivery checks](../../web/crates/runtime/src/runtime.rs), and the [export-lease outcome and delivery correction](../plans/module-deepening/issues/10-export-leases.md#outcome), implemented in `28b9d81d2`, `3d432f520` and `11ac3786b`.
