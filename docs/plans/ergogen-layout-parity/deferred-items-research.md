# Deferred Ergogen layout-parity items — research and proposed plan

Research date: 2026-10-24. Scope: the two items the layout-parity build
explicitly deferred. Out of scope (background only, no plan): Ergogen YAML
import/export, Ergogen override precedence, `orient` / column-compounding
shift / zone-or-global rotation, default pitch. First-build parity claim and
deferral: [map.md](map.md#deferred-beyond-this-build),
[ADR-0034](../../adr/0034-matrices-follow-ergogen-point-layout.md) (amendments
of 2026-10-23), [ticket 10](issues/10-slice-the-build.md#answer).

## 1. Aggregate anchors (average / intersection)

### 1.1 Upstream semantics (first-party source)

Primary source is the pinned upstream checkout
`/home/chris/.cache/ergogen-ref/ergogen/src/anchor.js` at `8286e18f`
(v4.2.1, same pin the parity harness uses). Section references below are
to that file.

- An anchor carries at most one of `ref` / `aggregate`; both together throws
  (`anchor.js:96-98`: "Fields `ref` and `aggregate` cannot appear together").
- `aggregate` normalizes to `{method, parts}` with default method `average`
  (`anchor.js:112-116`). Unknown methods are asserted against the
  `aggregators` table, so only `average` and `intersect` exist upstream.
- `average` returns the arithmetic mean of `x`, `y` **and `r`** over the
  parts; zero parts returns `new Point()`, i.e. the world origin with zero
  rotation (`anchor.js:20-30`). Rotation averaging is a naive mean, not a
  circular mean.
- `intersect` requires exactly two parts and throws otherwise
  (`anchor.js:61`). Each part contributes its heading-rotated Y axis as a
  line: the local segment `[x, y-1000]–[x, y+1000]` rotated by `point.r`
  about the point (`anchor.js:36-46`; the 1000 mm offset is an arbitrary
  stand-in for infinity chosen because Maker.js cannot extend to infinity).
  The result is the first Maker.js intersection point with rotation forced to
  `0` (`anchor.js:63-73`). Non-intersecting (parallel) inputs throw
  (`anchor.js:65`: "The points under … do not intersect!").
- Each `parts[i]` is a **full recursive anchor** resolved against the same
  `points` table, `start`, and `mirror` flag (`anchor.js:119-124`), so parts
  can themselves nest `ref`/`aggregate`/`orient`/`shift`/`rotate`/`affect`
  chains, including array-form multi-step anchors.
- Aggregate resolution happens **before** `orient`/`shift`/`rotate`/`affect`
  on the outer anchor (`anchor.js:92-163` ordering: ref/aggregate block first,
  then the orient/shift/rotate/affect block). The running-order finding —
  fixed order `orient → shift → rotate → affect` regardless of source key
  order — is recorded in [ticket 02](issues/02-ergogen-anchor-and-mirror-semantics.md#answer)
  and the [anchor research](../../research/2026-10-22-ergogen-anchor-and-mirror-semantics.md)
  (§"Anchor fields and order").
- Aggregate parts see only **already generated earlier points**, same as
  `ref`: zones render in declaration order and a zone anchor sees points from
  earlier zones only
  ([anchor research](../../research/2026-10-22-ergogen-anchor-and-mirror-semantics.md),
  §"References, zone transforms, and mirror axis", citing
  `points.js:312-390`). Unknown part references assert
  (`anchor.js:101-107` for the `ref` base case, reused per part).
- Mirroring interacts per part: the `mirror` argument toggles `mirror_` on
  each part reference (`anchor.js:5-13`, `53-60`), and relative shifts on
  mirrored points negate unless `resist` is set
  ([ticket 02](issues/02-ergogen-anchor-and-mirror-semantics.md#answer)).
  BoardStudio deliberately has no `orient` and no `resist`
  ([spec §1](spec.md#1-scope-and-preserved-contracts)); mirrored-anchor
  reflection is instead fixed: relative Origin X and Rotation negate
  ([ticket 07](issues/07-matrix-anchors.md#answer), decision 6).

### 1.2 What BoardStudio built instead (and why aggregates don't fit it)

- Stored shape is single-key only:
  `Matrix.anchor?: { matrixId, row, column, follow }`
  ([spec §2](spec.md#2-persisted-shape-and-validation),
  `core/src/model.rs:408-414`). No aggregate field exists, and the map states
  this build "covers accepted parameters and single-key anchors, not complete
  Ergogen points coverage"
  ([map.md](map.md#deferred-beyond-this-build)).
- The glossary defines **Matrix anchor** as "a key of another matrix in the
  same layout" ([GLOSSARY.md](../../../GLOSSARY.md), "Matrix anchor"): a
  singular key, so an aggregate representation would need a glossary change
  via the `domain-modeling` skill (per the map's standing rule).
- Evaluation is single-target dependency order: `evaluation::order` does a
  DFS over each matrix's one `anchor.matrix_id`
  (`core/src/matrix/evaluation.rs:76-108`), with cycle refusal naming the
  matrix; the kernel consumes exactly one resolved anchor pose via
  `RawAnchorContext::Key { pose, follow, basis }` and applies
  candidate-then-`Follow` filtering
  (`core/src/matrix/kernel.rs:372-424`, `compose_matrix_frame`). Ticket 19
  owns this graph and states "No aggregate arm"
  ([ticket 19](issues/19-anchor-layout-groups.md), Outcome §).
- The single-key contract is load-bearing downstream: compaction remaps one
  `(row, column)` reference per anchor; removal/shrink detaches dependants
  keeping the world frame ([ticket 19](issues/19-anchor-layout-groups.md#answer);
  spec §4); linked reflection maps one target through the pair correspondence
  ([ticket 20](issues/20-reflect-linked-layouts.md#answer)); Relations shows
  one "Follows …" link per anchor ([spec §8](spec.md#8-inspector-toolbar-and-relations)).
  An aggregate anchor needs a defined answer in each of these consumers.

### 1.3 Degenerate cases upstream (each needs a BoardStudio policy)

| Upstream case | Upstream behavior | BoardStudio gap |
|---|---|---|
| `average` over zero parts | world origin, rotation 0 (`anchor.js:22-24`) | Silent origin anchor would look like a detach; needs refusal vs. explicit-origin policy |
| `intersect` with ≠2 parts | throws (`anchor.js:61`) | Needs a validation-time refusal naming the anchor |
| Parallel / coincident lines | throws "do not intersect" (`anchor.js:65`) | Needs evaluation-time refusal; result rotation is forced 0, which must compose with `Follow` rotation semantics |
| Unknown part reference | throws "Unknown point reference" (`anchor.js:101-107`) | Maps onto existing missing-target refusal ([ticket 19](issues/19-anchor-layout-groups.md#answer)), extended to N parts |
| `ref` + `aggregate` together | throws (`anchor.js:96-98`) | Needs a `deny_unknown_fields`-style shape refusal at validation |
| Parts in not-yet-evaluated matrices | impossible upstream (earlier-points-only) | Needs multi-target dependency ordering + cycle refusal over the part set |

### 1.4 Proposed implementation plan (TDD slices, owning seams)

Shape proposal (decision required, §1.5): extend the stored anchor with an
aggregate arm, e.g. `anchor.target: { single: {matrixId,row,column} } |
{ aggregate: { method, parts: [single…] } }`, keeping top-level `follow`.
`follow` (BoardStudio's `affect`) stays outermost per the fixed upstream
order (§1.1). Nested aggregate-in-part recursion is **not** proposed for a
first slice: parts are single-key references only, matching the existing
`MatrixAnchor` record and glossary.

1. **RED/GREEN tracer — average over two ordinary keys.** Fail a public
   `Open`/`Edit` test creating a 1×3 matrix with an average aggregate of two
   earlier-matrix keys, then move one part key and assert the dependant
   follows the mean. Owning seams: `core/src/model.rs` (anchor shape),
   `core/src/matrix/evaluation.rs::order` (multi-target DFS visit),
   `core/src/matrix/kernel.rs::compose_matrix_frame` (mean pose as the
   `Key.pose` input; rotation mean mirrors upstream naive mean). No UI.
2. **Part-reference validation.** Refuse unknown part keys, empty part
   lists, `intersect` with ≠2 parts, and `ref`+`aggregate` coexistence
   atomically with the document unchanged, reusing the ticket-19 refusal
   style (`core/src/matrix/anchors.rs::validate_target` extended to a part
   list). One test per refusal, all via public routes.
3. **Evaluation order over part sets.** Matrices evaluate in dependency
   order over the union of part targets, independent of document list order;
   cycles through any part refuse naming the matrix
   (`evaluation.rs::order`). Tests: reordered matrix lists evaluate
   identically; 2-cycle and self-part refuse atomically.
4. **Intersection.** Kernel slice resolving the two part poses to the
   heading-axis intersection with rotation 0, following upstream's
   Y-axis-line construction; parallel lines refuse at evaluation time
   without mutating the document. Seams: `kernel.rs` (+ a pure helper for
   the line intersection, unit-tested) fed by `evaluation.rs` part-pose
   resolution. Tolerance precedent: `1e-6` upstream / `1e-9` BoardStudio-only
   ([spec §10](spec.md#10-fixtures-and-bundled-examples)).
5. **Lifecycle parity for aggregates.** Compaction remaps every part
   reference to the surviving physical key; removal/shrink past any part
   detaches the dependant keeping its world frame (spec §4 rule, extended
   from single-key). Seams: `core/src/matrix/anchors.rs` (remap/shift),
   `core/src/matrix/deletion.rs` (grouped-edit exclusion pattern from
   ticket 21).
6. **Linked reflection.** Aggregate targets map through the pair
   correspondence by identity; relative-negate rules apply to the resolved
   aggregate pose, not per part (mirror of the whole candidate, matching
   ticket 20's transported-filter precedent). Seams:
   `core/src/matrix/layout.rs::sync`, `reflected`, kernel `FollowBasis`.
   Tests mirror ticket 20's: linked Add with aggregate, non-origin-axis
   Follow transport, actual-Local part keys.
7. **Follow/affect over aggregates.** `follow` filters the completed
   aggregate candidate exactly as today (kernel `compose_matrix_frame`);
   unfollowed components come from world zero. Cover the partial-x-only
   upstream observation (`(14.330127,0,0)`) as a BoardStudio golden.
8. **Fixtures + UI.** Upstream-pinned golden for average and intersection
   through `scripts/generate-ergogen-parity-fixtures.cjs`
   ([spec §10](spec.md#10-fixtures-and-bundled-examples)); Inspector Anchor
   section gains method/parts editing, Objects lists part links, Relations
   explains multi-contributor chains (spec §8; tickets 23/24 own those
   surfaces — this slice only supplies the Core facts).

### 1.5 Open questions (need a user decision)

1. **Representation:** extend `MatrixAnchor` with an aggregate arm as
   proposed, or a separate record type? Must `follow` stay outermost only,
   or may parts carry per-part affect?
2. **Rotation averaging:** keep upstream's naive `r` mean (with its
   wraparound artifacts, e.g. mean of 179° and −179° is 0°), or refuse
   `average` over keys whose headings differ by more than a tolerance?
3. **Empty/degenerate policy:** refuse empty `average` at validation, or
   accept upstream's silent world-origin? Refuse parallel `intersect` at
   evaluation (document unchanged), or detach keeping pose?
4. **Nesting:** are single-key parts enough for the first aggregate slice,
   or must parts support nested aggregates / chained anchors from day one?
5. **UI scope:** does the Inspector need a visual multi-key picker (canvas
   multi-Pick), or is a list of key references sufficient? How are N part
   links shown in Objects/Relations?

## 2. Native matrix-level exception transitions

### 2.1 What is proven vs. what is refused

- Same-board Layout-link lifecycle is fully proven through the actual
  prepared route: Local/Omitted resize independence, Make-Local detach,
  Local→Omitted reset, Rejoin adoption + atomic refusal, confirmed detach
  (`DetachMatrixAnchorConfirmed`), source-removal retention
  ([ticket 21](issues/21-exception-lifecycle.md#answer); spec §5).
- Native (mirrored-half) lifecycle is only part-level proven. Matrix-level
  Local/Omitted + membership-change transitions (expansion, removal) are
  **refused by the reconciliation kernel** — the documented residual in
  [ticket 21](issues/21-exception-lifecycle.md) (§Progress, "Residual", and
  §Answer "Known limit"): "native matrix-level Local/Omitted +
  membership-change transitions stay refused by the reconciliation kernel
  (`reconcile.rs:5168`) and belong to a follow-up". Native part-level
  lifecycle is already proven by "parent19" (ticket 19's group-level
  coverage); native origin-only edits with a Local mirror succeed.
- Repro (from ticket 21 Residual): matrix-level `set-half-exception
  state=local` on the mirror matrix, then source `set-matrix` expansion or
  `remove-matrix` — refused at `validate_matrix_association`.
- The refusing gate demands the transition matrix association carry
  `state == Linked` with `explicit_state == false` and no parents
  (`core/src/mirrored_half/reconcile.rs:4644-4711`); a matrix-level Local
  exception (`explicit=true`) therefore fails the gate at every call site
  that requires presence (`reconcile.rs:4861-4862, 5129, 5168`).
- Ticket 21 deliberately avoided the fix "to avoid risking the mature
  kernel" and names the required work: "validation + mirror-projection skip
  + member accounting" ([ticket 21](issues/21-exception-lifecycle.md),
  Residual §).

### 2.2 Owning seams for the follow-up

- `core/src/mirrored_half/reconcile.rs::validate_matrix_association`
  (gate), plus the retired-matrix and transition-context call sites above.
- `core/src/mirrored_half/lifecycle.rs` (native lifecycle; ticket 21's
  same-board hooks live in `core/src/matrix/layout.rs::sync`,
  `apply_exception_transitions`, `plan_linked_matrix_removal` — the native
  equivalents are the target).
- Mirror-projection skip and member accounting: `source_members` and
  surrounding membership kernel in `reconcile.rs`; native inverse/projection
  in `core/src/mirrored_half/inverse.rs` + `hosted.rs` (ticket 20's native
  mapping precedent).
- Same-board contract to mirror: Local keeps world pose via independent
  offset/rotation compensation only; carried Shift/Bend stay shared; Omitted
  keeps a live logical pose; Rejoin adopts source and validates atomically
  ([spec §5](spec.md#5-linked-groups-exceptions-and-inverse-edits)).

### 2.3 Proposed implementation plan (TDD slices, owning seams)

1. **RED/GREEN tracer — native Local matrix survives source expansion.**
   Fail a public-route test: native pair, matrix-level Local on the mirror
   matrix, then source `set-matrix` expansion; mirror keeps its dimensions
   and pose with valid correspondences (mirrors ticket 21's first slice,
   `linked_resize_keeps_local_partner_dimensions_and_poses`, but native).
   Seams: `reconcile.rs` gate + lifecycle only; no projection change yet.
2. **Native Omitted through source resize.** Omitted mirror matrix stays
   live-yet-absent through source expansion/shrink (mirrors
   `linked_resize_keeps_omitted_partner_live_and_absent`).
3. **Native Make-Local detach + Rejoin.** Making a native mirror matrix
   Local detaches its inherited anchor keeping pose in one Undo entry;
   Rejoin adopts source dimensions/layout/anchor keeping explicit child
   exceptions, refusing atomically on missing anchors (mirrors ticket 21
   slices 3–4).
4. **Native confirmed detach + source removal.** Detach needing a new Local
   exception refuses with proposal + confirmed variant in one entry;
   source removal retains a complete Local matrix, retires only defunct
   links, preserves unrelated links (mirrors ticket 21 slices 5–6). Native
   UI confirmation stays with ticket 23's ownership
   ([ticket 21](issues/21-exception-lifecycle.md), Ownership §).
5. **Member-accounting + projection-skip hardening.** Extend the
   ticket-21-style coverage to identity remaps, child exceptions,
   preview/cancel, one-Undo exact Redo, and frame/Follow restrictions
   through both halves' edits — all via public `Open`/`Edit` routes, never
   private-helper tests (per spec §11's seam rule).

### 2.4 Open questions (need a user decision)

1. **Risk posture:** the ticket-21 residual explicitly deferred this to
   avoid risking the mature reconciliation kernel. Is a kernel-flow change
   now acceptable, or should native matrix-level exceptions stay refused
   (documented limitation) until a larger native-lifecycle redesign?
2. **Parity depth:** must native match every same-board behavior (confirmed
   detach, Rejoin child-exception preservation, Omitted guards), or is a
   narrower slice (resize survival only) sufficient?
3. **Confirmation UX:** does native confirmed-detach reuse ticket 23's
   same-board confirmation UI verbatim, or does the native pair record
   need its own wording/flows?
