# F7.1 Shared viewer contract and renderer feasibility

This automatically published planning ticket refines F7.1 in the canonical 62-parent graph. F7.1 has no start or acceptance blockers. Shared authority and acceptance remain at `../dioxus-frontend-tranche-1/AUTHORITY.md` and `../dioxus-frontend-tranche-1/ACCEPTANCE.md`.

Before dispatch, prove the private page-side renderer contract and crate call path in [DISPATCH.md](DISPATCH.md). This ticket establishes the contract and feasibility decision for F7.3; it does not claim to implement the shared viewer. F7.3 retains its canonical `start_after: [F7.1]` and `acceptance_after: [INT.2, BND.1]`; those acceptance joins do not block fixture-backed F7.3 implementation.

The ticket is in [issues/01-common-viewer-contract.md](issues/01-common-viewer-contract.md). Source notes and initial/final independent Astra planning reviews are retained in `evidence/`.
