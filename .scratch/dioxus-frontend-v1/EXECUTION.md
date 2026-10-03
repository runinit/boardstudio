# Frontend execution map

Follow [CONSTRAINTS.md](../../CONSTRAINTS.md) for the delivery loop and skill overrides,
[agent routing](AGENT-ROUTING.md) for models, and
[delivery records](../../docs/agents/issue-tracker.md#frontend-delivery-records) for status.
This file does not maintain another checkpoint or approval sequence.

[coverage.json](coverage.json) assigns owners/consumers to the TSX, CSS, hooks and assets;
the [inventory](evidence/tsx-inventory.json) retains source hashes. F9.1 follows transitive
UI-owned TS imports and records replacement or retained-service disposition. A source
row closes through public behavior/integration evidence, not deletion or a renamed file.

[tasks.json](tasks.json) retains all canonical `start_after`, `acceptance_after` and
`depends_on` rationale. Proven child capabilities permit implementation before whole
parent closure; actual acceptance joins still apply. External gates require their
real condition, not elapsed time. Read exact public outcomes from existing issue/specs.

Use named isolated browser sessions/profiles for qualification; cleanup affects only
that session. The page binary and public library are different Rust crate boundaries:
prove consumed call paths, then make the smallest coherent interface change under the
existing API authority. Keep one document/history owner and record RF-002 implications.
The former team maps, adapter notes and checkpoints remain in
[history](../../docs/migration/history/OPERATING-RULES-before-20261003.md).
