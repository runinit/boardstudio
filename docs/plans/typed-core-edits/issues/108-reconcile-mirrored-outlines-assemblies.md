# 108: Forward reconciliation of outlines, versions and assemblies

Status: claimed
Type: build
Blocked by: [107](107-reconcile-mirrored-hosted-electrical-items.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

Complete forward derivation for outlines, outline versions and assembly references,
including anchored geometry. Continue to leave the kernel unactivated.

## Evidence and scope

Creation's `mirrored_half.rs::new_board` at `2478fb151` already maps outline identities,
part-anchored geometry and protected gaps; 18/26 record those contracts. Existing mechanical
pair ownership regenerates closure output; reconciliation must not copy `case-closure/*`
or replace linked mechanical features with independent clones.

- Keep stable outline/version identities, provenance and anchors while source geometry,
  versions, assemblies and referenced parts change.
- Use existing outline ownership/resolution behavior for fixed and linked versions;
  source removal cleans only genuinely dependent references/choices.
- Preserve shared document-level assembly definitions where creation shares them;
  reconcile owned references rather than cloning global definitions unnecessarily.
- Derive mechanical effects through the existing pair/feature ownership pipeline.
- Honor exceptions and owned projections; do not activate edit/Open reconciliation.

User-reported Inspector gap behavior is tracked separately in [141](141-mirrored-outline-gap-controls.md). Reuse its actual one-sided edit/control-identity/contour observations when available. Establish whether the controls belong to a linked correspondence or genuinely independent outlines before changing coupling or collapsing rows. This ticket's protected-gap mapping acceptance must preserve stable identities and ownership; a duplicated-looking control is not by itself proof of a generator defect. Coordinate Inspector changes with L3/E and reproduce any shared geometry bug before changing the outline engine.

## File ownership

Exclusive during this slice: `core/src/mirrored_half.rs` and private child modules,
`core/src/mechanical/halves.rs`, outline reconciliation tests in the established mirrored-half
test files. If an existing outline/mechanical helper requires editing, first report its exact
path/symbol and obtain root's ownership assignment; no broad outline subsystem refactor.

## Test-first and acceptance

- [ ] Direct-kernel tests first fail for source outline/version/anchor changes not propagating.
- [ ] Axis and rotated-map cases preserve stable identities and correctly map independent-part
      anchors, protected gaps and linked/fixed version ownership.
- [ ] Removal cleans dependent references while unrelated versions/choices remain intact.
- [ ] Assemblies remain consistent, no closure parts are copied, and a second pass is a no-op.

Independent verifier: mutation-check anchored geometry mapping, stable version/reference
mapping and closure-copy exclusion. Engine ordering and Open repair remain 111's responsibility.

## Ticket 141 gap-setting observations

Ticket141's symmetric mounted observation is now available at `/home/chris/.cache/boardstudio-baselines/mirrored-outline-gaps/141-symmetric-observation-captured.json` (SHA2564bca5eb4fdb9e51e897abcc4479a3c8a884a9c0562de5115af9e65e688386133). Inherited protection is recognized on both outlines, but a one-sided uncheck leaves the other protected at the current unactivated baseline. Use this as the real Inspector gap-setting case for the approved linked-outline reconciliation/inverse-edit/activation contract. Exact candidate geometry remains qualified pending same-writer captured-data analysis; no speculative identity/geometry fix or independent-setting policy change is implied.

141 geometry qualification subsequently rejects exact creation-baseline contour reflection (5.09/5.9444 maximum bidirectional distances at0.002 tolerance), so inherited protection association alone must not certify the same reflected recess. Evidence: `mirrored-outline-gaps/141-geometry-qualification.md` SHA163e39a6711e4e7daa86c5d4a8283636caea45ec625f3c8fb38c2a6ae49dcac7 under the baseline cache. E is tracing fixture/profile/frame/generator causality read-only; no Core fix is assigned from this observation alone.

E completed141's bounded source trace: symmetric fixture/profile/pose/control mappings appear consistent, yet captured unprotected outlines also diverge (max6.3813). Exact next diagnostic under B108Core ownership is the postCreate/pre-edit world courtyard rings plus automatic bridge endpoints on both halves, before cleanup; compare input rings first. Trace pointer: `/home/chris/.cache/boardstudio-baselines/mirrored-outline-gaps/141-causal-trace.md` SHAe2a677b8dd88a3edece697d5941cafa90fa3dbadc113c36437a400b7c0e05bf5. This is bounded causal observation, not permission for a speculative generator rewrite or107scope expansion.

141 user clarification supersedes any claim that the separate-outline observation reproduces their issue: the report involves ONE outline around BOTH halves. E is preparing that actual case. Preserve the separate-outline creation-contour evidence as related diagnosis only; it does not settle single-outline gap semantics or authorize a coupling/generator fix.

## Current bounded execution

107resolved/integrated; B clean synced6041b95b before SAME Luna high writer prepared108. Lease: NEW `core/src/mirrored_half/reconcile/outlines.rs` and `outlines/tests.rs`, narrow private module declaration/wiring in reconcile.rs only; no extraction/reformat of accepted106/107 or shared-helper writes. Five source-only cases cover rotated anchored geometry/settings/protection/version/provenance/fullrepeat, closure exclusion, Local/Omitted isolation with linked-neighbor update, mapped worldRect, and source-version removal preserving unrelated target choice/pair bytes. Inert private candidate seam awaits a meaningful compiled behavioral RED; fixture-validation failure is not that evidence. Three original mutation guarantees remain. Draft/source binding: `/home/chris/.cache/boardstudio-baselines/ticket108-outlines/implementation/`. Root107/139 integration cohort passed; RED/fix/GREEN/affected-check cycle follows short root capture windows. No108runtime or fix acceptance yet.
