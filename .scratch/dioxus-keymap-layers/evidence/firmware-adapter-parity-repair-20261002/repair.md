# Firmware adapter parity repair — 2026-10-02

Defect correction of independent review `/tmp/keymap08-firmware-source-review-20261002.md`, baseline `c98d50c08f9a1af1ef0a71455da5d0f7176b1af6` (adapter `192b7bcab30cc341e06a58f9fa851b47e23d8537`). Isolated worktree `firmware-adapter-parity-repair-20261002`; current root CONSTRAINTS confirmed execution authority was read. No root source writes or external/public contract changes.

The adapter combined central/peripheral encoder identities in the central request while resolving all GPIO through the central plan. It also swallowed failure while replacing a recursive peer's legacy overlays, and selected legacy terminal B where React selects C. The repair restores the source contract in `app/src/firmwareHandoff.ts` and `firmwarePeripherals.ts`: each request's encoder rows/IDs come from that plan; global sensor order remains only in the legacy overlay path; outer overlay failure propagates; legacy C maps to its actual resolved function. Existing Runtime, Core worker, export lifecycle, archive and MIME code is unchanged by this repair.

Actual production-adapter regressions import the unchanged private module into the existing integration harness. Before correction, all three new tests failed as expected (four existing tests passed): central IDs included the remote knob; mixed-profile outer conversion returned an incomplete request with empty peer overlays instead of error; React's legacy C fixture failed to find a GPIO. After correction all seven pass. These are real adapter calls, not a copied policy or mock implementation.

Coverage:
- Distinct rotary IDs and shared logical IDs across separate physical plans retain each half's exact GPIO and physical-instance metadata.
- A profiled D-terminal peer is valid by itself; paired with a legacy central encoder, the outer legacy override must report its missing C-terminal mapping. The test proves recursive local conversion succeeds first, so it reaches the previously swallowed error.
- The source-matched React legacy fixture uses A→encoder-a and C→encoder-b, verifies B-channel GPIO, identical shared sensor overlay order, disabled remote nodes, and per-plan encoder IDs.
- Existing matrix/direct scans, wired split UART reversal/row offset, display config/overlay and module diagnostics remain green.

Command: `TMPDIR=/home/chris/.local/share/boardstudio/tmp CARGO_TARGET_DIR=/home/chris/.local/share/boardstudio/worktrees/keymap08-runtime-mime-join-20261002/web/target cargo test --locked --manifest-path web/Cargo.toml --test firmware_request_adapter`. Red: four passed/three failed. Green: seven passed. Native test invocation also builds the baseline non-WASM bin and emits its existing twelve dead-code warnings; no suppression or unrelated cleanup was added. A first test compilation used the private Core inputs path; it was corrected to the existing public model reexports before the behavioral red above. That compile failure is not counted as regression evidence.

Raw command outputs remain in `/tmp/firmware-adapter-parity-{red,green}-20261002.log`; durable copies only normalize trailing whitespace. The author supplied the existing pinned React test receipt (14 tests), reused as supporting reference coverage. New Rust fixtures use direct source expectations; no new React full package or provider acceptance is claimed.

Open gates: independent repair review, complete Keymap export source reack, real provider stale/error execution and output ZIP/MIME comparison, fresh paired public export workflow, and all existing F5.2/F8.2/Keymap/frontend parent joins. Existing RF-009 conversion accounting applies; no new structural ledger entry or graph mutation.

Affected checks: strict WASM page all-target Clippy (`cargo clippy --locked --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --no-default-features --features page --all-targets -- -D warnings`) passed; formatter and source/evidence staged diff checks passed. This adapter is page-owned request conversion; the Runtime package/provider/public checks remain separate.
