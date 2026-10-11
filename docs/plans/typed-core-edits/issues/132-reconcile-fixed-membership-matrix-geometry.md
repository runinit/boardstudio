# 132: Reconcile fixed-membership matrix geometry and retained keys

Status: implemented (awaiting verification)
Type: build
Lane: B
Blocked by: 105
Parent: [106](106-reconcile-mirrored-matrices-layouts-parts.md)
Execution gate: independent bounded acceptance passed; root commit/integration and cohort batch are pending. Source write and mutation leases are released.

Generalize the private unactivated reconciliation prototype for matching fixed active cells and multiple retained keys. This bounded child does not complete 106 or activate reconciliation. Baseline: `lane/mirror-core` at `0ea2411de21b58e177e5cd06bf1f1b1c76ab8f46`.

Implementer: fresh Luna 6 high. Verifier: distinct fresh Sol 6.1 xhigh. No Fast/priority request. Allowed source: only `core/src/mirrored_half/reconcile.rs`; existing tests must remain. Root owns all ticket/map edits.

Planner: existing lane B Sol 6.1 max session. Independent planning reviewer: `/root/review_fixed_membership_plan`, Sol 6.1 max, PASS with no numeric corrections. Reviewed packet SHA-256 `ad44fd55206e37074733f0e39ee80bea2e2d4ab15814db081932a5b9125eb14b`; independent review SHA-256 `298dad1d855c00cbcd9cd303cff54fe2c44bea4185a910081c0fc664d09c3495`. Both are retained in `/home/chris/.cache/boardstudio-baselines/ticket106-planning/`.

## Required dispatch clarifications

- Build both six-key matrices, all six canonical associations and the requested pair map coherently before entry. Validate both matrices/document validators and literal source poses before the expected current membership-guard RED.
- Require matching unique active slots and an exact disjoint linked/retained partition; reject aliases, missing endpoints, incomplete owners, membership changes and nonfinite consumed inputs/results atomically.
- Calculate the inclusive signed splay sum locally with typed Mirror matching. The sibling `column_rotation` stays private; no visibility or shared-helper change is authorized.
- Mask both retained keys in incoming generation and previous ownership, materialize once, keep normalization, then overlay active retained cells and canonical row-major membership.
- Preserve nonempty dependencies including one on the second retained key. Assert literal sorted changed IDs: `matrix/mirror-matrix/r0c1`, `matrix/mirror-matrix/r0c2`, `matrix/mirror-matrix/r1c0`, `matrix/mirror-matrix/r1c2`, `mirror-matrix`. Require empty repeat IDs and exact whole-document equality.
- Independent verifier separately mutation-checks inverse order and second-retained-key protection; restore exact source bytes, touch, then rerun green. Runtime proof remains unexecuted.

## Accepted implementation plan and numeric oracle

The retained packet below describes the completed planning phase; allocation and current execution gates are stated above.

The captured private heading helper (`/home/chris/.cache/boardstudio-baselines/ticket106-planning/raw-column-rotation-344-355-0ea2411d-20261011T015220540Z.txt`) sums splays through the column and negates that sum for MirrorX or MirrorY. No helper dependency remains unresolved. Provenance (`/home/chris/.cache/boardstudio-baselines/ticket106-planning/raw-column-rotation-344-355-0ea2411d-20261011T015220540Z.json`) records the complete matrix-file hash and graph staleness of two commits.

The one-session change should be confined to reconcile.rs (`/home/chris/01_Projects/ts-boardstudio2/.worktrees/lane-mirror-core/core/src/mirrored_half/reconcile.rs`): generalize private geometry validation/view construction and add focused tests. Keep shared helpers, visibility and document schema unchanged.

The private view builder consumes canonical source/mirror matrices, the projected geometry and retained keys resolved to actual cell slots. It returns incoming-generation suppression, filtered previous ownership and final retained-cell data. Continue the two-mask setter path, retaining its normalized result before installing complete active cells and canonical membership.

For each retained key, undo global rotation/translation and mirror, then undo splays in **ascending index order around the captured pivots**. Subtract pitch, Vec2 row/column offsets and cumulative stagger. Set cell rotation to entry heading minus global rotation minus signed cumulative splay. Accept these generated offsets on subsequent passes.

The complete independent numeric fixture is:

- Both matrices: **2×3**, pitch `(10,20)`, every cell active.
- Entry mirror: origin `(0,0)`, rotation0, MirrorNone; placement arrays empty.
- Current source, ordinarily materialized before entry: origin `(3,-2)`, rotation0, MirrorNone.
- Source row offsets: `[(0,0),(1,2)]`.
- Column offsets: `[(0,0),(3,4),(-2,1)]`.
- Staggers: `[1,2,-1]`; splays: `[90,-90,90]`.
- Column origins: `[Some(2,3), None, Some(17,-4)]`. The middle default pivot is `(10,3)`.
- Only source r0c2 has cell offset `(2,-3)` and rotation90; other cell offsets/rotations are zero. No assemblies, definition overrides or variants.
- Pair translation `(100,-30)`, rotation90.
- Retained mirror keys: r0c0 and r1c1. Remaining four keys are linked.
- Reuse accepted fixture definitions and non-pose data; preserve the existing tests.

The position arithmetic reduces to:

\[
S_0(x,y)=(5-y,x+1),\quad
S_1(x,y)=(x-8,y+8),\quad
S_2(x,y)=(5-y,x-13)
\]

Source world positions add `(3,-2)`. The pair maps world `(x,y)` to `(100-y,-30-x)`.

| Key | Source world pose | Required final mirror pose |
|---|---|---|
| r0c0 | `(7,-1)@90` | retained `(0,0)@0` |
| r0c1 | `(8,13)@0` | linked `(87,-38)@90` |
| r0c2 | `(8,5)@180` | linked `(95,-38)@-90` |
| r1c0 | `(-15,0)@90` | linked `(100,-15)@0` |
| r1c1 | `(9,35)@0` | retained `(10,20)@0` |
| r1c2 | `(-17,4)@90` | linked `(96,-13)@0` |

The final mirror frame is origin `(102,-33)`, rotation90, MirrorX. Its retained cells require:

| Key | Final offset | Cell rotation |
|---|---|---|
| r0c0 | `(101,37)` | `0` |
| r1c1 | `(-59,55)` | `-90` |

These are independently calculated literals. Use `1e-9` pose tolerance; verify exact retained identity/non-pose data and complete active-matrix consistency through disposable materialization.

Acceptance for this bounded batch:

- Fixture validation succeeds before the meaningful RED reaches today’s geometry/count restriction.
- Owners remain unchanged for all twelve affected source/mirror keys; unrelated objects remain outside that projection.
- Exact sorted changed IDs are the mirror matrix and the four linked mirror Parts listed above. Source, retained Parts and references stay unchanged.
- Second reconciliation returns empty IDs **and leaves the complete document unchanged**.
- Invalid associations, ownership, aliases, missing endpoints, nonfinite geometry, membership changes and unsupported representations reject atomically before materialization.
- Independent verification mutates inverse traversal order and protection of the second retained key separately.

Run focused tests, then affected `repo`, `lint`, `test`, `typecheck` and applicable boundary-parity checks under root’s gates. Assemblies, topology changes, deletion evidence, independent-object reconciliation and production activation remain excluded; this does not close full106.

Process note: the fact lookup completed in one released window. Numeric reasoning required no further root window or retrieval.

---

## Appendix: symbol, cache and source-revision crosswalk

This appendix supplements the preceding plan, which is preserved verbatim.

- Lane: `/home/chris/01_Projects/ts-boardstudio2/.worktrees/lane-mirror-core`.
- Verified clean branch and HEAD during the bounded heading lookup: `lane/mirror-core`, `0ea2411de21b58e177e5cd06bf1f1b1c76ab8f46`.
- The geometry capture below was taken at the older `5bcf4d381aa18cf609d96485e17464dd5fe3de2f` revision. Root confirmed the shared matrix helper source remained unchanged through checkpoint `0ea2411d`; it was reused without a live geometry recapture.
- The GitNexus alias `boardstudio-lane-mirror-core` reported behind two commits during the heading lookup. Its use was named-symbol location only, not a current-index or caller-completeness claim.

| Symbol / fact | Source and captured revision | Exact raw cache | Provenance cache |
|---|---|---|---|
| Private `matrix::location`: offset before cumulative stagger/splay, descending forward splay order, pivot encoding, mirror before global transform | `core/src/matrix.rs:284–342`, `5bcf4d381aa18cf609d96485e17464dd5fe3de2f` | `/home/chris/.cache/boardstudio-baselines/ticket106-planning/raw-location-59-5bcf4d381aa1-20261011T000258608Z.txt` | `/home/chris/.cache/boardstudio-baselines/ticket106-planning/raw-location-59-5bcf4d381aa1-20261011T000258608Z.json` |
| Private `matrix::column_rotation`: sum splays through column inclusive; negate for MirrorX or MirrorY, otherwise retain sign | `core/src/matrix.rs:344–355`, `0ea2411de21b58e177e5cd06bf1f1b1c76ab8f46` | `/home/chris/.cache/boardstudio-baselines/ticket106-planning/raw-column-rotation-344-355-0ea2411d-20261011T015220540Z.txt` | `/home/chris/.cache/boardstudio-baselines/ticket106-planning/raw-column-rotation-344-355-0ea2411d-20261011T015220540Z.json` |
| `matrix::set_matrix`: enabled/deleted gating, destructive old-ID cleanup, normalized persistence | Exact cached ranges `core/src/matrix.rs:744–769,784–815,912–934,961–979`, `5bcf4d381aa18cf609d96485e17464dd5fe3de2f`; this is a bounded capture, not the whole function | `/home/chris/.cache/boardstudio-baselines/ticket106-planning/raw-set-matrix-100-5bcf4d381aa1-20261011T000834433Z.txt` | `/home/chris/.cache/boardstudio-baselines/ticket106-planning/raw-set-matrix-100-5bcf4d381aa1-20261011T000834433Z.json` |
| Proposed private view-builder edits and focused tests | Sole proposed tracked file: `/home/chris/01_Projects/ts-boardstudio2/.worktrees/lane-mirror-core/core/src/mirrored_half/reconcile.rs`; baseline binding is clean checkpoint `0ea2411de21b58e177e5cd06bf1f1b1c76ab8f46` | The verbatim plan above is a proposal, not an implemented symbol or source capture | Root must allocate/review the bounded child and grant its write scope before dispatch |

Heading capture integrity:

- Complete `core/src/matrix.rs` SHA-256 at the verified heading lookup: `69fd15a07ae8a44c871616ac7e39c52ec71af96dbfefdef451f232b5de4e8440`.
- Numbered heading raw-capture SHA-256: `7bddd95a9a595125a7ec70b7e5145b7d63bec05f7e76034a308827d37a7db04c`.
- Heading visibility is private (`fn`, no `pub`); the inverse implementation must not widen it.
- Neither capture alone proves graph freshness or caller completeness.

Packet status: external planning evidence only. No child allocation, implementation, tracked edit, test run, index action, or additional investigation was performed while preserving this packet. No Fast/priority request or configuration change was made.

## Independent bounded verification

Verified source commit: `bdd56fd6`. Fresh all/staged graph analysis covered 21 symbols in one file and eight reported test-rooted flows with HIGH risk; full responses are retained beside the independent assessment. No partial/truncated response was reported; analyzer flow-discovery limits remain, so absent flows are not evidence of unused code.

Luna 6 high implemented the fixed-membership extension; a different fresh Sol 6.1 xhigh session `/root/verify132_fresh` independently verified the complete bounded scope. Final source SHA-256 `15cb447aac39128111e2c9a7b872cc3899f2a33b69e17dbed11adab418236e86`. All five original focused test bodies remain unchanged; focused 7/7 and final repo/lint/test/typecheck passed (83.6 s).

The separate inverse-order mutant compiled and failed the retained-offset assertion. Omitting the second retained key from previous ownership filtering compiled and failed with its missing-board-owner error. Each mutation restored exact bytes, touched the source and passed 7/7. A small verifier fix added reversed matrix-association endpoint guards and a compiling RED/GREEN regression to ensure rejection precedes materialization; final affected checks passed afterward.

Complete independent verdict, binding, raw graph, source diffs and logs: `/home/chris/.cache/boardstudio-baselines/ticket132/independent-fresh/independent-assessment.md`. The verifier reused the valid-fixture historical RED; it did not claim to rerun that baseline. The second-key mutation failed the owner guard before later dependency-snapshot assertions. No shared helper, visibility, API, schema, activation or generated-output change was made. Root integration/batch gates remain; parent 106 and 19 stay open.
