# KiCad integration checks

PCB preview and KiCad export run inside Rust Core using `boardstudio-footprints`.
This package retains independent KiCad CLI integration checks for source import,
artwork, export and connectivity. Generator export and rotated front/back preview
coverage lives in `core/tests/generator_export.rs`; the recorded provider baseline
lives in `footprints/tests/golden`.

Run `pnpm --dir kicad test` or `cargo test --manifest-path core/Cargo.toml`.
