# Firmware lifecycle harness repair

Date: 2026-10-02. Category: defect correction. Starting integrated source:
`8cd53167bb6b85d573576d724001cd413d0aa36d`.

The integrated firmware export change made `KeymapPanel.on_export` required.
The existing native lifecycle harness omitted it, so `cargo test` failed with
E0061 at the real component mount before any lifecycle test could run.

This repair only changes `web/tests/keymap_panel_lifecycle/harness.rs`. It supplies
an export callback probe and explicitly enables the fixture export action. All
six existing tests and assertions remain intact. A seventh test dispatches a real
HTML click to the mounted export listener and asserts exactly one callback,
preserved search/key/layer, and no key-selection callback. Dioxus mounts the static
Export listener before dynamic layer/tab children; the callback-count assertion
verifies that the captured first listener is the expected target.

No production source, visibility, provider, saved format, or runtime contract
changes. A separate production/browser build is not applicable to this test-only
repair; firmware workflow acceptance remains with the existing integrated packet.
Rollback is reverting this repair commit. Independent source review remains with
the root coordinator before serial integration.

## Executed checks

All Cargo commands used `--locked` and the root's explicitly released idle
compiler cache, `CARGO_TARGET_DIR=/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/web/target`.
The cache is compiler reuse, not substituted test/build evidence. Commands ran
against this sibling worktree's source.

| Command | Result |
| --- | --- |
| `cargo test --locked --manifest-path web/Cargo.toml --test keymap_panel_lifecycle` before repair | **RED**, E0061 / missing required `on_export`, exit 101. |
| Same focused command after repair | **PASS**, all seven tests. |
| `cargo test --locked --manifest-path web/Cargo.toml` | **PASS**, 219 tests, zero failures/ignored tests. Unchanged native binary reports 12 dead-code warnings for WASM-owned modules. |
| `cargo clippy --locked --manifest-path web/Cargo.toml --test keymap_panel_lifecycle -- -D warnings` | **PASS**, strict native affected-harness lint. |
| `cargo clippy --locked --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --no-default-features --features page --all-targets -- -D warnings` | **PASS**, strict WASM page checks. |
| `cargo clippy --locked --manifest-path web/Cargo.toml --all-targets -- -D warnings` | **FAIL**, unchanged native binary dead-code warnings become errors; binary test target also contains unused WASM-only items. No suppression or production changes made. This broad native gate remains open. |
| `cargo fmt --manifest-path web/Cargo.toml --all -- --check` and `git diff --check` | **PASS**. |

Initial export-probe attempts failed because Dioxus omits static text from the
recorded mutation list and because dynamic children mount after the parent
listener. Those attempts and the temporary diagnostic output are retained below;
no diagnostic logging remains in the final source.

Harness SHA-256: `dcae39c5715f4c6655176b879fcf03c9a694881f22039ce5968dcb825f4be89a`.

Raw logs retained at `/home/chris/.local/share/boardstudio/retained-tmp/20261002/`:

| Log | SHA-256 |
| --- | --- |
| `firmware-lifecycle-export-harness-red.log` | `bbdb7ec60b3e10807aeee5c0e51dd4eb9239aa83e551945374025261981b64a5` |
| `firmware-lifecycle-export-harness-green.log` | `06fa9adb786a2e5bbe02fed24e9a95d9558f43c6b4f8575601e56707c0056f05` |
| `firmware-lifecycle-export-harness-green-final.log` | `0c613e86f912a4111005d51ac0294c10eac6246a9410ce936e36197a106cfca0` |
| `firmware-lifecycle-export-harness-probe.log` | `8bf6f668b77cef9afb8c1db4aac1c00e08a7457969931a68dd21629c54467a84` |
| `firmware-lifecycle-export-harness-native.log` | `7fa130bacd0c5500f3c34ba11446a7a626d72e1f3707ba2345df849948c1acda` |
| `firmware-lifecycle-export-harness-wasm-clippy.log` | `5e750cca7dca9cc2c927ba037e495bcefa50ea96ffa7ab2b25a8b83f27546c1b` |
| `firmware-lifecycle-export-harness-native-clippy.log` | `4d208a35d107f86118d57bdb384cb6700a6111fb90905277fc29afc2904be1b4` |
| `firmware-lifecycle-export-harness-focused-clippy.log` | `d45006197a58d3591511e8220a2f4a8bd039e9a092cd6a66dbadf79d6dc77af2` |
