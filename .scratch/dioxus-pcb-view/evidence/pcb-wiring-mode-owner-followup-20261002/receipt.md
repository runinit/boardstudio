# PCB wiring mode owner follow-up

This follow-up is based on `dea33675f3a83b1051ff93850b0c67766585b202`; it addresses the bounded findings from the exact `dea33675` source rereview. The original mode repair remains independently frozen and no Runtime/Core/API/schema boundary changed.

The mounted fixture now represents a legal board-only context: the selected board exists, it has a real switch member for selection-context coverage, there is no selected part or physical instance in the primary board-context journey, and the accepted source/scope agree. Retained callbacks are tested after project, session, board, revision, selection, workspace, and scope-generation changes. Each changed-context callback submits no event.

Feedback now retains the requesting plan identity. Pending and failed states display only while their captured accepted plan remains current; successful Saved feedback is allowed to survive the expected accepted token/revision advance. A production mounted-owner regression first reproduced the stale-failure display with the plan guard removed, then passed with the guard restored.

## Verification

- `focused-native-green.log`: mounted production owner tests, 5 passed, 0 failed. SHA-256 `671320c963fb853c01d09847ca752a5b9c5eab8187b6e30e50d4564c705bfa8f`
- `expected-red-without-plan-guard.log`: expected failure at the stale feedback assertion, exit 101. SHA-256 `cc51529d457c91917c52e103577306a23289856624bafc162b5040dc9bd45b80`
- `fmt.log`: `cargo fmt --manifest-path web/Cargo.toml -- --check`, passed. SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`
- `diff-check.log`: `git diff --check`, passed. SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`

No new RF item is warranted. Preserve RF-001/RF-006/RF-009. The paired packaged journey, Undo/Redo, save/reopen, current public source/build acceptance, and F5.1/F5.2/F5.3 parent joins remain open. Wiring Apply remains a separate child.
