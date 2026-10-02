# Native Keymap projection contract harness

Harness is isolated under `/tmp/frontend-run/keymap-projector-tests`; it freezes and includes the exact `web/src/presentation/keymap/view.rs` source at `crate::presentation::keymap::view`. It uses the actual Reviung project fixture, boardstudio_application `Session`, and `boardstudio_core::CoreEngine`. The fixture enters `Session::Open`, the actual Core reply is completed, and acceptance persistence is committed before the projector is called. Keymap-specific variants modify that real ProjectDoc before Core acceptance. No fabricated scene/snapshot, fake Part IDs, or production source/API edits were used.

## Provenance

- Worktree HEAD at preparation and run: `75139d74bd164695c84683a3174b8ec705ec2763`.
- Current and frozen Keymap view source SHA-256: `2bdc79466dd88e1a42f18158188fc74cad89cb1fef6d45450c76b9ba1c27dc9e`.
- Reviung fixture copied byte-for-byte from `web/target/builds/frontend-parts-context-aria-20261002/site-root/assets/fixtures/reviung41.json`; source and copy SHA-256: `b577dd2009fffbf00489cc8d0f2ccc088861f62a7f30af42470d35c11534bdae`.
- The accepted fixture has 85 actual board member parts and 41 actual switch parts.

## Executed command and result

```text
CARGO_TARGET_DIR=/home/chris/.cache/boardstudio/keymap-projector-tests/target cargo test --manifest-path /tmp/frontend-run/keymap-projector-tests/Cargo.toml -- --nocapture

running 5 tests
5 passed; 0 failed
```

The five tests cover virtual Base plus legacy hardware binding fallback and exact real switch membership (including exclusion of diode/stabilizer companions); persisted typed layers overriding legacy data and distinguishing `Transparent` from `None`; pose projection from the actual accepted Core scene transform; stale active layer fallback to the first persisted layer while serialized accepted ProjectDoc/revision remain unchanged; and the private missing-macro display fallback. The missing-macro fallback is a helper-level test only: Core validates macro references at document acceptance, so no invalid accepted snapshot is fabricated.

This is native projection/application/Core evidence only. It does not test Dioxus rendering, browser behavior, JS, or WASM. Cargo output was directed to the separate home-cache target above, not the worktree. Root `view.rs` hash was unchanged after the run; the worktree's existing modified status for that file was present before this harness.
