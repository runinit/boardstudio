# Vendored source provenance

Published crates.io `monstertruck-mesh` 0.4.1, Apache-2.0, upstream commit `e6520c2b1bde6f46a85649024663583cd03f484c`. Archive SHA256: `875dbf22ba985790b87e652bfaa1ed24852d3746b1ac022ac7b6c28ba35121c6`. The archive declares the license but omits its text; LICENSE-APACHE is copied from the sibling vendored Monstertruck package, SHA256 `a60eea817514531668d7e00765731449fe14d059d3249e0bc93b36de45f759f2`.

Only src/polyline_curve.rs differs from published source: exact and nearest Range searches examine clipped source segments, preserve exact interval endpoint labels and handle singleton curves. No epsilon expansion or change to None/Parameter search behavior. Reviewed full patch SHA256 `cb5a5dff6e4705acf47a5a9d1cc50a7b54f122e8fce2e7656e0ef133f4517593`; evidence `.evidence/monstertruck-qualification/polyline-active-range-v2/`. Spec and Standards successor reviews are under `polyline-active-range-v2-spec-review/` and `polyline-active-range-standards-review-v2/`.

The published Cargo manifest is unchanged. Only the standalone qualification workspace selects this local crate; production Cadrum dependencies are unchanged. Root composition evidence is under `vendor-integration/mesh-range-v39/`. Full strict cut-wrapper, STEP, Boolean, WASM runtime and production replacement acceptance remain separate requirements.
