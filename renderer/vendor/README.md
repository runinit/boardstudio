# Renderer math dependency

The workspace patches crates.io `cgmath` 0.18.0 to source from `cgmath-next`
0.18.1 at upstream revision `ca1e234d333d184894d1838cd67451a871f33ba1`. The
vendored manifest keeps package name `cgmath` and version `0.18.0` because
`three-d` and `three-d-asset` require that package identity. `Cargo.toml.orig`
preserves the upstream manifest, and `cgmath-next-security.json` verifies the
source revision and every vendored file.

The successor fixes the unsafe swap implementation and adds runtime layout
guards before tuple reference conversions. Its `tests/soundness` suite covers
those conversions and the swap regressions; CI runs that suite under Miri.

The vendored source removes obsolete `#[inline]` attributes on macro calls and
unreachable `simd` and `rustc-serialize` cfgs that emitted compiler warnings.
The removed SIMD code was not a declared Cargo feature. The explicit Rust 2015
edition matches Cargo's previous default.
