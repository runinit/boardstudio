# F7.2 Authored Case workspace

This automatically published planning ticket refines F7.2 in the canonical 62-parent graph. It starts after accepted INT.1 and has no additional acceptance joins. Shared authority and acceptance remain at `../dioxus-frontend-tranche-1/AUTHORITY.md` and `../dioxus-frontend-tranche-1/ACCEPTANCE.md`.

Before dispatch, attach the private page-local mount, accepted document/selected-board/session scope, existing `SetCase` edit path, callbacks, fixtures, and evidence locations in [DISPATCH.md](DISPATCH.md). F7.2 does not wait for F7.1, F5, F3, F4, F6, or F7.4; it preserves the generated/mismatch presentation join while leaving configuration and disable behavior to F7.4.

The ticket is in [issues/01-authored-case-workspace.md](issues/01-authored-case-workspace.md). Source notes and initial/final independent Astra planning reviews are retained in `evidence/`.

The exact-parity follow-up is implementing as [09: Case assembly Objects tree and contextual Inspector](issues/09-case-contextual-objects-inspector.md) against its independently reviewed [private contract](evidence/contextual-workspace-issue09/implementation-contract-reviewed.md). It reuses the published body editor and F7.4 settings controller; root retains shared dispatcher registration and mount wiring. Parent F7.2/F7.4 acceptance remains open.
