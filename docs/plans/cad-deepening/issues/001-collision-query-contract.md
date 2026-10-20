# 001: Collision query contract shared by host and CAD worker

Status: ready-for-agent
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

- [ ] Collision query, operand, source-digest and transform validation have one definition in contracts, used by host and CAD worker.
- [ ] No local copies of those types or validators remain in the host or the CAD worker.
- [ ] Existing qualified-collision host and worker tests pass unchanged in behaviour.
- [ ] Contracts has unit tests for digest and transform acceptance and rejection.
