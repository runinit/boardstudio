# 07: Test dropping `touch core/src/lib.rs`

Status: resolved
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

## Answer

2026-10-09: the `touch` is not needed. Every run in this effort (baseline, new full check, quick
and browser reruns) omitted it, and results and executed counts matched the earlier runs that used it,
including `boundary_parity` inside the 463 core integration tests. Cargo behavior without it:
`cargo build -p boardstudio_core` with no change did not recompile core; appending a line to
`core/src/lib.rs` recompiled it; reverting recompiled it again. The old handoff's "shared target
leak" had no recorded mechanism and this evidence found none. The `touch` appears only in the
superseded outline-phase handoff and past plan records, which stay as history; README and
AGENTS.md never carried it. Cost avoided: core and everything downstream rebuilding in each
configuration on every run (about a minute per configuration).

