# 139: Generalized mirrored constraint model

Status: resolved
Type: build
Blocked by: none
Parent: [19](19-reconcile-mirrored-halves.md)
Consumer: [114](114-mirrored-edit-compatibility-constraints.md)

## Approved scope

The user chose support through a separately scoped model extension on2026-10-11: mirrored constraint editing must support slanted mirror axes and relationships between linked and Local parts. Do not substitute permanent refusal for these requested cases or silently narrow parent19. This authorization covers necessary model/schema/API changes for this concern after a source-grounded design and explicit file ownership; it is not blanket visibility or unrelated configuration approval. Current106 correction and138 retirement continue independently.

## Design before implementation

LaneB owns bounded planning, implementation and one fresh independent Sol6.1 medium acceptance for each coherent implementation unit under standing lane authority. Reuse `/home/chris/.cache/boardstudio-baselines/ticket114-planning/decision-packet.md` and110 operation ledger; do not repeat the inventory. Establish the smallest coherent general representation for arbitrary reflection axes and constraints whose endpoints have different linked/Local ownership. Specify world/local frames, target/source direction, durable constraint identity/ownership and canonical edit semantics from either half. Include how MakeLocal/Omit/Rejoin/Unlink affects those constraints, valid Local independence, atomic missing-endpoint/cycle refusals and preservation of unrelated constraints. Do not assume that every mixed relation can be represented by a fixed transformed Offset.

Record an implementation-ready design with exact code/serialization/adapter/test paths, graph/source-bound impact, legacy compatibility and migration/version decision, literal geometry oracles, affected checks and bounded implementation units if necessary. Root reviews the design and assigns shared model/solver paths before code starts; no repeated human approval for the extension already chosen. Escalate only actual unresolved product semantics rather than private representation choices. No production activation outside111.

## Acceptance

Re-scoped 2026-10-12: this ticket closes on bullets 1 and 2, the private model, solver and
projection. The other criteria are not met here and are handed on (next section), with their
original text.

- [x] Reviewed source-grounded representation and endpoint/ownership semantics are explicit;114 consumes the accepted result.
- [x] Independently authored synthetic tests first fail for slanted-axis and mixed linked/Local cases; exact intended geometry and stable identities are asserted.
  - Honestly: RED-first is evidenced for the pure-math unit only (the 7/14 failures recorded
    under "Initial private math handoff"). The solver, binding and key/companion tests of the
    2026-10-12 work were written with or after the code; each is mutation-checked instead
    (`.evidence/139-mutations`, 8 of 8 killed, local and gitignored).

## Handed to 114, 115, 111

These criteria are open, unticked and unchanged in wording. Each moves where its seam is.

- [ ] Editing from either linked half respects the constraint while retaining unaffected linked
  relationships and deliberate Local behavior. Handed to **114/115**: the operation seam
  (110's canonical operation, composed by 115) is absent. 139 supplies the pure retune helper and
  the driving-part lookup it will call.
- [ ] Exception transitions, omission/reference cleanup, rejoin and unlink remain valid without
  resurrecting omitted parts or corrupting unrelated relations. Handed to **114/115**: 109 is on
  dev, so what is missing is the persisted `Constraint::Framed` arm to rewrite and the operation
  seam to rewrite it in one transaction; the design's transition table is the contract.
- [ ] Existing horizontal/vertical Mirror and Offset documents preserve behavior and remain
  loadable; serialization/version/migration choices are tested where required. Handed to
  **111**: the persisted arm and the CURRENT_VERSION change land together there (design,
  "Format version and migration"), so the migration is untestable before. What 139 did cover:
  the stored solver arithmetic is untouched, and the private serde types exist and are tested.
  What remains is listed in 111's "Inherits from 139".
- [ ] Canonical operation reaches apply and all required resolution stages; atomic refusal
  preserves the accepted document/history. Handed to **115** (transaction composition) and
  **111** (activation): the operation seam and history routing are theirs.
- [ ] Fresh independent acceptance reuses sufficient source-bound checks and targets concrete
  sensitivity gaps; complete graph delta and required native/WASM/affected checks pass. Handed to
  **111**, with the locked/conflicting-movement refusal fixture (design, "Independent synthetic
  geometry oracles") landing where movement can be expressed.

## Ownership and execution

Planning is read-only in the stable lane-mirror-core model/constraints/layout/mechanical/engine sources; exclude active106reconcile edits. Root owns map/ADRs/CONTEXT/goldens, index refresh and integration. Proposed production paths are not an edit lease until the design names and assigns them. Use the central workflow: free read-only commands and disjoint source work; one GLOBAL HEAVY slot for compilation/tests/browser/timing/refresh, whole bounded debugging/check cycles, one writer per worktree. Never stash/discard/reset/broadly stage/push/merge to dev or run release checks.

## Accepted design checkpoint (2026-10-11)

Root reviewed the completed Sol6.1 MAX read-only design at `/home/chris/.cache/boardstudio-baselines/ticket139-planning/design.md` (copied into the repository as [the design](../design/139-generalized-mirrored-constraint-model.md)), SHA256 `ca7cbd334e6b41c9fb9fbce1e87b7544c2c2b904030604b76fa3719785884274`, bound to lane HEAD22f31e218 and23 stable source hashes. Accepted live framed endpoints, arbitrary reflection line, stable constraint IDs/visible getters, shared authoritative driving lookup, canonical operation through apply and both resolvers, and format6 activated only with111. Rejoin source-wins retires only incoming Local-target constraints; Undo restores them, outgoing/unrelated relations survive. No unresolved product question or additional human approval gate remains.

After the current106 completed writer STOP and ONE fresh SolMEDIUM acceptance, B may prepare one coherent private vocabulary/math/projection unit: `core/src/constraints.rs` module declaration only; new `core/src/constraints/relation.rs` and `endpoints.rs`; `core/src/mirrored_half/derive.rs` only for necessary private math reuse after impact analysis. No model/lib/migration/layout/placement/production or browser edits in that initial lease. Shared solver/lifecycle/adapter paths remain sequenced with actual109/110/114/115/117 prerequisites; support and parent19 are still open. Root alone refreshes/indexes/integrates.

## Initial private math handoff

Isolated worktree `.worktrees/lane-mirror-constraints`, branch `codex/mirror-constraints-139`, HEAD `e3eeb1be5`. Same Luna high writer STOPPED and explicitly released HEAVY. Exactly3leasedfiles: constraints.rs SHA256 `5cb76d5e6d1fafbdeebdd510fb0ffdc363942cdc4311d19cfb473dc0bb8f79f2`, constraints/relation.rs `a21a3bbce29a828693dcb9364ac7a0d03fd8e26ae9f15729ef369f084df77c4b`, constraints/endpoints.rs `f61303a42770116a094da8e733860a69ec3094a7f1f38ad37c9f509197ce5f15`. Meaningful compiling pure-math RED7/14 intended failures→GREEN14/14; final Core Clippy/fmt/diff PASS, narrow justified private-unused expectations, no visibility change. Writer `implementation/assessment.md` binds raw evidence. ONE fresh Sol6.1 medium read-only acceptance active; root unique index/complete graph/commit remains pending. This completes neither139's later solver/lifecycle integration nor111format6 activation.

Initial private math unit's ONE fresh Sol medium read-only review completed PASS/STOP. Artifacts: `/home/chris/.cache/boardstudio-baselines/ticket139-private-math/independent-medium/independent-assessment.md` and `source-evidence-binding.json`. Three final hashes above verified; literal reflection/transported-line/independent-pair/mixed-frame/nearest-heading/wire-identity/finite guards reviewed. The14/14 GREEN predates only final unused-lint annotations, while final Clippy all-targets/fmt compiled the final bytes; evidence is not relabeled. Root unique source-bound index/complete graph/sourcecommit/integration gates remain pending, and this unit does not resolve the full139solver/lifecycle/activation scope.

Root completed those source gates and committed `4e55187d`, merged no-ff into integration at `5f6e9a12983448b3c3a72b694704ee939f47bfbf`. Unique `boardstudio-lane-mirror-constraints` analyzer succeeded in46s; wrapper exit1 reported the three dirty paths, while every reviewed/working/staged/indexed SHA256 matched exactly. Complete staged delta:50/50symbols,3files,MEDIUM,4affected private inverse flows; whole-index flow/call caps remain disclosed lower-bound coverage. Evidence: `/home/chris/.cache/boardstudio-baselines/ticket139-private-math/root-final-capture/{final-binding.json,graph-delta.json,staged.diff}` and `/tmp/boardstudio-139-private-math-index.log`. No new source edit or duplicate review. Coherent integration batch checks and integration index refresh remain pending alongside107; full139/111 remain open.

Integration cohort107/139 PASS at0bcc292412: default repo/tooling/lint/build/test/browser859.9s, explicittypecheck25.5s,72presentationtests/0failures. Accepted four source hashes unchanged before/after. Durable `/home/chris/.cache/boardstudio-baselines/cohort-107-139/integration-result.json` binds both raw logs and source bytes. This is integrated preparation evidence, not full19/111activation acceptance.

## Implementation progress (2026-10-12)

Private, unactivated units on branch `worktree-agent-a1258f0b7ce0babff`, all in `core/src/constraints/`:

- **143 handoff.** `ConstraintRelation::PartOffset` (offset turned by the source heading, inverse
  recovered from the target heading) and `ConstraintRelation::from_legacy`, the one compiler for
  Offset, PartOffset and H/V Mirror (vertical `c` is the line through `(c,0)` heading 90,
  horizontal is `(0,c)` heading 0).
- **Endpoint projection.** `EndpointFrame::{LinkedKey, LinkedCompanion}` with the design's
  literal, stabiliser, duplicate and tweaked companion rules, forward and inverse, pinned by the
  design table's oracles. Matrix keys and companions are bound from the live matrices (a
  companion's rule comes from its definition, correspondence mode and tweak, over the pair
  default) and measured against the key's resolved pose, so a cycle through the key is refused.
- **Prepared solver.** `resolve_prepared(doc, positions, framed)` is the one engine; stored
  records keep their exact arithmetic and production calls it with no framed records. Endpoints
  bind to the live pair (map direction from the construction source), targets are ordered by
  authority, and Local/Omitted mirrors, stale correspondences, derived-mirror `Part` endpoints,
  shared authorities, alias cycles and non-finite axes refuse atomically. An explicit framed
  relation outranks the implicit companion placement, as a stored one does.
- **Retune and lookup.** `retune_target` is the pure drag helper (design oracle: visible (12,1)
  gives authority (1.6,11.2) and offset (9,-3)); `constraints::driving_constraint` finds the
  one driving record through either half (crate-visible; 114/115 call it). It counts two stored
  records on one authority as a conflict, which the production solver still accepts.
- An unbound companion frame (`key: None`) refuses for the rules that read the key.
- Axis-aligned Mirror records compile to the stored arithmetic bit for bit (round 3, below);
  a slanted axis has no stored counterpart and uses the general reflection.

What remains is in "Handed to 114, 115, 111" above. The accepted design is in the repository at
`docs/plans/typed-core-edits/design/139-generalized-mirrored-constraint-model.md`.

## Outcome

Bullets 1 and 2 are met and the ticket is resolved; the rest is handed on above. Round 3
(2026-10-12, user-approved decisions) changed:

- **Exact Mirror.** `MirrorLine::apply` takes the stored formulas for a line at exactly 90 or
  0 degrees (position `2c - x` or `2c - y`, heading `180 - r` or `-r`, keeping a zero's sign),
  which is every line `from_legacy` produces. An exact-equality test against the stored
  `constraint_pose` covers both axes, several coordinates (including negative zero) and sources;
  slanted axes keep the near-equal pin. The earlier "not bit-exact" caveat is retired.
- **One companion map.** `CompanionRule::frame` states each rule once for `project` and
  `unproject`. A test-only window onto `derive::companion_assembly` pins the stabiliser,
  duplicate, tweaked and literal rules to it at three key headings (within 1e-9: the two compose
  the same rotations in a different order), and a repaired companion's persisted literal
  correspondence is pinned to derive's fallback path.
- **Unproduced companions refuse.** A key or companion of a disabled or deleted cell, or a
  companion whose key part is absent, is an explicit refusal, no longer an independent part.
- **`Side` enum** replaces the `"source"`/`"target"` string; the tolerant equivalence test says so.

Evidence honesty for bullet 2: RED-first was observed for the pure-math unit only (7/14 failures
under "Initial private math handoff"). The 2026-10-12 solver, binding and projection tests were
written with or after their code and are mutation-checked instead (`.evidence/`, local and
gitignored). Of the round-3 tests, the two that change behaviour were red first: the exact-Mirror
test failed on the general formula by one ulp, and the unproduced-companion test failed with
`Ok`. The parity and repaired-companion tests passed on first run, because they pin behaviour
that already agreed with derive; they are covered by the mutation run instead.

Deferred, with owners:

- The `(parts, constraints, assemblies, resolved, visiting)` argument clump in the solver and
  the test-only accessors on `ConstraintEndpoint`: **114** (its "Inherits from 139").
- ~~The hand-formatted key id at `mirrored_half.rs`~~: done in round 4. The three copies of
  `format!("matrix/{}/r{}c{}", ...)` in `mirrored_half.rs` now call `matrix::member_id`; the
  existing mirrored-half tests are the guard and nothing else changed.
- The locked/conflicting-movement refusal fixture: **115**. The migration candidate and the
  framed serde round-trip: **111**.

Round 4 (2026-10-12, review of round 3):

- **Companion arithmetic is bit-pinned.** Round 3's single affine form had changed operation
  order the framed record will persist (the tweaked heading order, a stabiliser's `-0.0`
  heading, a zero shift folded into literal positions). `frame` keeps each rule's reviewed
  order again (an explicit `Heading` and an optional shift), and
  `companion_rules_are_bit_pinned_to_the_arithmetic_that_was_reviewed` asserts bit patterns
  captured from 58e4f7bc4, forward and inverse, with negative zeros in the pose, key and tweak.
  It was red first (stabiliser heading). 111 inherits the duty to keep it pinned.
- **Parity test** uses `matrix::splay::rotate`; **key ids** use `matrix::member_id`.
- **Repaired companion, not a real repair.** The parity test still sets the persisted
  literal correspondence by hand: driving `materialise_missing_companions` needs the
  `synthetic_half` fixture and a full `create`, which lives in `core/tests/support` and is not
  reachable from a library unit test. The real repair is covered at the engine level by
  `core/tests/mirrored_half.rs` (the missing-companion test asserts the durable
  `literal-mirror` correspondence and the mapped mirror pose); this test pins what that
  persisted correspondence then means to the endpoint rules.

