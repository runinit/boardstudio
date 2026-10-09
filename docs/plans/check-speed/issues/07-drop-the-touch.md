# 07: Test dropping `touch core/src/lib.rs`

Status: ready-for-agent
Type: task
Blocked by: 02
Map: [map.md](../map.md)

## Question

Is `touch core/src/lib.rs` before each run needed? Its only record is one line in an old handoff
("Shared target leak ... `touch core/src/lib.rs` before each run"). It forces `boardstudio_core`
and everything downstream to rebuild in each configuration (core compiled 5 times in one run).

## Scope

AFK. On a clean tip, run the affected gates twice: with the touch and without it (the shared
`CARGO_TARGET_DIR`, as before): `cargo test -p boardstudio_core --test integration` including
`boundary_parity`, the WASM host `boundary_parity::` test, and the layout browser suite. Include a
case that edits a core file between runs to prove cargo still rebuilds on a real change. Do not
run workspaces in parallel while comparing.

## Answer

Record whether results and executed counts match, how much time the touch costs per run, and the
decision. If identical, remove the touch from the live docs (the README, AGENTS.md and any
current handoff; superseded plans stay as history) and note the cause as undocumented. If not,
keep it and document the cause.
