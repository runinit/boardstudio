# Red observation (from tool history)

Command:

```sh
cargo test --manifest-path application/Cargo.toml --locked failed_export_is_removed_and_not_cancelled_again_after_reopen
```

The test compiled and ran, then failed at the expected behavior assertion:

```text
test failed_export_is_removed_and_not_cancelled_again_after_reopen ... FAILED

thread 'failed_export_is_removed_and_not_cancelled_again_after_reopen' panicked at tests/durable_session.rs:1040:5:
assertion failed: !session.export_is_current(OperationId(83), token, &scope)

failures:
    failed_export_is_removed_and_not_cancelled_again_after_reopen

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 12 filtered out

error: test failed, to rerun pass `--test durable_session`
```

The tool showed a failing Cargo test result (nonzero command outcome); its numeric shell exit code was not retained in the recorded output. This is an excerpt from the observed tool result, not a reconstructed full raw log. The source fix in `9b865587` removes the matching operation from `Session.exports` before settling it as `ExecutorFailed`.
