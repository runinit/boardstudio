# 109: Durable mirrored-half exceptions and Unlink lifecycle

Status: ready-for-agent
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

Use the existing Corne seed/create fixture in `core/tests/mirrored_half.rs`: a new-board
native pair has four linked closure holes per board. Do not substitute an empty-hole pair.
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

- [ ] Local/Omit/Rejoin and schema validation tests fail for the expected missing behavior first.
- [ ] Local geometry and omission survive serde round-trip plus direct reconciliation; Rejoin
      reflects current source with retained modes/tweaks, unrelated and child exceptions intact.
- [ ] Matrix/circuit owner exceptions affect only owned projections; joined nets, connectors
      and independent layout objects survive. Source deletion cleans only actual removed records.
- [ ] Unlink follows the existing callable route with internally normalized mounting-hole
      context and passes the required nonempty new-board/reuse-board closure regressions.
- [ ] Existing valid v5 data remains valid; no inferred links or format-version change.

Independent verifier: disable omitted-absence allowance, durable association, local exclusion,
child-exception retention and mechanical materialization separately. Save/Open integration is 111.

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
