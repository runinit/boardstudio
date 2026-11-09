# Vendored source provenance

This is the published crates.io `Inflector` version `0.11.4` package, whose
manifest declares BSD-2-Clause (allowed by explicit decision for cold-build
ticket 07).

The source archive `Inflector-0.11.4.crate` has SHA-256
`fe438c63458706e03479442743baae6c88256498e6431708f6dfc520a26515d3`.

One change: `[features] default` is `[]` instead of `["heavyweight"]`, so the
transitive `regex` and `lazy_static` stay out of the CAD trees. `step-p21` and
`step-p21-derive` call only `to_pascal_case()` and `to_screaming_snake_case()`,
whose modules reference neither regex nor any gated item.

Path dependencies do not get cargo's lint cap, so the crate's `#![deny(warnings)]`
fires on this toolchain. Twelve no-op `.clone()` calls on `&str` (all in tests)
were removed, and the now-unused `use cases::case::*;` in `classcase` carries an
allow. All semantics-preserving; no other source file is modified.
