# 108: Forward reconciliation of outlines, versions and assemblies

Status: ready-for-agent
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

## Execution and verification gates

This is a child of [19](19-reconcile-mirrored-halves.md); resolving it does not resolve 19.
Use the current central workflow and lane registry; they supersede historical serial/per-command rules. B may advance from independently accepted107 to this dependency-ready scope under standing authority, without another routine START. One writer owns the Core paths; disjoint worktrees may prepare source concurrently. Heavy builds/tests/browser/index runs retain explicit owner/release; activation and timing retain their integration gates.
Use `/home/chris/01_Projects/ts-boardstudio2/.worktrees/lane-mirror-core`, branch `lane/mirror-core`;
report path/branch/status and inspect overlaps before editing. Every shell call has that
absolute `workdir`; commands run one at a time in the foreground.

Read `docs/agents/agent-workflow.md`, `verification-process.md`, `gitnexus-index.md` and
parent 19. Luna 6 high implements test-first and records `implemented (awaiting verification)`;
a fresh Sol6.1 medium independently accepts the coherent unit, with justified escalation only when needed, then may set `resolved`.
Run bound GitNexus impact before every symbol edit; list callers, processes and risk.
`apply`/`layout::resolve` are known CRITICAL paths: warn before editing. UNKNOWN, stale or
missing answers need source confirmation and honest limitation reporting, not an all-clear.
Use `boardstudio-lane-mirror-core`; root alone refreshes. Run complete `detect_changes` before
any commit; truncated/partial evidence is not clean.

Request a complete bounded debugging/affected-check window, not a separate slot for every command. Run focused regressions, affected Core lint/tests and actual WASM compilation; add boundary parity when its behavior/transcript changes. Preserve the three original mutation guarantees with explicit evidence/disposition, without automatic full-suite or duplicate review runs. Full workspace/browser checks run at coherent integration boundaries. No `release` or background checks; active laptop mode is Cargo4/native test threads4/one suite/Binaryen4.
Maps, ADRs, CONTEXT.md, scripts/check.py and generated goldens remain root-owned.
No merges/push/PR, stash/reset/discard or broad staging; explicit paths only.
Commit only after root permits it, with the actual-author trailer. Outcome records exact
commands/results/timings, mutation failures/restoration and every unrun or blocked gate.

Ticket141's symmetric mounted observation is now available at `/home/chris/.cache/boardstudio-baselines/mirrored-outline-gaps/141-symmetric-observation-captured.json` (SHA2564bca5eb4fdb9e51e897abcc4479a3c8a884a9c0562de5115af9e65e688386133). Inherited protection is recognized on both outlines, but a one-sided uncheck leaves the other protected at the current unactivated baseline. Use this as the real Inspector gap-setting case for the approved linked-outline reconciliation/inverse-edit/activation contract. Exact candidate geometry remains qualified pending same-writer captured-data analysis; no speculative identity/geometry fix or independent-setting policy change is implied.

141 geometry qualification subsequently rejects exact creation-baseline contour reflection (5.09/5.9444 maximum bidirectional distances at0.002 tolerance), so inherited protection association alone must not certify the same reflected recess. Evidence: `mirrored-outline-gaps/141-geometry-qualification.md` SHA163e39a6711e4e7daa86c5d4a8283636caea45ec625f3c8fb38c2a6ae49dcac7 under the baseline cache. E is tracing fixture/profile/frame/generator causality read-only; no Core fix is assigned from this observation alone.

E completed141's bounded source trace: symmetric fixture/profile/pose/control mappings appear consistent, yet captured unprotected outlines also diverge (max6.3813). Exact next diagnostic under B108Core ownership is the postCreate/pre-edit world courtyard rings plus automatic bridge endpoints on both halves, before cleanup; compare input rings first. Trace pointer: `/home/chris/.cache/boardstudio-baselines/mirrored-outline-gaps/141-causal-trace.md` SHAe2a677b8dd88a3edece697d5941cafa90fa3dbadc113c36437a400b7c0e05bf5. This is bounded causal observation, not permission for a speculative generator rewrite or107scope expansion.
