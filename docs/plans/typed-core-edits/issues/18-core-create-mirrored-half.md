# 18: Core CreateMirroredHalf

Status: ready-for-agent
Type: build
Blocked by: [17](17-mirrored-half-schema-and-format-5.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md) · [16](16-native-mirrored-halves.md)

## Request

Create the second half from the first in Core. Today `CreateMirroredPair`
(`core/src/lib.rs:1512`) works only for two layouts on one board and maps by a vertical axis.

## What to do

- TDD at the Session/Core seam. `EditOperation::CreateMirroredHalf {pairId,
  sourceInstanceId, mode: new-board{boardId, instanceId} | reuse-board{instanceId},
  mapping, idRule}`. Validate fresh ids, an existing source, a finite map, an unpaired source.
- Generalize `layout::reflected` (`core/src/matrix/layout.rs:93`) to the pair's `Pose2` map
  with the single heading rule and source numbering from ADR-0010. Copy and reflect
  matrices, layouts, parts, the board, fixed outline versions, modules, nets and the
  electrical board config; never copy `case-closure/*` (regenerated). Link the mechanical
  pair and fill the correspondence table.
- Report every new id plus the source ids; one Undo step. `reuse-board` mode makes a
  flipped instance and copies nothing, matching what physical setup produces today.

## Acceptance

- [ ] A one-half Corne seed plus the operation reproduces the current right-half key
      positions within 1e-9 by correspondence.
- [ ] Changed ids reported exactly; one Undo step; fresh-id and double-pair errors do not mutate.
- [ ] Reuse-board equals today's physical-setup result.

## Verification

Core native tests; boundary transcript appended and passing native and WASM.
GitNexus `impact` on `layout::reflected` and the edit apply path first.
