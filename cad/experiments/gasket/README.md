# Bounded gasket generation comparisons

Isolated experiments against production revision `49d33a91`. These scripts do not
replace the shipped Cadrum crate, WASM bundle, renderer, cache, or export path.
See [the protocol](../../../docs/generation-optimization-experiments.md) and
[results](../../../docs/gasket-comparison-results.md).

## Reproduce

Use a **new, empty staging path**. The existing pinned Cadrum build container,
OCCT archives, Rust toolchain, and application Playwright installation are required.
`prepare.py` downloads Cadrum 0.8.20 and checks its published crate SHA-256 before
patching the staged copy. Do not patch the Cargo registry or production artifacts.

```sh
npm ci --prefix cad/experiments/gasket --ignore-scripts --no-audit --no-fund
python3 cad/experiments/gasket/prepare.py /tmp/gasket-comparison-new
node cad/experiments/gasket/build-wasm.mjs /tmp/gasket-comparison-new
```

Run native geometry checks separately from performance measurements. Set
`OCCT_ROOT` to the absolute native OCCT directory printed by
`node cad/scripts/prepare-cadrum-occt.mjs native`. For each `regression`, `edit-0`,
`edit-2`, and `edit-5` input, run the ignored test with these environment variables:

```sh
OCCT_ROOT=/absolute/native/occt \
CARGO_TARGET_DIR=/tmp/gasket-comparison-new/target \
CAD_EXPERIMENT_INPUT=/tmp/gasket-comparison-new/inputs/regression.json \
CAD_EXPERIMENT_OUTPUT=/tmp/gasket-comparison-new/geometry-regression \
cargo test --release --locked \
  --manifest-path /tmp/gasket-comparison-new/cad/wasm/Cargo.toml \
  bounded_gasket_equivalence -- --ignored --nocapture
node cad/experiments/gasket/validate-step.mjs /tmp/gasket-comparison-new/geometry-regression
```

Also run the staged crate's ordinary native tests using the same OCCT/target
environment, without the test name or `--ignored`. No production build is replaced.

After builds and correctness tests finish, on a quiet host:

```sh
node cad/experiments/gasket/benchmark.mjs /tmp/gasket-comparison-new /tmp/gasket-comparison-new/browser
node cad/experiments/gasket/validate-preview.mjs /tmp/gasket-comparison-new /tmp/gasket-comparison-new/browser
node cad/experiments/gasket/summarize.mjs /tmp/gasket-comparison-new/browser
```

The benchmark uses five fresh workers, ten samples per fixture per session,
reversed variant order on alternating iterations, and three initial observations
per variant retained separately. Initialization, JSON parsing/serialization,
validation, and object destruction are outside generation timing. Geometry
construction, lazy Boolean evaluation, tessellation, normals, and expanded f32
render buffers are inside. No exact-solid cache is used. Timers are disabled for
speed samples and enabled separately for attribution. This is a single-bottom
headless diagnostic, **not the full CAD/live performance acceptance protocol**.
WASM memory values are allocated capacity, not live allocations or process RSS;
Cadrum variants share a module within each session. Manifold does not expose heap
capacity through this package API, so it remains unmeasured.

Cadrum variants: control, skip edge polylines, skip face-ID output, both (`lean`),
and height-band profiles plus analytic hole cuts (`profiles`, also lean).
`split-timing` supplies a separate OCCT PaveFiller to distinguish intersection from
result construction; it is a validated attribution variant, not the unchanged
control. History timing measures the bridge's history relay, not all internal OCCT
history work. Copy timing measures the post-Boolean deep copy, not every copy in
the application. Ownership protections remain enabled.

The profile prototype restores unmodified authored vertices after 2D overlay
quantization. Native material differences and independent STEP checks are required;
matching volume and bounds alone is insufficient. The supported fixture family
has plate-kind bottoms, polygon openings, circular through holes, and no bosses,
region cavities, or separate gasket features. It is not a general replacement.

Manifold uses the same height bands and polygon openings with circular holes
approximated by 32-sided cylinders. Their maximum chord error is about 0.0053 mm,
stricter than the existing 0.1 mm / 0.5 rad mesh settings. Shared height endpoints
are assigned exactly to avoid floating-point gaps; no extra overlap is inserted.
Every Embind geometry handle is deleted. This creates a **preview**, not exact
CAD or STEP. Cadrum remains responsible for manufacturing validation and export.

## Monstertruck feasibility gate

```sh
CARGO_TARGET_DIR=/tmp/gasket-comparison-new/monstertruck-target \
cargo build --release --locked --manifest-path cad/experiments/gasket/monstertruck/Cargo.toml
python3 cad/experiments/gasket/run-monstertruck.py \
  /tmp/gasket-comparison-new/monstertruck-target/release/gasket-monstertruck-comparison \
  /tmp/gasket-comparison-new/inputs/regression.json \
  /tmp/gasket-comparison-new/monstertruck-runs
node cad/experiments/gasket/validate-monstertruck.mjs \
  /tmp/gasket-comparison-new/monstertruck-runs/overrun-0.01-tol-0.01 \
  /tmp/gasket-comparison-new/inputs/regression.json
```

This direct-construction probe uses analytic polygon extrusions and circular
cutters, followed by the existing polygon openings. Monstertruck 0.4.1 exposes
binary Booleans, so these cuts are sequential, unlike Cadrum's batched expression.
Thirty-second per-process timeouts stop expensive failures. Cutter overrun changes
only material outside the bottom and diagnoses coincident-face sensitivity.
Coarse Boolean tolerances are explicitly diagnostic and cannot pass the unchanged
geometry contract. Native failure wall times are not browser latency results.
Diagnostic STEP files let OCCT independently validate and subtract the very same
input solids before attributing the failure to the kernel.

## Remaining-plan OCCT comparison (round two)

Use a fresh stage and retain the first round's artifacts. After `prepare.py`, run:

```sh
python3 cad/experiments/gasket/extend-occt.py /tmp/gasket-occt-new
node cad/experiments/gasket/build-wasm.mjs /tmp/gasket-occt-new
```

This adds independent variants for wrapper history relay, kernel history,
OBB filtering, a guarded dedicated multi-tool cut, non-destructive processing
with all copies retained, combined mounting/opening cuts, analytic mounting holes
in extrusion profiles, and final cleanup. It also creates `rotated-stress.json`
from the regression bottom using a rigid 0.31-radian XY rotation. This is labelled
synthetic stress coverage and does not replace actual rotated project coverage.

Run the native ignored equivalence test for the four original inputs plus
`rotated-stress`, using the same commands above and the new staging directory.
Select the variant list for independent STEP validation and the browser run:

```sh
export CAD_EXPERIMENT_VARIANTS=control,no-relay,no-history,obb,multi-cut,non-destructive,combined-cuts,profile-holes,final-clean
export CAD_EXPERIMENT_FIXTURES=regression,edit-0,edit-2,edit-5,rotated-stress
node cad/experiments/gasket/validate-step.mjs /tmp/gasket-occt-new/geometry-regression
node cad/experiments/gasket/benchmark.mjs /tmp/gasket-occt-new /tmp/gasket-occt-new/browser
node cad/experiments/gasket/validate-preview.mjs /tmp/gasket-occt-new /tmp/gasket-occt-new/browser
node cad/experiments/gasket/summarize.mjs /tmp/gasket-occt-new/browser
```

Repeat STEP validation for every geometry directory. Run builds and geometry
checks before timing. The dedicated cut accepts only one DNF clause with one
positive operand and at least one negative operand; all other expressions retain
CellsBuilder. All copy protections and Boolean tolerances remain unchanged.
Analytic profile holes are an isolated prototype for these verified inputs;
production use requires additional containment/contact/overlap/fallback guards.
No production adoption follows automatically from a screening result.

`audit-eligibility.mjs <output.json>` records the actual tabbed-plate grid sizes
and unchanged primitive inputs across the six captured edits. It does not claim
that the E4 mesher extension or E6 reuse experiment has been implemented.
