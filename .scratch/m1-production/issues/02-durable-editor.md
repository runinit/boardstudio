# 02: Open, edit, durably save and recover one copied keyboard

**What to build:** A Dioxus editor opens REVIUNG41 through a real Rust worker and session, previews/commits numeric and pointer moves, supports Undo/Redo and retries aborted saves without another engine edit.

**Blocked by:** 01

**Status:** ready-for-agent

**Category:** behavior-preserving migration; ticket 02 includes the already accepted recovery behavior.

**Authority:** [M1 specification](../spec.md), existing six module specifications and ADR 0003.

- [ ] Public session tests cover durable ordering, stale scope/replies, retained retry and exact caller settlement.
- [ ] Pointer final sample, Escape/cancel, modifier/snap semantics and one-step history match the reference.
- [ ] Real IndexedDB transaction abort/retry and worker crash/close behave truthfully; isolated schema stays compatible.
- [ ] Editor reuses styles with named controls and reachable keyboard focus.
