# Keymap and located findings

BoardStudio now separates logical Keymap editing from physical Keycaps settings. The persisted Rust model supports key press, mod tap, layer tap, momentary/toggle/go-to/sticky layers, sticky keys, transparent/unassigned keys, ordered macros and per-layer encoder rotation/push bindings. Layer references use stable IDs; export resolves their indices in the same physical scan order used by the electrical handoff.

Rust validates edits, saved keymaps and export requests. Invalid references and DTS delimiters are rejected. Legacy base bindings remain usable, and legacy binding edits update an existing typed base layer. Macro editing and encoder direction edits update individual fields to preserve sibling changes. Export pins ZMK v0.3.0, declares macro/sensor behaviors, enforces the 256 expanded-binding limit and increases the behavior queue for accepted macros. Explicit waits queue a no-op press followed by the requested delay and restore the default wait. Every subsequent action explicitly selects its tap/press/release mode.

Both split halves register sensors in the same global order, disabling remote encoder nodes. A board with only encoder push inputs uses a real direct scanner. Controller/GPIO setup remains in PCB. Current EC11 defaults remain 80 steps and 20 triggers per rotation.

Finding actions use local markers at corners where finishing reduced the requested size, resolved keycap envelopes for clearance, precise existing fabrication markers before object fallback, and component courtyards for applicable object findings. Navigation preserves focus across workspaces and scopes lookup to the selected board. Assembly findings highlight known mount/gasket handle positions; diagnostics without a point location retain whole-layer selection rather than inventing one. Covered board corner warnings replace their redundant aggregate warning.

## Validation

Logs and red regressions are preserved in [evidence/keymap](evidence/keymap/).

- Rust core: 334 tests; renderer: 26 tests.
- App: 469 tests with eight workers. Unrestricted runs hit a storage-test timeout; the isolated storage suite and bounded full suite pass. Intermediate compile errors and browser regressions were fixed before final verification.
- Browser: 23 scenarios covering layers, hold taps, macros, encoder rotation/push export, keycap profiles/legends/colors, Undo/Redo, persistence, CAD preview, STEP/ZMK downloads, narrow controls, corner-location focus, Keycaps-to-Layout focus and the existing outline editing/version/snapping workflows.
- CAD: 23 native and 49 JavaScript tests; KiCad: 35 tests; Ergogen catalogue, runtime and verification checks pass.
- Contracts generation/check, app TypeScript/build, repository checks and native/WASM parity (17 core requests, nine archive requests) pass.
- Standards and spec reviewers' findings were resolved: board lookup, imported model validation, macro capacity/expansion, directional sibling edits, encoder-only export and assembly location focus. The bounded design review is documented separately.

Source export is verified; a real west/Zephyr firmware build and device behavior were not run (west is unavailable). Keycode expressions are checked for safe syntax, not membership in every ZMK symbol. User-entered symbols must exist in the pinned ZMK headers. Hold-taps use ZMK's default timing configuration. This release does not claim complete ZMK parity: custom hold-tap timing, combos, tap dance, mod morph, conditional layers, Bluetooth/system behaviors, parameterized macros, pause-until-release and live keyboard connection remain future scope. No extra TODO entries were added for that roadmap.
