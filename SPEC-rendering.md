# Spec: rendering milestone contracts

Status: Phase 1 authorized in advance; technical validation required, 2026-10-01.
Module id: `rendering`; reference `96dd51d3`. Reuse [the map](CAPABILITY-MAP.md)
and [ADR 0003](docs/adr/0003-rust-application-ownership.md).

## Assumptions and objective

Keep the existing Rust `renderer/` three-d 0.19.0 WebGL2 implementation and
SVG 2D presentation mapping. The renderer consumes geometry and renders/picks;
it does not own documents, session selection, manufacturing readiness or export
authority. GPU state and live 3D camera belong here; session owns 2D navigation
and logical selection. Host owns actual canvas/listener/observer/frame handles.

M1 must display committed/provisional layout and current exact case geometry,
preserve camera/selection behavior and dispose resources without late results.
No OffscreenCanvas, WebGPU/backend replacement or universal scene-graph crate
is proposed. Existing private payload/helper APIs stay private; adapters use
the generated WASM entrypoints.

## Neutral provider contracts

Providers own scene meaning: core owns 2D semantic scenes and generation owns
prepared 3D/asset output. Add application-boundary metadata without changing
their existing wire encodings. Use two provider views, not a new universal
geometry authority. Shared records may live in the existing contract leaf
after interface validation; this is not a new crate instruction.

| Contract | Required fields and invariant |
| --- | --- |
| Scene identity | Document/session epoch, board/optional instance, source revision, job generation/executor epoch and scene ID. Tokens are transient. A revision number alone cannot distinguish two reopened documents. |
| Geometry and coordinates | Stable provider object/body IDs; explicit millimeter units, model-local geometry and transform into scene coordinates. Preserve existing handedness, mirror/board/instance transforms and tolerances. Mesh/contour layout comes from its provider encoding; never infer it from GPU handles. |
| Full / delta | A full scene establishes `scene_id`. A delta names `base_scene_id`, a new scene ID, upserts and removals. Apply only to the matching source/scope/base; reject missing bases, ambiguous duplicate IDs and incompatible buffers before GPU publication. Existing provider cache updates remain separate from display acceptance. |
| Appearance | Material intent, model colors and transparency semantics, with existing defaults preserved. No Dioxus node or backend material object enters the provider contract. Selection/hover/explosion/camera are separate view overlays. |
| Fidelity / readiness / freshness | Distinguish provisional geometry from exact-CAD-derived tessellation. Exact-derived does not mean analytic surfaces are reproduced exactly by triangles. Readiness comes from domain/services; freshness comes from session identity. Retained old geometry cannot become current/export-ready by relabeling it. |
| Ownership | Immutable records and owned/borrowed buffers with an explicit lifetime. Host transfer detaches an owned sender buffer; renderer holds its own eligible representation. No OCCT object, browser object, Rust pointer from another WASM memory or GPU handle crosses the neutral boundary. |

Rendering adapters map these views to [existing renderer exports](renderer/src/wasm.rs).
Keep provider IDs and transform semantics intact; the mapper is not a place to
recompute outline validity, CAD topology or snapping. Session authorizes current
scene publication, including same-revision/scope supersession.

## Lifecycle and rationale

The observed problem is browser/private renderer payload coupling and revision-
only guards. A neutral envelope makes provenance and full/delta prerequisites
explicit while retaining the existing backend. Direct Dioxus/GPU scene types
would simplify initial wiring but bind document/services to presentation.
A generic scene framework adds abstractions unsupported by this milestone.
The selected adapter adds mapping/copy costs; measure them rather than claiming
the Rust frontend makes them zero.

Prepare heavy scenes off the page thread. Preserve incremental/full preparation,
bounded caches, stable picking IDs and current stale-revision behavior. Valid
raced provider/cache deltas must be consumed before obsolete display is dropped.
Preserve unchanged-scene reuse, camera-reset rules, DPR/resize and frame cadence.
Teardown prevents publication, cancels pending frames, removes host observers/
listeners and disposes GPU resources. Context-loss behavior must fail honestly;
automatic recovery is not assumed without P2 proof.

## Structure and code style

Retain backend algorithms and native tests in `renderer/`; SVG mapping stays a
module consumed by web presentation. Host maps payloads and attaches the canvas.
Use existing Rust/serde conventions. This real [renderer record](renderer/src/lib.rs)
shows typed geometry, not a browser/GPU handle:

```rust
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelMesh {
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colors: Option<Vec<f32>>,
}
```

This excerpt is not a new scene implementation. Do not expose private renderer
modules to use native typed shortcuts.

## Commands and testing strategy

Existing provider/reference commands:

```sh
cargo test --manifest-path renderer/Cargo.toml --locked
cargo fmt --manifest-path renderer/Cargo.toml --all -- --check
cargo clippy --manifest-path renderer/Cargo.toml --locked --all-targets -- -D warnings
pnpm run build:renderer
pnpm run check:boundaries
pnpm --dir app exec vitest run src/renderClient.test.ts src/assemblyPreview.test.ts
```

Native tests characterize transforms, mesh parsing and picking inputs; real
Chromium/P2 proves canvas mount, resize/DPR, final frames and disposal. Paired
M1 layout/case tests and existing resource/performance assertions are required
under [CONSTRAINTS.md](CONSTRAINTS.md). Do not substitute callback completion
for a painted-frame measurement. Existing private WASM source/manifest is the
version-matched API evidence; no new three-d API behavior is assumed here.

## Success criteria

- R1: Provider scenes produce equivalent visible geometry, materials, mirrored
  transforms and picking identities without renderer/UI types entering providers.
- R2: Missing-base/stale/superseded scenes cannot replace current geometry;
  unchanged-scene reuse retains its established camera and revision behavior.
- R3: Provisional/retained geometry is truthful and cannot satisfy exact/export
  readiness; overlays do not mutate durable geometry.
- R4: Resize, DPR, mount/unmount, pending frames and GPU disposal pass P2 and
  affected browser/resource checks; late publication is impossible after disposal.

## Boundaries and open validation

Always preserve provider semantics, buffers/lifetimes and frozen budgets. Ask
before backend substitution, private API widening or public encoding changes.
Never make rendering an export/domain authority, assume transferred buffers
remain usable by the sender, or erase failed memory/performance evidence.

Neutral envelope mappings and binary layout checks still need characterization;
P1/P2 must establish packaging and mounted-canvas behavior. Live camera/GPU
resource behavior remains runtime evidence, not a spec or dependency-resolution
result. No renderer production changes are made by this document.
