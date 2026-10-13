# 158: Retain a Local controller own-pin net through Omit

Status: claimed
Type: build
Blocked by: [157](157-classify-matrix-membership-by-identity.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Related: [147](147-mirror-keycap-settings-and-plain-nets.md), [111](111-integrate-mirrored-half-reconciliation.md)

## Observed regression

The public111 Local-controller fixture establishes an accepted Local controller with its own
mirror-authored pin on a corresponded net. Omitting the source net's last non-controller member
removes the mirror net despite that surviving Local pin. The public assertion fails before
controller Rejoin, so this is Omit loss, not the accepted linked-controller pin-loss behavior.
Evidence: main `.evidence/ticket157/activation-proof/public-after-oracle-fix.log/json` and
`.evidence/ticket111/integration/net-lifecycle-diagnosis.md`.

## Existing contract and scope

147 says Local parts retain their owned pins; ADR0010 retains a net while mirror-authored data
uses it. capture_omitted_copper currently excludes the mirror controller unconditionally from
still_used. Count its surviving pin as use when its Part association is Local, retaining the
existing linked-controller exclusion. Preserve the private-net predicate, circuit-use guard,
owner/board/correspondence validation and copper capture/restore semantics.

Own only lifecycle.rs and existing lifecycle/tests.rs. No visibility, schema, production routing,
new ownership/net policy or electrical resolver changes. Associated Local circuit owner/member
projection is a distinct subsequent preparation ticket, not part of this predicate correction.

## Acceptance

- [ ] A compiled private regression fails for Local controller net/pin/board registration loss
      during Omit before the fix, using a valid accepted fixture.
- [ ] Local controller own pin retains its mirror net and registration through Omit, repeated
      preparation/Open; controller Rejoin retains existing source-authoritative behavior.
- [ ] Linked controller pin remains excluded, while another surviving mirror-owned pin or circuit
      reference continues retaining its net. Malformed ownership is still refused atomically.
- [ ] The original public111 Local-controller Preview/Commit test passes unchanged, including
      accepted Preview isolation and existing Rejoin pin-loss assertions.
- [ ] Independent Standards/Spec review, meaningful compiled mutations, affected/full checks and
      native-baseline additions pass; root performs local merge and post-merge checks. No push.

## Comments

Claimed after157 fully integrated and post-merge repo/test/lint passed1847 native tests; dev
bookkeeping e03a3be0. Root owns integration/baseline/fullchecks. No human policy decision pending.
