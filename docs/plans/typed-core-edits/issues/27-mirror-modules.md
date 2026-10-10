# 27: Mirror modules in CreateMirroredHalf

Status: ready-for-agent
Type: build
Blocked by: [18](18-core-create-mirrored-half.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

Ticket 18 refuses a source board that carries modules (a named, located refusal), although
ADR-0010 says modules are reflected. No shipped split half has a module (only
`vik-module-review`, a single board). User, 2026-10-10: follow-up before [19](19-reconcile-mirrored-halves.md),
because 19 requires "a module edit moves both halves".

## What to do

- Reflect each module: `at` through the pair map, `rotation` through the reflected heading;
  keep the definition, faces, gap, attachment and supports. Map the module id,
  `host_connector_part_id`, `bus_id`, the upstream ids and `host_instance_id` (to the mirror
  instance). Add `module` correspondences (kind exists since 17).
- Resolve chirality with the user before building (see Questions): module geometry mirrors
  only when the host is `flipped` (`placement.rs` ~:347), so the same definition at a mirrored
  pose is not a mirror image. For `reuse-board`, `resolve` already reflects modules for a flipped
  host (`placement.rs` ~:545-552); only modules pinned by `host_instance_id` need mapping.
- Never change module or generator parameters.

## Questions for the user

- A module whose geometry is not its own mirror image (a connector, a display, a chiral
  module): should the mirror use the same definition at the mirrored pose, be refused, or be
  flagged with a finding?

## Acceptance

- [ ] A source board with modules mirrors; ids and correspondences exact; one Undo step.
- [ ] Chirality behaviour as answered, with a test per case.
- [ ] `vik-module-review`-style fixture used for the tests.

## Verification

Core tests, boundary parity native and wasm, `repo lint test typecheck browser`.
