# F5.6 ticket 05 implementation handoff — accepted physical-setup proposals

## Delivered boundary

The private typed-intent model proposal builder consumes an immutable `ProjectDoc`, returns a cloned proposal, and never submits a Session event or changes selection. Topology preserves the retained primary's orientation and authored state, keeps electrical board state, falls back from selected primary mechanics to board-matching mechanics to the existing initializer, and creates a fresh secondary from the effective reversible setting. Existing `sharedConstruction` is preserved or initialized from the resolved mechanics. Transport edits touch only transport. Reversible intent writes the project-level flag, normalizes eligible definitions through the existing packaged module, updates only already-present eligible part overrides, and projects all instance flips. Errors leave the accepted input untouched.

The real packaged Gateron JS module is imported in a WASM test and used by the same private catalogue normalization helpers as proposal construction. The test observes normalized `reversible`, `hotswap=false`, and `solder=true` output plus project/part projections and immutable source state. The proposal modules/adapters are test-scoped in this prerequisite commit because there is no production caller until ticket 06 mounts the unconditional Editor owner; that dependent slice will compile the same path into the page binary with its first real consumer. This keeps ticket 05 independently warning-clean without adding lint suppressions or widening the catalogue loader/normalizer members.

## Refactor ledger handoff

- **RF-006:** `web/src/physical_setup.rs::propose_topology` confirms the board and instance lineage split. It retains a still-present explicitly selected instance first, then the first instance matching the selected board. This ticket only prepares proposal IDs and documents; it does not reconcile selection. The accepted-only selection owner remains in ticket 06.
- **RF-009:** `web/src/presentation/parts/catalogue.rs::construction_definition_with_support` reuses the current packaged `isErgogen`/`parameters`/`normalizeDefinition` path and reports support from the source metadata used in that decision. `web/src/physical_setup.rs::propose_reversible` keeps project setting, eligible definitions, conditional part override, and instance-flip projections in one immutable proposal. The WASM test invokes the packaged Gateron implementation, not a mocked normalizer.
- No new RF ID or post-port redesign proposal arose from this bounded implementation. Existing root request lifetime and selection authority remain outside this ticket.

## Verification

- `cargo test --manifest-path web/Cargo.toml --features page --locked`: passed; includes proposal and existing web tests.
- `pnpm run test:web:physical-setup`: passed; compiles and runs the WASM test under Node against the generated packaged Ergogen artifact.
- `cargo clippy --manifest-path web/Cargo.toml --all-targets --features page -- -D warnings`: passed.
- `cargo clippy --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --features page --locked -- -D warnings`: passed.
- `cargo check --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --features page --locked`: passed.
- `cargo build --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --features page --locked`: passed.
- `cargo fmt --manifest-path web/Cargo.toml -- --check`: passed.
- Strict `cargo clippy` for both native all-targets and WASM page builds: passed with `-D warnings` and no new lint allowances.
- `pnpm run build:core` and then `pnpm run check:boundaries`: passed. The first boundary invocation was blocked by the clean worktree's absent generated `core/pkg` artifact; it passed after building that expected artifact.

F5.6a's paired TypeScript/Dioxus browser journey, Editor-lifetime operation owner/selection reconciliation (ticket 06), and F5.6/F5.8 parent joins remain open. No shared ledger or task graph was changed.
