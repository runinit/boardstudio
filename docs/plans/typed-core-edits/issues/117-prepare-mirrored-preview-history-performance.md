# 117: Prepare preview isolation, history and performance evidence

Status: ready-for-agent
Type: build
Blocked by: [116](116-prepare-mirrored-open-repair.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

Prepare a bounded native-pair preview/history strategy and paired performance harness using
115/116's inert transaction path. Production activation remains 111; parent 19 owns final cost evidence.

## Scope and evidence

At `2478fb151`, `lib.rs::preview_edit` temporarily mutates accepted data with PreviewBackup;
reconciliation may touch far more than the original operation. `edit` publishes one history
snapshot; `history` restores documents. Existing `matrix_performance.rs` alone may not exercise pairs.

- Prepare a private route that uses the existing cloned preview path for native-paired documents
  if complete optimized backup would expand scope. Preserve the optimized legacy/unpaired route.
  Root reviews any alternative; do not invent a second broad rollback mechanism.
- Exercise private transaction publication/history preparation so one successful edit has one
  snapshot and refused/retired/no-op previews publish none. Do not enable operation routing yet.
- Prepare a representative paired-document cost measurement around existing 115 composition,
  alongside the existing performance guard. Record baseline/candidate costs only in root's exclusive
  quiet slot; final activated cost is measured during parent 19 acceptance.
- Prepare additive boundary scenarios using existing harness, with no golden blessing by children.

## File ownership

Exclusive: private preparation helpers in `core/src/lib.rs` or existing private child modules,
preview/history cases in established mirrored-half tests, narrowly additive paired measurement in
`core/tests/matrix_performance.rs`, `core/tests/support/boundary.rs` additive scenarios only.
Production preview/history routing and all generated goldens remain untouched until 111/root.

## Test-first and acceptance

- [ ] Direct helper red/green cases show complete accepted-document/revision/history isolation
      after successful, refused and retired paired previews, including local/mixed operations.
- [ ] Prepared commit and Undo/Redo restore both halves and exception state with one snapshot;
      nonempty-closure Unlink Undo restores the paired snapshot and Redo restores independent
      materialized configurations/IDs. Prepared reopen preserves the independent result.
- [ ] Existing guard plus paired harness can report added cost; no timing claim without an
      exclusive root-granted slot and actual observed measurements.
- [ ] Boundary scenarios are additive and ready for root blessing; no partial runtime activation.

Independent verifier: mutate preview isolation, one-snapshot publication and exception history
restoration. Final production performance/Undo/Open evidence remains a separate parent review.
