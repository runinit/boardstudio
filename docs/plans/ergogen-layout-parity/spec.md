# Ergogen layout parity — implementation contract

Status: specified; implementation pending.

This consolidates the accepted decisions [01](issues/01-name-the-carried-parameters.md#answer),
[02](issues/02-ergogen-anchor-and-mirror-semantics.md#answer),
[03](issues/03-parity-fixture-pipeline.md#answer),
[04](issues/04-layout-kernel-order-of-operations.md#answer),
[05](issues/05-this-column-splay-representation.md#answer),
[06](issues/06-origin-edits-hold-keys-still.md#answer),
[07](issues/07-matrix-anchors.md#answer),
[08](issues/08-uniform-pitch-consumers.md#answer),
[09](issues/09-inspector-and-toolbar-prototype.md#answer),
[11](issues/11-mirrored-halves-rules.md#answer),
[12](issues/12-relations-and-highlight.md#answer),
[13](issues/13-format-reset-and-recovery.md#answer) and
[14](issues/14-starters-and-demos.md#answer). Later qualifications in 11–14 apply
where an earlier formula or promise was less specific. Definitions live in
[GLOSSARY.md](../../../GLOSSARY.md#matrix-layout); authority remains in Core/Session.

## 1. Scope and preserved contracts

This build implements the accepted point-layout parameters and **single-key matrix
anchors**, not complete Ergogen points coverage. The user explicitly deferred
average/intersection aggregate anchors in [10](issues/10-slice-the-build.md#answer).
There is no aggregate representation, YAML importer/exporter, `orient`, `resist`,
column-level compounding Shift, or world-origin zone rotation in this build.

Deliberate differences remain: independent offsets add, rather than using Ergogen's
override precedence; default pitch is 19.05 mm. Keep in-place X/Y mirror, Column
rotation, hold-still origin edits, companion modes, constraints, identity, key sizes,
scan topology and named editing. Preserve the contracts in the
[dependents inventory](../../research/2026-10-22-matrix-layout-dependents-inventory.md),
not its superseded old formulas, migration suggestions or modifier names.

No production implementation, persisted-format bump or data deletion is performed by
this planning document. The throwaway prototype is design evidence, not code to merge:
branch `prototype/ergogen-parity-inspector`, commit
`44c80fa984035cf0113e9d7922ccc0bbb7710959`, variant D.

## 2. Persisted shape and validation

Evolve `Matrix`, not an editable zone tree. Replace its old parallel vectors with:

```text
columnLayout[c]: { spread?, stagger, splay, rotation, splayOrigin: {x,y}, offset: {x,y} }
rowLayout[r]:    { padding?, shift: {x,y}, bend, offset: {x,y} }
MatrixCell:     existing identity/state/assembly fields + shift? + bend?
Matrix.anchor?: { matrixId, row, column, follow }  // follow subset of X, Y, rotation
Layout.matrixIds: one or more matrix identities
```

`columnLayout.rotation` is the independent **Column rotation**, not another Splay.
Existing cell offset and rotation remain independent. Missing/default fields and
short layout lists supply defaults; default records need not be persisted. Absence
of Spread/Padding means the current pitch, not a stored copy of it. Relative splay
origin zero means the column base. All other new translations/angles default to zero; Follow defaults
to all three components.

Values must be finite. Lists longer than their axis counts are refused. Column 0
Spread and row 0 Padding are refused. Pitch must exceed edge gap. Spread/Padding may
be zero or negative; overlaps produce collision findings, not schema refusal. Growth
adds defaults, shrink truncates, and actual row/column removal removes its record
without hidden compensation. Explicit Spread/Padding stops following later pitch edits.

Every matrix belongs to at most one Layout; all logical members share its board.
Layout membership is nonempty, including logical Omitted members resolved through
their correspondence. Anchor targets stay in the same Layout. Keep coordinate/member
identity and explicit compaction remapping, not list-index matching or ID reuse to
retarget a reference. Existing unrelated assembly, hardware and authored data survive.

For same-board links, extend the follower Layout's existing `mirrorLink` owner with
explicit `correspondences` and `exceptions`, reusing the existing correspondence/state
record forms for its matrices and matrix members. Keep `sourceId`/`axisX`; do not create
a physical-board pair to hold this metadata. Core link creation populates identities
and validates uniqueness/ownership. Native halves retain their existing pair-record
owner. Local/Omitted overlays retain correspondence identity; a retained independent
matrix after source deletion has no fabricated source. This is a proposed v7 shape,
not a claim that the current `LayoutMirrorLink` already contains these fields.

## 3. One Core geometry module

Use one pure layout kernel for materialization, draft/actual scenes, frame publication,
inverse placement and parity tests. The interface receives matrix layout data and a
resolved anchor context (raw anchor-key pose and Follow basis, or world context).
It applies matrix Origin/Rotation/Follow once; callers must not pass an already
composed frame and accidentally apply them again. Exact Rust type names are an
implementation detail, not a second public compatibility interface.

For a column, let `b` be its base in unreflected matrix coordinates:

```text
b[0] = (0, stagger[0])
b[c] = b[c-1] + (spread[c] ?? pitch.x, stagger[c])
origin[c] = b[c] + splayOrigin[c]
```

Carry both base and origin through every preceding Splay. Turn this column's starting
frame about that carried origin by `splay[c] + rotation[c]`. Only Splay propagates to
later columns; Column rotation does not. Steps are not reinterpreted as offsets.

For each column, walk rows from 0 upward using a running frame `F`:

```text
r > 0: F = F · move(0, padding[r] ?? pitch.y)
shiftFrame = F
F = F · move(rowShift[r] + keyShift[r,c]) · turn(rowBend[r] + keyBend[r,c])
offsetFrame = F
key = F · move(rowOffset[r] + columnOffset[c] + keyOffset[r,c]) · turn(keyRotation[r,c])
```

The independent move/turn does not change the running frame. Disabled/deleted/Omitted
keys still contribute their logical stored layout; filter materialization afterward.
The local X or Y mirror reflects local position and negates the **whole** local
heading, including Bend and Key rotation. Preserve full headings, not modulo 180°.

### Anchor frame and Follow order

For an unanchored matrix, its frame is `move(origin) · turn(rotation)` in world space.
For an anchored matrix with actual final anchor-key pose `A`:

```text
candidate = A · move(matrix.origin) · turn(matrix.rotation)
frame = FollowFilter(candidate)
world key = frame · localMirror(key)
```

Follow filters the completed candidate's position/heading; it does not run before
the relative Origin/Rotation. Unfollowed components use world zero. On linked
partners the filter is transported as specified in §5, not reapplied in partner-world
axes. The anchor key includes independent offsets/Key rotation and any explicit Local
pose. Do not anchor to a substitute invisible linked pose when the actual key is Local.

## 4. Anchor graph, ownership and lifecycle

Evaluate matrices in anchor-dependency order, independently of document list order.
Refuse self-reference, cycles, cross-board/cross-Layout targets, missing counterpart
targets and constraint-driven anchor keys. Also refuse constraining a key which a
matrix anchors to. Lay out matrices before solving constraints; constraints referencing
anchored keys consume their final anchored poses. Preserve companion precedence.

Disabled/deleted/Omitted keys and Omitted matrices retain logical anchor poses without
fabricating visible parts. Actual row/column compaction remaps references to surviving
physical keys. Removing an anchor slot or matrix, or shrinking past it, detaches
surviving dependants and preserves their frames; never retarget to a reused coordinate.
The asymmetric linked confirmation rule in §5 also applies to such removal.

Core owns atomic Add anchored matrix, anchor change and Detach intents. Creation from
a key makes a 1×3 matrix in the same Layout at relative Origin `(0, -pitch.y)` and
reflects its counterpart when linked. Detach writes the current effective world frame
to Origin/Rotation and clears the anchor. Matrix dragging inverse-solves through its
actual frame and Follow permissions. No web-side whole-document geometry builder.

## 5. Linked groups, exceptions and inverse edits

Both same-board Layout links and native mirrored halves use the same matrix/key Local,
Omitted and Rejoin meanings, with no per-column exceptions. Match whole matrix groups
by identity/correspondence, not positions in `matrixIds`. Keep companion/hardware rules
separate from key tilt reflection and preserve existing conflicting-ownership refusals.

For a pair map `T(p) = R(φ) · (-p.x,p.y) + t`:

- Copy column/row layout values and cell Shift/Bend/offset/Key rotation unchanged.
  Toggle None/X local mirror; do not negate stored Key rotation a second time.
- Unanchored Origin uses the point map, Rotation becomes `φ − rotation`. Inverse
  edits use the exact inverse point map, not an assumed second forward mapping.
- Anchored relative Origin X and Rotation negate; relative Origin Y stays. Map the
  target to the corresponding key, then evaluate using that actual anchor frame.
- A linked Y mirror remains refused; independent matrices retain Y mirror support.
- Resolve shared edits once to the authored value, then reconcile in the same Core
  transaction. Moving both linked aliases ambiguously remains refused.

For ordinary linked anchors, filter the source candidate then reflect the completed
frame. For actual Local anchors, evaluate the partner candidate with its own actual
key and reflected relative values, then use the transported filter `T ∘ Filter ∘ T⁻¹`.
For example, Follow X off and a mirror line X=50 yield source frame X=0 and partner
X=100, not partner X=0. This includes transported rotation and slanted pair maps.

- A **Local key** keeps its world pose through shared layout edits; compensate only
  independent offset/Key rotation, not carried Shift/Bend. Its carried-field edits
  remain shared and may move neighbours. Explicit independent pose edits stay local.
- A **Local matrix** keeps its own dimensions/layout on source resize. Making it Local
  detaches its inherited anchor and keeps pose, with one Undo entry; it may re-anchor.
- An **Omitted** item suppresses inherited parts but keeps a live logical pose. Omit
  replaces Local state, not a hidden frozen Local pose. Report dependent movement.
- A dependent follows its actual Local key while its own parameters remain linked;
  inherited placement differences do not automatically make that dependent Local.
- Rejoin adopts current source layout/dimensions/anchor and keeps explicit child
  exceptions. Validate the resulting graph atomically. A missing key in a smaller
  Local parent is refused without resizing/Rejoining that parent or choosing another key.
- Shared Detach normally clears both anchors and keeps both poses. If that requires
  an additional counterpart Local exception, obtain explicit confirmation before any
  change. Cancel leaves the whole edit unchanged. Apply this to removal-triggered
  detachment too; never silently jump geometry or create an exception.
- Source resize is not blocked by Local/Omitted partner matrices. Source deletion
  retains a complete Local matrix, not loose parts, retiring only the defunct link.
  If the source Layout disappears, retain its surviving Local group independently.
  Linked counterparts follow deletion; Omitted records retire. No silent Rejoin to a
  replacement source. Preserve unrelated links and surviving anchored dependants.

The exact production membership paths—not only private lifecycle helper tests—must
prove these promises. Existing helper/production discrepancies are recorded in [11].

## 6. Named placement and exactness

Inspector fields, tools, preview/commit and nudges use accepted-state Core intents,
reusing the existing placement/splay operation seams. Key dragging writes only its
independent offset through the inverse offset frame. Authored companions carry;
constraints remain solver-owned. No second splay/mirror/Bend algorithm in the web layer.

Origin edits hold keys still by default. Let `Δ` change the stored relative origin
and `θ = Splay + Column rotation`, in unreflected storage coordinates:

```text
this column step compensation = (R(θ) - I) Δ
next column step compensation = (I - R(Column rotation)) Δ   // only if rotation != 0
```

For column 0 write the corresponding translation through matrix Origin instead of
inventing Spread. Use the actual mirror/anchor/Follow inverse. Compensation pins an
explicit Spread when written. A world drag/Pick must solve for the requested final
origin **after compensation**; it is not generally just an inverse incoming-frame
delta. Test nonzero θ and carried/mirrored frames. Reset to column base sets relative
origin zero with hold-still compensation.

Ctrl locked at pointer-down (or Ctrl-click Pick) swings keys and writes only the
origin. Fields/nudges always hold still. Alt means no snap. The column base is a snap
target; Ctrl-dragging onto it is the ergogen-style reset. One drag commits on release
as one Undo entry including all compensation/confirmed exception changes.

Core rigid-column translation backs Column Align, writing this column's step and a
compensating next step (column 0 uses matrix Origin). Do not use column offsets or
per-key offset scattering under Bend. Matrix Align uses its actual frame; Key/Component
Align uses the existing MoveParts semantics. Row Align remains unsupported.

An exact operation may not silently violate Follow permissions or explicit Local pose
ownership. Refuse a requested exact Align/hold-still result that cannot be represented
with the agreed writes; explain the restriction and the explicit action needed.
Never silently enable Follow, detach, Rejoin or scatter compensations. Ctrl swing is
not a hold-still promise. Ordinary matrix drags are constrained to permitted Follow
directions; disable matrix Rotate when Follow Rotation is off.

Convert offset to Stagger remains Y-only and pose-preserving. Offer it only when this
column's Splay, Column rotation and every key's carried Bend are zero. Keep Core's
availability helper authoritative; transfer into this Stagger and subtract from the
next. Nonrepresentable edits/refusals leave document, revision and history unchanged.

## 7. Scene and consumer contract

Publish per-key `shiftFrame` and `offsetFrame` as world origins plus mirror-aware axes.
Shift starts after Padding, before its own Shift/Bend; offset starts after that step
with its resulting heading. Padding uses shift-frame Y; independent moves/rotation
use the offset frame. Publish column base, relative-origin/custom status, Column
rotation and exact Spread/Stagger axes (Spread absent for column 0). Row gestures use
the dragged key's frames, not a pretend universal column-0 frame.

Core supplies typed contribution/source/dependency/exception facts for Relations;
presentation owns their grouping and wording. Cumulative Bend sums row/key Bend only.
Cumulative Shift transports each contribution through its running heading into matrix
axes; do not raw-sum differently oriented vectors. Stagger totals are before turns,
not final positions. Mixed values say Varies.

Draft, materialized and accepted scenes share the kernel. Keep logical invisible
contributors distinct from rendered member IDs. Pitch has only three relevant roles:
default steps, key/keycap/plate-opening size (pitch minus gap), and snap unit. Spread,
Padding, Shift and Bend move centres, not sizes or electrical coordinates. Preserve
pose-driven keycap/pair previews, outlines, CAD/PCB export, collision checks, scan,
keymap and companion identity; never replace their positions with `row * pitch`.

## 8. Inspector, toolbar and Relations

Use accepted variant D: parameter-specific tools and grouped Carried/This-only fields,
toggleable passive canvas handles, and one collapsible **How it is placed** chain.
Spread and Stagger have separate horizontal/vertical handles; no Spread handle on
column 0. Keep the Splay scope choice selecting Splay or Column rotation without
converting values. Active Origin/Splay can expose the origin when passive handles
are hidden. State named changes and default pitch badges clearly.

Passive hover must not replace its live SVG target or let hints intercept gestures.
Actual edit previews are amber, with pre-drag ghosts. Real pointer capture, release,
cancel/lost-capture recovery and exact Undo require browser input evidence, not only
dispatched synthetic events. Visibility is view-only, never document/history state.

Relations has How it is placed, **Independent adjustments**, and Authored constraints.
Use later rows, not above; origin, not pivot. Separate matrix **Inherited by…** from
direct **Follows… / Followed by…** anchor links and Follow subset. This-only means
within this matrix: independent offsets/rotations may still move anchored dependants.
Explain Local pose pinning, invisible contributors and constrained Follow directions.

Violet dashed reference hover/focus highlights the named contributors or receivers,
not a pretend movement simulation. Clear on leave/blur/unmount/context change/removal;
resolve current identities, never reused coordinates. Do not alter camera, selection,
visibility, document or history on hover. Do not resurrect Disabled/Deleted/Omitted
parts or hidden layers; explain unavailable highlights in text and permit logical
inspection without bypassing Omitted restrictions.

Click parameter entries to select/open their owner/tool; references navigate. Reset,
Convert and other writes remain explicit actions with accessible blocked reasons.
Cross-matrix selections have no fabricated common frame and get frame-free actions
only. Objects lists matrices flat in evaluation order with Follows detail.

## 9. Format, persistence and recovery

Write document **formatVersion 7**, retaining `boardstudio/v2`, IndexedDB version 1
and existing optional archive metadata. Validate every storage/list/load/import/Open
route before migration, typed interpretation or replacement side effects. Older and
unversioned inputs are unsupported, newer inputs need a compatible build, malformed
inputs fail validation. No migration, version stamp, legacy editor or tag-only repair.

Keep incompatible saved records/assets unchanged and visible alongside healthy records.
Offer JSON-only raw download (not an asset-complete archive), fresh New project and
guarded confirmed per-project Delete. No global data wipe or legacy archive exporter.
Preserve package integrity limits, hashes and path checks. Refused import leaves the
original file, accepted project, drafts, history and storage unchanged with no partial
asset writes. A valid replacement's failed save follows retained-result recovery,
not false success or replay.

Validate before switching; confirm draft discard, wait for owned saves/queued edits,
and retain save-only retry, explicit unknown-outcome recovery and stale-tab guards.
Publish only after storage commits. Successful replacement starts a fresh Editor
session/history; listing/refusal/cancel do not clear current state. No Undo-survival
promise across executor failure/reopen. Preserve active editors' offline assets and
delayed worker activation. [13] is the complete action/error contract.

## 10. Fixtures and bundled examples

Opt-in `scripts/generate-ergogen-parity-fixtures.cjs` pins Ergogen
`8286e18f18ac064aa6d2ebca9c449bf2f6ba4d37` (4.2.1), exact Maker.js/Math.js and a lockfile.
Use a cached external checkout and ignore install scripts. Commit upstream config,
equivalent new-model inputs, named expected poses, source version and mappings to
`(matrix,row,column)` including mirror names. Retain licence notices. Normal Rust/app
tests do not run Node/Ergogen. Compare upstream poses within **1e-6** mm/degrees;
BoardStudio-only regressions use **1e-9**. Do not fabricate upstream counterparts for
BoardStudio-only extensions or claim aggregate support from omitted cases.

Replace the old TypeScript-captured projection oracle, preserving extended-feature
scenario intent. Regenerate affected positive current-schema documents, boundary
scene/transcript and layout expectations deliberately, not by bulk version stamping.
Keep intentional old-version negative inputs for refusal. Preserve identity, changed-ID,
companion/constraint, persistence and outline contracts. Unrelated CAD/footprint/hardware
goldens change only for demonstrated dependencies, not snapshot-blessing convenience.

Rebuild exactly Split 42, Studio 60 and Macro 12 from matrix parameters, leaving blank
New project blank and saved user copies untouched. Every example stays synthetic,
geometry-only, 19.05 mm pitch, 1.05 mm gap and 18×18 mm caps. Studio/Macro preserve
their 5×12/3×4 poses via Row 2 Shift X=4.7625 only. Split uses two same-board linked
Layouts, each with 3×6 Main plus anchored 1×3 Thumb. Preserve left/thumb poses; approve
right-main Row 2 X −5 mm and Row 3 X −10 mm for exact reflection. [14] gives exact
origins, mirror flags, anchor coordinate and sparse records; do not substitute a new
hardware recipe. Keep exhaustive edge cases in tests, not default examples.

## 11. Delivery and verification

The build DAG is in [map.md](map.md#build-tickets). All build tickets use one coordinated,
**unshipped cutover lane**. A staged pure kernel is not production delivery. The live
field replacement is atomic across its callers; do not leave old-vector or singular
Layout readers, add compatibility aliases, ship feature-flagged duplicate engines,
or migrate old input to hide incomplete work. Until the complete cutover passes [27],
use isolated test stores/profiles only; do not promote a version-7 writer for user data.

Pre-agreed test seams are the pure kernel/fixtures ([04]), public Core/Session prepared
edits including refusal/preview/Undo ([11]), real BrowserStore/archive/session lifetimes
([13]), mounted controls/scene facts and trusted mouse gestures ([09], [12]). Add one
failing behavioral case, implement it, then take the next; no horizontal test dump,
private-helper success substituted for production support, or parallel UI oracle.

Focused native tests use actual Cargo package/filter names; new test names in tickets
are scenario descriptions, not claims of existing tests. Use the repository scheduler
for broader checks, preserving test-name guards and receipts:

```sh
python3 scripts/check.py quick --files <changed-paths>
python3 scripts/check.py test
python3 scripts/check.py browser --only <crate-names>
python3 scripts/check.py
```

Browser `--only` is a partial subset; native tests do not validate WASM presentation.
Rebuild packaged providers/page before trusted real-page evidence as README requires.
No overlapping manual Cargo/wasm-pack launches per target. Redirect long logs to
`.evidence/`; report observed scope, failures and unrun checks, not invented receipts.
Planning-time numeric harnesses and source inventories are not Rust/browser acceptance.

[11]: issues/11-mirrored-halves-rules.md#answer
[13]: issues/13-format-reset-and-recovery.md#answer
[14]: issues/14-starters-and-demos.md#answer
[27]: issues/27-end-to-end-acceptance.md
[04]: issues/04-layout-kernel-order-of-operations.md#answer
[09]: issues/09-inspector-and-toolbar-prototype.md#answer
[12]: issues/12-relations-and-highlight.md#answer
