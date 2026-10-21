---
status: accepted
---

# Test real workflows natively and browser behavior mounted

Native workflow tests drive the real Session and Core through the Runtime surface with in-memory persistence, rather than a second application implementation that fabricates settlement. Native and browser test adapters share controlled delay/failure semantics so pending work, successful saves and recovery are exercised through real application transitions. Keep targeted mounted browser tests for presentation and browser integrations that native execution cannot establish; this does not require duplicating every test in both environments. The user confirmed this boundary on 2026-10-19, choosing fast realistic workflow tests plus browser-specific coverage over fake application logic or exclusively browser-driven testing.

The [native Runtime decision](../plans/module-deepening/issues/02-decide-native-test-runtime.md#answer) records the earlier choice of shared gate semantics rather than compiling all browser effects natively or maintaining unrelated test behavior. [Edit-ticket regressions](../../web/crates/runtime/src/edit_ticket.rs), including `native_runtime_settles_a_ticket_through_the_real_session_and_core` and `held_core_and_save_keep_a_ticket_pending_until_released`, exercise the boundary. The [architecture](../architecture.md) retains mounted checks; native success is not proof of DOM, worker, storage or renderer correctness.
