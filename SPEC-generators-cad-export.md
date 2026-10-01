# Spec: generators-cad-export milestone contracts

Status: Phase 1 authorized in advance; technical validation required, 2026-10-01.
Module id: `generators-cad-export`; base `96dd51d3`. Reuse the
[approved map](CAPABILITY-MAP.md) and [ADR 0003](docs/adr/0003-rust-application-ownership.md).

## Assumptions and objective

M1 uses embedded supported part definitions, Rust mechanical preparation and
the existing exact CAD kernel for case generation and committed STEP export.
Retain core artifact/archive services and `cad/wasm/`; this capability family
does not justify a new umbrella crate. Current Cadrum 0.8.20 / OCCT
`8_0_1_rev2` remains an allowed dependency. Dynamic generator authoring and
complete PCB/firmware export are outside M1, with their data retained.

Supply exact, revision-bound case results and artifacts without conflating
preview display with manufacturing readiness. Separate service algorithms
and cache protocol from session scheduling and host execution/delivery.

## Provider contracts

| Boundary | Required meaning |
| --- | --- |
| Core preparation | Consume supported mechanical settings and geometry through the existing core public contract; return the prepared case representation with existing dimensions, IDs, tolerances and findings. Engine remains the domain authority. |
| CAD package | Use [existing exported entrypoints](cad/wasm/src/lib.rs), including initialization, case/assembly construction and cached export. Its `model` implementation remains private; Rust-to-package interop is a P1 question, not permission to expose internals. |
| Generation request/result | Carry captured prepared input, revision and external job identity. Return exact-CAD-derived meshes/body identities, progress, valid cache updates and explicit success/error/cancel outcomes according to current provider protocol. Tessellation remains an approximation of exact solids; the fidelity flag names provenance. |
| Full/delta output | A full result establishes a scene/cache base. A delta identifies its base, changed bodies and removed bodies; a consumer cannot apply it to another base or executor lifetime. Keep existing cache keys and complete input/fingerprint rules. |
| Export | Build from a captured, eligible committed document/prepared input. Export CAD has an independent bounded lifetime/cache; do not require the preview renderer or a provisional mesh to reconstruct the authoritative solid. Services own bytes/format/filename semantics; host owns final delivery. |
| Archive/assets | Preserve existing Rust validation/serialization, archive paths, size/hash checks, deduplication and imported KiCad/model source identities. Do not reinterpret generator bodies while importing a copied fixture. |

Session scope metadata is external to existing provider encodings. Services
must return enough provenance for [session guards](SPEC-editor-session.md),
not independently own the current UI document/session. Provider readiness
rejects blocked mechanical/export inputs; UI cannot promote them to ready.

## Lifecycle and full-Rust disposition

Keep preview, export and scene-preparation work off the page thread. Cancellation
can prevent dispatch or take effect at cooperative boundaries between yielded
operations; synchronous OCCT cannot be interrupted by merely cancelling a Rust
future. Settle all callers. Apply valid raced cache deltas before rejecting
their obsolete display; preserve cache consistency after failure/supersession.
Reset invalid executor/cache lifetimes explicitly and retain existing bounds.

The observed problem is authored TS orchestration and executable generator
semantics remaining outside Rust. Port supported application-owned policy and
generator semantics to Rust as later reviewed slices; embedded default geometry
or a JS interpreter is not full-Rust parity. A constrained build-time translation
is an alternative only after its language/parameter corpus is proven; it is not
selected as a generic transpiler. Keep imported source as data. Every temporary
runtime bridge needs an accountable owner and removal evidence before use.

For M1, retaining the existing separately built CAD package minimizes kernel
and API risk but keeps JS/WASM crossings/copies. Direct Rust linking might
reduce them, but private construction APIs and build prerequisites make it a
separate experiment requiring review if P1 fails. Do not silently substitute
Manifold or another kernel. No geometry, tolerance, cache-budget or artifact
contract changes are selected.

## Structure and code style

Existing source owners are `core/src/artifact/`, core archive/mechanical modules,
`cad/wasm/src/`, and the reference CAD/generator/export packages. Future Rust
service/generator modules are placed by their consumers and build prerequisites,
not one crate per operation. No new production package is created here.

This actual [CAD boundary excerpt](cad/wasm/src/lib.rs) illustrates a private
implementation with explicit selected exports, not blanket visibility:

```rust
mod model;
pub use model::{
    build_assembly, build_case, build_keycaps, export_cached_assembly, preview_body,
    read_step_model,
};
```

Retain existing Rust formatting, names and error contracts. Export helpers and
model constructors do not become public solely to simplify an adapter.

## Commands and testing strategy

Existing commands from repository root:

```sh
cargo build --manifest-path core/Cargo.toml --locked --example prepare_case
cargo test --manifest-path cad/wasm/Cargo.toml --locked
pnpm run build:cad
pnpm --dir cad test
pnpm --dir ergogen test
pnpm --dir kicad test
cargo test --manifest-path core/Cargo.toml --locked
```

Use existing native CAD/STEP oracles, cache/geometry tests and core artifact
regressions; compare cached/uncached export and reopened STEP, not mesh images
alone. P1 proves actual package initialization, transfer and lifecycle. Paired
M1 case-setting/generation/export supplies browser evidence. Run affected
formatting/Clippy/build and [constraint gates](CONSTRAINTS.md) at task acceptance;
do not present an unavailable native prerequisite or comparator as a pass.
Preserve measured tolerances, all applicable frozen CAD budgets and caller/
memory checks. No fresh performance claims are made by this spec.

## Success criteria

- G1: Characterized case settings produce equivalent prepared inputs, exact
  solids, meshes/body IDs and readiness through the new coordination boundary.
- G2: Superseded/cancelled/failed jobs settle callers and leave valid cache state;
  wrong-base deltas and old executor results cannot become current.
- G3: Current committed STEP matches existing geometry/material/bounds oracles,
  reopens successfully and is independent of provisional/preview cache state.
- G4: Archive exchange preserves supported definitions, parameters and asset
  identities; unsupported authoring/export work is truthfully outside M1.
- G5: Remaining authored JS/TS and upstream non-Rust dependencies have explicit
  dispositions; M1 is not advertised as generator or full-application completion.

## Boundaries and open validation

Always retain exact-kernel/readiness/fingerprint semantics and existing coverage.
Ask before private API widening, kernel substitution, format changes or a runtime
bridge exception. Never replace parameter-dependent generators with frozen
defaults, use renderer meshes as STEP authority, or hide failed budgets.

P1 must prove existing CAD-package interop and usable production assets at root
and subpath. Copy/crossing costs are measurements, not assumed zero-copy gains.
Generator semantic corpus and broader document/export parity remain incomplete;
no whole-migration generator task backlog is introduced.
