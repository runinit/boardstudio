# 11: Decide mirrored-half rules for the new layout fields and anchors

Status: resolved
Type: grilling
Blocked by: [04](04-layout-kernel-order-of-operations.md), [05](05-this-column-splay-representation.md), [06](06-origin-edits-hold-keys-still.md), [07](07-matrix-anchors.md)
Map: [map.md](../map.md) · Decision: [ADR-0034](../../../adr/0036-matrices-follow-ergogen-point-layout.md)

## Question

Linked halves (ADR-0008, ADR-0010) must reflect every new field deterministically. The kernel
fixes the reflection rule: values are copied unchanged, and the mirror negates the local heading
([ticket 04](04-layout-kernel-order-of-operations.md#answer), decision 5). For each new field,
decide how `layout::reflected`, `mirrored_half/inverse` and `mirrored_half/reconcile` treat it,
and which Local or Omitted exceptions apply. The fields are:

- the `columnLayout` records (spread, stagger, splay, Column rotation, splay origin, offset);
- the `rowLayout` records (padding, Shift, Bend, offset);
- cell Shift and Bend;
- the matrix anchor and `Layout.matrixIds`.

Also decide:

- how a hold-still origin edit on column 0 changes the follower's matrix Origin so both halves
  hold still ([ticket 06](06-origin-edits-hold-keys-still.md#answer), decision 2);
- how an anchor maps to the partner's matching key through the Matrix correspondence or the
  Layout mirror link, and how a Local exception on an anchored matrix behaves
  ([ticket 07](07-matrix-anchors.md#answer), decision 6);
- how an edit on the mirror half inverse-maps each field back to the source, including detach
  and keep-pose;
- which refusals linked halves need, such as the existing refusal of a local Y mirror.

## Answer

Decided with the user on 2026-10-23 in four rounds; all 15 recommendations were
accepted. These are design requirements, not claims that the current code implements
them. The Comments preserve the rounds and the source-inspection limitations.

### Reflection, identity and inverse edits

Same-board linked Layouts and native mirrored halves use the same matrix/key
Make Local, Omit and Rejoin meanings. There are no per-column exceptions. Existing
companion modes, hardware choices, independent-part rules and constraint ownership
are not replaced by the key-layout reflection rules.

For a pair map `T(p) = R(φ) · (-p.x, p.y) + t`, use the exact point inverse and the
key-frame heading rule `r' = φ − r`. The rules for linked values are:

| Value | Forward reflection and inverse edit |
| --- | --- |
| Column Spread, Stagger, Splay, Column rotation, relative splay origin and offset | Copy unchanged. |
| Row Padding, Shift, Bend and offset | Copy unchanged. |
| Key Shift, Bend, independent offset and Key rotation | Copy unchanged; the kernel negates the whole local heading, not the stored Key rotation again. |
| Unanchored matrix Origin / Rotation | Map the Origin as a point; Rotation becomes `φ − rotation`. The inverse uses the exact inverse point map, not a second forward mapping. |
| Matrix local mirror | Toggle the None/X frames as today; refuse an independent Y mirror while linked. Independent matrices retain their Y mirror. |
| Anchored matrix Origin / Rotation | Reflect relative Origin X and relative Rotation; keep relative Origin Y. Resolve through the actual anchor frame, not an unanchored world-point formula. |
| Anchor target | Use the corresponding matrix and same physical key, remapped on compaction. Never pair matrices by list position or silently retarget a missing key. |
| Follow | Keep the chosen subset, with its source-world zero and axes transported through the pair map. |
| Layout `matrixIds` | Map membership by matrix identity/correspondence, preserving Local members and logical Omitted membership. Document list order is not anchor evaluation order. |

Ordinary shared edits made through either half resolve once to the authored values,
then reconcile the counterpart in the same Core transaction and Undo entry. Do not
apply compensation a second time on the follower. Local final-pose edits remain
local; carried layout edits still belong to their matrix. Existing refusals of
ambiguous simultaneous edits, conflicting ownership and constraint-driven anchor
keys remain in force.

### Follow and actual anchor poses

For an ordinary linked dependency, apply Follow to the source frame, then reflect
the completed frame. Do not zero coordinates independently in each board's world.
For example, Follow X off gives source X = 0; a reflection about X = 50 gives partner
X = 100, not X = 0. The rule includes slanted pair mappings and unfollowed rotation.

A dependent matrix follows its actual Local anchor key, as drawn. Its own values
remain linked; its placement difference is inherited from that explicit exception.
For this case, evaluate the partner's candidate frame using its actual anchor and
reflected relative values, and apply Follow in the transported source-world basis.
Equivalently, if `F` is the source Follow filter and `T` maps frame poses, the partner
filter is `T ∘ F ∘ T⁻¹`. This reduces to reflection of the completed source frame when
the anchor has no exception; it never substitutes an invisible source-derived key
for a visible Local anchor.

Matrix placement drags are constrained to the permitted Follow directions, including
the transported directions on the partner. Disable the matrix Rotate gesture with
an explanation when Follow Rotation is off. Key/row tools remain available.

### Local, Omitted and Rejoin

- **Local key:** retain its world pose through shared layout edits by adjusting only
  its independent offset and Key rotation. Do not compensate with carried Shift or
  Bend. Editing Shift/Bend at that key still changes shared layout and can move linked
  neighbours on both halves while the Local key stays fixed. Offset/Key rotation
  edit its independent pose. Explain the distinction in the Inspector.
- **Local matrix:** retain its own dimensions and layout when the source resizes.
  Making an anchored matrix Local detaches its anchor and preserves its world pose,
  reporting both changes in one Undo entry. It can be explicitly anchored again.
- **Omitted:** suppress inherited visible geometry, not logical anchor poses. Evaluate
  Omitted matrices and keys for anchors without materialising their inherited parts.
  Omit does not detach dependants or freeze the last pose. Explicit child exceptions
  keep their own state. Switching a Local key to Omitted replaces its Local state;
  its logical pose returns to the current shared layout. Preview/report any resulting
  movement of dependent matrices.
- **Resize:** Local/Omitted partner matrices do not block source resizing. Local
  dimensions/layout stay independent; Omitted geometry stays absent.
- **Rejoin:** adopt the current source dimensions, layout and anchor, preserving
  explicit child exceptions. Validate the complete resulting dependency graph
  atomically. Refuse creation or restoration of an anchor whose corresponding key is
  missing, for example in a smaller Local parent. Name the missing counterpart; do
  not silently resize/Rejoin the parent or choose another key.

### Keep-pose operations and representability

Column-0 hold-still writes its compensation into matrix Origin, as ticket 06 chose.
The follower gets the corresponding reflected Origin change as part of the same edit,
so both halves hold still. For anchored matrices, solve using their actual frames and
Follow directions; verify the promised poses on all affected halves.

If the required compensation is outside the permitted Follow directions, refuse the
hold-still edit or exact Align. Explain which Follow component must be enabled, or
offer explicit Detach. Never silently enable Follow, detach, or scatter compensation
over keys. Ctrl swing remains available because it does not promise hold-still. This
is the narrow qualification to tickets 06 and 08: Origin cannot translate a frame in
a coordinate that Follow discards.

Detach on a linked matrix normally clears both corresponding anchors and preserves
both poses. If their current poses cannot remain linked after detachment because an
anchor exception made them differ, show the necessary counterpart Local exception
before commit and ask for confirmation. Without confirmation, change nothing. Apply
the same rule when actual anchor-key/row/column/matrix removal triggers detachment.
Never silently jump geometry or create an exception; the confirmed edit is one Undo
entry, including the removal when applicable. Omit alone does not trigger this path.

### Actual source removal

Deleting a source matrix preserves its Local counterpart as a **complete independent
matrix**, with its dimensions, layout and keys, not loose flattened parts. Retire only
the defunct source correspondence; unrelated links remain intact. If the entire
source Layout disappears, preserve the surviving Local group as an independent
Layout. Linked counterparts follow deletion; Omitted counterparts stay absent and
their records are retired. A retained independent matrix cannot Rejoin a deleted
source or silently attach to a replacement. Apply the keep-pose rules to affected
surviving anchored dependants in the same transaction.

### Required implementation acceptance

Use the real prepared edit route, preview/commit and Undo/Redo, not only direct
lifecycle/kernel helpers, for these cases:

- Every new carried/independent field reflects and inverse-edits correctly, including
  unchanged stored Key rotation, translated/slanted pair maps and both linking systems.
- Multi-matrix membership, reordering and compaction preserve correspondence and
  anchor identity; no list-position pairing or silent retargeting occurs.
- Full/partial Follow, a non-origin mirror line, unfollowed rotation and actual Local
  anchors use the correct effective frames in both editing directions.
- Local keys retain world poses through shared layout changes; their Shift/Bend still
  carry, while independent pose edits remain local.
- Source resize with Local/Omitted partner matrices succeeds with the accepted state
  and dimensions. Source removal retains a complete Local matrix and valid Layout
  ownership, retires only defunct identity records, and preserves unrelated links.
- Omitted anchors keep evaluating; Local-to-Omitted changes the logical pose as
  specified. Rejoin preserves explicit child exceptions and refuses invalid anchors
  atomically, without revisions/history or unrelated changes on refusal.
- Hold-still covers column 0, Column rotation, Bend, anchors and both halves. Masked
  compensation/Align failures leave state unchanged; swing remains available.
- Make Local and Detach keep poses. An asymmetric detach/removal requires explicit
  confirmation of additional Local state; cancellation restores exactly and accepted
  operations create one Undo entry.
- Follow-constrained matrix drags/Rotate availability, linked Y-mirror refusal and
  existing cycle/constraint/ownership refusals remain consistent in preview and commit.

Verification for this decision: inspected production paths and existing tests, and
checked 1,000 seeded frame/inverse cases over all eight Follow subsets, plus numeric
counterexamples for partial Follow and asymmetric Detach. The decision-level check
and output are `.evidence/ergogen-layout-parity-11/check-frames.py` and `frames.log`
(gitignored). No Cargo builds or production runtime tests were run. Direct-test versus
prepared-route discrepancies are implementation risks to lock down with the acceptance
above, not claims of support.

## Comments

### 2026-10-23 — round 1 accepted

The user answered “yes to all” to these five recommendations. At that point the ticket
remained claimed while dependent anchor/exception questions were worked through.

1. Same-board linked Layouts gain the same per-matrix and per-key Make Local, Omit
   and Rejoin behaviour as native mirrored halves. There are no per-column exceptions.
2. A Local key retains its world pose when shared layout changes. Core compensates
   through its independent key offset and Key rotation, not carried Shift or Bend.
3. Local or Omitted partner matrices do not block source resizing. A Local matrix keeps
   its own dimensions and layout; an Omitted matrix remains absent. Rejoin adopts the
   source's current dimensions and layout.
4. For linked matrices, apply partial Follow in the source's frame, then reflect the
   complete resulting frame, including its source-world origin and axes. Do not apply
   independent partner-world zeroing. This also covers slanted pair mappings. Unlinked
   matrices retain ticket 07's ordinary world-zero behaviour.
5. Preserve the refusal of an independent in-place Y mirror on a linked matrix:
   “Unlink the halves before changing to a horizontal matrix mirror”. Unlinked matrices
   retain their Y mirror; there is no silent parameter conversion.

Source inspection confirmed that current Local key-Part handling can preserve world
pose, that the current source-membership-edit path refuses Local/Omitted partner
matrices, and that same-board Layout links lack native typed per-item overlays.
The resize finding is source-path evidence, not an executed runtime reproduction.

### 2026-10-23 — round 2 accepted

The user answered “Yes to all” to these four recommendations:

6. Making an anchored matrix Local detaches its anchor and keeps its current world
   pose. Report both changes in one Undo entry. It may be explicitly anchored again.
7. A linked dependent matrix follows its actual Local anchor key, as drawn. Its own
   parameters remain linked; the placement difference is inherited from the explicit
   anchor exception, not an automatic Local exception on the dependent. Edits use its
   actual anchor frame.
8. An Omitted matrix, like an Omitted key, retains logical layout poses for dependent
   anchors without materialising visible parts. Omit neither detaches dependants nor
   freezes the last pose. Actual removal still follows ticket 07's detach-and-keep-pose
   rule.
9. Refuse a column-0 hold-still edit or exact Align when its required compensation
   cannot be represented with the current Follow subset. Explain which Follow
   component must be enabled, or offer explicit Detach. Do not silently enable Follow,
   detach or scatter compensation across keys. Ctrl swing remains available. This is
   a narrow amendment to ticket 06's hold-still promise and ticket 08's Align inverse.

Numeric checks confirmed that a disabled Follow coordinate discards Origin
compensation in that direction, and that an ordinary independent partial-Follow frame
cannot always represent its formerly linked frame. These were mathematical checks,
not production runtime validation.

### 2026-10-23 — lifecycle verification caveat

Owner Rejoin's explicit-child preservation is covered by direct lifecycle tests,
and the production exception-edit route calls that lifecycle preparation function.
However, direct lifecycle tests are not proof of every production membership-edit
path: `owner_source_deletion_preserves_local_objects_and_retires_only_removed_sources`
retains a Local matrix descriptor, while the prepared membership route has separate
materialised-matrix and Linked-association checks, plus a paired-matrix removal path.
The exact production Local-owner removal result has not been reproduced.

Implementation acceptance must exercise the real prepared edit route for source
resize and removal with Local/Omitted partner matrices, including descriptor retention,
Local child geometry, correspondence retirement and anchored dependants. Do not cite
direct-kernel tests as delivered support for those combinations. No builds or runtime
tests were run during this verification.

### 2026-10-23 — round 3 accepted

The user answered “Y to all” to these five recommendations:

10. A Local key owns its final pose, not the matrix's carried fields. Its Shift/Bend
    edits still change shared layout and can move linked neighbours on both halves,
    while its Local pose stays fixed. Offset and Key rotation edit its independent
    pose. The Inspector explains the distinction.
11. Detach on a linked matrix normally clears both anchors and preserves both poses.
    If anchor-induced asymmetry means a counterpart must become Local to retain its
    pose, show that required exception before commit and ask for confirmation.
    Without confirmation, change nothing. The same rule applies when actual anchor
    removal triggers detachment. Never silently move geometry or add an exception;
    the confirmed edit is one Undo entry.
12. Refuse creation or restoration of a linked anchor when its counterpart key does
    not exist, for example because a Local parent matrix is smaller. Name the missing
    counterpart; do not silently resize/Rejoin its parent or choose another key.
    Rejoin uses current source values, preserves explicit child exceptions and
    validates the resulting dependency graph atomically.
13. Omit replaces Local rather than retaining a frozen Local pose. The omitted key's
    logical anchor pose returns to the current shared layout and keeps following it.
    Preview/report resulting dependent movement; other explicit child exceptions
    remain intact.
14. Matrix placement drags are constrained to the directions Follow permits; on the
    partner, these are the reflected source-frame directions. Disable the matrix
    Rotate gesture with an explanation when Follow Rotation is off. This does not
    disable key/row tools, and exact Align/hold-still retain decision 9's refusal rule.

### 2026-10-23 — round 4 accepted

15. The user answered “Y”: deleting the source of a Local matrix retains the complete
    counterpart as an independent matrix, not loose parts. Retire only the defunct
    correspondence; unrelated links remain. If the source Layout disappears, retain
    the surviving Local group as an independent Layout. Linked counterparts follow
    deletion; Omitted counterparts remain absent and their records are retired. A
    retained matrix cannot Rejoin a deleted source or attach to a replacement silently.

All branches are settled. The ticket is resolved as a design decision, not an
implementation claim.
