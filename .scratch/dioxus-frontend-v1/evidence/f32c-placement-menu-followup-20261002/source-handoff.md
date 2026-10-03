# F3.2c placement-menu review follow-up

This source follow-up is based on reviewed F3.2c correction `5bb696dfac11c293b063cfafc3b15ff4043d9217`, retaining that immutable review candidate in its original worktree. The production chooser now applies `catalogue_choices` to both its default groups and search results, preventing retired `nice_nano_pretty` definitions and unassigned assembly snapshots from being offered. Add-menu search now uses the same `PartsQuery` signal as the Parts library and advances its existing selection-intent generation when edited. Browse all parts switches to Parts and reveals the actual Objects panel: it opens compact Objects or pins the desktop panel.

The Browse regression mounts the production `AddObjectComponentChooser`, dispatches a real search input and clicks its actual Browse button. It verifies the typed query remains `battery`, the Parts workspace is selected, and each compact/desktop panel path is revealed. The catalogue regression runs the existing production eligibility predicate against ordinary, retired and assembly-snapshot definitions and checks both grouped choices and search results.

## Verification

- `cargo fmt --manifest-path web/Cargo.toml --all -- --check` — pass.
- `git diff --check` — pass.
- `cargo check --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --no-default-features --features page --bin boardstudio-web` — pass.
- `cargo test --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --no-default-features --features page --bin boardstudio-web --no-run` — pass.
- `env CHROMEDRIVER=/usr/bin/chromedriver wasm-pack test --headless --chrome --mode no-install web --no-default-features --features page --bin boardstudio-web -- add_object_menu_tests` — 4 passed.
- Strict WASM all-targets Clippy has not been rerun for this follow-up. The coordinator should include it with the joined package checks.

The regressions failed against temporary source mutants that reproduce the two defects:

- [Catalogue-filter expected red](catalogue-filter-expected-red.log), SHA-256 `fe397e70354793c07e2b79f969a5912e2d2121868646c8fcd2e5183438aaa8a3`; unfiltered choices yielded 7 instead of 5.
- [Browse-query expected red](browse-query-expected-red.log), SHA-256 `5e149b7eed850bd630d6cebea4568451905ccd8b8638e1039b6d958e51e8170e`; local chooser query remained empty after Browse.
- [Panel-reveal expected red](panel-reveal-expected-red.log), SHA-256 `a1a6f42837fda77a5ef5c85cb4b45920906574adcea09417a8a2eabe3e6b45ae`; the compact path remained closed and the desktop panel remained collapsed.
- [Follow-up green run](green.log), SHA-256 `b4fabdcc476e108ebe5beff532b144acdbb8d4c11e7799d25697a13fb0f86288`.

Trailing whitespace from the test harness was stripped from the retained logs so the committed evidence passes `git diff --check`; the test messages and outcomes are otherwise unchanged.

No public menu acceptance is claimed here. Root retains serial integration, joined strict checks and paired browser verification. The existing baseline receipt is [general-placement-public-20261002](</home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/.scratch/dioxus-frontend-v1/evidence/general-placement-public-20261002/receipt.md>). No new refactoring takeaway was observed; the bounded record is in [POST-PORT-REFACTOR](../../../../docs/migration/POST-PORT-REFACTOR.md#f32c-placement-chooser--2026-10-02).
