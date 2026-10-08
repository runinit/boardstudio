# 09: Case adapter, physical setup and reviewed linking

Status: ready-for-agent
Type: build
Blocked by: 08
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Parent: [02](02-mechanical-settings-patch.md)

## Question / goal

Migrate `MechanicalSettingsController` and the host projection/default helpers to
adapters over Core. Keep async catalogue preparation and `flush_prepared` ordering.
Address queued opening and vertex changes by durable id, never vector index. Add
physical-half setup/linking and per-feature actions using the accepted model,
including the reviewed-linking fingerprint that retires when relevant inputs changed.
Update `DESIGN.md` and `docs/architecture.md` wording for explicit mechanical pairs
while preserving concurrent edits.

## Seams under test

- The controller's native and WASM tests (ports, ordering, queueing, drafts).
- Physical setup proposal tests.
- Opening/vertex edits after remove/reorder, and stale reviewed inputs.

## TDD cases

1. Removing or reordering an opening cannot redirect an earlier queued edit to a
   different opening or vertex.
2. Physical setup establishes a pair for reused-PCB and distinct-PCB cases; a
   layout-only pair does not invent physical instances.
3. Reviewed linking/rejoining retires when relevant source/destination/mapping
   inputs changed and applies when unrelated edits intervened.
4. Removing a physical member materializes the survivor's geometry and clears the
   pair; a queued edit to a removed member retires.
5. Controller ports and `flush_prepared` ordering are unchanged; existing native
   tests pass.

## Verification

`cargo test -p boardstudio-web-case --locked`, `wasm-pack test --headless --chrome
web/crates/case --locked --lib`, WASM `typecheck`.
