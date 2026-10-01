# M1 browser host slice

This evidence covers the maintained `web/` host implementation at the host slice commit. It is development evidence only; a real browser run against the integrated Dioxus app is still required.

- `cargo fmt --manifest-path web/Cargo.toml -- --check` — pass.
- `cargo test --manifest-path web/Cargo.toml --locked` — pass, 5 tests (full-u64 text frame, frame version, safe/unsafe storage integer contract, SHA-256).
- `cargo clippy --manifest-path web/Cargo.toml --locked --all-targets -- -D warnings` — pass.
- `cargo check --manifest-path web/Cargo.toml --locked --target wasm32-unknown-unknown --no-default-features --features core-worker` — pass.
- `cargo clippy --manifest-path web/Cargo.toml --locked --target wasm32-unknown-unknown --no-default-features --features core-worker -- -D warnings` — pass.
- `cargo check --manifest-path web/Cargo.toml --locked --target wasm32-unknown-unknown --no-default-features --features page,test-harness` — pass.
- `cargo clippy --manifest-path web/Cargo.toml --locked --target wasm32-unknown-unknown --no-default-features --features page,test-harness -- -D warnings` — pass.

The core-worker and page feature sets compile independently. No browser acceptance is claimed here. `test-harness` exposes a host-side forced IndexedDB abort entry point for browser verification; it is not enabled by default. Persistent documents retain JSON-compatible object representation with `id` keyPath, project writes wait for transaction completion, asset references are checked in the same transaction, and unsafe JS-number integers are rejected before serialization.
