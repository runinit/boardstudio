# F3.2c bounded placement-menu correction

This isolated source candidate is based on `551f2994ff5ffd9d98ffe683207ad7eff635881e` on branch `codex/f32c-placement-menu-parity-20261002`. It corrects the Layout Add object path only. The menu presents Layouts before Parts, shows the active board, exposes the real board-layout target selector, searchable available catalogue entries and React's four default groups, and routes Browse all parts to the existing Parts workspace. The trigger is accent-filled with an accessible text name and a decorative plus mark. Accepted general placement now selects the committed component in the tree and sets the Layout tool context to Part.

The chooser reuses `layout_target`, the current bundled/project catalogue merge, and the existing private placement action. The accepted component context is derived from the accepted snapshot after the placement operation completes. No public API, schema, or Runtime surface was widened. The menu includes only the existing new mirrored-pair and matrix actions. Existing-half mirroring, Board outline launch, and Geometry scripts remain outside this correction because the current Layout mount does not expose those callable menu actions; no placeholder controls were added.

## Verification

- `cargo fmt --manifest-path web/Cargo.toml --all -- --check` — pass.
- `git diff --check` — pass.
- `cargo test --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --no-default-features --features page --bin boardstudio-web --no-run` — pass.
- `env CHROMEDRIVER=/usr/bin/chromedriver wasm-pack test --headless --chrome --mode no-install web --no-default-features --features page --bin boardstudio-web -- add_menu_uses_the_four_reference_groups_for_available_definitions` — 1 passed.
- Same wasm-pack command filtered to `accepted_general_placement_selects_the_component_and_part_tool_context` — 1 passed after fix.
- `cargo clippy --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --no-default-features --features page --all-targets -- -D warnings` — pass after introducing the private `GroupMatcher` alias.

The mounted placement regression was run against a temporary local mutant with accepted placement clearing its selection and leaving the tool context unchanged. It failed at `Key` versus expected `Part`; restored production source passes. Logs:

- [expected red](expected-red.log), SHA-256 `9831177b89b8f7aac2a11abf915ce8371e69ebb997268995e84b87c98ed7193a`.
- [expected green](expected-green.log), SHA-256 `c2f7421690580098336d72792abd1230e0ac6e37d702745ccb186051942b9b07`.

The existing public baseline receipt is [general-placement-public-20261002](../general-placement-public-20261002/receipt.md) at integrated source `7d09d0a60fbb2cc12e259541614b9483ee618d29`. It records the prior missing contextual component selection. This isolated source candidate has not yet been rebuilt into a served package or independently reviewed; no new public acceptance is claimed here. Root owns serial integration and paired browser verification. React oracle remains pinned at `5a472a9426e6e38993361da402cd4ec730feb369`.

No new refactoring takeaway was observed; see [POST-PORT-REFACTOR](../../../../docs/migration/POST-PORT-REFACTOR.md#f32c-placement-chooser--2026-10-02).
