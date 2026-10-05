# Queued absolute-position diagnostic — retained, not a migration gate

A temporary 106-line native integration test was authored in `application/tests/durable_session.rs`, run against real `Session` and `CoreEngine`, then removed from the live test file after the independent scope decision. Its exact patch is `queued-position-native-diagnostic.patch`; it is not applied source and is not a product regression gate.

The test opened J1 at revision 0 with X=66.675 and Y=-47.625. It issued an absolute `MoveParts` X edit to (60,-47.625), let Core return and the first Persist effect pause Session, then queued a Y edit composed from accepted revision 0 at (66.675,-40). It committed both saves and one Undo. The native RED output in `queued-position-native-red.log` observed persisted and accepted (66.675,-40); one Undo returned (60,-47.625). This confirms that Session rebase of the queued command's revision does not change its explicitly supplied absolute coordinates.

The matching candidate and pinned TypeScript rapid public traces have the same final geometry and Undo behavior. Under the current migration contract, generic Session `MoveParts` means apply the submitted absolute positions; changing this path to merge coordinate axes would add a new axis-intent protocol, affecting grouped positioning and app/Session intent semantics. The independent reviewer therefore ruled this outside current F3.7-C02/F8 scope. Root is tracking the shared rapid-axis behavior as RF-016 for future product-contract review. Do not treat the expected-failure diagnostic as a current migration failure.

Test command, run from `application/`:

```sh
cargo test --test durable_session queued_position_axis_edits_preserve_prior_axis_and_undo_in_sequence -- --exact --nocapture
```

No production files changed. `application/tests/durable_session.rs` has been restored to its HEAD bytes; only this evidence directory retains the diagnostic.
