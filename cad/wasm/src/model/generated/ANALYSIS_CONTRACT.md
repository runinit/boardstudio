# Generated analysis wire contract

This is the authoritative boundary between `generated.rs` and the Host CAD worker
(`web/crates/host/src/cad_worker.rs`, with validation in `cad_jobs.rs`). Tickets
010 and 014 share the [contract fixture](../../../tests/fixtures/generated-analysis-contract.json).
Change the producer, consumer and their fixture tests together when this boundary changes.

## Request and ownership

The on-demand worker operation is `analyze-generated-body`. It uses the existing
captured prepared assembly and `generatedAnalysisRequest`:

```json
{"bodyId":"contract-plate","geometryIdentity":"<worker-derived identity>","targetDeflectionMm":0.01}
```

The worker validates that exactly one prepared body matches `bodyId`, that body
and assembly revisions match the captured snapshot, and that `geometryIdentity`
matches the complete typed `PreparedCaseIR`. The identity is
`m1/generated-analysis-v1/<SHA-256 of serde_json::to_vec(prepared body)>`.
`m1` is the existing generated geometry cache version; `generated-analysis-v1`
versions this analysis contract and algorithm. Bump the relevant version when
its semantics change. These are manually versioned identities, not accuracy certificates.

The worker invokes `analyze_generated_body(selected PreparedCaseIR, options)`;
`options` contains only optional `targetDeflectionMm`. Omission or null selects
the existing 0.1 mm default; a supplied target must be finite and positive, and
the selected deflection is `min(0.1, target)`. The backend reports the actual
selection in `deflectionMm`. Requested meshing deflection, requested collision
accuracy and the physical clearance threshold are separate quantities.

Ordinary preview and STEP output do not request this operation. It may reuse
the exact generated body cache; display f32 buffers cannot become analysis data.

## Direct backend result

The backend returns this object directly, without a `generatedAnalysis` wrapper,
`geometryIdentity`, `sourceSha256` or `vendorStep`:

| Field | Meaning |
| --- | --- |
| `bodyId`, `revision` | Selected prepared body identity and accepted revision; revision is an integer no greater than 2^53 - 1. |
| `surfacePositionsMmF64` | Nonempty finite `Float64Array`, xyz triangle soup, length divisible by 9. Coordinates use the exact generated body's millimetre frame. |
| `triangleFaceOccurrenceIndices` | `Uint32Array`, one valid occurrence index per triangle. |
| `sourceFaces` | Generated face records: `solidIndex`, `shellIndex`, `faceIndex` (u32), and nonempty `supportKind`. These are generated indices, not STEP entity IDs. |
| `faceOccurrences` | Records with `sourceFaceIndex` and `solidOccurrenceIndex` (u32). Every triangle tag resolves through an occurrence to a retained generated source face. |
| `analysisErrorBoundMm` | Null until a symmetric source-to-mesh bound is qualified. A supported future value must be finite and nonnegative. |
| `meshClosedOriented` | Measured mesh topology metadata; it does not certify containment, source approximation or numerical query error. |
| `deflectionMm` | Finite positive selected meshing setting; it does not certify an error bound. |

The worker checks `bodyId` and `revision`, attaches its validated
`geometryIdentity`, and wraps the result under `generatedAnalysis`. Transfer the
original typed buffers with the metadata; validate sizes, index ranges, finite
coordinates, identity, revision and settings again at the consumer. Reuse the
existing 64 MiB collision buffer limit. Apply the physical `rowTransformF64`
exactly once when querying, not when storing these buffers.

Cache identity includes the captured snapshot, complete generated geometry
identity and requested refinement settings. A coarser result cannot satisfy a
tighter request. Cancellation or a changed snapshot prevents publication.

## Collision and vendor identity

Each collision operand contains `modelId`, `rowTransformF64`, and exactly one of:

- `sourceSha256`: verified immutable STEP bytes, with the matching STEP import options;
- `generatedGeometryIdentity`: the generated identity above, with default STEP import options.

Missing or dual identities reject. Generated identity never occupies a STEP
SHA field. Vendor `targetDeflectionMm` and `previewRepairBudgetMm` both enter
derived asset cache identity. Preserve declared source precision separately
from explicit `selectedRepairBudgetMm`. Unmeasured actual repair movement is
null (`maxRepairDisplacementMm` and per-face `repairDisplacementMm`), not zero
or a sampled support residual.

Raw surface intersection or separation is a mesh observation. Unknown source,
query-numerical, containment or contact-location bounds remain uncertain. A
nested solid can have separated surfaces; zero unsigned distance does not
establish penetration depth or a unique contact location.

## Shared fixture checks

The fixture independently authors a 20 x 10 x 2 mm plate and a valid example
of its direct wire payload. JSON arrays must be materialized as the typed arrays
above at the WASM/worker boundary. Face and triangle ordering are not pinned;
the producer may tessellate differently while preserving the bounds, complete
coverage and resolvable provenance.

The producer test invokes the actual WASM export with `preparedBody`, verifies
the fixture's direct field shapes/ownership, analytic bounds, default/refined
settings, null accuracy and valid provenance. The Host consumer test reads this
same file, materializes `directPayload`, adds the worker identity/wrapper and
checks decoding and request/reply validation. Mutations must reject stale
body/revision/identity, invalid provenance and malformed buffers. Both tests
must run in their owning acceptance commands; passing only one side does not
accept the boundary.
