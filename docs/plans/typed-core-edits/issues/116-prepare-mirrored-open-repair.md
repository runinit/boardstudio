# 116: Prepare inert Open repair and warning derivation

Status: claimed
Type: build
Blocked by: [115](115-prepare-mirrored-edit-transaction.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

Prepare source-authoritative Open reconciliation and its warning, without enabling the
production Open hook. Reuse verified kernels; do not invent a second reflection implementation.

## Scope and evidence

`core/src/lib.rs::handle(Open)` at `2478fb151` resolves scripts/layout/components then validates
halves but does not derive linked mirrors. Parent 19 requires altered linked mirror data to be
re-derived with a warning; local/omitted data and ADR-excluded fields remain preserved.

- Prepare private Open draft ordering: existing normalization/resolution, linked derivation,
  cleanup, final validation. Compare relevant mirror inputs before overwriting to detect repair.
- Produce a located warning through an existing finding/report shape. Reopening the repaired
  accepted document is stable and does not repeatedly report a new repair.
- Preserve labels, per-item modes/tweaks/exceptions, key bindings, parameters/side, references
  and copper exclusions. Malformed or ambiguous associations follow 109's compatibility rules.
- Missing-object recovery is deliberately bounded: restore only a missing independently
  corresponded Part when its stored target identity, source ownership and derived reference are
  unique. Other missing linked targets remain strict validation refusals unless an existing
  Omitted owner rule allows them to remain absent; do not infer additional target identities.
- No storage format change or archive/browser decode refusal path is added. Production Open
  activation and end-to-end boundary evidence belong to 111 and parent acceptance.

## File ownership

Exclusive: private Open preparation in `core/src/lib.rs` or its existing private child modules,
private mirrored-half helpers and established mirrored-half tests. Existing finding shapes may
be reused; any additional schema/archive/storage path needs root approval/ownership first.
No production Open branch change, visibility widening or generated fixture edit.

## Test-first and acceptance

- [ ] Direct prepared-Open red/green cases cover altered linked mirror geometry, a missing
      independently corresponded Part within the recovery boundary above, ordinary saved data,
      local/omitted exceptions and malformed association refusal.
- [ ] Existing matrixless pairs still repair linked non-matrix data, and an absent Omitted Matrix
      remains absent through preparation.
- [ ] Repair reports a located warning and stable second preparation; unrelated local/ADR-excluded
      data remains unchanged and the accepted input is not mutated on failure.
- [ ] Existing legacy/unpaired/reuse-board behavior is preserved; runtime Open is still unchanged.

Independent verifier: disable repair, pre-overwrite warning detection and local/omitted protection
separately; assert meaningful failures. Actual save/reopen and native/WASM transcript are final gates.
