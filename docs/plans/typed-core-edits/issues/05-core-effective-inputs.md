# 05: Core effective mechanical inputs

Status: ready-for-agent
Type: build
Blocked by: none
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Parent: [02](02-mechanical-settings-patch.md)

## Question / goal

Core owns the effective mechanical view. Add one synchronous read-only Core
function, `effective_inputs(document, board_contours, target) -> Result<MechanicalInputs, String>`,
that resolves the configuration a board or physical half actually uses: canonical
settings, the legacy shared overlay with first-instance fallback, instance settings,
board thickness, wireless battery defaults and flipped-instance handling. The
existing host helpers (`captured_case_document`, `effective_case_inputs`) become
adapters over it, and the mechanical defaults the Case crate still copies move into
Core.

This is the shared read path that apply, evaluation, findings and presentation all
reuse. It replaces the host `effective_instance_configuration` serde overlay and the
Case `initial_settings` defaults.

## Seams under test

- `boardstudio_core::mechanical::effective_inputs` (public Core read path).
- `boardstudio_core::mechanical::defaults` (Core-owned defaults).
- The existing `CoreEngine` scene/document replies, which must keep using the same
  effective view.

## TDD cases

1. A canonical configuration with no hardware resolves to itself.
2. A physical instance with `Legacy` ownership and a `shared_construction` resolves
   the overlay (method/style/dimensional/foam/gasket/processes), keeping instance
   `mounts`, `openings`, `battery` and `profiles` local.
3. A `Legacy` instance with no shared source falls back to the first instance's
   configuration, preserving today's behaviour even when the retired flag was false.
4. A `Local` instance ignores the shared overlay.
5. A flipped instance keeps board-local X reflection and the board thickness.
6. A wireless transport supplies the battery default when none is stored.
7. Missing/ineligible targets return a Core error, not a panic.

## Verification

`cargo test -p boardstudio_core --locked` and `cargo test -p boardstudio-web-case
--locked`; WASM `typecheck`. Update the boundary transcript only if the wire shape
changes.
