# 170: Bound WRL traversal before it can stall or trap the renderer

Status: ready-for-agent
Type: build
Priority: P1
Blocked by: None (can start immediately)
Related: [Input-boundary audit research](../../../research/audit-input-boundaries-2026-10-18.md)

## What to build

Importing a deeply nested or heavily reused WRL model returns a clear unsupported
input error before exhausting the decoder's stack or work budget. The same shared
renderer instance remains usable for the next ordinary model. Keep valid bounded
node reuse, existing cycle refusal, and the current output-size protection.

The audit reproduced a packaged WASM trap with 4,096 nested groups in a
94,245-byte file; a box then trapped in that same instance. A separate 1,174-byte
reference graph took about 5.2 seconds despite adding no geometry. These are
historical reproduction measurements, not acceptance timing thresholds.

## Acceptance criteria

- [ ] First reproduce the nesting and acyclic repeated-reference regressions
  through the real decoder, then demonstrate deterministic bounded refusal.
- [ ] Enforce a finite traversal depth before descending, and a cumulative
  expanded-occurrence/work budget shared across the whole decode. Empty groups,
  repeated references and separate roots consume work even without triangles.
- [ ] A limit breach returns an input-specific error without a WASM trap, accepted
  partial model, silent truncation or unbounded expansion. Cycle and output-size
  refusals remain effective; do not replace them with only the new limits.
- [ ] Boundary tests cover admitted versus refused depths/work, noncyclic shared
  nodes, multiple roots, cyclic references and a normal colored/transformed model.
  Choose and document finite limits against representative supported assets.
- [ ] Fresh packaged WASM evidence uses one module instance: reject each hostile
  input, then successfully decode a normal box in that same instance. Verify the
  application model-loading path reports refusal without poisoning subsequent
  model use; creating a new decoder for the control is insufficient.
- [ ] Run affected renderer checks. Resource assertions use bounded counters and
  behavior, not sleeps, weakened limits or a machine-specific speed threshold.

## Comments

Created from the user's audit findings after primary-source research. Independent
of 172's polygon-admission correction; coordinate overlapping decoder edits.
This ticket adds bounded input handling, not a new WRL engine or renderer reset
fallback that conceals the original trap.
