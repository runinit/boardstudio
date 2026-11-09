---
status: accepted
---

# Keep one document authority across editor lifetimes

This reconstructs the ownership decision approved by the user on 2026-10-01 in the removed ADR (`git show 323967ff3a67b9b62db4f201b477396d62b55ccd:docs/adr/0003-rust-application-ownership.md`). Its deletion in `3cdeb2ac2b2b3e11891d10122b03e6dc835e03dd` retired React and migration machinery; it did not record a reversal. The old trial and migration constraints are not restored. The user confirmed this reconstruction and its ownership and lifetime policy on 2026-10-19.

Keep Core as the only writable document/history authority. The headless application Session orders work and owns the accepted-snapshot read model and interaction state; Runtime composes Session with execution, persistence, and browser workflows; Host supplies browser storage, worker, and renderer integrations, while Runtime still retains browser-bound state and handles; presentation owns local draft, focus, workspace, and display state, not a second writable project document. This records ownership, not ADR-0005's edit-resolution policy or ADR-0006's replacement direction.

Async publication is guarded by the identity relevant to that operation, not a universal identity tuple. For example, an edit ticket captures application `Scope` (session epoch, document ID, board ID, optional instance) for observation, and permanently retires that observation after an observed owner/scope departure; this does not cancel the Session-owned operation. Core replies separately match request and executor epochs, and other jobs use their own captured identities. A successful reopen advances the Session epoch even when document ID and revision are reused; returning to a board does not revive an already-retired observation. Unmounting a panel drops its observation, not its operation; editor Close is separate, and the Session drains active work before closing.

The original ADR rejected coupling application lifetimes to a mounted Dioxus store and putting browser/application coordination inside Core. Later module-deepening choices kept this ownership boundary while making captured-Scope retirement and per-workspace presentation state explicit; they did not freeze exact crate edges or require further Host extraction.

Evidence: [current responsibility map](../architecture.md), [Core](../../core/src/lib.rs), [Session](../../application/src/session.rs), [Runtime](../../web/crates/runtime/src/runtime.rs), [edit-ticket implementation and regressions](../../web/crates/runtime/src/edit_ticket.rs), and the [module-deepening outcomes](../plans/module-deepening/map.md).

## Amendment: preview recovery respects automatic preview (2026-10-20)

A failed 3D preview shows a clear unavailable/error message while forms and 2D
editing remain usable. Recovery rebuilds the preview from the current accepted
design through its existing ownership checks; it must not reload the project,
replay edits or publish obsolete scene/pick results into a replacement owner.

The user confirmed recovery governed by the existing automatic-preview setting:
when automatic preview is enabled for the affected view, make one automatic
recovery attempt after failure; when disabled, wait for **Retry preview**. If that
recovery attempt fails, retain the visible error and require explicit Retry rather
than looping. Ordinary rerenders must not continually replenish the automatic retry.
Keep Retry available in the failed state and respect a paused automatic-preview mode.
This does not introduce a new global preference or require automatic generation
in a view that has none.

For successful recovery in the same project and view, restore the last usable
viewpoint when available; otherwise fit an overview of the current accepted design.
The user confirmed this fallback on 2026-10-20. Ordinary workspace departure and
return within the same editor session also restore the prior usable viewpoint for
the same project, board/physical instance and view, with fitted overview as fallback.
View state from another scope must not be borrowed, and restoring a viewpoint must
not restore obsolete geometry or override a deliberate Fit/navigation action.
The user chose session-only retention: no durable viewpoint persistence is required
across project reopening or browser restarts. A new editor session may start with a
fitted overview; do not add saved-document fields or a cross-session preference
merely to retain the viewpoint.

Current [renderer lifecycle handling](../../web/crates/host/src/renderer_host.rs),
`install_lifecycle`, fail-stops on context loss/render failure; the Case
[Live preview control](../../web/crates/case/src/cad_presentation.rs), `CasePanel`,
currently controls generation. Those facts do not establish implementation of the
accepted renderer recovery policy or genuine GPU-loss acceptance.
