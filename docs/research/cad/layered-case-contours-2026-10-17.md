# Line-and-arc Boolean candidate for layered cases

Evaluated 2026-10-17 for ticket 013. This is a candidate evaluation for the private prototype only; it does not recommend a production cutover.

## Candidate and API

Pin `cavalier_contours = "=0.9.0"`. The crates.io release is dated 2026-08-20, declares Rust 1.88+, is dual licensed `MIT OR Apache-2.0`, and its package metadata points to source commit `22887a824d22cd8ab50968a551f34cdb4508127f`. The probe lockfile records registry checksum `85e06038a969c5e4d93565f12b044814d3b1a48d50c4ef868d588e79956fea51`. [Published crate metadata](https://docs.rs/crate/cavalier_contours/0.9.0), [tagged release](https://github.com/jbuckmccready/cavalier_contours/releases/tag/0.9.0)

Use the library's own `Polyline<f64>` as the line-and-arc contour. `PlineSource::boolean` and `boolean_opt` take another `PlineSource` plus `BooleanOp` and return `BooleanResult<Polyline<f64>>`. Keep the complete result: `pos_plines` are positive remaining-space contours and `neg_plines` are negative subtracted-space contours. The pair API has no hole-parent or multi-ring set type; `Shape` supports multi-polyline offsets, not multi-ring Boolean operations. Keep role/ownership explicit in the prototype and reject unsupported compositions rather than folding only the positive contours. [Polyline API](https://docs.rs/cavalier_contours/0.9.0/cavalier_contours/polyline/trait.PlineSource.html), [BooleanResult fields](https://docs.rs/cavalier_contours/0.9.0/cavalier_contours/polyline/struct.BooleanResult.html), [candidate limitations](https://docs.rs/crate/cavalier_contours/0.9.0)

Bulges are exact circular arcs, not polygon approximations, but each segment is limited to bulge `[-1, 1]` (at most a semicircle); larger sweeps need multiple segments. Boolean operations are documented for two closed, non-self-intersecting polylines. The Boolean default `pos_equal_eps` is an absolute `1e-5` position epsilon; optional `collapsed_area_eps` can discard result loops by absolute area and defaults to `None`. [Candidate documentation](https://docs.rs/crate/cavalier_contours/0.9.0), [Boolean options](https://docs.rs/cavalier_contours/0.9.0/cavalier_contours/polyline/struct.PlineBooleanOptions.html)

The 0.9.0 release focuses on offset topology. Its notes say that the topology changes might improve Boolean robustness in a future refactor, but Boolean had not yet been updated. The 0.7.0 notes also record a fix for same-circle, opposite-direction arc endpoint contact, which the probe rechecks in both argument orders. Treat tangent and degenerate Boolean cases as a known risk rather than inferring robustness from the offset work. [Release notes](https://github.com/jbuckmccready/cavalier_contours/releases)

## Bounded probe

The isolated probe pins the dependency and checks exact bulge circles, a polygon with a circular hole, a circle cut through a wall, a boss union plus hole, disconnected results, coincident circles in both directions, opposite-direction arc endpoint contact, nearby distinct arcs, and an external tangent. It compares each area to analytic circle, circular-segment, or polygon expressions and counts retained bulges to check that arcs remain analytic. It also checks open-input rejection, a caller-side finite-value guard, a tiny-scale ring with an explicitly tighter epsilon, and a translated ring.

Native run and `wasm32-unknown-unknown` compile both pass using Rust/Cargo 1.98.0. The exact tangent-circle union is the important failure witness: the library returns one closed positive loop with four exact arcs and area `2π`, but the loop repeats the tangent point and `scan_for_self_intersect()` reports `true`. A closed/finiteness check alone would accept this invalid contour. A circle subtraction returns a positive-area `neg_plines` ring, confirming that negative means subtractive role, not clockwise orientation. Coincident union and XOR behaved as expected in these fixtures. Native log (`../../../.evidence/layered-case-013/contours-eval/logs/native-run.log`), WASM compile log (`../../../.evidence/layered-case-013/contours-eval/logs/wasm-check.log`)

The `2e-5` concentric gap passed at the origin and at `(10000,-10000)` with the default epsilon; the translated area check used `2e-8` absolute tolerance. A ring of radius `1e-6` passed with explicit epsilon `1e-12`. These are bounded witnesses only, not a scale-invariance guarantee. The implementation constructs vertices without validating finiteness, and its Boolean gate rejects fewer than two vertices or open polylines but does not reject non-finite coordinates/bulges. Callers must validate finite values, closure, vertex count, bulge bounds, and self-intersection before Boolean calls. The API also documents a panic if its numeric type cannot be cast to/from `u16` for spatial indexing. [Tagged Boolean implementation](https://github.com/jbuckmccready/cavalier_contours/blob/0.9.0/cavalier_contours/src/polyline/internal/pline_boolean.rs), [polyline API implementation](https://github.com/jbuckmccready/cavalier_contours/blob/0.9.0/cavalier_contours/src/polyline/traits.rs)

Probe sources, lockfile, logs, and SHA-256 manifest are in `.evidence/layered-case-013/contours-eval/`. No production code, dependency manifest, 3D Boolean, or layered B-rep assembly was evaluated here.

## Recommendation

Proceed only with the bounded private prototype, pinning 0.9.0 and preserving every native result contour. Require caller validation and reject self-touching outputs, ambiguous hole ownership, and unsupported multi-ring composition. Exact line-and-arc operations and WASM compilation work for the cases probed, but tangency creates a self-touching path; the library only offers pairwise Boolean calls; role is separate from winding; and epsilon is absolute. This evidence is not enough to recommend adoption by ticket 006 for production cases.

## Sources

- [Published `cavalier_contours` 0.9.0 crate](https://docs.rs/crate/cavalier_contours/0.9.0)
- [`PlineSource` Boolean API](https://docs.rs/cavalier_contours/0.9.0/cavalier_contours/polyline/trait.PlineSource.html)
- [`BooleanResult` result contour fields](https://docs.rs/cavalier_contours/0.9.0/cavalier_contours/polyline/struct.BooleanResult.html)
- [`PlineBooleanOptions` and epsilon behavior](https://docs.rs/cavalier_contours/0.9.0/cavalier_contours/polyline/struct.PlineBooleanOptions.html)
- [Tagged 0.9.0 release notes](https://github.com/jbuckmccready/cavalier_contours/releases/tag/0.9.0)
