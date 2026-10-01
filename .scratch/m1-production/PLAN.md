# M1 production execution plan — revision 1

Authority: user request 2026-10-01; [specification](spec.md). All six tasks are bounded before execution. Local files are the tracker; root TODO and RUN link this frontier. Implementation uses an integration branch and isolated implementer branches, then exact-candidate reviews.

Graph: 01 → 02 → {03,04}; 04 → 05; {03,05} → 06.

The provider reconciliation is the required prefactor. Remaining tickets deliver complete user paths, including application, host, UI and verification. Work only the eligible frontier for acceptance/integration. Independent reversible development of session/host/browser composition and dependent worker-policy modules may proceed beside unfinished provider/browser gates because these consume approved public contracts; its production adoption and final verification still require reconciled providers. Merge the current integration into the worker before acceptance. Preserve accepted feasibility evidence but rebuild assets from the reconciled source. No automatic scratch-code promotion.

## Documentation deliverables

- The to-spec parent and six numbered tickets define user behavior, dependencies, oracles, task categories and exclusions.
- Provider reconciliation evidence pins both repositories, overlapping edits, preserved checkouts/stashes and affected checks.
- [Session design](SESSION.md) records typed effects, identities, durable transitions, gesture/job/export behavior and tested public seams.
- Host boundary inventory records worker/storage/renderer/file/service-worker contracts, generated glue, ownership/copies and retirement criteria.
- Build/run documentation records exact toolchain, locked commands, fixture preparation, isolated databases, root/subpath stages and asset hashes.
- Compatibility/evidence records retain native and browser command outputs, archive and STEP oracles, visual/a11y/performance/resource limits and failed gates.
- [User-test handoff](HANDOFF.md) provides the editable copied-project launch procedure and remaining acceptance limits.
- Canonical RUN/TODO/current-run state and architecture point to the accepted implementation and remaining blockers. Historical P1/P2/P3 records remain unchanged.

## Integration and verification

Reference provider input: main checkout dev 5a472a9426e6e38993361da402cd4ec730feb369; migration input cd1efb34. Do not apply stashes. Native and browser seams follow the parent specification; the previously approved public session and real-browser seams are the default; optional feedback may extend them. Each ticket records red-green evidence, affected checks and exact source before coordinator integration. Review changes at the fixed integration base. Keep worker branches/checkouts recoverable; do not delete pre-existing worktrees.

M1 acceptance requires ticket 06. Source or partial browser success alone cannot close it. Existing API/schema visibility changes and new budgets/gates require explicit decisions, with independent work continuing.
