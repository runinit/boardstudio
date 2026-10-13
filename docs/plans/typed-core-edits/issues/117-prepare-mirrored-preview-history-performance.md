# 117: Prepare preview isolation, history and performance evidence

Status: resolved
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

- [x] Direct helper red/green cases show complete accepted-document/revision/history isolation
      after successful, refused and retired paired previews, including local/mixed operations.
- [x] Prepared commit and Undo/Redo restore both halves and exception state with one snapshot;
      nonempty-closure Unlink Undo restores the paired snapshot and Redo restores independent
      materialized configurations/IDs. Prepared reopen preserves the independent result.
- [x] Existing guard plus paired harness can report added cost; no timing claim without an
      exclusive root-granted slot and actual observed measurements.
- [x] Boundary scenarios are additive and ready for root blessing; no partial runtime activation.

## Outcome (2026-10-13)

- Paired-preview behavior is exercised by calling the inert `prepare_transaction` path directly.
  Production `CoreEngine::preview_edit` remains on its existing route until ticket 111; no
  generated golden was changed.
- Prepared publication/history coverage proves one snapshot, paired Undo/Redo and Local exception
  preservation. Nonempty-closure Unlink Undo/Redo and prepared reopen preserve the materialized
  configurations and IDs.
- The additive boundary scenario is ready for root blessing. The existing unpaired 6x5/90-part
  guard measured 520 µs p50 / 555 µs p95. A paired 6x5 case with 90 matrix-owned parts per half
  measured 9,336 µs p50 / 10,451 µs p95. These workloads contain different amounts of data, so the
  figures are directional; parent 19 owns final activated performance acceptance.
- The native baseline update recorded 1,785 tests. The targeted preparation module passes 33 tests;
  the paired performance target compiles, the additive boundary case passes, and the mutation
  verifier killed all 3 mutants. Evidence and logs are under the gitignored `.evidence/ticket117/`.
- Independent review: Spec found no remaining issue. Standards found no hard violation; it noted
  the prepared-publication parameters as a mild Data Clumps judgment call.

Independent verifier: mutate preview isolation, one-snapshot publication and exception history
restoration. Final production performance/Undo/Open evidence remains a separate parent review.
