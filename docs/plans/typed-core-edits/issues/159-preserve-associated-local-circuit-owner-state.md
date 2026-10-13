# 159: Preserve associated Local circuit owner data and inherited child state

Status: claimed
Type: build
Blocked by: [158](158-retain-local-controller-private-net-on-omit.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Related: [111](111-integrate-mirrored-half-reconciliation.md), [147](147-mirror-keycap-settings-and-plain-nets.md)
Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Observed failure and existing contract

The unchanged public 111 Local circuit case fails at the accepted ReplaceDocument boundary:
its Local owner overlay survives, but source projection overwrites its mirror host-net port and
netIds before Omit. The latest public suite is 61/62; this is its only failure. Evidence:
main `.evidence/ticket158/activation-proof/public-after158.log/json`.

Ticket 19 settles ownership: one owner exception; owned circuit members inherit its state;
independent joined host nets and unrelated correspondences retain their own states; explicit
child Local/Omitted exceptions survive owner Rejoin. Preserve the complete Local owner payload,
owned members, embedded definitions and private nets. Preserving only ports/netIds is incomplete.
The initial target-reader inspection suggested an inverse inheritance gap, but the real
mirror-side member MoveParts regression passes: hosted::member_state already enforces circuit
owner state on that route, and private net targeting has its own existing owner-state handling.
Retain those real behavior controls; change inverse production only for a demonstrated operation
gap, not an artificial direct Links::target classification assertion.

## Scope and seam

Correct existing private owner-state reads and forward projection. A shared private parent
helper may serve inverse, forward and settings callers; no public or member visibility widening,
new wire vocabulary/schema, general lifecycle engine or ownership/net policy is authorized.
Explicit child exceptions retain precedence. Keep linked source projection and Omitted closure
behavior; do not materialize inherited child exceptions.

Validate only associated native-pair Local target circuit identity, ownership and permitted
closure references. Joined host nets are permitted independent references, not circuit-private
owned nets. Reuse creation/lifecycle ownership conventions and exact board/identity helpers.
Do not impose a generic validator on unpaired or legacy circuits, or silently repair malformed
Local data. Prepared Open and ReplaceDocument must both enforce the scoped Local target check.
ReplaceDocument can skip every changed-membership matrix in reconcile_pairs, so run scoped
owner validation independently of that loop rather than forcing matrix projection.

Root assigns these paths as needed: core/src/mirrored_half.rs (private shared helper only),
core/src/mirrored_half/inverse.rs and its existing inverse/tests.rs or inverse/hosted_tests.rs,
core/src/mirrored_half/reconcile.rs and its inline tests,
core/src/mirrored_half/reconcile/prepare.rs and prepare/tests.rs,
and existing reconcile/settings.rs plus settings_tests.rs if inherited member settings/pins
need the same owner-state read. No other files or visibility grants. Root owns integration,
public 111 proof, baselines, mutations and full acceptance checks.

## Acceptance

- [ ] A compiled regression fails before production changes for associated Local owner payload
      overwrite; retain durable owner/child correspondences. Cover inherited child inverse
      mapping through real operations, preserving the existing green MoveParts behavior.
- [ ] Source edits preserve valid Local owner fields, owned member poses/properties, embedded
      definitions and private-net names/pins. Mirror-side inherited Local member edits stay local.
      Member keycap settings and mirror-owned member pins on plain host nets follow the same
      effective Local state; board-level settings and independent net policies stay intact.
- [ ] Joined host nets obey their own linked/Local/Omitted state; linked neighbors still update.
      Explicit child Local/Omitted states and owner Rejoin preserve the established precedence;
      no omitted child is resurrected and no inherited child exception is synthesized.
- [ ] Valid private prepared Open, repeat and source/mirror transactions preserve the Local
      closure. Linked and Omitted owner controls remain green.
- [ ] Malformed Local identity/ownership/reference cases refuse atomically, including duplicate
      owners, wrong board, missing/foreign members or member definitions, invalid net/port refs,
      and ReplaceDocument with all source matrices skipped by membership classification.
- [ ] The original public 111 Local circuit ReplaceDocument/Omit test passes unchanged; the
      accepted host-net binding is asserted before testing Omit lifetime.
- [ ] Independent Standards/Spec review, meaningful compiled mutations, affected/full checks,
      native baseline additions, local merge and post-merge verification pass. No push.

## Comments

Claimed after 158 resolved and post-merge repository/test/lint passed all 1,849 native tests;
MAIN bookkeeping `1ecc91b7`. Detailed read-only readiness and bypass audit are in main
`.evidence/ticket159/readiness-design.md`. The scope refinement resolves its earlier general
validator question: this is the existing native-pair Local contract, with no new owner decision.

The inherited-child inverse hypothesis was checked against an actual MoveParts operation and
found already correct (`.evidence/ticket159/green-inverse-local.log`). The direct internal target
classification probe was removed; it bypassed the existing hosted-state adapter and did not
represent a product failure. This preserves the required Local behavior without unnecessary
inverse changes. Owner-payload overwrite remains the compiled RED driving this preparation.
