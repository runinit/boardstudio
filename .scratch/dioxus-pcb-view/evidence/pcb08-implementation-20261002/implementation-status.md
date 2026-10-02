# PCB08 source checkpoint (2026-10-02)

Status: isolated, targeted source implementation under review preparation; no parent, browser, or pairing closure.

The package adapter calls the existing packaged `isErgogen` and `parameters` exports from the Parts catalogue owner. A real packaged browser test now verifies the schema returned for `infused-kim/smd_0805` (`net_1_from`, `net_1_to`, and `net_6_to`, including default values). The source package's 36-generator catalogue currently has no `anchor`-typed schema entries; the UI keeps the anchor branch conditional and does not manufacture a test source or claim it is reachable today.

The `pnpm run test:web:physical-setup` runner now serves the generated module over local HTTP with CORS and runs only `boardstudio-web` packaged tests headlessly in Chromium. The prior Node invocation had reported success while saying the binary suite was browser-only and skipped; after switching to the browser runner, a broad substring also selected the independent preview-worker test and correctly failed because that test requires its own runner. The final specific filter reports exactly two packaged PCB/physical-setup tests passing, with no unrelated worker test selected.

The Editor owner now guards asynchronous schema preparation against teardown and rechecks the live workspace/scope signals before submitting an edit. It also marks schema preparation as busy and prevents a second edit from replacing the exact pending outcome owner. The owner path and projected controls remain private Dioxus state; they do not change Core or serialized contracts.

Checks completed:

- `pnpm run test:web:physical-setup` — 6 focused browser tests passed in headless Chromium: four PCB08 projection/eligibility/schema-shape tests and two exact-packaged-module tests.
- `cargo clippy --locked --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --no-default-features --features page --all-targets -- -D warnings` — passed after the final PCB08 owner and test changes.
- `cargo fmt --manifest-path web/Cargo.toml --check` — passed.
- `git diff --check` — passed.

The paired user-visible Inspector journey, save/history/reopen, delayed-schema stale-source exercise, full browser integration, and F5.2/F5.3/F5 joins remain open. See `paired-walkthrough.md` for the explicit React/Dioxus steps.
