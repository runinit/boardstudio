# Vendored source provenance

This is the published crates.io `monstertruck-io` version `0.4.1` package.
Its manifest declares Apache-2.0 and attributes Yoshinori Tanimura and Moritz
Moeller; the Apache License, Version 2.0 is included as `LICENSE-APACHE`.

The source archive `monstertruck-io-0.4.1.crate` has SHA-256
`b9db95ba8ed9ba913084f04e865b8e522813f9885f40497b8717ede6c6f1542e`.
The nullable-description candidate is `.evidence/monstertruck-qualification/corpus-inventory/lcd-diagnosis/qualification-change.diff`,
SHA-256 `b5b144885ed571c98893f67c16f3ed88c0e14cf67c73f546deea020456b9e057`.
It changes `src/step/load/step_types/product.rs` so the EXPRESS optional
`PRODUCT_DEFINITION.description` accepts `$`, and adds an inline minimal STEP
regression to `tests/corpus_conversion_census.rs` that checks null and populated
descriptions survive parsing and assembly conversion.

The qualification candidate also applies the reviewed direct-edge B-spline
parser hardening from
`.evidence/monstertruck-qualification/knot-diagnosis/parser-hardening-active.diff`
(SHA-256 `cde84fdf052ae49e7dfd2130f907d9b76d95998b39b0334220653b4e9a6e5166`).
It confines normalization and malformed-knot guards to direct `EdgeCurve`
tiny-span splines; generic conversion and wrapper curves remain unchanged. The
final topology source SHA-256 is
`7e213fff7b7551e47c53501a7ad8392f979204924316ed9dd3d2dc1a8e2d1dc3`. The
reviewed increment adds strict knot-order and multiplicity upper-bound guards
with two regressions; the complete focused module has 17 passing tests.

The published crate archive does not contain upstream `rustfmt.toml`. The
added parser code was checked with the upstream root formatter config from
revision `d2ad1ed7aa420b7f32cd2c1e3ad319b46d593652`, using nightly rustfmt;
the pristine published topology passes the same check. Formatting only the
added helper and tests produces the reviewed whitespace delta at
`.evidence/monstertruck-qualification/vendor-integration/knot-candidate/fmt-compare/added-code-format-only.diff`
(SHA-256 `1f7d207204479155b242539800bac93fd45943d857a272468769a0b047a1ae39`).

This local patch exists only in the standalone qualification workspace. It does
not change the production CAD dependency graph.

The original package test sources are retained. The
qualification addition is a workspace-member/path patch so this regression
and the existing mandatory-ID control can be run against this exact vendored
source using the qualification lockfile.

Existing IO and meshing tests refer to fixtures from the `ricosjp/resources4truck`
submodule. The qualification worktree uses a sparse checkout at
`vendor/resources/`; `/vendor/resources/` is ignored by
`cad/monstertruck-qualification/.gitignore`, so these assets and the checkout's
`.git` metadata are not part of the qualification source snapshot. The selected
11 files match the tree at commit `9bf9de1426c5fc2f2b8d63f501dca0d3c53ebd91`:

| Path | Size | Git blob | SHA-256 |
| --- | ---: | --- | --- |
| `obj/by_occt.obj` | 33,602 bytes | `8694d01e7972fabe5dad09385451d720cbb01309` | `1dca2f98a72ba97fd8bcb9322a9108250789c5b62fde031aa9df71dcd8d8195e` |
| `shape/bottle.json` | 86,023 bytes | `31e46eaeff551d96304b7bca58e34f87f5087b91` | `89e0860d7368fc14c27ac647eef1266dda343e517b75e1ba5f451e3e9a9c52ef` |
| `shape/cube-in-cube.json` | 21,736 bytes | `17e85ed94a476409e1f7a099aaa63265a3a932d2` | `9ccfbeec8ade2f23032a642f2cf58c19251dcff7c60ca872dbdb39eeb6923b9d` |
| `shape/cube.json` | 10,886 bytes | `2e2743a7d7a090538738328fb09c14e5952d999b` | `61d05ae8c3626374da167287ffd692b83008efc592cbc861453e3a4027192763` |
| `shape/large-torus.json` | 25,164 bytes | `4ce39ac0aa999cfdc6a12610a38fc5d2d34770b3` | `8199da9fe0b85722862ea1c4bda22f58f765e29c6b878c76a4e5e9d6354db781` |
| `shape/punched-cube.json` | 47,631 bytes | `3bc91d124bb7ddc2f690343a2fdc896916e68185` | `cbcde0f060fbbe2abe5f6c342753db245550b93c45a58d4ea33abe054ecd78e2` |
| `shape/sphere.json` | 76,041 bytes | `032c37f47ee8162bef7936603e01b3cb01b74372` | `3792e05c1ed3d279fd6af95c52f40cacdb174c5b8f82c446039a677434db07e5` |
| `shape/torus-punched-cube.json` | 27,072 bytes | `0899b36abed16e443930cce6d402d87e29c1b4db` | `b26926a118367e22e8e7d80db7081f8a7a674d1ec4cf4d033e6f28833549ceef` |
| `shape/torus.json` | 25,206 bytes | `bb1d052892378706a8e10afbf90c1c94d5a96f3f` | `8551b57f7ac25cfa8095355236bb7a5106040a956208fdff3af80cccb6fd04c7` |
| `step/occt-cylinder.step` | 6,128 bytes | `2a4b6c15967cd5ac86e47627c65eac5fde5b428e` | `211ef20838894225f3b0c18c21ad20c16c199f69f3e7c6d0fdc91d3d27d7078e` |
| `step/occt-torus.step` | 2,332 bytes | `e783e03d41ebdfeecad49eb7b6198c4e47e8bdc1` | `9f20f54fa321f9224f7557dfb2eb242415ca90ba581aec6dc90ea9ba6e0778b5` |

The pinned resources4truck tree contains no `LICENSE` or `COPYING` file. The
parent `ricosjp/truck` source checkout has an Apache-2.0 license, but that does
not establish a license grant for assets in its submodule. No separate license
grant for these fixture assets has been identified. The crate's
`LICENSE-APACHE` remains provenance for the published IO crate, not a claim
about the fixture license. Keep the local resources checkout ignored and do
not include these assets in a source distribution until their license is
resolved. The backup of the seven files previously copied here is preserved
under ignored `.evidence/monstertruck-qualification/vendor-integration/`.

The qualification candidate also applies the reviewed scale-safe STEP placement
conversion changes from
`.evidence/monstertruck-qualification/placement-integration/cumulative-v4/placement-cumulative-v4.patch`
(SHA-256 `f38841968966b551334120c5d5259c708a1d9818e91752c0383e458eacd6a5f0`).
The cumulative patch is based on the current IO shared-source hashes recorded
in its handoff and includes regressions for omitted reference directions,
scale-safe normalization, explicit invalid directions at the generic fallible
`Matrix4` conversion boundary, and actual Circle conversion. It preserves the
STEP default reference orientation, rejects explicit parallel or near-parallel
frames at fallible conversion paths, and retains existing analytic-surface
caller guards where conversion is infallible. It is qualification-only and
makes no production dependency or API change.

The qualification-only unit normalization seam adds
`ItemDefinedTransformation::normalized_matrix_mm` to compose independently
scaled child/parent frame origins. The typed unit resolver and shell/plane
PCURVE normalization live in the qualification harness, not the IO library.
Their reviewed candidate is
`.evidence/monstertruck-qualification/step-units-placement/normalization/consolidated-v2/REPORT.md`
(SHA-256 `ab9a07a13dab145b9c2f61dac446907ef7b3453461a9dcd5d865e8e923834eab`).
That historical snapshot rejected missing units. The current qualification
adapter supersedes it with the reviewed `consolidated-v3` policy: absent length
units default to millimetres, while malformed contexts and incomplete source
occurrence mappings reject. This follows the approved replacement policy;
production still uses Cadrum.

The qualification harness now applies the reviewed `angle-unit-v2` candidate
from `.evidence/monstertruck-qualification/step-units-placement/normalization/`.
It normalizes represented cone semi-angles from their owning STEP context
before constructing the table, preserving UTF-8 and the existing Latin-1
fallback. The vendor's `ConicalSurface` converter therefore consumes radians
without the former magnitude-based degree guess. Direct table readers must
perform the same context normalization; the separate browser experiments have
not yet adopted it. This does not add general angle support to the IO API.

The reviewed sphere STEP writer patch is
`.evidence/monstertruck-qualification/dish-analytic-step-guard/revision3.patch`
(SHA-256 `813ac04310c69ab3215b78b6ec1728ef0ce16f181fd4cb1a003ba59bc4883d21`).
It transforms sphere centers correctly, emits analytic spheres for guarded
proper similarities, and uses the existing NURBS fallback otherwise, including
consistent entity counts. The similarity error estimate uses floating-point
arithmetic, not a certified bound. Persistent tests cover raw and wrapped
writers; this does not qualify dish trim accuracy or general STEP parity.

The qualification candidate corrects the STEP sphere's general mixed derivative
to follow its parameter mapping `P(u,v) = G(pi/2-v,u)`: derivative orders swap
and the sign depends on the order in `v`. Four regressions compare named
partials, finite differences, and higher mixed derivatives. Source and test
evidence is retained under
`.evidence/monstertruck-qualification/vendor-integration/sphere-derivative-v6/`.
This does not change sphere trim conversion, inverse hints, or pole meshing.

The subsequent reviewed sphere-hint-v7 correction maps inverse-search hints
from the STEP chart to the geometry sphere chart before delegation. Four
regressions cover exact/nearest periodic branches and their distinct pole
behavior; a fifth checks Range ordering and None passthrough. The current
underlying solver ignores Range hints, so that test checks mapping only.
Evidence is under
`.evidence/monstertruck-qualification/vendor-integration/sphere-hint-v7/`.

## Reviewed sphere pole qualification

Preserves a sole analytically classified spherical pole VERTEX_LOOP as an empty bound and declares the STEP sphere singularities. Current sphere derivative and inverse-hint corrections remain included. Reviewed combined source and controls: `.evidence/monstertruck-qualification/step-sphere-vertex-loop-v1/revision-v7-combined-current/`, freeze SHA-256 `f9cb21e0afd40c950452908e407445d8181f64a7a5c8612585c4dd2ed1cf588a`. Bounded qualification only; full STEP parity is not established.

## Tiny STEP composite on sphere-v8

Restricts endpoint-derived pcurve trimming to the proved affine cylinder-generator case; nonlinear, projective and ambiguous precision cases retain the fallback. The independently reviewed six-file candidate is `.evidence/monstertruck-qualification/normalized-cdt-sphere-v8-combined-v1/`; report SHA-256 `dd297d2451e73f6b1661e7ed2191f0f575b3b583520fc5ac7ddf5c79d7372d20`, artifact ledger `d9a587883e5ace2a587d02c5764264df73c9e457af5d45155e8582661c591989`. PICO geometry restoration is bounded evidence; its fixed-quantization strict gate still rejects.

## STEP sphere linear deflection

The reviewed sphere-linear-deflection-v2 candidate splits the radial sag allowance between both parameter axes, using a stable asin formula. It preserves the upstream minimum-tolerance assertion and STEP axis remapping for delegated cases. This changes only the STEP sphere wrapper; geometry Sphere and generic surface division remain unchanged. Focused baseline-red/candidate-green tests and five whole-facet probes are frozen in `.evidence/monstertruck-qualification/sphere-linear-deflection-v2/`. Linear-deflection evidence does not establish angular or mass-property accuracy, translated STEP behavior or all surfaces.

## Public STEP assembly writer

The reviewed public StepModel/StepDesign seam retains the source geometry borrow and delegates indexed formatting to existing solid/shell writers. It adds no public member/type visibility; generic constructors now require formatting trait bounds. Provenance: `.evidence/monstertruck-qualification/current-monstertruck-writer-v7/`, with persistent six-variant regressions and corrected focused evidence under `current-monstertruck-writer-v7-test-supplement/correction-final-rerun/`. The old supplement focused log is superseded there. This enables public assembly serialization, not independent full STEP validity or production export qualification.

STEP-v27 adds source-bound immutable checked shell/solid conversion and normalized closure certification. Exact endpoint selectors permit the reviewed unclamped source class; finite knot arithmetic is required. Scale transport binds source and transformed geometry, then re-proves actual coefficients. Evidence: `.evidence/monstertruck-qualification/step-certified-combined-v1/`; composed reviewed patch SHA256 `c661ecb3e164d359b4eb309da289f7f55834ee6adc332f79bbf0d6e913430e46`. Full-file loss and mesh gates remain required; closure certification alone does not accept a solid.

Choc-v41 integrates the reviewed finite-boundary coefficient proof and ordinary inferred-chart route, preserving successful aligned source samples before chart reconstruction. This is finite boundary equality, not periodicity or general surface injectivity. Final reviewed production patch SHA256 `2769aef56108ac12a86b8d2047cc8df976dcfdd3eb9263103122f79e5e1b4d1c`; final private source manifest `f2b66f3445084aacec04adab536bc78cc6919aaba0071caf1fb0bb69a7965bbe`. Two adjacent trait/derive capability additions are separately reviewed with Plane-v40 (composition patch `77697632c0345708a50e7641e82369826a7e5a3c211bffdd6b7365dba7b13c25`). Frozen evidence: `.evidence/monstertruck-qualification/choc-default-incidence-proof-v1/`; shared combined checks: `vendor-integration/choc-v41/`. Existing rejected heuristics and endpoint-loss predecessors remain rejected; whole STEP/feature acceptance and OCCT/C++ removal remain open.

Native Plane conversion retains the authored Axis2Placement matrix on a centered unit plane, preserving small native axes that world-origin addition previously rounded away. The three qualification normalization paths share a frame-preserving helper while UV lengths scale separately. The public large-origin source-equation and face/association normalization regressions and source-bound checks are recorded in `.evidence/monstertruck-qualification/native-plane-frame-v1/`; full STEP and production acceptance remain separate.

Import-only active-view integration (v48): reviewed context/strict-source producer prerequisites plus selected-master scalar views preserve source curves, ordered associations and source-unit normalization. Current-root cumulative13-file runtime patch SHA256 `8a0fec63554013a1a9ac3fb287e591e3202867ac5b0be097d58626ba58987dc4`; regression patch `ae36ed720e72a0e5fa24bce20c6b70e9a94218996c9fdde187c866e86775c3ed`. Evidence `.evidence/monstertruck-qualification/intersection-cut-step-roundtrip-v1/vendor-import-only-v2/review-v1/`; independent `import-only-active-view-{spec,standards}-review/`. The reviewed constructor documentation correction is comment-only (`00f31d7be21bf2f44a49f2bea92609d81bd22d2026898801ebba83f3c7c97edf`). This does not qualify all closed support families, real vendor completeness or the007 application adapter. Inherited table/Clippy limitations remain recorded in the handoff.

## Reviewed source-bound partial closed splines (2026-10-17)

Partial-closed-spline-v50 adds a private sufficient interval proof for validated finite polynomial splines already divided into Bezier spans. It retains every source join alias until geometry order selects a unique interval, authenticates original degree/knot/control metadata for this added admission, and refuses unsupported native-label reversal. Existing full-span, monotone, normalized-narrow and rational behavior remains unchanged. No source geometry, source uncertainty or caller repair budget is rewritten. Production patch SHA-256 `d3cbae01744bf947ff5646bc4fbaf8477b20c2b6aba68b9271b10a81ab682e4b`; regression patch `3b7eda8b6532e6b20e011cd871803e3c44fbb864362e5dd62d2a4fc152d14f3c`. Frozen source and independent reviews: `.evidence/production-step-007/partial-closed-spline-v1/`; actual shared receipts: `.evidence/monstertruck-qualification/vendor-integration/partial-closed-spline-v50/`. Conversion completeness is not a full mesh, trim-coverage or whole-file import certificate.
