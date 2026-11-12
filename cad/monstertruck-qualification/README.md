# Vendored Monstertruck crates

`vendor/` holds the Monstertruck 0.4.1 crates the CAD backend builds against, with local patches.
[`cad/monstertruck`](../monstertruck/Cargo.toml) and [`cad/wasm`](../wasm/Cargo.toml) patch their
dependencies to these trees, so the shipped build needs them.

- Each crate's `SOURCE.md` records the upstream archive, its checksum and every local patch.
- This directory is a Cargo workspace only so the vendored crates' own tests can run.
  `cad/scripts/cad_build.py` (`OWNER_TESTS`) selects the tests that guard the local patches; the other
  upstream test, bench and example targets were removed.
- `fixtures/profile-topology/` is read by the `cad/monstertruck` tests.

The qualification gates and probes that proved the backend before the migration are in Git history.
