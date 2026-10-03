# Layout model error settlement expected-red control

The owner-settlement regression was run against a disposable detached worktree at source commit `53b2d5447f9b44467820bf4144a51632a8fc46aa`. The isolated mutation changed only the production-used helper condition in `web/src/presentation/model_delivery.rs` from exact preview comparison:

```rust
current().is_some_and(|preview| preview.same_live_source(&expected))
```

to the old broad condition:

```rust
current().is_some()
```

Command:

```text
cargo test --manifest-path web/Cargo.toml --bin boardstudio-web --features page layout_viewer_source::tests::suspended_model_failure_settlement_cannot_target_replacement_preview_owner -- --exact --nocapture
```

Expected result observed: the test fails at `assert!(reported.borrow().is_empty())` after A's suspended error settles while replacement B is current. This shows the test detects the old any-current-preview behavior. The original helper was never changed in the author worktree. The disposable worktree was restored clean and removed after the run.
