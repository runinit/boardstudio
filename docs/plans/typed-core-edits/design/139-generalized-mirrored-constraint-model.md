> Repository copy of the accepted ticket 139 design, byte for byte below this line.
> Original: `/home/chris/.cache/boardstudio-baselines/ticket139-planning/design.md`,
> SHA-256 `ca7cbd334e6b41c9fb9fbce1e87b7544c2c2b904030604b76fa3719785884274`.
> Accepted by root 2026-10-11 (see ticket 139, "Accepted design checkpoint"). Ticket:
> [139](../issues/139-generalized-mirrored-constraint-model.md).

# Ticket 139: generalized mirrored constraints

Status: resolved (implemented and integrated via tickets 139, 114, 115, and 111).

## Scope and evidence boundary

- Source worktree: /home/chris/01_Projects/ts-boardstudio2/.worktrees/lane-mirror-core, branch lane/mirror-core.
- Planning started at 86821996751c8e1c80c1f5102c7c77c58f22c0e5; final stable source binding is 22f31e218cb820a6a4c525c32c5ec942e048b220. Root reported accepted corrective unit 0fc21ca5e, lane synchronization 22f31e218, and main integration 465137ee5.
- The final captured lane was clean. Source hashes below bind the actual files read. Model, solver, layout, mechanical, migration, and host source stayed unchanged across that synchronization; the inspector changed only its corrective E12 copy. A subsequent writer must inspect later overlap before taking a lease.
- Explicitly excluded from source reads and this design's leases: core/src/mirrored_half/reconcile.rs and core/src/matrix/deletion.rs. Current 106 corrective Parts/Layout work has priority. Existing accepted 106 behavior is an input, not reopened here.
- Reused /home/chris/.cache/boardstudio-baselines/ticket114-planning/decision-packet.md. Its fixed-Offset limitation, canonical-operation seam, and exact-ID removal findings are inputs; this packet does not repeat its inventory or the 106 audit.
- Read central agent-workflow.md, verification-process.md, lane-registry.md, tickets 139/114/110/19 and relevant 109/115/116 boundaries, ADR 0010 and ADR 0004. Applied codebase-design.
- Read-only source/graph work only. No tests, builds, browser runs, timing, index refresh, source/tracker/ADR edits, or extra agents. This file is the sole deliverable.

## Decision

Add one generalized, framed constraint form. Keep the existing Offset and H/V Mirror representations and their wire spellings. The generalized form combines explicit endpoint views with either an Offset relation or a reflection across an arbitrary line. Resolve it as a relation over live poses; do not reduce every linked/Local relationship to a fixed canonical Offset.

Use a small private constraint compiler/evaluator behind the existing constraint solver. It resolves endpoint ownership, folds aliases to authoritative parts, adds endpoint-owner dependencies, and evaluates the authored relation in the endpoint views. This gives the rest of Core one answer to “what drives this part?” and keeps reflection algebra, exceptions, and companion rules out of command/UI callers.

No new solver library, general affine-matrix wire format, constraint correspondence kind, pair-owned duplicate constraint list, public error hierarchy, or new edit-command family is required.

### Durable document form

Illustrative new JSON; field spellings are the proposed final wire contract:

~~~json
{
  "kind": "framed",
  "id": "constraint/mixed-7",
  "source": {"kind": "part", "partId": "local/P"},
  "target": {
    "kind": "linked-mirror",
    "pairId": "pair/A",
    "sourcePartId": "alpha/T",
    "mirrorPartId": "beta/T"
  },
  "relation": {
    "kind": "offset",
    "offset": {"x": 5, "y": -2},
    "rotation": 15
  }
}
~~~

Proposed Rust concepts, with names private unless they are fields of the serialized model:

~~~rust
Constraint::Framed {
    id: String,
    source: ConstraintEndpoint,
    target: ConstraintEndpoint,
    relation: ConstraintRelation,
}

ConstraintEndpoint::Part { part_id: String }
ConstraintEndpoint::LinkedMirror {
    pair_id: String,
    source_part_id: String,
    mirror_part_id: String,
}

ConstraintRelation::Offset { offset: Vec2, rotation: f64 }
ConstraintRelation::MirrorLine { at: Vec2, angle: f64 }
~~~

Serde uses the surrounding tagged-kind, kebab-case variant and camelCase field conventions. MirrorLine serializes as kind "mirror-line", at {x,y}, angle in degrees. Its angle is the direction of the line's tangent, counterclockwise in the same document plane as part.rotation. The finite anchor may be any point on the line; no artificial origin or slope singularity. Retain the authored numbers in the document; normalize angles only as needed for evaluation.

Part means an ordinary part, a source-authoritative part, or a Local part with identity view. LinkedMirror means the current linked counterpart of exactly that source part in exactly that native pair. The redundant visible mirror ID is deliberate: it identifies the authored visible endpoint and detects stale/reused correspondence instead of silently rebinding another part. Resolve the current mapping and effective state from the pair/109 association; never persist a frozen transform in the endpoint.

New commands normalize a Part endpoint naming an effective linked mirror to LinkedMirror. A loaded new framed record must have a consistent explicit binding; an inconsistent record is refused. Existing legacy Offset/Mirror records retain their JSON; their literal IDs are compiled into the equivalent live endpoint views when native ownership applies.

Constraint.id remains the single global durable ID. The record belongs to its authored visible target; the solver target can be that endpoint's different authoritative source. Editing through either half finds this same record. Do not clone a constraint on reflection, allocate an ID on counterpart selection, or introduce MirroredItemKind::Constraint.

Preserve Constraint::id(), source(), and target() as visible authored IDs, including mirrorPartId for LinkedMirror. Existing deletion/reference/UI code relies on those meanings. Introduce a separate private authoritative-target lookup; do not silently change the getters to canonical IDs. Add the read-only facade ProjectDoc::constraint_driving_part(&self, part_id: &str) -> Result<Option<&Constraint>, String> for browser adapters. It resolves ownership and the unique driving record without solving geometry, and returns a located error for inconsistent ownership/duplicate targets. Keep the compiler, inverse projections, and graph records private. This narrowly scoped new API is part of the already approved extension; root assigns its exact lease.

## Source-grounded seams and impact

The current implementation has these concrete seams:

| Existing seam | Verified behavior | Design consequence |
| --- | --- | --- |
| core/src/model.rs, Constraint around 735–783 | Offset and H/V Mirror only; source/target getters; ProjectDoc owns one constraints vector | Add the framed form at the schema boundary; preserve old getter and wire semantics. |
| core/src/constraints.rs, resolve / resolve_edit / resolve_with_positions | Both entry points share pose resolution; explicit constraints, assembly placements, DFS cycle detection, finite checks; writes after pose computation | Extend this engine with compiled endpoint views and dependencies. Keep one combined graph. |
| constraints.rs, constraint_pose around 106–137 | Offset is world translation plus rotation; V mirror is (2c-x,y), 180-r; H mirror is (x,2c-y), -r | Keep legacy branches exact; add line reflection and view/inverse-view composition. |
| core/src/lib.rs, edit around 518–558 and preview_edit around 644–665 | apply, resolve_edit, component synchronization, then resolve_edit again; each currently receives the command operation | Prepare one canonical operation and give that exact operation to apply and BOTH resolver calls. |
| lib.rs, MoveParts around 1216 | layout keys first; target Offset gets delta retuning; Mirror target refuses movement | Reuse a shared target-retuning helper for legacy and framed Offset. Do not change bulk-move ordering incidentally. |
| lib.rs, SetConstraint / RemoveConstraint around 1632–1653 | Upsert by ID; removal requires the exact existing ID | Reuse these commands. Canonicalization changes payload/endpoints, not ID/removal semantics. |
| core/src/matrix/layout.rs, set_component_pose / move_keys | Offset retuning and Mirror refusal also occur in key/companion paths | Route all these paths through the same compiled driving-constraint helper. |
| layout.rs, companion_mode / assembly_placements / has_linked_positions | Literal constraint lookup chooses constrained mode and preview/assembly behavior | Replace literal-only driving lookup with authority-aware lookup. Keep explicit-constraint precedence. |
| layout.rs, reconcile_constraint_keys | Reconciles matrix params and restores solved poses after generation | Validate the final materialized relation after reconciliation; never accept a half-updated projection. |
| core/src/mirrored_half/derive.rs | Shared independent pose and mode-specific companion derivation | Reuse these rules for live endpoint projection. Do not introduce a competing reflection rule. |
| core/src/mechanical/pair.rs and features.rs | Forward/inverse map and inverse_pose already exist | Reuse the pair's actual construction direction, including inverse_pose when construction source is the second member. |
| core/src/matrix/placement.rs, carry_authored_companions | Skips literal constraint targets | Use the shared driving query so framed targets stay constrained. |
| core/src/mechanical/edit.rs, dissolve_pair_ownership | Private Unlink ownership materialization, effective configs, pair removal | Invoke existing mechanical edit through 109; do not widen this helper's visibility. |
| core/src/migrate.rs / archive.rs / web host storage.rs | CURRENT_VERSION 5; common migration feeds archive and IndexedDB decoding | One format-6 migration; no parallel archive/host migration algorithm. |

Graph evidence used alias boardstudio-lane-mirror-core, query for concepts/flows, context for named functions, and impact upstream. Important reported walks:

| Target | Walk cap | Reported impact | Limit |
| --- | --- | --- | --- |
| resolve_with_positions | depth 4, limit 200 | 59 total, 2 direct: resolve and resolve_edit; 8 processes; CRITICAL | Bounded graph walk, not proof of every dynamic caller. Fresh CLI repeat agrees. |
| core lib apply | depth 3, limit 200 | 73 total, 7 direct including edit and preview_edit; 7 processes; CRITICAL | Affects accepted and preview command paths. |
| matrix::layout::resolve | depth 3, limit 200 | 70 total, 2 direct CoreEngine/apply; 9 processes; CRITICAL | Graph explicitly lost 57 receiver-typing calls; lower bound. |
| companion_mode | depth 3, limit 200 | 51 total, 4 direct; 10 processes; CRITICAL | Includes accepted 106 callers; no source audit of excluded files. |
| companion_assembly | depth 3, limit 200 | 45 total, 4 direct; 5 processes; CRITICAL | Creation and membership-transition paths share the helper. |
| migrate_value | depth 3, limit 200 | 21 total, 10 direct including migrate_json and host decode; CRITICAL | Zero reported process rows does not remove cross-boundary risk. |
| Constraint enum UID | depth 3, limit 200 | 0, UNKNOWN | Source search confirmed matches in solver, Core apply, layout, matrix, inspector and web layout; zero is unanswered. |
| presentation constraint_resolver | depth 3, limit 200 | 3 total, 2 direct; 1 editor process; LOW | Does not answer core/serialization risk. |

HIGH/CRITICAL impact was reported to root. No riskSharedAxes waiver. The graph's affected processes reach Core edit/preview, validation/contour/generator problems, accepted selection, mechanical/library/session paths and the browser boundary. Demo flows are dependency evidence only, never geometry oracles.

Final on-disk index metadata: lastCommit 22f31e218cb820a6a4c525c32c5ec942e048b220, indexedAt 2026-10-11T17:36:08.907Z, full content retention, 1,022 files / 23,628 nodes / 72,055 edges / 992 communities / 622 processes / 0 embeddings. The read-only refresh status said current. The MCP server retained an older cached “25 commits behind” warning after root refreshed the index. A read-only CLI impact against the current alias returned the same 59/CRITICAL resolver result without that stale warning. This is design evidence, not a clean implementation gate: a writer must use the root-current index, run fresh impacts for its exact edits, and bind fresh detect_changes evidence after editing. Root alone refreshes the lane index.

## Geometry and direction contract

All endpoint relations use the document's two-dimensional world plane: part.position and part.rotation. They do not use PCB-instance assembly pose, footprint-local coordinates, screen coordinates, or a new per-board origin. Offset.offset is a world vector, matching existing behavior.

For native pair mapping with rotation theta and translation t:

~~~text
A = R(theta) diag(-1, 1)
F(p) = A p + t
F^-1(q) = A (q - t)
~~~

Read construction_source, actual members, and the effective correspondence. If the construction source is the stored second member, use the existing inverse map; do not guess from left/right names, labels, or vector order. Current halves validation permits one native halves pair per board, so this change need not invent chained pair ownership.

A compiled endpoint has a visible ID, authoritative part ID, live projection kind, and the owner dependencies needed to evaluate that projection. For a source pose s and a target-authority pose t, the constraint equation is:

~~~text
view_target(t) = relation(view_source(s))
t = inverse_view_target(relation(view_source(s)))
~~~

This is directional: the source drives the target. “Edit either half” changes the authoritative edit location; it does not reverse the relationship or create a reciprocal equation.

For arbitrary-line reflection with anchor a, tangent angle phi, unit normal n=(-sin(phi),cos(phi)):

~~~text
p' = p - 2 n dot(n, p-a)
r' = 2 phi - r
~~~

Use the legacy H/V implementation unchanged for the old Mirror variant. Full pose headings remain full 360-degree headings; never identify a heading with heading+180 merely because the geometric line is unchanged. Domain projections can wrap as their existing derivation does. When an inverse projection has equivalent headings separated by 360, choose the equivalent nearest the previous authoritative heading, with a deterministic tie rule; do not create needless full-turn jumps.

Endpoint projection must match the actual item, including companion mode. With source key pose (K, k), companion pose (C, c), and pair (theta,t):

| Item/effective mode | Visible position | Visible heading |
| --- | --- | --- |
| Independent mirrored part | F(C) | theta + 180 - c, wrapped exactly as derive::independent_pose |
| Mirrored key | F(K) | theta - k |
| Literal companion | F(C) | theta - c |
| Stabilizer | F(C) | c; preserve the existing independent heading exception |
| Duplicate companion | F(K) + R(theta-2k)(C-K) | c + theta - 2k |
| Tweaked literal companion, tweak (u,v,w) | F(C) + R(theta-k)(u,v) | theta - c + w |

Use the current derive::companion_assembly behavior as the implementation oracle for domain policy, with independent literal tests below. Stabilizer treatment ignores companion mode/tweak as the existing helper does. Inverse companion projection requires the resolved source key pose; record that dependency rather than freezing its current value. Reuse 110's inverse/canonical ownership seam and extract private shared math from derive only if necessary. Do not expose domain helpers publicly for the convenience of a caller.

A native reused-board/one-PCB assembly does not automatically have a distinct mirrored part endpoint. LinkedMirror requires a real current correspondence and endpoint ID. Missing correspondence is a located endpoint error, not permission to fabricate parts or fall back to identity.

## Compiler, solver and canonical edit flow

### Compile once per resolution, with a stable operation

Proposed private records/functions are implementation choices, not existing APIs: PreparedConstraint, ResolvedEndpoint, compile_constraints, driving_constraint_for_part, retune_offset_target, and evaluate_relation.

1. Reuse 109/110 association lookup to classify effective Linked, Local, Omitted, source, and unrelated objects. Validate visible IDs, pair identity, source/mirror binding, effective state and finite relation values. Legacy records are compiled into views without rewriting their document shape.
2. Map every constraint target to its authoritative part. Reject two different IDs driving the same authority even when their visible target IDs differ. Keep the global ID and visible endpoint record.
3. Add explicit source-authority -> target-authority edges. Add every endpoint projection's owner-key dependency, and existing implicit key -> companion attachment edges. Explicit constraints retain current precedence over following-assembly placement; the additional projection-owner edges still apply.
4. Fold aliases before testing self-dependencies/cycles. A visible source and target can have different IDs and still be a canonical self-cycle. Use one DFS/dependency model for explicit relations, assembly attachments, and live endpoint projections.
5. Compute all poses first. Resolve endpoint owners before evaluating a live source view or inverse target view. Reuse the current resolver's assembly/constraint precedence, finite checks and failure shape.
6. Commit authoritative/Local poses and existing key/matrix reconciliation only after the graph has succeeded. Materialize linked views through the accepted derivation contract. Check the resulting final endpoint equation after reconciliation. An inconsistent result refuses the whole edit; do not add an unbounded fixpoint loop.
7. The existing second resolve after component synchronization remains necessary. It consumes the SAME prepared canonical operation as apply and the first resolve; do not normalize again against already changed geometry.

The private 110/114 canonical-operation preparation is the seam; 115 supplies the transaction boundary; 111 eventually activates it. Normalize source/target endpoint identity and MoveParts IDs/positions before apply. A MoveParts batch involving a key owner and companion must be normalized as one owner-before-dependent batch, using the owner intent for that batch. Never inverse-project every child against a stale pre-batch key independently. For owner motion induced by other constrained movements, the prepared dependency model must carry the live projection intent until that owner pose is resolved, then emit the canonical placement used by apply and both resolvers. This can be a private prepared-operation wrapper; it does not require a persisted/public EditOperation variant.

The prepared operation must therefore preserve both canonical authority IDs and any owner-dependent movement intent needed for assembly suppression/retuning. Do not solve once using canonical positions and then pass raw mirror IDs to resolve_edit. Do not mutate an Offset in both apply and resolve: retuning is an application action; resolution only evaluates it.

### Offset editing and counterpart identity

Keep SetConstraint as an explicit complete replacement/upsert by stable ID. Endpoint changes in that command are explicit retargeting. Position drags and inspector field edits retain the stored endpoint views unless the user actually retargets the relation. RemoveConstraint always removes the exact global ID; a counterpart adapter finds it through authority-aware driving lookup.

For Offset target movement, retune in the stored target view. Project the old and requested canonical target poses into that same view and add the visible-position delta to the stored offset. Retune relation.rotation only when the originating operation supplies a full heading; Position contains only id+at. Reuse one helper in Core MoveParts, layout::set_component_pose and key/companion movement.

Preserve existing bulk-move delta/order semantics: moving a source and its constrained target in one batch must not silently become a different “fit desired target against newly moved source” algorithm. Owner-dependent projection uses the prepared batch's resolved owner where needed. Existing locked/finite/multiple-selection conflict checks still apply.

Mirror and MirrorLine target position drags refuse as existing Mirror target drags do. Move the source, remove the constraint, or change the reflection relation through SetConstraint. This is an existing relationship rule, not a refusal to support mixed endpoints or slanted axes.

An inspector reached through either half displays and edits the one driving record. It must identify the record's authored source/target frame; it must not label a mixed relation's stored numbers as a universal canonical Offset. An equivalent selected-view relation can be shown only when it is actually representable in that view. For both endpoints under the same fixed reflection, Offset and MirrorLine can be transported algebraically; for mixed endpoint projections, preserve the explicit stored views. No silent new ID or default-zero Offset conversion.

## Exception and ownership lifecycle

Use 109's exact effective membership/exception closure, including durable omitted associations. Do not infer ownership from ID prefixes or a fixture's number of children.

| Transition | Required constraint effect |
| --- | --- |
| Make Local | Materialize current effective geometry first. For every endpoint whose actual visible part becomes Local, replace LinkedMirror with Part of that same visible ID. Keep ID, relation values and other endpoints. A source-owned constraint targeting the original source remains there and does not start driving the new Local object. |
| Local edits | Local geometry stops inheriting from the source/pair. A relationship explicitly authored to that Local part still applies in its stated direction; independence does not mean constraints are ignored. Later source/pair/mode changes cannot move it except through an intentional surviving relationship. |
| Omit | Remove constraints referencing an actually removed visible endpoint in the 109 closure. Keep source-owned constraints whose visible endpoints survive and every unrelated constraint. Preserve the durable omitted association; do not resurrect the removed constraint on reopen or Rejoin. |
| Rejoin | Source geometry wins. Retire an incoming constraint whose authored target is the rejoined Local part; otherwise it could drive the source instead of rejoining to it. Preserve outgoing relationships that target another surviving part and rebind their source endpoint to the live linked view. Preserve explicit child exception/tweak policy from 109/110. Undo restores geometry and the retired record together. |
| Unlink | Materialize all effective linked views. Convert LinkedMirror endpoints to Part of the same visible IDs, retaining ID, relation values and endpoint direction. Run existing mechanical ownership dissolution through 109. Do not clone source-owned constraints onto former mirrors. Preserve geometry, effective configs, Local/Omitted state outcomes and unrelated relationships. |
| Actual source/member deletion | Use the accepted deletion/membership closure. Remove only constraints whose visible endpoint is actually deleted; Local endpoints already rebound to literal IDs are independent. Do not broaden removal through canonical aliases alone. |
| Pair map or companion-mode change | Keep a live endpoint binding and its authored relation. Re-evaluate the graph with the new map/mode. A canonical source can move to satisfy an intentionally authored mixed relationship; a frozen copied offset cannot implement this. |

Rejoin retirement follows the already authorized source-wins semantics; it is not a new human approval gate. Preserving an incoming Local-owned target constraint by rebinding it to the canonical source would transfer Local authority into the source and can immediately contradict source-derived geometry. Retire that record within the Rejoin transaction. An outgoing relationship can survive because its target remains elsewhere, so rebinding its source does not override the rejoined geometry. Preserve unrelated records and restore retired IDs on Undo. No additional unresolved product choice remains in this design.

For Unlink, retain the existing internal normalization of the mounting-hole catalog/template used by mechanical::edit::apply and dissolve_pair_ownership. A future 109 fixture must contain its own nonempty closure holes, IDs and memberships; no Corne/demo expected geometry or historical 4/8 counts.

Open/ReplaceDocument repair belongs to 116. A valid binding with missing or altered generated mirror geometry can be source-authoritatively repaired with the prescribed warning before final endpoint validation. An edit that supplies a missing/omitted endpoint, wrong pair, stale binding, or unsupported effective state refuses atomically; it cannot create a missing endpoint as a constraint fallback. Keep load-repair sequencing in the designated cohort rather than independently activating it here.

## Atomicity and refusal contract

Use existing Result<_, String> conventions with constraint ID, endpoint side and pair/part IDs in located errors. No new dependency or error hierarchy.

Refuse empty/duplicate constraint IDs; missing visible/canonical endpoints; incorrect pair correspondence; LinkedMirror bound to Local/Omitted state; non-finite relation/axis/pose values; duplicate canonical targets; canonical self-cycles; longer explicit/attachment/projection-owner cycles; conflicting/locked normalized movement; and a final relation inconsistent with reconciled/materialized geometry.

Failures leave accepted document bytes, revision, undo/redo and unrelated constraints unchanged. Preview must restore exactly after success and refusal. Current PreviewBackup's literal source->target closure/fast MoveParts path cannot cover alias and owner-dependent framed mutations: 117 must include the compiled authority/owner closure and changed constraint records, or use the already authorized complete draft/backup path for native prepared edits. Preserve the old fast path for unaffected ordinary operations. Do not mistake a successful native test for WASM preview validation.

## Format version and migration

Final choice: format 6, activated with 111's persisted enum/production-route activation. Prepare private serializable types and migration candidate now; do not make live dispatch accept the cohort early.

- Keep Offset and Mirror JSON byte shapes/field spellings, H/V formulas, IDs, coordinates and relation values. Do not rewrite them to framed merely because a new model exists.
- Add a v5 -> v6 migration that stamps the version only. Do not invent pair ownership, regenerate IDs, re-express offsets, or alter part geometry during raw migration.
- Existing valid independent and legacy LayoutMirrorLink documents retain behavior. Native legacy endpoint IDs are interpreted by the private ownership compiler; equivalent equations remain equivalent. A newly exposed actual canonical target conflict/cycle is a located validation refusal, not silently overwritten data.
- Keep current-version migrate_json text unchanged, following the existing contract. All 1–5 inputs go through the existing chain once; raw migration keeps unrelated/unknown fields as the current migration does. Typed round-trip handling of unknown fields follows existing serde policy; this packet does not claim arbitrary unknown fields survive typed serialization.
- Versions greater than 6 refuse; the current newer-version test input 6 becomes 7. Invalid new endpoint kinds/bindings and non-finite line values refuse.
- Archive decoding already calls migrate_json; host IndexedDB decode_document already calls migrate_value before typed deserialization and writes migrated data back. Reuse those paths. No IndexedDB schema or archive-container version change, TypeScript schema, or new storage migration engine.
- An older format-5 build must reject a format-6 document rather than silently dropping a new framed constraint. The format bump provides that guard.

If shared enum work is necessarily staged before 111, its production route must explicitly remain guarded as a temporary preparation state. Such an intermediate cannot be presented as completed support. Prefer introducing the persisted arm and CURRENT_VERSION change together at 111; private types/tests can land earlier.

Required compatibility tests: literal v5 H/V Mirror and Offset input -> v6 with identical IDs/values and matching poses; unrelated fields preserved by raw migration; no new links on independent documents; current-v6 text idempotence; v7 rejection; framed serde round-trip including both endpoint kinds and MirrorLine; malformed binding errors; archive and IndexedDB each exercise the common migration once. Update current-version assertions, not old-input fixtures to hide migration.

## Independent synthetic geometry oracles

Use tiny purpose-built fixtures, unrelated nondefault constraints, a linked neighbor and a Local neighbor. Assert IDs, membership and poses explicitly; never pin demo counts, import Corne, derive expectations by calling production reflection helpers, or regenerate the expected geometry from the implementation being tested.

Pair P uses theta=36.86989764584402 degrees, t=(20,-7). Its independent position map is the rational expression:

~~~text
F(x,y) = (20 - 0.8x - 0.6y, -7 - 0.6x + 0.8y)
~~~

Slanted relation H uses anchor (1,-2), tangent (0.6,0.8), angle=53.13010235415598:

~~~text
H(x,y) = (-0.28x + 0.96y + 3.2, 0.96x + 0.28y - 2.4)
~~~

Use a justified small floating-point tolerance for calculated positions/headings; compare durable IDs, relation values and unrelated records exactly.

| Case | Literal expected result |
| --- | --- |
| Ordinary slanted reflection, S=(2,5), heading 25 | T=(7.44,0.92), heading 81.26020470831196. |
| Independent mirror of S and T above | S_m=(15.4,-4.2), heading 191.86989764584402; T_m=(13.496,-10.728), heading 135.60969293753207. |
| Equivalent reflected slanted axis | Anchor=(20.4,-9.2), tangent=(-0.96,0.28), angle=163.73979529168804. Tests line transport without assuming H/V. |
| Local L=(3,4), heading 40 drives independent linked target with d=(5,-2), rotation=15 | Visible T_m=(8,2), heading 55; authoritative T=(4.2,14.4), heading 161.86989764584402. |
| Move L to (4,6), same relation | Visible T_m=(9,4); authoritative T=(2.2,15.4). Canonical relative offset changes from (1.2,10.4) to (-1.8,9.4), proving one fixed canonical Offset is insufficient. |
| Change only pair translation to (22,-8), restore original L | Visible T_m stays (8,2); authoritative T=(5.2,16.4). Catches a frozen endpoint map. |
| Linked source S=(2,5), heading 25 drives Local Q with d=(5,-2), rotation=15 | Q=(20.4,-6.2), heading 206.86989764584402. Moving S to (3,7) gives Q=(18.4,-5.2). |
| Inverse target edit of the mixed example to visible (12,1) | Authoritative target position=(1.6,11.2); stored offset=(9,-3); ID, endpoints and rotation=15 unchanged. Both resolver calls preserve the edit. |
| Key versus independent heading | Key S=(2,5), heading 25 has mirror (15.4,-4.2), heading 11.86989764584402; independent heading differs by 180. |
| K=(2,5), k=0; C=(4,8), c=30; literal companion | Mirror C=(12,-3), heading 6.86989764584402. |
| Same key/companion, Duplicate | Mirror C=(15.2,-0.6), heading 66.86989764584402. With key heading 90 and C world pose held fixed: (15.6,-7.8), heading 246.86989764584402 modulo 360. |
| Literal tweak (1,-2,7) | Mirror C=(14,-4), heading 13.86989764584402. With key heading 90: (11,-5), same heading. |
| Stabilizer with same C | (12,-3), heading 30 in every companion mode; tweak does not introduce a second rule. |

Add one simultaneous owner-key/companion inverse-movement test with explicit literal requested positions and both resolver calls. Its expectation must use the owner intent for the same batch, not the owner's previous pose. Include source-side and mirror-side drag requests that yield the same authoritative accepted result.

Lifecycle assertions use these existing literals:

- Make Local the mixed target while it is at (8,2): endpoint becomes Part of that visible ID. Moving L afterward moves the intentionally constrained Local target; authoritative T remains (4.2,14.4). A separate source-owned constraint continues driving the source and does not control that Local target.
- Rejoin a Local target after independently setting its source to (1,1): visible source-derived point is (18.6,-6.8); retire only the incoming Local-target constraint. Outgoing relationships survive/rebind. Undo restores the Local geometry and incoming ID.
- Omit removes exactly the constraints with deleted visible endpoints; reopen/Rejoin does not resurrect them. Source-owned and unrelated nondefault constraints remain equal by value.
- Unlink keeps IDs, authored values, visible endpoint direction, world geometry and effective config; endpoint kinds become literal. Include a nonempty independently owned mounting-hole closure, tested by IDs and memberships.
- Refusal fixtures: missing endpoint; stale correspondence; two visible targets sharing one authority; direct canonical alias cycle; two-node cycle; companion-key projection cycle; locked/conflicting movement; and non-finite slanted axis.
- Each refusal compares accepted serialization, revision, undo/redo and preview restoration. Each success checks an unrelated constraint's full values, a Local neighbor and an unrelated linked object.
- Legacy v5 H/V/Offset fixture, new v6 framed fixture, current-text idempotence and archive/host round-trips are separate from generated geometry tests.

First meaningful failing tests should demonstrate slanted reflection and mixed-endpoint impossibility in the old model, then inverse editing/lifecycle. A small deliberate wrong-map/key-owner/second-resolver mutation can establish test sensitivity if the initial failure is inconclusive; do not add automatic audit layers.

## Exact production/adaptor/test boundaries

These are proposed leases, not granted leases. Root must assign exact files after reconciling current 106/109/110/114/115/116/117 work.

| Path relative to lane root | Change and owner boundary |
| --- | --- |
| core/src/model.rs | Shared framed schema and endpoint/relation concepts; persisted arm at 111. One root-assigned schema writer. |
| core/src/constraints.rs; new core/src/constraints/relation.rs, endpoints.rs, tests.rs | Private relation algebra, view compiler, authoritative driving lookup, dependency graph and retuning. Existing solver is shared/high impact. |
| core/src/mirrored_half.rs; new core/src/mirrored_half/constraints.rs | Private 110/114 canonicalization and 109 lifecycle adapter. Do not route production commands before 111. |
| core/src/mirrored_half/derive.rs | Only an actually needed private reuse extraction; no competing formulas. No lease on active reconcile.rs. |
| core/src/mechanical/halves.rs | Reuse accepted 109 effective associations; coordinate any required accessor with its owner. |
| core/src/matrix/layout.rs | Authority-aware constrained mode, movement retuning, placement/preview closure, final relation verification. Coordinate 106 before any edit. |
| core/src/matrix/placement.rs | Authority-aware carry_authored_companions constraint check. |
| core/src/lib.rs | Prepared operation and shared Offset retuning seams; BOTH resolve calls; preview/transaction work belongs to 115/117; production dispatch/Open activation to 111/116. |
| core/src/migrate.rs | Prepared v5->6 migration/tests; CURRENT_VERSION and dispatch activated with 111. |
| core/src/archive.rs and web/crates/host/src/host/storage_lifetime_tests.rs | Relevant current-version assertions and shared migration round-trip tests; existing production decoders reused. |
| new core/tests/mirrored_constraints.rs | Minimal public-operation synthetic integration tests when activation permits; private tests before activation belong beside private modules. |
| core/tests/key_move_assemblies.rs and existing mirrored-half tests | Run affected checks; edit only if a specific regression needs them and root has cleared F138/106 overlap. |
| web/src/presentation/layout_component_edits.rs | Stable-ID set/remove through driving lookup; mixed-source eligibility and stored-view payload, not a fabricated Offset. |
| web/src/presentation/inspector/layout_component_inspector.rs | Exhaustive new-kind values/display, frame meaning and source/target editing; preserve accepted E12 corrective copy. |
| web/crates/layout/src/matrix_relations.rs | Exhaustive new kind and shared driving-target query. |
| web/crates/layout/src/objects/layout_align_controller.rs | Replace literal-only driven eligibility where native framed targets apply. |
| web/crates/layout/src/objects/existing_half.rs | Preserve legacy link guards while acknowledging framed driving ownership. |
| web/src/presentation/layout_component_inspector_tests.rs; web/src/presentation/layout_remainder_tests.rs; web/crates/layout/src/matrix_relations/tests.rs | Targeted synthetic adapter coverage and compilation. |
| core/tests/support/boundary.rs; web/crates/host/src/boundary_parity.rs | Shared synthetic Core/archive request transcript; final expected golden updates belong to root at activation. |

Read-only/reuse boundaries unless a concrete regression proves a change: mechanical/pair.rs, features.rs, mechanical/edit.rs, matrix.rs literal deletion getters, archive/host decoder algorithm, scripts/check.py and crate/configuration files. Keeping visible getters avoids unrelated deletion rewrites. No source/ADR/map/CONTEXT edit is part of this cached design. Root owns ADR0010/addendum, wayfinder/ledger, goldens, index and integration.

### Bounded implementation units and dependencies

Units are coherent writer scopes, not automatic new tickets, new reviewers or a review per row.

| Unit | Scope | Depends on / completion |
| --- | --- | --- |
| A: relation vocabulary/math | Private serializable endpoint/relation types, arbitrary line algebra, legacy equations and v6 migration candidate tests | Can prepare before ownership work; no format/dispatch activation. |
| B: endpoint projection | Live binding validation, source direction, independent/key/companion forward/inverse projections and driving lookup | A plus accepted 109/110 association/domain contracts. Pure projection work can proceed while lifecycle integration waits. |
| C: prepared dependency resolution | Compile authority graph, owner dependencies, canonical-target conflict/cycles and one retuning helper; owner-before-child MoveParts intent | A+B; private entry point/tests only. |
| D: lifecycle/transaction integration | 109 MakeLocal/Omit/Rejoin/Unlink endpoint transformations and 115 prepared operation used by apply and both resolvers; 117 restoration closure | Accepted 109/110/114 plus C; exact leases coordinated. |
| E: adapters/compatibility completion | Shared driving query, exhaustive browser consumers, stable ID/payload and common migration round-trips | A–D interfaces fixed; root assigns browser paths and visibility. |
| 111 activation | Persisted arm/version 6, actual dispatch/Open/ReplaceDocument/preview/history/boundary enablement and cohort checks | All 105–117 designated gates including 116 repair and 117 history/performance; root owns final integration. |

Keep each writer on its narrow group with the repository's Luna writer policy and fresh Sol6.1 independent acceptance for completed coherent work. Root determines whether adjacent private units share one acceptance; do not generate a reviewer per ticket or unit by default. Main remains root-only for integration/document/index/golden work.

## Check and acceptance plan — not executed here

- For implementation, run the smallest new synthetic regression RED for the expected old limitation, implement, then GREEN. Use cargo test -p boardstudio_core --locked with the concrete module/test filter. Run affected existing constraint, key assembly, mirror lifecycle, migration and preview tests once after the relevant change.
- Run affected python3 scripts/check.py steps: repo, lint, test and typecheck for shared model/solver plus browser consumers; build where the affected step requires it. Do not substitute native tests for WASM compilation.
- The existing typecheck command builds boardstudio-web for wasm32-unknown-unknown with --no-default-features --features page --bin boardstudio-web. Keep the repository command rather than an invented shortcut.
- Native boundary: cargo test -p boardstudio_core --locked --test boundary_parity.
- Shared browser boundary: wasm-pack test --headless --chrome web/crates/host --locked --features page --lib -- boundary_parity:: (the final token is the Rust test filter).
- Host storage/browser checks use the existing check.py browser step, whose mounted host tests are filtered to host::storage::. Presentation changes require both typecheck and browser. Add the independent framed/storage case to the actual mounted suite.
- Reuse sufficient passing evidence across the cohort, bound to exact HEAD/paths/hash. Fresh independent acceptance focuses on direction, arbitrary line, live/inverse mixed relation, companion owner dependencies, stable IDs, lifecycle, both resolver calls, and atomic preview restoration.
- Writer pre-edit graph impacts are required for existing symbols. Fresh final detect_changes(scope all), complete/nontruncated, is required before commit; UNKNOWN/cross-language omissions require source confirmation. Root alone refreshes after accepted changes.
- Root runs the full repository check before a PR and the activation integration. 117 timing runs only in its exclusive heavy window. No release, dependency/config change or browser run was performed for this design. deny is needed only if dependencies actually change.

## Stable source SHA-256 bindings at 22f31e218

These hash the source files used, not generated expectations or an unseen later writer state.

| Path | SHA-256 |
| --- | --- |
| core/src/model.rs | d89fa24ba0aa03ed5e1ce8beadcef360cb86e7889b0f9c61326f4dd854710404 |
| core/src/constraints.rs | fa26f82db43decbe225be9eac37292fb9a4e6167ced7dcee10bc6409ea0ec26b |
| core/src/lib.rs | da6ec8555f8157029fcbadcc0135b3f164e89f03269824c0adae68661353140e |
| core/src/migrate.rs | f86875b4692ff54a7df752679921df536520b03116d48df3ba13c0108b6805a1 |
| core/src/mechanical/halves.rs | 710e82bcfac3fbc35992275e06059f57f801ca623ce75789561c89920ae5ca12 |
| core/src/mechanical/pair.rs | 250e5e3828624a52d4d4e8ecf4b1bfd151e26f54901aaf2b1ee915130fe556cd |
| core/src/mechanical/features.rs | 0e22550b0488054c765a6f591e559fe79b35315a6c5b811ce9026491ef8aa75a |
| core/src/mechanical/edit.rs | de073093bd9f8a68dfea4e5b03af43cc12db98fd9bcc8c26016c91590a60f02f |
| core/src/mirrored_half.rs | b9f4e8cf7216aa9c3f2723a75800e13ee5a778ce11a082e55b6074d7ffd0eb0f |
| core/src/mirrored_half/derive.rs | 767f4378313e2b32a313061e695aa60c997c5985654ebaae9e8da2d7dd2d5795 |
| core/src/matrix/layout.rs | 38e1bb663b890713fe95400e9715c4f10668ad4f3061a1ed39126b12d4658c16 |
| core/src/matrix/placement.rs | 341d79054cee9d3eacc19e1191957a9233cfa127e6883356aed266748e7f2a27 |
| core/src/matrix.rs | 69fd15a07ae8a44c871616ac7e39c52ec71af96dbfefdef451f232b5de4e8440 |
| core/src/archive.rs | 3ee94e9363144c4b549aae67bed7b6d42bb475a1f29f3a2c4a022824d67a3929 |
| web/src/presentation/layout_component_edits.rs | d88e16032939dd330761c42ad0bf55a729414dcab9f47a91022e91fff156a7ce |
| web/src/presentation/inspector/layout_component_inspector.rs | 6aabef4873cd3f4acc1db24028275b1fba2805db39e7f31bd92a0e91ca062ca7 |
| web/crates/layout/src/matrix_relations.rs | f151744d7ed3ebc7dd6fcff830588dc267cd0a666927aa9f9af812a4b6914797 |
| web/crates/layout/src/objects/layout_align_controller.rs | a957f6823a7862b425d8132dff43795a51e42c6d321b147f601d7e7f2b382c99 |
| web/crates/layout/src/objects/existing_half.rs | 14bb615d93dd286b8b367c38255c515f1e6b0436e2b86efcb0a93c3f6a44604e |
| web/crates/host/src/host/storage.rs | 4d9d5101e0ebcef217c5c6b497555a4dd36776126fb242f84e321ac35169df27 |
| core/tests/support/boundary.rs | 1527fcb452a3a4b7dd068b6c65be7ca3edd127499f93764090b9db99fd04d861 |
| web/crates/host/src/boundary_parity.rs | 02c4b4bf74198d63c7695d05520f20923203ba65bea274715bd4ff748e72cecc |
| scripts/check.py | e31c95a3591cb5bce2ccbab1a3a764e264e0595a8ff5bd10822b38034c7c0f22 |

The absent active 106 source hashes are intentional. This packet does not claim those files were audited. Rejoin retirement implements the already authorized source-wins rule. No unresolved product semantics remain; representation, axis convention, direction, ownership, migration and activation choices are settled here for root review and exact path assignment.
