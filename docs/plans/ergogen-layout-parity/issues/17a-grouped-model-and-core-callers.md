# 17a: Grouped model and Core callers

Status: ready-for-agent
Type: build
Blocked by: none
Parent: [17](17-cut-over-format-and-model.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)

## Outcome

Persist the grouped layout records and cut every removed-field Core caller over to them. The five currently failing integration tests must pass: `core_engine::deleting_matrix_column_compacts_both_halves_and_preserves_local_assemblies`; `outline_linked_resolution::a_failed_split_resolution_keeps_its_last_valid_display_without_a_false_blocker`; `outline_linked_resolution::a_split_linked_refinement_derives_its_region_count_and_never_blocks_as_disconnected`; `outline_linked_resolution::merged_fixed_and_generated_outlines_keep_their_region_count_semantics`; `outlines::manual_connections_respect_linked_split_boundaries`.

## Acceptance

- Persist column `rotation` as well as Spread/Stagger/Splay/origin/offset, row records, cell Shift/Bend and optional single-key anchor; replace Layout with `matrixIds`.
- Extend the same-board follower link's explicit correspondence/exception owner as specified; do not fake a physical pair or infer matrix pairs from list order.
- Remove fields to inventory compiler errors, then update every caller together: Core model/validation/materialization, matrix layout/splay/deletion, mirror inverse/reconcile, geometry/outline ownership, boundary projection, web tree/selection, setup/Inspector/transform consumers and their constructors/fixtures. No old formula survives as a fallback.
- The five failing integration tests listed in the Outcome are fixed.

## Checks

Source anchors: `core/src/model.rs`, `matrix.rs`, `matrix/layout.rs`, `mirrored_half/`, `geometry.rs`, `outline_versions.rs`, `web/crates/ui-model/src/tree.rs`. Use compiler-driven inventory, the focused public tests above and `check.py quick --files` for changed callers.

## Progress

- [ ] 
