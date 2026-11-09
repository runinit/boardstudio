# 109: Durable mirrored-half exceptions and Unlink lifecycle

Status: resolved
Type: build
Blocked by: [108](108-reconcile-mirrored-outlines-assemblies.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

Implement durable exception representation and internal lifecycle helpers for Make local,
Omit, Rejoin and Unlink. Production operation dispatch is enabled only by 111.

## Approved contract and evidence

Root approved this technical ownership policy on 2026-10-10; ADR-0008/0010 and 19's existing
behavior stay unchanged. Source at `2478fb151`: `MirroredException` has no source identity;
`halves::validate_halves` requires even omitted geometry and disallows its correspondence.
`mechanical/features.rs::omit_member/rejoin_member` retain association while state changes.
`mechanical/edit.rs::dissolve_pair_ownership` materializes effective geometry/construction.

- Retain durable kind/source/mirror association while omitted geometry may be absent.
  Rejoin uses current source data; source removal cleans associations for objects actually removed.
  Existing v5 records need a safe compatibility path: retain optional-field defaults, do not
  infer ambiguous associations or silently link independent legacy objects. Report an unresolvable
  stored association before inventing a migration or changing format version.
- An exception affects the selected object and only the projection it owns. Matrix-owned
  cells/companions and circuit-owned members follow their owner's exception. Independent layout
  components, module connectors, joined host nets and unrelated correspondences keep their states.
  Store one owner exception, not implicit neighboring local exceptions. Preserve explicit child
  exceptions when the owner rejoins; removing an owner follows existing domain deletion closures.
- User-confirmed selective exceptions apply to individual screens/encoders and other independent parts, and to diode/LED companions under their ownership rules. Normal linked edits propagate; explicit MakeLocal keeps the selected visible part independently editable, Omit remains durably absent without resurrection, and Rejoin derives from the current source. Keep the rest of the pair linked and clean only targeted owned references. Users need not Unlink the whole pair for one-off changes/removals. Companion placement-mode controls in40 remain companion-only and do not restrict general part exceptions.
- Make local preserves current visible geometry; later source edits do not overwrite it.
  Omit removes only owned mirror geometry and stays omitted on later reconciliation.
  Rejoin clears only the selected exception, restores source derivation and retains item mode/tweak.
- Unlink materializes effective linked mechanical data using existing dissolution policy,
  removes the relationship, and preserves stored board data and visible placement.
- Refusals are atomic. Implement internal helpers/payloads now; defer public EditOperation
  dispatch and production validation-hook changes to 111. No visibility widening.

## Source-verified Unlink route and internal context

At committed `2478fb151`, the existing callable seam is
`mechanical::edit::apply(doc, board_contours, &MechanicalEdit)` (`mechanical/edit.rs::apply`).
Implement the native-half coordinator inside `core/src/mechanical/halves.rs`:

1. Resolve the exact requested pair and its recorded construction-source member; validate
   that association before dispatch. Do not dissolve a different pair inferred from a label.
2. Obtain the normalized built-in mounting-hole definition via existing
   `generators::catalogue() -> Result<Vec<PartDefinition>, String>`, selecting the exact
   generator source `ceoloide/mounting_hole_npth`. `footprints/src/definition.rs::catalogue_entry`
   normalizes it before returning. Propagate lookup/normalization failure atomically.
3. Construct an internal `MechanicalEdit` with target
   `MechanicalTarget::Instance { instance_id: member }`, change
   `MechanicalChange::UnlinkMirrorPair { instance_id: member }`, and
   `mounting_hole: Some(Box::new(template))`; call `mechanical::edit::apply(doc, &[], &edit)`.
4. This existing branch resolves the target and calls `unlink_mirror_pair`, which reaches
   private `dissolve_pair_ownership` and `project`. It does not consume board contours, so
   the empty contour slice is specific to this existing Unlink route, not a general shortcut.

Do not remove the pair or clear `halves` before dispatch. Do not call/expose the private
dissolution helper or duplicate its materialization. Discover the template only during
Unlink, not on every edit. No new public hardware context, stored template or visibility
change is needed. `clearance` requires a template whenever projected closure holes remain,
including Unlink: passing `None` is incorrect for nonempty linked closure mounts.
Confirm route freshness and bound impact on the coordinator/callable path before coding.

## Required nonempty-closure regression

Port these semantic obligations to independently specified minimal synthetic nonempty-closure fixtures. The historical Corne seed, geometry and pinned counts below are route evidence, not expected-value oracles; preserve nonempty materialization, exact membership/identity and refusal/preservation guarantees without copying demo data.

Use an independently authored minimal synthetic native pair in `core/tests/mirrored_half.rs`
with explicit nonempty linked closure holes on both boards. Specify the fixture’s own exact
identities and expected memberships; do not copy Corne geometry or its pinned hole counts.
Do not substitute an empty-hole pair.
Direct coordinator tests belong here; production JSON/Undo/Open cases are prepared in
115–117 and independently checked after 111 activation.

- Submit the internal equivalent of only `UnlinkHalves { pairId: "pair-right" }`, with
  context discovered internally. Compare full effective mechanical configurations before/after.
- Pair/halves disappear; both instances have local construction ownership. Stored member
  configurations materialize their four mounts and existing openings without visible movement.
- Preserve hole/definition ID sets, four holes per board/eight total, and exactly-once board
  memberships. Preserve independent parts, matrices/modules/nets, labels and flip state.
- Check changed IDs against the actual document diff, including pair removal and ownership.
- Add reuse-board coverage: four holes on one board remain four, with flipped geometry preserved.
- Mutation replacing internal template discovery with `None` must fail the nonempty-hole case;
  bypassing materialization must fail effective/stored configuration assertions.

Existing `mechanical/edit.rs::unlinking_keeps_geometry_and_localizes_ownership` demonstrates
the callable lifecycle, but does not replace this native-half/nonempty-context regression.

## File ownership

Exclusive during this slice: `core/src/model.rs` for durable optional association fields,
`core/src/mechanical/halves.rs`, `core/src/mirrored_half.rs` and private child modules,
exception/schema cases in the mirrored-half test files. `core/src/mechanical/edit.rs` and
`core/src/generators.rs` are read-only callable-route evidence; existing APIs suffice without
editing their visibility or lifecycle.
Any other schema/storage/migration path requires root assignment first.

## Test-first and acceptance

Updated 2026-10-12 after the schema decision in the ADR-0010 amendment (a correspondence is the
identity; an exception is a state overlay). Evidence is in the Resolution section below.

- [x] Local/Omit/Rejoin and schema validation tests fail for the expected missing behavior first.
      (The connector regression and the pin and copper behavior were red before their code; the
      validator unit tests and the electrical projection were written after their code, and the
      eight mutations below show every one of them fails when its code is removed.)
- [x] Selective independent-part and owned-companion changes/removals preserve the rest of the linked pair, visible Local independence, durable omission and targeted reference cleanup.
- [x] Local geometry and omission survive serde round-trip plus direct reconciliation; Rejoin
      reflects current source with retained modes/tweaks, unrelated and child exceptions intact.
- [x] Matrix/circuit owner exceptions affect only owned projections; joined nets, connectors
      and independent layout objects survive. Source deletion cleans only actual removed records.
- [x] Unlink follows the existing callable route with internally normalized mounting-hole
      context and passes the required nonempty new-board/reuse-board closure regressions.
      (Its tests were written after the code; see the limits.)
- [x] Existing valid v5 data remains valid; no inferred links or format-version change.
- [x] Omit removes the mirror footprint, private nets and owned copper only, frees the item's
      pins and locks mirrored items to the source's pins; Rejoin restores from the current source.

Independent verifier: disable omitted-absence allowance, durable association, local exclusion,
child-exception retention and mechanical materialization separately. Save/Open integration is 111.

## Resolution (2026-10-12)

Schema: a correspondence records source ↔ mirror identity in every state and an exception is a
`(kind, mirrorId)` overlay on it; `MirroredException.sourceId` was removed and an optional
`removedCopper` added. Old unassociated exceptions keep their meaning. See the ADR-0010
amendment. The mirror-only replacement-part behavior is split to
[109b](109b-mirror-only-replacement-parts-auto-pins.md).

Behavior verified in `core`. It landed on dev in `4cb850483`; re-verified 2026-10-12 against
`4a9b806a0` with the 139 lib `mirrored` tests, 26 `halves` tests and 46 integration
`mirrored_half` tests green:

- the lifecycle tests, including the previously red connector case, now clean only the selected
  connector reference (the module is kept with `connection` cleared, as deleting the connector
  clears it);
- Omit frees an item's pins and locks every mirrored item to the source's, removes private nets
  and exactly the copper that belonged to the item, and keeps the source half unchanged;
- Rejoin restores nets, pins and saved copper from the current source, restoring copper only if
  the pads are where they were, with a notice otherwise; a conflicting mirror-authored assignment
  is released; a Local controller keeps its mapped locks and assignments.

Eight isolated mutations were each killed and restored byte for byte: omitted-absence allowance,
durable association, Local exclusion, explicit child retention, Unlink template `None`,
bypassed mechanical materialization, skipped copper saving, skipped pin freeing.

### Honest limits

- The Unlink tests were added after its implementation. The historical probes 01–05 were invalid
  setups and probe 06 was a positive control; the missed test-first step is not relabelled.
- Copper is matched to an omitted pad only where a trace end or a via lies on the pad centre (to
  1e-6 mm), using `outline_controls::world` and the definition's pad positions. A trace that
  merely crosses a pad, or ends on a pad that is not an omitted item's, is not owned by it.
- Board nets are not re-projected by reconciliation in general (only embedded circuit nets are).
  Rejoin adds back the nets and pins of the parts it rejoins; it does not repair other drift.
- The notice a Rejoin raises is carried on the private transition output; surfacing it belongs to 111.
- The legacy unassociated-exception path in the hosted-circuit projection (`exception_target`)
  and the hosted omission baselines remain, because the kernel-only tests still exercise
  unassociated records. Removing them needs those fixtures converted first.
- An explicitly Omitted child cleared under a Local owner (a tombstone already in the owner's
  descriptor) has no dedicated test; the tested child-clear cases are Local children.
- The candidate had deleted the kernel test
  `membership_transition_expands_with_planned_pair_identities_and_references`; it was restored.
- Function keys of the electrical board still map part ids by substring replacement (as before);
  the new Omitted-item detection matches whole path segments.
- Not run: browser/WASM runtime, `release`, Save/Open and production dispatch (111).
