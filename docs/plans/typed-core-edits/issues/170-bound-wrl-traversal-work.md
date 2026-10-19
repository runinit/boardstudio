# 170: Bound WRL decoding and refuse unsupported polygons

Status: resolved
Type: build
Priority: P1
Blocked by: None (can start immediately)
Related: [Input-boundary audit research](../../../research/audit-input-boundaries-2026-10-18.md)

## What to build

Importing a deeply nested or heavily reused WRL model returns a clear unsupported
input error before exhausting the decoder's stack or work budget. The same shared
renderer instance remains usable for the next ordinary model. Keep valid bounded
node reuse, existing cycle refusal, and the current output-size protection.

A model outside the preview's supported convex polygon subset also receives a
clear unsupported-input error rather than a mesh that fills empty source space.
Use explicit refusal as the smallest complete fix; general concave polygon
triangulation is not required for this ticket.

The audit reproduced a packaged WASM trap with 4,096 nested groups in a
94,245-byte file; a box then trapped in that same instance. A separate 1,174-byte
reference graph took about 5.2 seconds despite adding no geometry. These are
historical reproduction measurements, not acceptance timing thresholds.

The audit also found that a valid planar U-shaped face declaring nonconvex
polygons produces a triangle fan covering its empty notch. VRML97 permits this
face; the preview must acknowledge its narrower supported subset.

## Acceptance criteria

- [x] First reproduce the nesting and acyclic repeated-reference regressions
  through the real decoder, then demonstrate deterministic bounded refusal.
- [x] Enforce a finite traversal depth before descending, and a cumulative
  expanded-occurrence/work budget shared across the whole decode. Empty groups,
  repeated references and separate roots consume work even without triangles.
- [x] A limit breach returns an input-specific error without a WASM trap, accepted
  partial model, silent truncation or unbounded expansion. Cycle and output-size
  refusals remain effective; do not replace them with only the new limits.
- [x] Boundary tests cover admitted versus refused depths/work, noncyclic shared
  nodes, multiple roots, cyclic references and a normal colored/transformed model.
  Choose and document finite limits against representative supported assets.
- [x] First demonstrate the authored U-shaped face incorrectly covers a point
  inside its empty notch. After the fix the model is explicitly refused, with no
  accepted partial mesh or silent omission of the unsupported face.
- [x] Conservatively refuse nontriangular faces declared nonconvex before triangle
  emission. Explain this supported-format limit in the error/documentation; a
  nonconvex declaration does not prove that every face is geometrically concave.
- [x] Keep valid triangles and supported convex polygons working, including
  indexed per-vertex/per-face colors and transforms. Independently authored
  controls cover default-convex and explicitly convex polygons, a nonconvex-declared
  triangle and the nonconvex-declared U-shaped polygon.
- [x] Fresh packaged WASM evidence uses one module instance: reject each hostile
  input, then successfully decode a normal box in that same instance. Verify the
  application model-loading path reports refusal without poisoning subsequent
  model use; creating a new decoder for the control is insufficient.
- [x] The fresh packaged decoder probe also proves U-shaped refusal and supported
  polygon controls. The application model-loading path surfaces that refusal
  instead of showing a filled notch, and subsequent model use remains healthy.
- [x] Run affected renderer checks. Resource assertions use bounded counters and
  behavior, not sleeps, weakened limits or a machine-specific speed threshold.

## Comments

Created from the user's audit findings after primary-source research. At the
user's request, merged the traversal and polygon-admission findings into this
single decoder slice; [172](172-refuse-unsupported-wrl-polygons.md) is retired as
merged, with all its acceptance requirements retained here.

This ticket adds bounded input handling, not a new WRL engine or renderer reset
fallback that conceals the original trap. It does not claim complete VRML97
support or broaden CAD analysis to use display meshes.

## Comments

### Implementation evidence (DeepSeek V4.1 Flash Free sub-agent)

Decoder: `renderer/src/wrl.rs` adds `MAX_WRL_TRAVERSAL_DEPTH = 64` and
`MAX_WRL_TRAVERSAL_VISITS = 250_000`, both documented in the module header and charged
through one shared `WalkState` before descending (`decode_wrl` passes it across every
root). A `convex FALSE` face with more than three vertices is refused before emission.
Calibrated on the bundled KiCad switch model (2.8 MB, nesting depth 4, 931 `Shape`
nodes, no reuse). Evidence under `.evidence/ticket-170/`.

- Native RED (`red-native-summary.log`): 4 new tests failed before the fix (no refusal existed).
- Native GREEN (`green-native-final.log`): 33 passed, 0 failed, 1.89 s. `green-clippy-final.log`
  Clippy exit 0; `cargo fmt` clean. Cycle and output-size refusals covered.
- Packaged WASM RED (`red-wasm-single.log`, pre-fix `renderer/pkg`): depth 4096 traps with
  `RuntimeError: memory access out of bounds`; a 1,178-byte 24-level reference graph is accepted
  after 4,934 ms with no added geometry; the U-shape is accepted and 2 fan triangles cover its
  notch point.
- Packaged WASM GREEN (`green-wasm-sequence.log`, one module instance): each hostile input is
  refused with an input-specific error and a box then decodes in that same instance; supported
  convex/default-convex polygons, a `convex FALSE` triangle, and a colored transformed triangle
  still decode.

Gap retained: the wasm32-only Rust wrapper `renderer_host_page_extensions::decode_model` was not
executed end-to-end (needs the packaged page and renderer asset URL in a browser). The probe runs
the same packaged module the wrapper imports, with the wrapper's exact call/accessor shape and a
shared instance, and the wrapper is a stateless `Err(String)` mapping. Ticket left `claimed` for
the parent's integrated check to confirm the in-app path.

### Application-boundary evidence (DeepSeek V4.1 Flash Free sub-agent)

The retained gap is closed by a page browser test that calls the real application decoder,
`boardstudio_web_runtime::renderer_host_page::decode_wrl` — the exact function `Runtime` supplies
as the model-delivery `decode_wrl` port. The test runs in the page bin suite, so the packaged
renderer asset `web/assets/renderer/boardstudio_renderer_wasm.js` is served from the crate root
(the `wasm-bindgen-test-runner` asset fallback) and the wrapper's own `resource_url` import path
is exercised.

- `web/src/model_decode_boundary_tests.rs`, test
  `model_decode_boundary_tests::refuses_hostile_wrl_at_the_application_decoder_without_poisoning_the_shared_module`.
- One shared module instance for the whole test: `decode_wrl` caches its imported renderer in the
  `RENDERER_MODULE` thread-local, so every case below runs on that same instance. After each
  hostile input a `Box { size 10 8 4 }` must still decode to 108 positions/normals.
- Cases: 4096 nested groups refused (`depth`), the 24-level doubling reference graph refused
  (`work`), the U-shaped nonconvex face refused (`convex`); supported controls (default-convex and
  explicitly convex polygons, a `convex FALSE` triangle, a colored/transformed triangle) decode.

RED (`red-app-boundary.log`, pre-fix `web/assets/renderer` build 2026-10-18 22:39): the depth-4096
case surfaces the raw trap `memory access out of bounds` through the wrapper instead of a depth
refusal — the test fails on `unexpected depth refusal: memory access out of bounds`. That build's
wasm does not contain the decoder limit strings (`renderer/pkg` does).

GREEN (`green-app-boundary.log`, `green-app-boundary.json`): `executed 1, failed 0`,
`passed_count 1, failed_count 0, incomplete_count 0, complete true, problems []`. The fresh
renderer was staged by the documented provider flow: `wasm-pack build renderer --target web
--out-dir pkg --out-name boardstudio_renderer_wasm --release --locked` followed by the
`stage_runtime_assets` copy `renderer/pkg -> web/assets/renderer` (`green-provider-staging.log`);
staged wasm SHA-256 `74bcd861…e0f9e` matches `renderer/pkg`. `cargo fmt --all --check` clean
(`green-fmt.log`); `check-wasm-tests.py` reports no new wasm-only plain `#[test]`.

Scope limits: the outer WASM module is the page test binary, which imports the same packaged
renderer module the production page imports; the wrapper code path (`decode_model` -> `decodeWrl`
-> read `positions`/`normals`/`colors`) runs unmodified. `ModelDeliveryPorts::decode` mapping a
decoder `Err` to a delivery failure is covered by the existing native `decode rejected` test. The
full `build-web.py --providers-only` provider set (workers, CAD wasm) was not rebuilt, only the
renderer provider this ticket changes; the parent's integrated check owns the full build.
