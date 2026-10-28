# Deferred items — implementation plan

Date: 2026-10-24. Research: [deferred-items-research.md](deferred-items-research.md).
Scope: the two items the layout-parity build deferred. Out-of-scope items
(YAML import/export, override precedence, orient/compounding/zone rotation,
default pitch) stay out.

## Decisions (user, 2026-10-24)

| # | Question | Decision |
|---|---|---|
| A1 | Aggregate representation | Extend `MatrixAnchor` with an aggregate arm; `follow` stays outermost; parts are single-key refs |
| A2 | Rotation averaging | Upstream naive `r` mean (parity over wraparound purity) |
| A3 | Degenerate policy | Refuse: empty `average` at validation, parallel `intersect` at evaluation, document unchanged |
| A4 | Nesting | Single-key parts only; no nested aggregates in the first build |
| A5 | Aggregate UI | Visual canvas multi-Pick for parts, plus list editing and N part links in Objects/Relations |
| N1 | Kernel risk | Accept reconciliation-kernel change, proven by public-route tests |
| N2 | Parity depth | Full same-board mirror (all ticket-21 behaviors, native) |
| N3 | Confirmation UX | Reuse ticket 23's confirmation UI verbatim for native confirmed-detach |

## Workstream A — Aggregate anchors

Shape: `anchor.target: { single: {matrixId,row,column} } | { aggregate: { method, parts: [single…] } }`,
top-level `follow` unchanged. Glossary `Matrix anchor` entry needs a
`domain-modeling` update (singular key → single or aggregate target).

1. **Average tracer (RED/GREEN).** Public `Open`/`Edit` test: 1×3 matrix with
   an average aggregate of two earlier-matrix keys; move one part, dependant
   follows the mean. Seams: `core/src/model.rs` (shape),
   `core/src/matrix/evaluation.rs::order` (multi-target visit),
   `core/src/matrix/kernel.rs::compose_matrix_frame` (mean pose as `Key.pose`,
   naive rotation mean). No UI.
2. **Part-reference validation.** Atomic refusals, document unchanged:
   unknown part keys, empty part list, `intersect` with ≠2 parts,
   `ref`+`aggregate` coexistence. Seam: `anchors.rs::validate_target`
   extended to a part list, ticket-19 refusal style. One test per refusal.
3. **Evaluation order over part sets.** Dependency order over the union of
   part targets, independent of document list order; cycles through any part
   refuse naming the matrix. Tests: reordered lists evaluate identically;
   2-cycle and self-part refuse atomically.
4. **Intersection.** Kernel slice: two part poses → heading-axis intersection
   (upstream Y-axis-line construction), rotation 0; parallel lines refuse at
   evaluation without mutating. Pure line-intersection helper, unit-tested.
   Tolerance precedent: spec §10 (`1e-6` upstream / `1e-9` local).
5. **Lifecycle parity.** Compaction remaps every part ref; removal/shrink past
   any part detaches the dependant keeping its world frame (spec §4 rule).
   Seams: `anchors.rs` (remap/shift), `deletion.rs` (ticket-21 grouped-edit
   exclusion pattern).
6. **Linked reflection.** Part targets map through the pair correspondence by
   identity; relative-negate rules apply to the resolved aggregate pose, not
   per part. Seams: `layout.rs::sync`, `reflected`, kernel `FollowBasis`.
   Tests mirror ticket 20's (linked Add, non-origin-axis transport,
   actual-Local parts).
7. **Follow over aggregates.** `follow` filters the completed candidate as
   today; unfollowed components from world zero. Cover the partial-x-only
   upstream observation (`(14.330127,0,0)`) as a BoardStudio golden.
8. **Fixtures.** Upstream-pinned average + intersection goldens through
   `scripts/generate-ergogen-parity-fixtures.cjs` (spec §10).
9. **UI.** Inspector Anchor section: method/parts editing with canvas
   multi-Pick reusing the ticket-23 armed-pick pattern; Objects lists part
   links; Relations explains multi-contributor chains (spec §8). Core-facts
   only from this workstream; surfaces owned by tickets 23/24.

## Workstream B — Native matrix-level exception transitions

Target: full ticket-21 mirror on the native (mirrored-half) pair. Gate to
change: `reconcile.rs::validate_matrix_association` (`4644-4711`) plus call
sites (`4861-4862, 5129, 5168`); lifecycle in `mirrored_half/lifecycle.rs`;
projection skip + member accounting in `reconcile.rs` (`source_members`),
`mirrored_half/inverse.rs` + `hosted.rs` (ticket-20 native precedent).
Contract to mirror (spec §5): Local keeps world pose via independent
offset/rotation compensation only; carried Shift/Bend stay shared; Omitted
keeps a live logical pose; Rejoin adopts source and validates atomically.

1. **RED/GREEN tracer.** Native pair, matrix-level Local on the mirror
   matrix, then source `set-matrix` expansion; mirror keeps dimensions/pose
   with valid correspondences. Seams: `reconcile.rs` gate + lifecycle only.
2. **Native Omitted through resize.** Omitted mirror stays live-yet-absent
   through source expansion/shrink.
3. **Native Make-Local detach + Rejoin.** One-Undo detach keeping pose;
   Rejoin adopts source dimensions/layout/anchor keeping explicit child
   exceptions, atomic refusal on missing anchors.
4. **Native confirmed detach + source removal.** Refuse-with-proposal +
   confirmed variant in one entry (ticket-23 UI reused verbatim); source
   removal retains a complete Local matrix, retires only defunct links.
5. **Hardening.** Identity remaps, child exceptions, preview/cancel, exact
   one-Undo Redo, frame/Follow restrictions through both halves' edits —
   all via public `Open`/`Edit` routes (spec §11 seam rule).

## Parallel lanes

A touches `core/src/matrix/*` + the `MatrixAnchor` shape in `model.rs`; B
touches `core/src/mirrored_half/*`. Overlap is only the A1 shape change and
shared integration-test registration/baseline, so the workstreams run in
parallel with two sync points. Lanes 1 and 4 share one checkout because their
file sets are disjoint (`matrix/*` vs `mirrored_half/*`); a lane pair with
overlapping files must split into separate worktrees/branches instead.
Integrate via joint gate (never two writers on the same files).

- **Lane 1 — A eval/kernel:** A1 → A3 → A4 (kernel wiring) → A7. The chain:
  shape + multi-target visit, then ordering, then intersection wiring, then
  Follow. Sequential within the lane (each extends the last).
- **Lane 2 — A validation/lifecycle:** A2 → A5. Starts once the A1 shape
  lands; A2's refusal tests can be drafted in parallel with A1 since they
  target the already-agreed shape.
- **Lane 3 — A reflection/fixtures:** A6 (starts once A1 lands) → A8 (needs
  A4 + A7 behavior for goldens).
- **Lane 4 — B native kernel:** B1 → B2 → B3 → B4 → B5, one owner,
  sequential. All five share `reconcile.rs`/`lifecycle.rs`; splitting them
  across writers guarantees merge conflicts. B1–B2 double as the kernel-risk
  validation for the whole plan.
- **Lane 5 — A UI (A9):** starts once A1/A3 Core facts exist. The
  ticket-23 armed-pick reuse spike has no dependencies and can run anytime,
  including now.
- **Floating tasks (no dependencies, anytime):** the A4 pure
  line-intersection helper with unit tests; RED-test drafting for each lane's
  next slice while the current slice implements (tests target public routes
  that already exist, so they fail correctly without the implementation).

Sync points: (1) A1 shape merge — unblocks lanes 2 and 3; (2) per-lane green
gates before merge; (3) final joint full gate + single baseline update after
all lanes merge. Lanes 1+4 should start first (both validate the riskiest
kernel assumptions); lanes 2, 3, 5 follow at sync point 1.

## Conventions

- TDD RED-first, small slice commits, keep tickets claimed until integration.
- Quick + full gates per lane before merge; one joint full gate + baseline
  via `check.py test --update-native-baseline` (additions only, reviewed
  diff) after all lanes merge.
- `web/crates/layout/prototype/` untouched; no Ergogen/Node in normal tests.
