# 106: Forward reconciliation of matrices, layouts and parts

Status: claimed
Type: build
Blocked by: [105](105-mirrored-half-derivation-kernel.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

Build the forward reconciliation kernel for matrices, keys, layouts and parts, using
stable source/mirror associations. Test it directly; do not wire it into production edits.

## Current approved execution authority (2026-10-11)

The user explicitly approved this removal-only internal wrapper in `core/src/matrix/deletion.rs`:

```rust
pub(crate) fn remove_parts_by_id(doc: &mut ProjectDoc, removed_ids: &BTreeSet<String>) -> Vec<String>
```

It converts the removed IDs to removal-only changes and calls the existing private `remap_parts`; `PartChanges` and `remap_parts` remain PRIVATE. Replace the duplicate transition cleanup in the staged reconciliation path, preserving the Local parent when its Omitted companion alone is removed. Current unit lease: `core/src/mirrored_half/reconcile.rs` and this narrow `core/src/matrix/deletion.rs` wrapper only; no other visibility, public API or schema change is authorized in this unit. The earlier HUMAN visibility HOLD is lifted.

B owns the standing mirror chain under parent19 and the central workflow: complete the coherent unit with Luna, STOP at the completed handoff, obtain one fresh Sol 6.1 medium acceptance review, then continue dependency-ready approved children without another per-helper/test/child root START. Closely coupled private slices may share that coherent review; record each ticket’s satisfied acceptance. Preserve all known counterexamples and nondefault/preservation proof obligations; do not claim full106/19 acceptance from the bounded 44/44 suite. This private backend wrapper alone enables no user-facing operation; production behavior still requires the approved activation chain.

## Setup

Claimed by Luna 6 high in `/home/chris/01_Projects/ts-boardstudio2/.worktrees/lane-mirror-core`
on `lane/mirror-core` at `767b31fa`. The checkout was clean and matched the granted base.
The bound index `boardstudio-lane-mirror-core` was queried for `derive::part` (HIGH risk;
direct caller `new_board`, through `create` and `lib.rs::apply`; 10 creation flows) and
`matrix::layout::reflected` (CRITICAL risk; direct callers include `apply`, `new_board`,
`sync`, plus layout tests; 8 affected processes including edit and key movement). No
existing shared helper or activation path is included in this ticket's initial write lease.
The Rust skills were read from the configured project skill paths, along with the TDD
skill and repository Rust instructions. The initial setup command window was released
before test or build work; all runtime behavior remains unimplemented. The claim commit
puts the unique lane index one commit behind HEAD; no refresh was run. The initial direct
test draft covers matrix projection only and does not establish the full 106 acceptance.

## Evidence and scope

Source evidence at `2478fb151`: `lib.rs::apply(RemoveMatrix)` owns matrix parts, layouts
and empty matrix-owned nets; `matrix.rs::removed_members` removes a switch with its
companions, but deleting a companion removes only that assembly;
`matrix/layout.rs::remove_matrix` preserves independent layout components.

- Re-derive linked objects through the pair map with existing identities and source numbering.
- Handle source additions, removals, matrix resizing, memberships and dependent references;
  report the actual changed identities without churning unchanged objects.
- Keep matrix assemblies and materialized parts consistent. Preserve each item's mode/tweak
  through correspondence rebuilds; retain labels, parameters, side and key bindings.
- Honor durable local/omitted association and owner-projection inputs supplied by 109;
  establish the reconciliation input seam now, without exposing lifecycle actions.
- Keep legacy same-board `LayoutMirrorLink` behavior; a matrix cannot have both link systems.
  New native pairs must not use `boardstudio.mirroredComponentSource`.
- Do not remove unrelated independent parts/layout components or reconcile hosted/outline data here.

## File ownership

Exclusive during this slice: `core/src/mirrored_half.rs` and its private child modules,
`core/src/mechanical/halves.rs` for reconciliation helpers, narrowly required
`core/src/matrix.rs` and `core/src/matrix/layout.rs` helpers, matrix/part reconciliation
cases in `core/tests/mirrored_half.rs`. A separate test file may be registered with a
single entry in `core/tests/integration.rs`; root assigns any additional path first.
`core/src/lib.rs` production dispatch remains untouched.

## Test-first and acceptance

- [ ] Direct-kernel tests fail for the expected absent reconciliation behavior first.
- [ ] Source move/rotation, cell placement/splay/resize, part parameters, additions and
      removals produce the expected linked mirror under axis and rotated maps.
- [ ] Switch/companion pose, tweak, stabiliser and fallback contracts remain intact.
- [ ] A second reconciliation is a no-op; IDs and unrelated local values remain stable.
- [ ] Removing a matrix or switch follows existing ownership; independent layout parts survive.
- [ ] Local/omitted test inputs are excluded without changing neighboring states.

Independent verifier: disable forward mapping, one addition/removal cleanup site and mode/tweak
retention separately; each assertion must fail. Engine Undo/Open/preview are deferred to 111.

## Execution and verification gates

This child does not resolve parent19. Follow [19](19-reconcile-mirrored-halves.md), the current central [lane registry](../lane-registry.md) and `docs/agents/agent-workflow.md` standing authority. B coordinates required planning → Luna implementation → ONE fresh Sol 6.1 medium acceptance per coherent unit → dependency-ready continuation; no repeated root START for approved private helpers, tests or fixed-scope children. Keep each ticket’s acceptance explicit, all meaningful regressions/counterexamples intact, and production activation at111. Already approved feature API/schema contracts are authorized as specified; genuinely new/unsettled choices or visibility changes still escalate.

Use `/home/chris/01_Projects/ts-boardstudio2/.worktrees/lane-mirror-core`, branch `lane/mirror-core`; inspect overlaps and source/index binding before edits. Run bound impact and complete nonpartial/nontruncated change analysis; warn on HIGH/CRITICAL and source-confirm unresolved UNKNOWN/stale answers. Root alone refreshes `boardstudio-lane-mirror-core`, integrates and owns goldens/shared tracking.

Request one bounded foreground sequence from root, then run its commands sequentially without per-command approval; release explicitly. Approved disjoint non-shell source work may overlap another lane’s ordinary checks. Affected checks: `repo lint test typecheck`, plus native/WASM boundary parity when behavior/transcript changes. Reuse sufficient source-bound evidence; target concrete gaps and important claimed guarantees. No full check per slice, `release` or background checks; Cargo uses12 jobs. Preserve explicit staging, actual-author trailers and all Git safety rules. Report completed acceptance, exact evidence, unrun/blocked gates and the explicit release; root owns integration commits.

## Guarded prototype outcome (2026-10-10)

Status remains **claimed** and the ticket is **incomplete**. This outcome records a guarded structural prototype only; it does not claim implementation or verification of full ticket 106. The approved Rust snapshot is 480257d66a6085acdc0f30b879622a2b73e89675855ff4c003ee0d1a36d3d7d7; lane HEAD is 5bcf4d381aa18cf609d96485e17464dd5fe3de2f. Root integration, GitNexus refresh/delta review and commit are pending.

The prototype covers only a 1x2 matrix with one linked and one retained unsplayed cell, without assembly. Its private atomic guard uses canonical data, an incoming inactive cell and filtered previous ownership; it calls the unchanged set_matrix once and finishes with the retained cell active, row-major normalized membership, and validators run. The bounded diff reports actual Matrix/Part changes and no-ops. It does not exercise constraint solving, envelope geometry or generalized matrix behavior.

The initial independent review found two P2 proof gaps: dependency comparisons were empty and global outline was omitted; the repeated-call assertions checked only empty IDs rather than document equality. An attempted test repair used the wrong source keys and omitted the source board and envelope, retained MIRROR; it also referenced Pad.number instead of the actual Pad.id. Its focused pass therefore did not prove retained dependencies. This scope error and invalid pad reference were corrected in the approved test patch; they are not evidence of the earlier patch's correctness.

A fresh Luna corrective pass changed only the approved single test. It now projects the explicit probe pose (4, 2) @ 0, while the actual retained mirror is registered MIRROR-only at pad-one + probe / pad-two; it checks the probe-to-retained offset (-4, -2) @ 0, global envelope inclusion of both MIRROR and probe, MIRROR-only registration, source-board exclusions, and non-empty before/after pad-ID, endpoint, owner and registration assertions. It also checks exact global outline and full project-document equality after the second call. The original four ownership assertions and all 286 original test lines remain in order, with 275 lines added. The independent Sol 6.1 xhigh reviewer assessed the actual corrected patch and closed both P2 findings for this guarded structural scope only. Non-test bytes match the approved Rust snapshot exactly (SHA c1539b25c72b8b440b228b19d8e7dd13143c86afdc49d12cf99ef24545408511).

Corrected focused check passed (1/1, 310 filtered): CARGO_BUILD_JOBS=12 BOARDSTUDIO_CHECK_JOBS=1 cargo test --locked -p boardstudio_core --lib mirrored_half::reconcile::tests::changed_source_matrix_projects_linked_neighbor_after_entry_snapshot -- --exact (5.76s; reconcile-focused-test-480257d6.log). Final affected checks passed: CARGO_BUILD_JOBS=12 BOARDSTUDIO_CHECK_JOBS=1 python3 scripts/check.py repo lint test typecheck (repo 0.1s, lint 8.5s, test 83.2s, typecheck 10.2s; total 102.0s, wall 1m42.157s; reconcile-affected-checks-480257d6.log, SHA 35057f17f0b115c71fb0e860a5c91ce8ee4c5d5ea9839b519817d3f8171963c5). The approved Rust source hash matched before and after. Earlier five-module/90.2s results are superseded by this final affected-check run. Optional post-hoc test corruptions were not authorized, needed or performed; no production-kernel mutation verification is claimed.

Evidence is under /home/chris/.cache/boardstudio-baselines/ticket106-planning/: guarded-p2-independent-final-assessment-480257d6.md; reconcile-corrected-reference-test-480257d6.rs, reconcile-corrected-reference-test-480257d6.patch, reconcile-corrected-reference-test-480257d6.crosswalk.txt, and reconcile-corrected-reference-test-480257d6.provenance.json (snapshot/diff SHA 9074c15d2f7f09fc94492468e93c908e4346dff90966b66ddfd563fb9352b594); plus reconcile-pre-reference-test-c1539b25.rs and its provenance. raw-pin-pad-id-validation-319-372-c1539b25.txt/json records that Pin.pad_id == Pad.id; raw constructor packets were also captured.

The remaining ticket scope includes generalized matrices, retained keys, geometry, splay and ownership; assemblies and per-item mode/tweak/stabiliser/fallback behavior; independent parts/layouts and legacy-link exclusivity; complete actual-document diffs; additions, expansion and source numbering; owned removals, shrinking and eligible local omitted-owner projection; deletion from a valid pre-edit baseline and caller-111 access; source-atomic failure; independent production mutations; and full combined acceptance. GitNexus private-gate evidence remains a lower bound with UNKNOWN/behind-one caveats; root refresh and delta review are pending. Proposed next serialized batches are planning notes, not authorization: (1) generalized matrix with fixed membership; (2) assembly metadata; (3) independent parts/layouts with fixed topology; (4) full actual diff and transition-evidence contract; (5) additions/membership; (6) owned removals, combined convergence and independent mutations. Ticket 109 owns exception lifecycle/final-owner input; ticket 111 owns baseline and outer-rollback/private-caller-access decisions. No new API, schema or visibility, ticket-109 policy or ticket-107 work is included.

## Accepted membership-transition unit (2026-10-11)

Bounded private context/common-emitter/owned-cleanup unit accepted by ONE fresh independent Sol6.1 medium session after the preserved Luna writer. Source commit0fc21ca5e, lane integration sync22f31e218, no-ff integration465137ee5. Final reconcile.rs SHA9598c3e302863b42bcc462e453001b2c3b2ad64a7d8a07c949b583779c1b7390 and deletion.rs SHA58401caf581c797535435d0ad630fd72943ba70d4b2107d395f2c83b4263548e. Reviewer found shifted Local-parent/Omitted-child compaction failure by compiled expectedRED; same writer corrected identity-map lookup and retained the regression. Corrected focused45/45 and affected repo/lint/test/typecheck95.1s PASS. ONE new-wrapper bypass mutation produced the intended populated-keymap cleanupRED; exact restoration and independent45/45GREEN passed. Four earlier guarantee controls explicitly reused, not silently waived.

Independent final authority: `/home/chris/.cache/boardstudio-baselines/ticket106-transition/validated-context-independent-medium/independent-assessment.md` SHA34407f688f928f51c88e808d222ce53f2f64ff2903bafd8be23eb454e9c45f05. Complete source-bound graph163/163symbols,2files,32processes CRITICAL; exact staged/unstaged delta unchanged across staging (SHAee2848a0747d3cd2555bc290e5e57e3d21f0c56d2fe287ab8d363723b849b672). Bindings/raw logs under validated-context-implementation, including shifted-fix-final-index-binding.json, shifted-fix-final-graph-delta.json and accepted-staged-delta-binding.json. Integration full batch remains queued with138; no production activation claimed.

Status stays claimed/full106OPEN. Next bounded unit: fixed-topology independent Parts/Layout reconciliation with stable IDs/full payloads, nonempty unrelated layout-component preservation, exact repeat, positive legacy preservation/exclusivity at its actual seam. Generalized additions/removals, whole-matrix removal, remaining combined move/rotation/splay/resize/parameter/stabiliser/fallback cases and complete106 acceptance remain explicit. Durable109 lifecycle and111/115 production history/Open/preview gates remain separate.

### Accepted independent Parts and fixed Layout unit (2026-10-11)

Source commit `52ee10b2` is integrated with no-ff merge `9d6439eeb90ba937b2cf73279fa18262c3bc515d`. ONE fresh Sol6.1 medium accepted the completed Luna6 high unit: `/home/chris/.cache/boardstudio-baselines/ticket106-independent-parts-layouts/independent-medium/independent-assessment.md`, SHA256 `6ec690fbf3f51092bffdef4a5b040715c4bc5c6925034a0f7a176f4360eba5c1`. Existing independent Part correspondences project full source payload and mapped pose while retaining target identities; fixed paired Layout memberships follow source order and retain unmatched target locals. Real unrelated Parts/Layout/owner data and exact second-output convergence are covered. Generalized topology, additions/removals and whole-matrix removal remain open.

Independent baseline passed; two compiled mutations failed for the intended literal mapping and target-local membership guarantees; exact restore returned all47 focused tests to GREEN. Final source SHA256 `ed7e400b61af0b49162b46d8f3e15be17dd0c413b75713cb31fc0116448f3a02`. Writer affected repo/lint/test/typecheck PASS99.6s is reused from recovered actual tool observations; intermediate affected stdout was truncated and full raw logs/nested exit metadata are not claimed. Final independent raw logs and exact exit/hash records are in `independent-medium/probe-results.json`. Root analyzer succeeded37s; dirty wrapper warning is separate from exact indexed source coverage. Complete4/4-symbol, one-file, HIGH/eight-process delta is saved in `root-capture/graph-delta.json`. Staged diff SHA256 `a5549e3ef601c71be68c67bb5e97fd2057fcce23e5068da17187b99bba2da2be` exactly matches the reviewed delta, so complete graph analysis was reused unchanged.

Next B unit completes remaining document-wide Layout/source additions-removals and whole-matrix closure together under standing authority, with one writer and fresh independent acceptance. Ticket139 isolated math preparation stays queued behind this blocker. Full106/19 and production activation remain open; one integration batch check is pending with138.
