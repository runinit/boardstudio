# Trimmed upstream source provenance

This crate is trimmed from the published crates.io `Inflector` version `0.11.4`
(`Inflector-0.11.4.crate`, SHA-256
`fe438c63458706e03479442743baae6c88256498e6431708f6dfc520a26515d3`,
`BSD-2-Clause`, author Josh Teeter <joshteeter@gmail.com>).

Only the `to_pascal_case()` and `to_screaming_snake_case()` conversions used by
`step-p21` and `step-p21-derive` are retained, with `[features] default = []`
and no `regex` or `lazy_static` dependencies (cold-build ticket 07).
