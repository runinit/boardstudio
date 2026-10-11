# 106: Forward reconciliation of matrices, layouts and parts

Status: claimed
Type: build
Blocked by: [105](105-mirrored-half-derivation-kernel.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

Build the forward reconciliation kernel for matrices, keys, layouts and parts, using
stable source/mirror associations. Test it directly; do not wire it into production edits.

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

This is a child of [19](19-reconcile-mirrored-halves.md); resolving it does not resolve 19.
Core children execute serially with exclusive Core file/API leases. The user approved
independent A/F streams on 2026-10-10; follow the central lane registry and parallel-streams
decision for audited disjoint edits. Every shell/check command remains globally sequential;
activation and timing require exclusive windows. The prerequisite batch and delivered 37
verification slice are complete; root must refresh the bound index and grant this slice.
Use `/home/chris/01_Projects/ts-boardstudio2/.worktrees/lane-mirror-core`, branch `lane/mirror-core`;
report path/branch/status and inspect overlaps before editing. Every shell call has that
absolute `workdir`; commands run one at a time in the foreground.

Read `docs/agents/agent-workflow.md`, `verification-process.md`, `gitnexus-index.md` and
parent 19. Luna 6 high implements test-first and records `implemented (awaiting verification)`;
a fresh Sol 6.1 xhigh independently verifies/mutation-checks, then alone may set `resolved`.
Run bound GitNexus impact before every symbol edit; list callers, processes and risk.
`apply`/`layout::resolve` are known CRITICAL paths: warn before editing. UNKNOWN, stale or
missing answers need source confirmation and honest limitation reporting, not an all-clear.
Use `boardstudio-lane-mirror-core`; root alone refreshes. Run complete `detect_changes` before
any commit; truncated/partial evidence is not clean.

Request every test/build/browser/performance command slot from root before running it.
Affected checks: `repo lint test typecheck`, plus native/WASM boundary parity when its
behavior/transcript changes; targeted red/green and verifier mutations also need slots.
No full check per slice, no `release`, no background checks; Cargo uses 12 jobs.
Maps, ADRs, CONTEXT.md, scripts/check.py and generated goldens remain root-owned.
No merges/push/PR, stash/reset/discard or broad staging; explicit paths only.
Commit only after root permits it, with the actual-author trailer. Outcome records exact
commands/results/timings, mutation failures/restoration and every unrun or blocked gate.


## Guarded prototype outcome (2026-10-10)

Status remains **claimed** and the ticket is **incomplete**. This outcome records a guarded structural prototype only; it does not claim implementation or verification of full ticket 106. The approved Rust snapshot is 480257d66a6085acdc0f30b879622a2b73e89675855ff4c003ee0d1a36d3d7d7; lane HEAD is 5bcf4d381aa18cf609d96485e17464dd5fe3de2f. Root integration, GitNexus refresh/delta review and commit are pending.

The prototype covers only a 1x2 matrix with one linked and one retained unsplayed cell, without assembly. Its private atomic guard uses canonical data, an incoming inactive cell and filtered previous ownership; it calls the unchanged set_matrix once and finishes with the retained cell active, row-major normalized membership, and validators run. The bounded diff reports actual Matrix/Part changes and no-ops. It does not exercise constraint solving, envelope geometry or generalized matrix behavior.

The initial independent review found two P2 proof gaps: dependency comparisons were empty and global outline was omitted; the repeated-call assertions checked only empty IDs rather than document equality. An attempted test repair used the wrong source keys and omitted the source board and envelope, retained MIRROR; it also referenced Pad.number instead of the actual Pad.id. Its focused pass therefore did not prove retained dependencies. This scope error and invalid pad reference were corrected in the approved test patch; they are not evidence of the earlier patch's correctness.

A fresh Luna corrective pass changed only the approved single test. It now projects the explicit probe pose (4, 2) @ 0, while the actual retained mirror is registered MIRROR-only at pad-one + probe / pad-two; it checks the probe-to-retained offset (-4, -2) @ 0, global envelope inclusion of both MIRROR and probe, MIRROR-only registration, source-board exclusions, and non-empty before/after pad-ID, endpoint, owner and registration assertions. It also checks exact global outline and full project-document equality after the second call. The original four ownership assertions and all 286 original test lines remain in order, with 275 lines added. The independent Sol 6.1 xhigh reviewer assessed the actual corrected patch and closed both P2 findings for this guarded structural scope only. Non-test bytes match the approved Rust snapshot exactly (SHA c1539b25c72b8b440b228b19d8e7dd13143c86afdc49d12cf99ef24545408511).

Corrected focused check passed (1/1, 310 filtered): CARGO_BUILD_JOBS=12 BOARDSTUDIO_CHECK_JOBS=1 cargo test --locked -p boardstudio_core --lib mirrored_half::reconcile::tests::changed_source_matrix_projects_linked_neighbor_after_entry_snapshot -- --exact (5.76s; reconcile-focused-test-480257d6.log). Final affected checks passed: CARGO_BUILD_JOBS=12 BOARDSTUDIO_CHECK_JOBS=1 python3 scripts/check.py repo lint test typecheck (repo 0.1s, lint 8.5s, test 83.2s, typecheck 10.2s; total 102.0s, wall 1m42.157s; reconcile-affected-checks-480257d6.log, SHA 35057f17f0b115c71fb0e860a5c91ce8ee4c5d5ea9839b519817d3f8171963c5). The approved Rust source hash matched before and after. Earlier five-module/90.2s results are superseded by this final affected-check run. Optional post-hoc test corruptions were not authorized, needed or performed; no production-kernel mutation verification is claimed.

Evidence is under /home/chris/.cache/boardstudio-baselines/ticket106-planning/: guarded-p2-independent-final-assessment-480257d6.md; reconcile-corrected-reference-test-480257d6.rs, reconcile-corrected-reference-test-480257d6.patch, reconcile-corrected-reference-test-480257d6.crosswalk.txt, and reconcile-corrected-reference-test-480257d6.provenance.json (snapshot/diff SHA 9074c15d2f7f09fc94492468e93c908e4346dff90966b66ddfd563fb9352b594); plus reconcile-pre-reference-test-c1539b25.rs and its provenance. raw-pin-pad-id-validation-319-372-c1539b25.txt/json records that Pin.pad_id == Pad.id; raw constructor packets were also captured.

The remaining ticket scope includes generalized matrices, retained keys, geometry, splay and ownership; assemblies and per-item mode/tweak/stabiliser/fallback behavior; independent parts/layouts and legacy-link exclusivity; complete actual-document diffs; additions, expansion and source numbering; owned removals, shrinking and eligible local omitted-owner projection; deletion from a valid pre-edit baseline and caller-111 access; source-atomic failure; independent production mutations; and full combined acceptance. GitNexus private-gate evidence remains a lower bound with UNKNOWN/behind-one caveats; root refresh and delta review are pending. Proposed next serialized batches are planning notes, not authorization: (1) generalized matrix with fixed membership; (2) assembly metadata; (3) independent parts/layouts with fixed topology; (4) full actual diff and transition-evidence contract; (5) additions/membership; (6) owned removals, combined convergence and independent mutations. Ticket 109 owns exception lifecycle/final-owner input; ticket 111 owns baseline and outer-rollback/private-caller-access decisions. No new API, schema or visibility, ticket-109 policy or ticket-107 work is included.
