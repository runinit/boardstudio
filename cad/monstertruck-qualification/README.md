# Monstertruck backend qualification

This standalone crate qualifies the CPU-only Rust dependency slice proposed for
the CAD backend replacement. Its four Monstertruck dependencies are pinned to
published version `=0.4.1`; before applying the qualification-only patches
listed below, their published `src/` trees were compared byte-for-byte with the
checkout at upstream revision `d2ad1ed7aa420b7f32cd2c1e3ad319b46d593652`.
For Boolean refinement qualification, the published `monstertruck-traits 0.4.1`
package is vendored with the independently reviewed surface-axis refinement
patch. The vendor source provenance, checksum, patch hash, and exact changed
files are recorded in [SOURCE.md](vendor/monstertruck-traits/SOURCE.md). This
patch is scoped to this standalone qualification crate; it does not alter the
production CAD dependency graph.

The qualification workspace also vendors reviewed candidate changes to
`monstertruck-meshing 0.4.1` and `monstertruck-io 0.4.1`. Their provenance,
archive checksums, patch hashes, and manifest metadata delta are recorded in
their respective [meshing SOURCE.md](vendor/monstertruck-meshing/SOURCE.md)
and [IO SOURCE.md](vendor/monstertruck-io/SOURCE.md). These are isolated
qualification candidates, not production integration or parity claims.
The meshing vendor also includes the independently reviewed co-sample pairing
refinement recorded in its provenance note.

For Boolean endpoint, conversion and SSI-closure diagnosis, the qualification
workspace also vendors the published `monstertruck-solid 0.4.1` archive with
three reviewed local patches. Their archive, patch, license and formatting provenance is in
[vendor/monstertruck-solid/SOURCE.md](vendor/monstertruck-solid/SOURCE.md). The
endpoint fix checks exact edge endpoints before approximate curve search and
keeps the existing tolerance; coincident endpoints still select the first
matching edge. The converter fix retains finite, distinct sub-tolerance samples
for the existing degree-1 intersection curve while removing only consecutive
exact duplicates. It does not drop short edges or alter generic tolerances. These
are narrow qualification candidates; they address observed failures but do not
make the Boolean gates pass. The SSI-closure candidate derives a closure flag
from exact equality of the emitted polyline's first and last coordinates, then
uses that flag during refinement and loop storage. It cannot recover endpoints
already merged into one legacy `PointIndex` cell. The current solid test suite
passes 19 unit tests, 4 Boolean smoke tests and 1 doctest.

The direct aliases are `monstertruck-modeling`, `monstertruck-solid`,
`monstertruck-meshing`, and `monstertruck-io`. Defaults are disabled; selected
features are `marching-ssi`, `tessellation` + `analyzers`, and `step`.
`i_overlay = 9.0.0` is included for the support-footprint qualification gate.

From the repository root, record compilation logs under the ignored evidence
directory and keep Cargo artifacts there:

```sh
mkdir -p .evidence/monstertruck-qualification/candidate-build \
  .evidence/monstertruck-qualification/vendor-integration
export CARGO_TARGET_DIR="$PWD/.evidence/monstertruck-qualification/target"
set -o pipefail
timeout 30m cargo build --manifest-path cad/monstertruck-qualification/Cargo.toml --lib \
  --locked 2>&1 | tee .evidence/monstertruck-qualification/candidate-build/native-build.log
timeout 30m cargo build --manifest-path cad/monstertruck-qualification/Cargo.toml --lib \
  --target wasm32-unknown-unknown \
  --locked 2>&1 | tee .evidence/monstertruck-qualification/candidate-build/wasm-build.log
```

These commands build the `rlib` dependency graph. The wasm build does not create
the application's browser `cdylib`, run wasm-bindgen, or execute in a browser;
it establishes target compilation only. No `RUSTFLAGS` or wasm-specific Cargo
config was needed in the recorded build. The repository Cargo config sets job
count and wasm test timeout only. `set -o pipefail` keeps a failed Cargo command
visible in the recorded log pipeline. Compile the gate binaries separately so
build time and Cargo lock waits are not mistaken for geometry timeouts:

```sh
timeout 30m cargo build --locked --release \
  --manifest-path cad/monstertruck-qualification/Cargo.toml \
  --bin boolean_gate --bin volume_gate
timeout 30m cargo build --locked \
  --manifest-path cad/monstertruck-qualification/Cargo.toml \
  --bin step_gate --bin accuracy_gate
```

Then run the gates independently. The commands capture their stdout/stderr;
the binaries do not create log files themselves. Nonzero status means the gate
is unmet, including GNU `timeout` status 124. With the reviewed
surface-refinement candidate, both Boolean gates return within 15 seconds but
fail their geometry checks: the simple round cut reports
`NotClosedShell`, and the exact fixture reports `EmptyOutputShell`. STEP exits 1
on failed files, and accuracy exits 2 for unqualified parity.

```sh
timeout --signal=TERM 15s "$CARGO_TARGET_DIR/release/boolean_gate" \
  2>&1 | tee .evidence/monstertruck-qualification/candidate-build/boolean-gate.log
timeout --signal=TERM 15s "$CARGO_TARGET_DIR/release/boolean_gate" --simple-round-only \
  2>&1 | tee .evidence/monstertruck-qualification/candidate-build/simple-round-gate.log
timeout 3m "$CARGO_TARGET_DIR/debug/step_gate" \
  2>&1 | tee .evidence/monstertruck-qualification/candidate-build/step-gate.log
timeout 30s "$CARGO_TARGET_DIR/debug/accuracy_gate" \
  2>&1 | tee .evidence/monstertruck-qualification/candidate-build/accuracy-gate.log
timeout 30m "$CARGO_TARGET_DIR/release/volume_gate" \
  2>&1 | tee .evidence/monstertruck-qualification/vendor-integration/volume-gate.log
```

The STEP gate defaults to four paths in the checked-in vendor assets and accepts
explicit file paths as arguments. It is a bounded diagnostic importer, not the
production import adapter. The accuracy gate retains measured old-backend
values; running it does not require the old C++ oracle. See the
[qualification findings](../../docs/research/cad/monstertruck-qualification-2026-10-16.md)
for the limits of each result and the next implementation gates.

To exercise checked source-certified STEP meshing, pass
`--source-certified-meshing`; optionally add
`--edge-angular-tolerance=0.5` (finite positive radians). The angular option
requires the checked route. Linear tolerance remains 0.0005 mm after the existing
unit normalization. Conversion must be lossless, and closure certificates bind
the actual normalized surfaces. Whole-file completeness and mesh checks still
apply; a surface certificate alone does not establish a valid solid. Without
these flags, the existing gate route is unchanged.

`volume_gate` is a fixture-only empirical signed-volume check. It tests
origin/translated cylinders, square and annular holes, a deliberately
inward-oriented negative frustum, and the corrected frustum at mesh tolerances
`1e-3` and `1e-4`; it asserts signed analytic-oracle error and cross-tolerance
convergence for those fixed solids. It accepts no fixture paths or other CLI
input and does not qualify a general B-rep volume API or production parity.
Quadrature convergence is empirical, not a certified error bound. The current
fixed solids use regular plane parameterizations; singular/degenerate plane
parameterizations are not covered and can panic in the upstream `Plane::parameter`
operation. A malformed `PLANE_EDGE_INTERVALS` value falls back to the default
32 subdivisions, and the plane/faces trace environment variables only add
diagnostic output. The five local unit tests include malformed/non-finite,
open-boundary and valid skinny-triangle controls. Run them serially with the
qualification candidate:

```sh
cargo test --locked --offline --manifest-path cad/monstertruck-qualification/Cargo.toml \
  --bin volume_gate -- --test-threads=1
```

Under the integrated traits, meshing and IO candidate, `volume_gate` completed
all 20 signed tolerance evaluations; the largest absolute analytic-reference
error was `1.19e-11 mm³`, below the fixture threshold of `1e-6 mm³`. The inward
frustum remains negative in both translated and origin runs. These measured
values and successful checks are limited to the listed fixtures.

Use the same `CARGO_TARGET_DIR` export for gate runs. To inspect the actual
resolved Rust dependency graph, including build and target-specific edges:

```sh
cargo tree --locked --manifest-path cad/monstertruck-qualification/Cargo.toml -e all
cargo tree --locked --manifest-path cad/monstertruck-qualification/Cargo.toml --target wasm32-unknown-unknown -e all
cargo tree --locked --manifest-path cad/monstertruck-qualification/Cargo.toml --target all -i cc -e all
```

The native Linux and wasm dependency trees contain no C/C++ CAD packages. The
all-target graph includes `cc` only beneath the Haiku-specific `iana-time-zone`
build dependency; it is not reachable for either built target. Resolution and
successful Rust compilation do not establish that the backend matches
BoardStudio behavior. The gates cover only the fixtures and contracts they
name; they are not production integration or browser-worker qualification.

The vendored trait crate tests use shared process-wide division counters and
must run serially. Run them as a workspace member so they use the qualification
lockfile and local vendor source:

```sh
cargo test --locked --offline \
  --manifest-path cad/monstertruck-qualification/Cargo.toml \
  -p monstertruck-traits -- --test-threads=1 \
  2>&1 | tee .evidence/monstertruck-qualification/candidate-build/traits-tests.log
```

Run the vendored solid library tests serially with the qualification lockfile:

```sh
cargo test --locked --offline --manifest-path cad/monstertruck-qualification/Cargo.toml \
  -p monstertruck-solid --lib -- --test-threads=1
```

With the endpoint-priority patch integrated, the simple round Boolean returns
`InvalidOutputShell(NotClosedShell)` in 4.658 seconds and the exact gasket cut
returns `EmptyOutputShell` in 2.064 seconds. The box control passes at
0.875 mm³ and the profile-hole extrusion passes at 13,904 mm³. The exact-endpoint
classification regression is fixed, but circular subtraction remains
unqualified because both Boolean gates still fail their shell checks. The run
logs are under ignored
`.evidence/monstertruck-qualification/vendor-integration/endpoint-candidate/`;
these results do not establish production parity.

The combined reviewed traits and meshing candidate passes the LCD and
TrackPoint STEP controls, while THQ reports 1,809 invalid mesh edges and MX
reports 644; the gate exits 1 as expected. With direct-edge parser hardening,
THQWGD001C 2-pin and 4-pin inputs convert without entity or conversion loss
(11 solids each); both still fail the mesh gate with 22,098 invalid and one
degenerate mesh edge. The reviewed IO candidate preserves
optional `PRODUCT_DEFINITION.description` values, including `$`, through
assembly conversion. The IO topology parser candidate adds narrowly scoped
direct-edge B-spline guards for malformed knot schemas; 17 focused regressions
pass while generic curve conversion and wrapper paths remain unchanged. The exact
patch and source hashes are recorded in the IO `SOURCE.md`. Focused IO and
meshing test logs are under the ignored
`.evidence/monstertruck-qualification/vendor-integration/` directory. The
earlier five-fixture gate run is logged at
`vendor-integration/step-gate-lcd-plus-4.log`; the latest seven-fixture run
after parser hardening has per-file logs and a status index under
`vendor-integration/knot-candidate/`. The recorded checks passed native and
`wasm32-unknown-unknown` library builds, qualification-package formatting, and
workspace all-target Clippy with
warnings denied. Serial tests passed: traits 23 passed / 1 ignored; the full
default-feature meshing suite had 39 library tests passed / 1 ignored, 12
analyzers, 2 closed-trim, 9 filters, 12 tessellation and 22 doc tests passed.
The VTK integration target is feature-disabled. Solid library tests passed
14 / 14; selected IO tests passed: library 51 passed / 9 ignored, corpus
conversion census 9 passed / 1 ignored, and output 20 passed. The parser-specific
subset passed 17 / 17. The latest short-edge converter checks are under the
ignored `vendor-integration/converter-candidate/` directory.
Ignored tests are upstream benchmark or large-corpus measurement harnesses.
The Boolean
gates still fail their shell checks; these results remain an isolated candidate
for further fixes, not production qualification.

Reproduce the integration checks with a private artifact directory:

```sh
export CARGO_TARGET_DIR="$PWD/.evidence/monstertruck-qualification/vendor-integration/target"
cargo test --locked --offline --manifest-path cad/monstertruck-qualification/Cargo.toml \
  -p monstertruck-meshing --lib -- --test-threads=1
cargo test --locked --offline --manifest-path cad/monstertruck-qualification/Cargo.toml \
  -p monstertruck-io --lib -- --test-threads=1
cargo test --locked --offline --manifest-path cad/monstertruck-qualification/Cargo.toml \
  -p monstertruck-io --test corpus_conversion_census -- --test-threads=1
cargo test --locked --offline --manifest-path cad/monstertruck-qualification/Cargo.toml \
  -p monstertruck-io --test output -- --test-threads=1
cargo clippy --locked --offline --manifest-path cad/monstertruck-qualification/Cargo.toml \
  --workspace --all-targets --no-deps -- -D warnings
```

Before running fixture-dependent IO or meshing tests in a fresh worktree,
populate the ignored local fixture checkout. The qualification
`.gitignore` excludes `vendor/resources/`; the sparse checkout pins the exact
submodule commit and materializes the 19 required files. The resources
tree has no license file; see the IO `SOURCE.md` before distributing any
fixture assets.

```sh
resources=cad/monstertruck-qualification/vendor/resources
resources_commit=9bf9de1426c5fc2f2b8d63f501dca0d3c53ebd91
git clone --filter=blob:none --no-checkout --depth=1 \
  https://github.com/ricosjp/resources4truck.git "$resources"
git -C "$resources" fetch --depth=1 origin "$resources_commit"
git -C "$resources" sparse-checkout init --no-cone
printf '%s\n' \
  'obj/by_occt.obj' \
  'shape/bottle.json' \
  'shape/cube-in-cube.json' \
  'shape/cube.json' \
  'shape/large-torus.json' \
  'shape/punched-cube.json' \
  'shape/sphere.json' \
  'shape/torus-punched-cube.json' \
  'shape/torus.json' \
  'step/occt-assy.step' \
  'step/occt-cone.step' \
  'step/occt-cube.step' \
  'step/occt-sphere.step' \
  'step/abc-0000.step' \
  'step/abc-0006.step' \
  'step/abc-0008.step' \
  'step/abc-0035.step' \
  'step/occt-cylinder.step' \
  'step/occt-torus.step' | git -C "$resources" sparse-checkout set --no-cone --stdin
git -C "$resources" checkout --detach "$resources_commit"
test "$(git -C "$resources" rev-parse HEAD)" = "$resources_commit"
while IFS= read -r path; do
  expected=$(git -C "$resources" rev-parse "$resources_commit:$path")
  actual=$(git -C "$resources" hash-object "$path")
  test "$actual" = "$expected"
done <<'EOF'
obj/by_occt.obj
shape/bottle.json
shape/cube-in-cube.json
shape/cube.json
shape/large-torus.json
shape/punched-cube.json
shape/sphere.json
shape/torus-punched-cube.json
shape/torus.json
step/occt-assy.step
step/occt-cone.step
step/occt-cube.step
step/occt-sphere.step
step/abc-0000.step
step/abc-0006.step
step/abc-0008.step
step/abc-0035.step
step/occt-cylinder.step
step/occt-torus.step
EOF
```

The hash check compares each local file's Git blob ID with the tree at the
pinned commit. `vendor/resources/` is local test data, not part of the source
snapshot or its inventory. The pinned resources tree has no `LICENSE` or
`COPYING` file; no separate license grant for these fixtures has been
identified. See the IO `SOURCE.md` for exact sizes and SHA-256 values.

Run the direct-edge parser regressions serially with the qualification
workspace lockfile:

```sh
cargo test --locked --offline --manifest-path cad/monstertruck-qualification/Cargo.toml \
  -p monstertruck-io --lib edge_normalization_tests -- --test-threads=1
```

The reproducible IO test commands are:

```sh
cargo test --locked --offline --manifest-path cad/monstertruck-qualification/Cargo.toml \
  -p monstertruck-io --test corpus_conversion_census \
  optional_product_definition_description_preserves_null_text_and_assembly_node \
  -- --exact --test-threads=1
cargo test --locked --offline --manifest-path cad/monstertruck-qualification/Cargo.toml \
  -p monstertruck-io --test corpus_conversion_census \
  swallowed_entity_records_are_counted_and_the_null_label_family_now_loads \
  -- --exact --test-threads=1
```

The latest qualification-only solid candidate also includes the reviewed
topology-aware SSI closedness patch. In the classic intersector, closure is
derived by exact equality of the emitted polyline's first and last coordinates;
the resulting flag is passed to endpoint refinement and loop storage. It does
not recover distinct endpoints that the legacy tolerance-sized `PointIndex`
bucket has already merged. The corrected near-open graph control straddles
adjacent cells. Exact provenance and the same-cell limitation are in
the solid vendor [SOURCE.md](vendor/monstertruck-solid/SOURCE.md).

For this candidate, the full `monstertruck-solid` suite passed 19 unit tests, 4
Boolean smoke tests, and 1 doctest; workspace Clippy with warnings denied, the
native Boolean-gate binary build, the wasm32 library build, and rustfmt checks
on the three changed solid files passed. The workspace-wide rustfmt check still
was not authoritative because stable rustfmt ignored the upstream nightly-only
configuration. The built Boolean gate passes its box and
profile-hole controls but still exits 1 with `EmptyOutputShell` on the exact
regression difference. Evidence, source inventories, and the separately
reviewable formatter-only delta are under
`.evidence/monstertruck-qualification/vendor-integration/closedness-candidate/`.
This candidate is not a production integration or full Boolean qualification.

The STEP qualification gate now normalizes directly owned solids to millimetres
using typed representation units, normalizes assembly frame origins in their
respective units, and handles attached and direct planar PCURVE charts. Absent length units default to millimetres; malformed or conflicting declarations
remain errors. The gate checks source occurrence identity, supported PDS/CDSR
placement links, and graph traversal coverage to reject incomplete assembly
imports. Conflicting or indirect solid ownership remains unsupported. These
checks are qualification code, not the production import adapter.
The IO candidate includes the guarded sphere writer and its NURBS fallback.
Trimmed-CDT triangle mode preserves its UV producer winding, with maintained
sphere regressions. See the IO and meshing vendor `SOURCE.md` files for reviewed
patch provenance. These changes do not clear the remaining Boolean, mesh
accuracy, or complete STEP compatibility gates.

## Sphere pole regression

The qualification workspace vendors geometry and derive forwarding for the reviewed default-none analytic singularity capability. A sole spherical pole VERTEX_LOOP is retained by the STEP converter, and explicit pole rows mesh as fans. These changes preserve the sphere derivative and inverse-hint fixes. Provenance is in each vendor SOURCE.md; fixtures are in `fixtures/sphere-pole/`.

```sh
cargo run --locked --release --manifest-path cad/monstertruck-qualification/Cargo.toml --bin sphere_vertex_loop_gate -- cad/monstertruck-qualification/fixtures/sphere-pole/v3/base-radius-5-pole.step
```

This checks the preserved radius-5 fixture, not general STEP or browser parity.

Tiny STEP regression fixtures are retained in `fixtures/tiny-cylinder/`. The reviewed normalized-CDT and affine cylinder-pcurve repair restores the PICO caps and scaled faceted volume while retaining the strict gate's quantization refusal; NANO/MILLI are controls. This does not qualify arbitrary-scale geometry.

## Exact collision qualification

The reviewed collision candidate uses outward interval rejection with exact rational fallback, a finite-coordinate sweep and exact endpoint-distance ordering. It fixes the reproduced oblique boundary miss and tiny-scale endpoint collapse without coordinate welding. The final rebased source is frozen in `.evidence/monstertruck-qualification/astra-collision-integration-v2/` (source inventory `d96077094eb10a7f83226541804896bb6124403c14e9c1c961201f295ffce437`). Rational arithmetic is a runtime dependency; robust predicates are used only by test diagnostics. The scratch candidate passed 68 meshing tests (1 ignored) and the WASM library check. Full production throughput, WASM runtime and complete Boolean qualification remain open.

The STEP sphere linear-deflection probe checks whole-facet radial plane bounds; angular quality and volume are diagnostics only. Run `cargo run --locked --release --manifest-path cad/monstertruck-qualification/Cargo.toml --bin sphere_deflection_probe -- cad/monstertruck-qualification/fixtures/sphere-pole/v3/base-radius-5-pole.step 0.1`. The corrected STEP wrapper preserves the existing minimum-tolerance assertion and axis remapping.

## Contained through-prism subtraction

`certified_prism::subtract` is an isolated qualification route for valid outward z-axis extrusions with matching cap heights. It verifies source extrusion identity, whole-curve cap planarity at the existing tolerance and complete cutter-hull containment in one simple line-trimmed body region, preserving existing holes. It reuses source edges and surfaces. Unsupported overlap, contact, blind cuts or geometry return a typed refusal; this is not a fallback that treats general Boolean failures as success.

Run `cargo test --locked --offline --release --manifest-path cad/monstertruck-qualification/Cargo.toml --lib certified_prism`. The intentionally ignored test records the still-failing generic Boolean baseline; the ordinary tests include the actual mounting-hole fixture, hole preservation and invalid-ring controls. Full Boolean and production replacement gates remain open.

## Planar profile and lettering topology

Run `cargo run --locked --offline --manifest-path cad/monstertruck-qualification/Cargo.toml --bin profile_topology_gate`. The gate checks production-derived concave and35-hole profiles, reversed winding, two separately extruded split regions, and the bundled font's B outline with two counters. It checks the same constructed solids before and after lossless STEP round trips against analytic straight-segment area, bounds and strict mesh closure. It reuses `cad/wasm/assets/DejaVuSans.ttf`; no font copy is required. This is qualification of Monstertruck's profile API, not the production Cadrum extrusion path or Boolean/dish behavior. Frozen scratch provenance is in `.evidence/monstertruck-qualification/profile-topology/`.

The qualification workspace also vendors `monstertruck-modeling 0.4.1` for the reviewed actual-intersection-curve containment correction. Package and patch provenance are in [modeling SOURCE.md](vendor/monstertruck-modeling/SOURCE.md). Sphere inverse membership uses linear spatial tolerance; active spline-domain and near-pole follow-ups remain separately qualified. Production continues to use Cadrum until full acceptance and removal gates pass.
