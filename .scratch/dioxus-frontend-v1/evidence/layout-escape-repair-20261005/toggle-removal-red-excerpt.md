# Toggle-removal regression RED

Command:

```sh
python scripts/migration-deliver.py focused-test -- wasm-pack test --headless --chrome --mode no-install web --no-default-features --features page --bin boardstudio-web -- toggle_removal_keeps_the_inspector_and_commands_aimed_at_remaining_keys 2>&1 | tail -n 90
```

Exit status: 1. The focused browser test executed and failed at the intended assertion before the implementation change:

```text
running 1 test
test presentation::objects::matrix_transform_inspector::mounted_tests::toggle_removal_keeps_the_inspector_and_commands_aimed_at_remaining_keys ... FAIL

panicked at src/presentation/objects/matrix_transform_inspector_tests.rs:543:5:
removing C must move the key Inspector to a remaining selected member

test result: FAILED. 0 passed; 1 failed; 0 ignored; 306 filtered out
```

This is an excerpt transcribed from the captured tool output; the original RED terminal stream was not saved to a log. The passing run's complete terminal stream is in `toggle-removal-green.log`.
