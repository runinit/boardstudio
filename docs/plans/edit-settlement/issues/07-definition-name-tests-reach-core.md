# 07: Definition-name and generator tests reach the real Session and Core

Status: ready-for-agent
Type: build
Blocked by: 01
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md)

## What to build

This is a test prefactor with no user-visible change. Several browser tests put
Runtime into a "definition-name" test mode. In that mode Runtime returns a fixture
snapshot, read model and generation status, and records submitted events instead of
handing them to Session. Move those tests onto the in-process Core and persistence
adapter from ticket 01, so they open a real document and assert accepted results.
Then delete the definition-name interception from Runtime.

Tickets 08 and 15 need this: their accepted-result tests for definition fields and
generator Apply/upload would otherwise be written against a mode that is about to
disappear.

## Background you need

- Read ticket 01 (and its Outcome) for the adapter's helpers: install on a Runtime,
  open a document, gate or fail a Core reply or save, run pending effects.
- The interception in `web/crates/runtime/src/runtime.rs` (orientation only):
  - fields `definition_name_test_state`, `definition_name_test_model`,
    `definition_name_test_events`, `definition_name_test_generation`;
  - reads of those fields in the accepted-snapshot, scope, read-model and
    generation-status accessors;
  - the early return in `submit` that pushes the event and returns;
  - `set_definition_name_test_state`, `set_definition_name_test_model` and the
    matching take/settle helpers.
- Callers are wider than the name suggests. At `bf8ba7c`,
  `grep -rn set_definition_name_test_state web` finds:
  - `web/crates/parts/src/parts_definition_name.rs` (most uses);
  - `web/crates/parts/src/parts/generator_settings.rs`, `parts/preview.rs`, `parts.rs`;
  - `web/crates/pcb/src/pcb_board_reference.rs`;
  - `web/crates/case/src/cad_presentation.rs`;
  - `web/src/presentation/case_workspace.rs`.
  Some only need an opened document; some also stub the generation status.
- Generation jobs are outside ticket 01's adapter. If a test only needs a generation
  status to be shown, keep one narrowly named generation-status test hook on Runtime
  and record it in your Outcome. Don't keep the event interception for it.
- Inventory section 6 lists which harness each test uses:
  [inventory.md](../inventory.md#6-tests).

## Approach

1. List every caller and what it relies on: snapshot only, read model, generation
   status, captured events.
2. Port the tests in `parts_definition_name.rs` first. Assertions on captured
   `ReplaceDocument` events become assertions on the accepted document (the
   definition's name after the edit, Undo restoring it).
3. Port the remaining callers. Tests that only needed a snapshot open a real
   document through the adapter.
4. Delete the interception fields, accessor branches, `submit` early return and
   helpers. Check `grep -rn definition_name_test web` returns nothing (except a
   generation-status hook you deliberately kept and named differently).
5. Update `scripts/wasm-test-owners.json` for moved or renamed tests.

## Acceptance criteria

- [ ] Every previously covered definition-name and generator behaviour has a passing browser test against the real Session and Core.
- [ ] Runtime has no definition-name interception fields, accessor branches, `submit` branch or helpers.
- [ ] Any generation-status test hook that remains is named for what it stubs and listed in the Outcome.
- [ ] `python3 scripts/check-wasm-tests.py` passes; test-owner mappings are current.

## Verification

```sh
python3 scripts/check.py typecheck
python3 scripts/check-wasm-tests.py
wasm-pack test --headless --chrome web/crates/parts --locked --lib
wasm-pack test --headless --chrome web/crates/pcb --locked --lib
wasm-pack test --headless --chrome web/crates/case --locked --lib
python3 scripts/run-wasm-tests.py --files web/src/presentation/case_workspace.rs
```

## Out of scope

- Changing how any of these panels submit edits (tickets 08, 14, 15, 16).
- Generation, CAD or preview ports (spec: out of scope).

## Pitfalls

- Runtime is wasm-only. Native `cargo test` does not compile it; always run the
  wasm typecheck.
- Don't replace the interception with a new feature-named branch. The point is
  that tests take the production path.
