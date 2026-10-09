# 09: Case adapter, physical setup and reviewed linking

Status: resolved
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

## Outcome

Resolved 2026-10-09 at integrated `2a8d5e076`. The Case controller submits typed edits
against accepted state; reviewed links/rejoins carry a fingerprint of the result the user
actually reviewed, and the execution-time guard retires on relevant source, destination
or mapping changes while unrelated edits survive. The read-only projection port shows the
complete proposed result for the user's actual choices (map values, construction source,
per-feature sources) through a disposable Core engine without landing a replacement
document; gasket anchor rows show Core's resolved geometry and located fit errors.
Reachable controls: paired-assembly removal with explicit confirmation, the Q14
"Unlink and disable this half" label, and "Make construction local" for Legacy halves.
New setup instances are stamped `Local` (Legacy belongs only to migrated projects);
eligible pair proposals link through a disposable Core engine and submit one
`ReplaceDocument`. Page Case admission is decided per selected target through Core's
predicate. Key commits: `018563e14`, `5221aa8ab`, `0a8ab47e6`, `6fe78ed55`, `240357a81`,
`a8189222c`, `3810fc5d2`, `59cdaa1fb`, `75930d0d3`, `edb0eb498`, `591ee706b`, `41a9039cc`,
`e8967dc06`, `c91964fbc`, `af074a57c`, `05e4248d9`, `6bc9c32e9`, `591a0fb29`, `a67c266a2`.
