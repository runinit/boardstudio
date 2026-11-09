# 007: kicad-cli tests pass under an AppImage launcher

Status: resolved
Type: build
Blocked by: none
Parent: [006: Review follow-ups](006-review-follow-ups-spec.md)

## What to build

See the kicad-cli decision in [006](006-review-follow-ups-spec.md). The 14 Core tests that run
`kicad-cli` fail when the suite is started from an AppImage-hosted terminal, because
`APPDIR`, `APPIMAGE`, `LD_LIBRARY_PATH` and `GSETTINGS_SCHEMA_DIR` leak into the child.

## Acceptance criteria

- [x] When `APPIMAGE` or `APPDIR` is set, every test `kicad-cli` invocation drops the four variables; otherwise the environment is untouched.
- [x] The 14 tests pass both in the leaked environment and with the variables removed.

## Comments

- Added `kicad_command` to `core/tests/support/mod.rs`; it drops `APPDIR`, `APPIMAGE`, `LD_LIBRARY_PATH` and `GSETTINGS_SCHEMA_DIR` only when `APPIMAGE` or `APPDIR` is set. `kicad_integration.rs` (its `kicad-cli` and two `python3`/`pcbnew` spawns) and `generator_export.rs` (two `kicad-cli` spawns) use it. The ignored `mechanical_plate` unit test, in the lib crate and so out of reach of the test support module, has a three-line local `kicad_cli()`.
- Before: 14 failures in the `integration` binary (5 `generator_export`, 9 `kicad_integration`). After: `integration` 715 passed in the leaked environment and with the four variables removed; the ignored `kicad_10_exports_actual_gerber_and_npth_drill` passes in the leaked environment; `cargo clippy -p boardstudio_core --all-targets` is clean.
