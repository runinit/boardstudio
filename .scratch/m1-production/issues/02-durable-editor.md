# 02: Open, edit, durably save and recover one copied keyboard

**What to build:** A Dioxus editor opens REVIUNG41 through a real Rust worker and session, previews/commits numeric and pointer moves, supports Undo/Redo and retries aborted saves without another engine edit.

**Blocked by:** 01

**Status:** Implementation complete; 13 public session tests and recorded pointer/keyboard/storage checks pass; complete resource acceptance remains open.

**Category:** behavior-preserving migration; ticket 02 includes the already accepted recovery behavior.

**Authority:** [M1 specification](../spec.md), existing six module specifications and ADR 0003.

- [x] Public session tests cover durable ordering, stale scope/replies, retained retry and exact caller settlement.
- [x] Pointer final sample, Escape/cancel, modifier/snap semantics and one-step history match the reference.
- [ ] Real IndexedDB transaction abort/retry and worker crash/close behave truthfully; isolated schema stays compatible.
- [x] Editor reuses styles with named controls and reachable keyboard focus.

See [current acceptance evidence](../ACCEPTANCE.md#retained-evidence--release-8f509433-and-reviewed-overlays) for artifact scope and remaining gates. Checked items record bounded completed checks; ticket closure follows the task graph and full acceptance requirements.
