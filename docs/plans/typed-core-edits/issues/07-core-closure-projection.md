# 07: Core closure-mount initialization and clearance projection

Status: ready-for-agent
Type: build
Blocked by: 06
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Parent: [02](02-mechanical-settings-patch.md)

## Question / goal

One mechanical edit initializes absent closure mounts through the Core mechanical
resolver and owns the closure-clearance projection (generated definitions, parts,
references and board/layout memberships) atomically. Configuration, initialization
and projection form one accepted edit and one Undo step; a failure leaves neither
half changed. A no-op mechanical edit short-circuits before revision/history
mutation, and the operation is classified for `affects_outline` with its reasoning
in the doc comment.

## Seams under test

- `CoreEngine` `Edit` commits and `Undo`, observing the document and scene.
- `boardstudio_core::mechanical::edit::apply` changed-id reporting.
- `mechanical::edit` outline classification.

## TDD cases

1. An edit on a configuration with `closure_mounts: None` initializes them from the
   resolved assembly; explicit `Some(vec![])` is preserved as deliberate.
2. Generated clearance parts/definitions/memberships appear in the same reply and
   disappear together on one Undo.
3. A no-op edit adds no history step and no revision bump.
4. Changed ids name only generated parts/definitions/memberships and the
   configuration owner.
5. A projection failure rolls back the configuration change.
6. Pure settings edits do not rebuild board outlines.

## Verification

`cargo test -p boardstudio_core --locked`; WASM `typecheck`; regenerate the boundary
transcript for the combined integration.
