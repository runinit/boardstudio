# 001: Collision query contract shared by host and CAD worker

Status: resolved
Type: build
Blocked by: None
Parent: [CAD deepening](../map.md)

## What to build

A **Collision check** request crosses from the host to the CAD worker as a collision query
with operands. Today the query type, its operand type and their checks on source digests
and placement transforms are written separately on each side, the digest check four times.
Give them one definition in the shared contracts crate, as was already done for case wire
types, and delete the copies so drift becomes a build failure.

## Acceptance criteria

- [x] STEP import options, source-digest and placement-transform validation have one definition in contracts, used by host and CAD worker. (The query and operand types themselves stay separate; see comments.)
- [x] No local copies of those types or validators remain in the host or the CAD worker.
- [x] Existing qualified-collision host and worker tests pass unchanged in behaviour (worker 57, host 26, runtime 124 native tests).
- [x] Contracts has unit tests for digest, transform and import-option acceptance and rejection.

## Comments

The ticket assumed the query and operand types were copies. They are not: the host's carry
qualification limits, analysis references and meshes and a required `requestedAccuracyMm`,
while the worker's are the strict narrower form with `deny_unknown_fields`. Sharing one type
would have forced one side to carry fields it rejects, so only the leaves that were genuinely
duplicated moved to `contracts` (`CadStepImportOptions`, `is_sha256_hex`,
`validate_affine_row_transform` returning a typed `AffineTransformError`). Each side keeps
its own wording for a rejected transform. Core's `model` re-exports them, so the host needed
no new dependency. Not run: the wasm32 browser tests under `cad/wasm/tests`.
